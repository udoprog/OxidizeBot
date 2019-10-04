#[macro_use]
mod macros;
mod after_streams;
mod aliases;
pub(crate) mod commands;
mod matcher;
pub(crate) mod models;
mod promotions;
pub(crate) mod schema;
mod themes;
mod words;

use crate::{player, track_id::TrackId, utils};
use std::path::Path;

pub use self::{
    after_streams::{AfterStream, AfterStreams},
    aliases::{Alias, Aliases},
    commands::{Command, Commands},
    matcher::Captures,
    promotions::{Promotion, Promotions},
    themes::{Theme, Themes},
    words::{Word, Words},
};

pub use self::matcher::Key;
pub(crate) use self::matcher::{Matchable, Matcher, Pattern};

use chrono::Utc;
use diesel::prelude::*;
use failure::{format_err, Error, ResultExt as _};
use parking_lot::Mutex;
use std::sync::Arc;

embed_migrations!("./migrations");

/// Database abstraction.
#[derive(Clone)]
pub struct Database {
    pub(crate) pool: Arc<Mutex<SqliteConnection>>,
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
        result.with_context(|_| format_err!("error when running migrations: {}", output))?;

        if !output.is_empty() {
            log::trace!("migrations output:\n{}", output);
        }

        Ok(Database {
            pool: Arc::new(Mutex::new(pool)),
        })
    }

    /// Access auth from the database.
    pub fn auth(&self, schema: crate::auth::Schema) -> Result<crate::auth::Auth, Error> {
        Ok(crate::auth::Auth::new(self.clone(), schema)?)
    }

    /// Access settings from the database.
    pub fn settings(
        &self,
        schema: crate::settings::Schema,
    ) -> Result<crate::settings::Settings, Error> {
        Ok(crate::settings::Settings::new(self.clone(), schema))
    }
}

/// Convert a user display name into a user id.
pub fn user_id(user: &str) -> String {
    user.trim_start_matches('@').to_lowercase()
}

impl words::Backend for Database {
    /// List all bad words.
    fn list(&self) -> Result<Vec<models::BadWord>, Error> {
        use self::schema::bad_words::dsl;
        let c = self.pool.lock();
        Ok(dsl::bad_words.load::<models::BadWord>(&*c)?)
    }

    /// Insert a bad word into the database.
    fn edit(&self, word: &str, why: Option<&str>) -> Result<(), Error> {
        use self::schema::bad_words::dsl;

        let c = self.pool.lock();

        let filter = dsl::bad_words.filter(dsl::word.eq(word));
        let b = filter.clone().first::<models::BadWord>(&*c).optional()?;

        match b {
            None => {
                let bad_word = models::BadWord {
                    word: word.to_string(),
                    why: why.map(|s| s.to_string()),
                };

                diesel::insert_into(dsl::bad_words)
                    .values(&bad_word)
                    .execute(&*c)?;
            }
            Some(_) => {
                diesel::update(filter)
                    .set(why.map(|w| dsl::why.eq(w)))
                    .execute(&*c)?;
            }
        }

        Ok(())
    }

    fn delete(&self, word: &str) -> Result<bool, Error> {
        use self::schema::bad_words::dsl;

        let c = self.pool.lock();

        let count = diesel::delete(dsl::bad_words.filter(dsl::word.eq(&word))).execute(&*c)?;
        Ok(count == 1)
    }
}

impl player::Backend for Database {
    fn list(&self) -> Result<Vec<models::Song>, Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();
        let songs = dsl::songs
            .filter(dsl::deleted.eq(false))
            .order((dsl::promoted_at.desc(), dsl::added_at.asc()))
            .load::<models::Song>(&*c)?;
        Ok(songs)
    }

    fn push_back(&self, song: &models::AddSong) -> Result<(), Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();
        diesel::insert_into(dsl::songs).values(song).execute(&*c)?;
        Ok(())
    }

    /// Purge the given channel from songs.
    fn song_purge(&self) -> Result<usize, Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();
        Ok(diesel::update(dsl::songs.filter(dsl::deleted.eq(false)))
            .set(dsl::deleted.eq(true))
            .execute(&*c)?)
    }

    /// Remove the song at the given location.
    fn remove_song(&self, track_id: &TrackId) -> Result<bool, Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();

        let ids: Vec<i32> = dsl::songs
            .select(dsl::id)
            .filter(dsl::deleted.eq(false).and(dsl::track_id.eq(&track_id)))
            .order(dsl::added_at.desc())
            .limit(1)
            .load(&*c)?;

        let count = diesel::update(dsl::songs.filter(dsl::id.eq_any(ids)))
            .set(dsl::deleted.eq(true))
            .execute(&*c)?;

        Ok(count == 1)
    }

    /// Promote the song with the given ID.
    fn promote_song(&self, user: Option<&str>, track_id: &TrackId) -> Result<bool, Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();

        let ids: Vec<i32> = dsl::songs
            .select(dsl::id)
            .filter(dsl::deleted.eq(false).and(dsl::track_id.eq(&track_id)))
            .order(dsl::added_at.desc())
            .limit(1)
            .load(&*c)?;

        let count = diesel::update(dsl::songs.filter(dsl::id.eq_any(ids)))
            .set((
                dsl::promoted_at.eq(Utc::now().naive_utc()),
                dsl::promoted_by.eq(user),
            ))
            .execute(&*c)?;

        Ok(count == 1)
    }

    fn last_song_within(
        &self,
        track_id: &TrackId,
        duration: utils::Duration,
    ) -> Result<Option<models::Song>, Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();

        let since = match Utc::now().checked_sub_signed(duration.as_chrono()) {
            Some(since) => since,
            None => failure::bail!("duration too long"),
        };

        let since = since.naive_utc();

        let song = dsl::songs
            .filter(dsl::added_at.gt(&since).and(dsl::track_id.eq(&track_id)))
            .first::<models::Song>(&*c)
            .optional()?;

        Ok(song)
    }
}

#[derive(Debug, err_derive::Error)]
pub enum RenameError {
    /// Trying to rename something to a conflicting name.
    #[error(display = "conflict")]
    Conflict,
    /// Trying to rename something which doesn't exist.
    #[error(display = "missing")]
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
