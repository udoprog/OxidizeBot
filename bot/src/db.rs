mod commands;
mod counters;
pub mod models;
mod schema;
mod words;

use crate::player;

pub use self::{
    commands::{Command, Commands},
    counters::{Counter, Counters},
    words::{Word, Words},
};

use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use futures::{future, Future};
use std::sync::Arc;
use tokio_threadpool::ThreadPool;

embed_migrations!("./migrations");

/// Database abstraction.
#[derive(Clone)]
pub struct Database {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    thread_pool: Arc<ThreadPool>,
}

impl Database {
    /// Find posts by users.
    pub fn open(url: &str, thread_pool: Arc<ThreadPool>) -> Result<Database, failure::Error> {
        let manager = ConnectionManager::<SqliteConnection>::new(url);
        let pool = Pool::new(manager)?;

        // Run all migrations.
        embedded_migrations::run_with_output(&pool.get()?, &mut std::io::stdout())?;

        Ok(Database {
            pool: Arc::new(pool),
            thread_pool,
        })
    }

    /// Add an afterstream reminder.
    pub fn insert_afterstream(&self, user: &str, text: &str) -> Result<(), failure::Error> {
        use self::schema::after_streams::dsl;

        let c = self.pool.get()?;

        let after_stream = models::AfterStream {
            user: String::from(user),
            text: String::from(text),
        };

        diesel::insert_into(dsl::after_streams)
            .values(&after_stream)
            .execute(&c)?;

        Ok(())
    }

    /// Find user balance.
    pub fn balance_of(&self, name: &str) -> Result<Option<i32>, failure::Error> {
        use self::schema::balances::dsl::*;

        let c = self.pool.get()?;

        let b = balances
            .filter(user.eq(name))
            .first::<models::Balance>(&c)
            .optional()?;

        Ok(b.map(|b| b.amount))
    }

    /// Add balance to users.
    pub fn balances_increment<'a>(
        &self,
        channel: &str,
        users: impl IntoIterator<Item = String> + Send + 'static,
        amount_to_add: i32,
    ) -> impl Future<Item = (), Error = failure::Error> {
        use self::schema::balances::dsl;

        let channel = String::from(channel);
        let pool = Arc::clone(&self.pool);

        self.thread_pool.spawn_handle(future::lazy(move || {
            let c = pool.get()?;

            for user in users {
                let filter = dsl::balances
                    .filter(dsl::channel.eq(channel.as_str()).and(dsl::user.eq(&user)));

                let b = filter.clone().first::<models::Balance>(&c).optional()?;

                match b {
                    None => {
                        let balance = models::Balance {
                            channel: channel.to_string(),
                            user: user.clone(),
                            amount: amount_to_add,
                        };

                        diesel::insert_into(dsl::balances)
                            .values(&balance)
                            .execute(&c)?;
                    }
                    Some(b) => {
                        let value = b.amount + amount_to_add;
                        diesel::update(filter)
                            .set(dsl::amount.eq(value))
                            .execute(&c)?;
                    }
                }
            }

            Ok(())
        }))
    }
}

impl commands::Backend for Database {
    /// Edit the given command.
    fn edit(&self, channel: &str, name: &str, text: &str) -> Result<(), failure::Error> {
        use self::schema::commands::dsl;

        let name = name.to_lowercase();

        let c = self.pool.get()?;
        let filter = dsl::commands.filter(dsl::channel.eq(channel).and(dsl::name.eq(&name)));
        let b = filter.clone().first::<models::Command>(&c).optional()?;

        match b {
            None => {
                let command = models::Command {
                    channel: channel.to_string(),
                    name,
                    text: text.to_string(),
                };

                diesel::insert_into(dsl::commands)
                    .values(&command)
                    .execute(&c)?;
            }
            Some(_) => {
                diesel::update(filter).set(dsl::text.eq(text)).execute(&c)?;
            }
        }

        Ok(())
    }

    fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        use self::schema::commands::dsl;

        let name = name.to_lowercase();

        let c = self.pool.get()?;
        let count =
            diesel::delete(dsl::commands.filter(dsl::channel.eq(channel).and(dsl::name.eq(&name))))
                .execute(&c)?;
        Ok(count == 1)
    }

    /// List all available commands.
    fn list(&self) -> Result<Vec<models::Command>, failure::Error> {
        use self::schema::commands::dsl;
        let c = self.pool.get()?;
        Ok(dsl::commands.load::<models::Command>(&c)?)
    }
}

impl words::Backend for Database {
    /// List all bad words.
    fn list(&self) -> Result<Vec<models::BadWord>, failure::Error> {
        use self::schema::bad_words::dsl;
        let c = self.pool.get()?;
        Ok(dsl::bad_words.load::<models::BadWord>(&c)?)
    }

    /// Insert a bad word into the database.
    fn edit(&self, word: &str, why: Option<&str>) -> Result<(), failure::Error> {
        use self::schema::bad_words::dsl;

        let c = self.pool.get()?;

        let filter = dsl::bad_words.filter(dsl::word.eq(word));
        let b = filter.clone().first::<models::BadWord>(&c).optional()?;

        match b {
            None => {
                let bad_word = models::BadWord {
                    word: word.to_string(),
                    why: why.map(|s| s.to_string()),
                };

                diesel::insert_into(dsl::bad_words)
                    .values(&bad_word)
                    .execute(&c)?;
            }
            Some(_) => {
                diesel::update(filter)
                    .set(why.map(|w| dsl::why.eq(w)))
                    .execute(&c)?;
            }
        }

        Ok(())
    }

    fn delete(&self, word: &str) -> Result<bool, failure::Error> {
        use self::schema::bad_words::dsl;

        let c = self.pool.get()?;

        let count = diesel::delete(dsl::bad_words.filter(dsl::word.eq(&word))).execute(&c)?;
        Ok(count == 1)
    }
}

impl counters::Backend for Database {
    fn list(&self) -> Result<Vec<models::Counter>, failure::Error> {
        use self::schema::counters::dsl;
        let c = self.pool.get()?;
        Ok(dsl::counters.load::<models::Counter>(&c)?)
    }

    fn edit(&self, channel: &str, name: &str, text: &str) -> Result<(), failure::Error> {
        use self::schema::counters::dsl;

        let c = self.pool.get()?;
        let filter = dsl::counters.filter(dsl::name.eq(&name));
        let b = filter.clone().first::<models::Counter>(&c).optional()?;

        match b {
            None => {
                let command = models::Counter {
                    channel: channel.to_string(),
                    name: name.to_string(),
                    count: 0,
                    text: text.to_string(),
                };

                diesel::insert_into(dsl::counters)
                    .values(&command)
                    .execute(&c)?;
            }
            Some(_) => {
                diesel::update(filter).set(dsl::text.eq(text)).execute(&c)?;
            }
        }

        Ok(())
    }

    fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        use self::schema::counters::dsl;

        let c = self.pool.get()?;
        let count =
            diesel::delete(dsl::counters.filter(dsl::channel.eq(channel).and(dsl::name.eq(name))))
                .execute(&c)?;
        Ok(count == 1)
    }

    fn increment(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        use self::schema::counters::dsl;

        let c = self.pool.get()?;
        let count =
            diesel::update(dsl::counters.filter(dsl::channel.eq(channel).and(dsl::name.eq(&name))))
                .set(dsl::count.eq(dsl::count + 1))
                .execute(&c)?;
        Ok(count == 1)
    }
}

impl player::Backend for Database {
    fn list(&self) -> Result<Vec<models::Song>, failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.get()?;
        let songs = dsl::songs
            .filter(dsl::deleted.eq(false))
            .order((dsl::promoted_at.desc(), dsl::added_at.asc()))
            .load::<models::Song>(&c)?;
        Ok(songs)
    }

    fn push_back(&self, song: &models::AddSong) -> Result<(), failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.get()?;
        diesel::insert_into(dsl::songs).values(song).execute(&c)?;
        Ok(())
    }

    /// Purge the given channel from songs.
    fn song_purge(&self) -> Result<usize, failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.get()?;
        Ok(diesel::update(dsl::songs.filter(dsl::deleted.eq(false)))
            .set(dsl::deleted.eq(true))
            .execute(&c)?)
    }

    /// Remove the song at the given location.
    fn remove_song(&self, track_id: &player::TrackId) -> Result<bool, failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.get()?;

        let track_id = track_id.to_base62();

        let ids: Vec<i32> = dsl::songs
            .select(dsl::id)
            .filter(dsl::deleted.eq(false).and(dsl::track_id.eq(&track_id)))
            .order(dsl::added_at.desc())
            .limit(1)
            .load(&c)?;

        let count = diesel::update(dsl::songs.filter(dsl::id.eq_any(ids)))
            .set(dsl::deleted.eq(true))
            .execute(&c)?;

        Ok(count == 1)
    }

    /// Promote the song with the given ID.
    fn promote_song(&self, user: &str, track_id: &player::TrackId) -> Result<bool, failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.get()?;

        let track_id = track_id.to_base62();

        let ids: Vec<i32> = dsl::songs
            .select(dsl::id)
            .filter(dsl::deleted.eq(false).and(dsl::track_id.eq(&track_id)))
            .order(dsl::added_at.desc())
            .limit(1)
            .load(&c)?;

        let count = diesel::update(dsl::songs.filter(dsl::id.eq_any(ids)))
            .set((
                dsl::promoted_at.eq(Utc::now().naive_utc()),
                dsl::promoted_by.eq(user),
            ))
            .execute(&c)?;

        Ok(count == 1)
    }
}
