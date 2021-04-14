#[macro_use]
mod macros;
mod after_streams;
mod aliases;
pub(crate) mod commands;
mod matcher;
pub(crate) mod models;
mod promotions;
pub(crate) mod schema;
mod script_storage;
mod themes;
mod words;

use crate::task;
use crate::track_id::TrackId;
use crate::utils;
use anyhow::bail;
use std::path::Path;
use thiserror::Error;

pub use self::after_streams::{AfterStream, AfterStreams};
pub use self::aliases::{Alias, Aliases};
pub use self::commands::{Command, Commands};
pub use self::matcher::Captures;
pub use self::promotions::{Promotion, Promotions};
pub use self::script_storage::ScriptStorage;
pub use self::themes::{Theme, Themes};
pub use self::words::{Word, Words};

pub use self::matcher::Key;
pub(crate) use self::matcher::{Matchable, Matcher, Pattern};

use anyhow::{anyhow, Context as _, Error};
use chrono::Utc;
use diesel::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;

embed_migrations!("./migrations");

/// Database abstraction.
#[derive(Clone)]
pub struct Database {
    pool: Arc<Mutex<SqliteConnection>>,
}

impl Database {
    /// Find posts by users.
    pub fn open(path: &Path) -> Result<Database, Error> {
        let url = path.display().to_string();

        log::info!("Using database: {}", url);

        let pool = SqliteConnection::establish(&url)?;

        let mut output = Vec::new();

        // Run all migrations and provide some diagnostics on errors.
        let result = embedded_migrations::run_with_output(&pool, &mut output);
        let output = String::from_utf8_lossy(&output);
        result.with_context(|| anyhow!("error when running migrations: {}", output))?;

        if !output.is_empty() {
            log::trace!("migrations output:\n{}", output);
        }

        Ok(Database {
            pool: Arc::new(Mutex::new(pool)),
        })
    }

    /// Run a blocking task with exlusive access to the database pool.
    pub async fn asyncify<F, T, E>(&self, task: F) -> Result<T, E>
    where
        F: FnOnce(&SqliteConnection) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
        E: From<tokio::task::JoinError>,
    {
        let pool = self.pool.clone();

        task::asyncify(move || {
            let guard = pool.lock();
            task(&*guard)
        })
        .await
    }

    /// Access auth from the database.
    pub async fn auth(&self, schema: crate::auth::Schema) -> Result<crate::auth::Auth, Error> {
        Ok(crate::auth::Auth::new(self.clone(), schema).await?)
    }

    /// Access settings from the database.
    pub fn settings(&self, schema: crate::Schema) -> Result<crate::Settings, Error> {
        Ok(crate::settings::Settings::new(self.clone(), schema))
    }

    /// List all counters in backend.
    pub async fn player_list(&self) -> Result<Vec<models::Song>, Error> {
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
    pub async fn player_push_back(&self, song: &models::AddSong) -> Result<(), Error> {
        use self::schema::songs::dsl;

        let song = song.clone();

        self.asyncify(move |c| {
            diesel::insert_into(dsl::songs).values(song).execute(c)?;
            Ok(())
        })
        .await
    }

    /// Purge the songs database and return the number of items removed.
    pub async fn player_song_purge(&self) -> Result<usize, Error> {
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
    pub async fn player_remove_song(
        &self,
        track_id: &TrackId,
        played: bool,
    ) -> Result<bool, Error> {
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
    ) -> Result<bool, Error> {
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
        duration: utils::Duration,
    ) -> Result<Option<models::Song>, Error> {
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
