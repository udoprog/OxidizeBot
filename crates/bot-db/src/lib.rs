#![allow(clippy::field_reassign_with_default)]

#[macro_use]
mod macros;
pub mod schema;

mod after_streams;
pub use self::after_streams::AfterStreams;

mod aliases;
pub use self::aliases::Aliases;

pub mod commands;
pub use self::commands::Commands;

mod matcher;
pub use self::matcher::{Captures, Key, Matchable, Matcher, Pattern};

pub mod models;

mod promotions;
pub use self::promotions::{Promotion, Promotions};

#[cfg(feature = "scripting")]
mod script_storage;
#[cfg(feature = "scripting")]
pub use self::script_storage::ScriptStorage;

mod task;

mod themes;
pub use self::themes::Themes;

mod words;
pub use self::words::{Word, Words};

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use common::models::TrackId;
use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, HarnessWithOutput, MigrationHarness};
use parking_lot::Mutex;
use thiserror::Error;

pub const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!("./migrations");

/// Database abstraction.
#[derive(Clone)]
pub struct Database {
    pool: Arc<Mutex<SqliteConnection>>,
}

impl Database {
    /// Find posts by users.
    pub fn open(path: &Path) -> Result<Database> {
        let url = path.display().to_string();

        tracing::info!("Using database: {}", url);

        let mut pool = SqliteConnection::establish(&url)?;

        let mut output = Vec::new();

        // Run all migrations and provide some diagnostics on errors.
        let result: Result<()> = {
            let mut harness = HarnessWithOutput::new(&mut pool, &mut output);

            match harness.run_pending_migrations(MIGRATIONS) {
                Ok(..) => Ok(()),
                Err(e) => Err(anyhow!("{}", e)),
            }
        };
        let output = String::from_utf8_lossy(&output);
        result.with_context(|| anyhow!("error when running migrations: {}", output))?;

        if !output.is_empty() {
            tracing::trace!("Migrations output:\n{}", output);
        }

        Ok(Database {
            pool: Arc::new(Mutex::new(pool)),
        })
    }

    /// Run a blocking task with exlusive access to the database pool.
    pub async fn asyncify<F, T, E>(&self, task: F) -> Result<T, E>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
        E: From<tokio::task::JoinError>,
    {
        let pool = self.pool.clone();

        task::asyncify(move || {
            let mut guard = pool.lock();
            task(&mut guard)
        })
        .await
    }

    /// List all counters in backend.
    pub async fn player_list(&self) -> Result<Vec<models::Song>> {
        use self::schema::songs::dsl;

        self.asyncify(move |c| {
            let songs = dsl::songs
                .filter(dsl::deleted.eq(false).and(dsl::played.eq(false)))
                .order((dsl::promoted_at.desc(), dsl::added_at.asc()))
                .load::<models::Song>(c)?;
            Ok(songs)
        })
        .await
    }

    /// Insert the given song into the backend.
    pub async fn player_push_back(&self, song: &models::AddSong) -> Result<()> {
        use self::schema::songs::dsl;

        let song = song.clone();

        self.asyncify(move |c| {
            diesel::insert_into(dsl::songs).values(song).execute(c)?;
            Ok(())
        })
        .await
    }

    /// Purge the songs database and return the number of items removed.
    pub async fn player_song_purge(&self) -> Result<usize> {
        use self::schema::songs::dsl;

        self.asyncify(move |c| {
            Ok(
                diesel::update(
                    dsl::songs.filter(dsl::played.eq(false).and(dsl::deleted.eq(false))),
                )
                .set(dsl::deleted.eq(true))
                .execute(c)?,
            )
        })
        .await
    }

    /// Remove the song with the given ID.
    pub async fn player_remove_song(&self, track_id: &TrackId, played: bool) -> Result<bool> {
        use self::schema::songs::dsl;

        let track_id = track_id.clone();

        self.asyncify(move |c| {
            let ids: Vec<i32> = dsl::songs
                .select(dsl::id)
                .filter(
                    dsl::played
                        .eq(false)
                        .and(dsl::deleted.eq(false))
                        .and(dsl::track_id.eq(&track_id)),
                )
                .order(dsl::added_at.desc())
                .limit(1)
                .load(c)?;

            let count = diesel::update(dsl::songs.filter(dsl::id.eq_any(ids)))
                .set((dsl::played.eq(played), dsl::deleted.eq(!played)))
                .execute(c)?;

            Ok(count == 1)
        })
        .await
    }

    /// Promote the track with the given ID.
    pub async fn player_promote_song(
        &self,
        user: Option<&str>,
        track_id: &TrackId,
    ) -> Result<bool> {
        use self::schema::songs::dsl;

        let user = user.map(|s| s.to_string());
        let track_id = track_id.clone();

        self.asyncify(move |c| {
            let ids: Vec<i32> = dsl::songs
                .select(dsl::id)
                .filter(
                    dsl::played
                        .eq(false)
                        .and(dsl::deleted.eq(false))
                        .and(dsl::track_id.eq(&track_id)),
                )
                .order(dsl::added_at.desc())
                .limit(1)
                .load(c)?;

            let count = diesel::update(dsl::songs.filter(dsl::id.eq_any(ids)))
                .set((
                    dsl::promoted_at.eq(Utc::now().naive_utc()),
                    dsl::promoted_by.eq(user.as_deref()),
                ))
                .execute(c)?;

            Ok(count == 1)
        })
        .await
    }

    /// Test if the song has been played within a given duration.
    pub async fn player_last_song_within(
        &self,
        track_id: &TrackId,
        duration: common::Duration,
    ) -> Result<Option<models::Song>> {
        use self::schema::songs::dsl;

        let track_id = track_id.clone();

        self.asyncify(move |c| {
            let since = match Utc::now().checked_sub_signed(duration.as_chrono()) {
                Some(since) => since,
                None => bail!("duration too long"),
            };

            let since = since.naive_utc();

            let song = dsl::songs
                .filter(
                    dsl::added_at
                        .gt(&since)
                        .and(dsl::played.eq(true))
                        .and(dsl::track_id.eq(&track_id)),
                )
                .first::<models::Song>(c)
                .optional()?;

            Ok(song)
        })
        .await
    }
}

/// Convert a user display name into a user id.
pub fn user_id(user: &str) -> String {
    user.trim_start_matches('@').to_lowercase()
}

#[derive(Debug, Error)]
pub enum RenameError {
    /// Trying to rename something to a conflicting name.
    #[error("conflict")]
    Conflict,
    /// Trying to rename something which doesn't exist.
    #[error("missing")]
    Missing,
}

#[cfg(tests)]
mod tests {
    use super::user_id;

    #[test]
    fn test_user_id() {
        assert_eq!("oxidizebot", user_id("@OxidizeBot"));
    }
}
