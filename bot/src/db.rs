#[macro_use]
mod macros;
mod after_streams;
mod aliases;
mod commands;
pub(crate) mod models;
mod promotions;
pub(crate) mod schema;
mod words;

use crate::player;

pub use self::{
    after_streams::{AfterStream, AfterStreams},
    aliases::{Alias, Aliases},
    commands::{Command, Commands},
    promotions::{Promotion, Promotions},
    words::{Word, Words},
};

use chrono::Utc;
use diesel::prelude::*;
use futures::{future, Future};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio_threadpool::ThreadPool;

#[derive(Debug, err_derive::Error)]
pub enum RenameError {
    /// Trying to rename something to a conflicting name.
    #[error(display = "conflict")]
    Conflict,
    /// Trying to rename something which doesn't exist.
    #[error(display = "missing")]
    Missing,
}

#[derive(Debug, err_derive::Error)]
pub enum BalanceTransferError {
    #[error(display = "missing balance for transfer")]
    NoBalance,
    #[error(display = "other error: {}", _0)]
    Other(failure::Error),
}

impl From<failure::Error> for BalanceTransferError {
    fn from(value: failure::Error) -> Self {
        BalanceTransferError::Other(value)
    }
}

impl From<diesel::result::Error> for BalanceTransferError {
    fn from(value: diesel::result::Error) -> Self {
        BalanceTransferError::Other(value.into())
    }
}

embed_migrations!("./migrations");

/// Database abstraction.
#[derive(Clone)]
pub struct Database {
    pub(crate) pool: Arc<Mutex<SqliteConnection>>,
    thread_pool: Arc<ThreadPool>,
}

impl Database {
    /// Find posts by users.
    pub fn open(url: &str, thread_pool: Arc<ThreadPool>) -> Result<Database, failure::Error> {
        let pool = SqliteConnection::establish(url)?;

        // Run all migrations.
        embedded_migrations::run_with_output(&pool, &mut std::io::stdout())?;

        Ok(Database {
            pool: Arc::new(Mutex::new(pool)),
            thread_pool,
        })
    }

    /// Access settings from the database.
    pub fn settings(&self) -> Result<crate::settings::Settings, failure::Error> {
        Ok(crate::settings::Settings::new(
            self.clone(),
            crate::settings::Schema::load_static()?,
        ))
    }

    /// Add (or subtract) from the balance for a single user.
    pub fn balance_transfer(
        &self,
        channel: &str,
        giver: &str,
        taker: &str,
        amount: i64,
        override_balance: bool,
    ) -> impl Future<Item = (), Error = BalanceTransferError> {
        use self::schema::balances::dsl;

        let taker = user_id(taker);
        let giver = user_id(giver);
        let channel = String::from(channel);
        let pool = self.pool.clone();

        return self.thread_pool.spawn_handle(future::lazy(move || {
            let c = pool.lock();
            let c = &*c;

            c.transaction(move || {
                let giver_filter = dsl::balances
                    .filter(dsl::channel.eq(channel.as_str()).and(dsl::user.eq(&giver)));

                let balance = giver_filter
                    .clone()
                    .select(dsl::amount)
                    .first::<i64>(&*c)
                    .optional()?
                    .unwrap_or_default();

                if balance < amount && !override_balance {
                    return Err(BalanceTransferError::NoBalance);
                }

                modify_balance(c, &channel, &taker, amount)?;
                modify_balance(c, &channel, &giver, -amount)?;
                Ok(())
            })
        }));
    }

    /// Get balances for all users.
    pub fn export_balances(
        &self,
    ) -> impl Future<Item = Vec<models::Balance>, Error = failure::Error> {
        use self::schema::balances::dsl;

        let pool = self.pool.clone();

        self.thread_pool.spawn_handle(future::lazy(move || {
            let c = pool.lock();
            let balances = dsl::balances.load::<models::Balance>(&*c)?;
            Ok(balances)
        }))
    }

    /// Import balances for all users.
    pub fn import_balances(
        &self,
        balances: Vec<models::Balance>,
    ) -> impl Future<Item = (), Error = failure::Error> {
        use self::schema::balances::dsl;

        let pool = Arc::clone(&self.pool);

        self.thread_pool.spawn_handle(future::lazy(move || {
            let c = pool.lock();

            for balance in balances {
                let balance = balance.checked();

                let filter = dsl::balances.filter(
                    dsl::channel
                        .eq(balance.channel.as_str())
                        .and(dsl::user.eq(&balance.user)),
                );

                let b = filter.clone().first::<models::Balance>(&*c).optional()?;

                match b {
                    None => {
                        diesel::insert_into(dsl::balances)
                            .values(&balance)
                            .execute(&*c)?;
                    }
                    Some(_) => {
                        diesel::update(filter)
                            .set(dsl::amount.eq(balance.amount))
                            .execute(&*c)?;
                    }
                }
            }

            Ok(())
        }))
    }

    /// Find user balance.
    pub fn balance_of(&self, channel: &str, user: &str) -> Result<Option<i64>, failure::Error> {
        use self::schema::balances::dsl;

        let user = user_id(user);
        let c = self.pool.lock();

        let balance = dsl::balances
            .select(dsl::amount)
            .filter(dsl::channel.eq(channel).and(dsl::user.eq(user)))
            .first::<i64>(&*c)
            .optional()?;

        Ok(balance)
    }

    /// Add (or subtract) from the balance for a single user.
    pub fn balance_add(
        &self,
        channel: &str,
        user: &str,
        amount: i64,
    ) -> impl Future<Item = (), Error = failure::Error> {
        let user = user_id(user);
        let channel = String::from(channel);
        let pool = self.pool.clone();

        self.thread_pool.spawn_handle(future::lazy(move || {
            let c = pool.lock();
            modify_balance(&*c, &channel, &user, amount)
        }))
    }

    /// Add balance to users.
    pub fn balances_increment<'a>(
        &self,
        channel: &str,
        users: impl IntoIterator<Item = String> + Send + 'static,
        amount: i64,
    ) -> impl Future<Item = (), Error = failure::Error> {
        use self::schema::balances::dsl;

        let channel = String::from(channel);
        let pool = Arc::clone(&self.pool);

        self.thread_pool.spawn_handle(future::lazy(move || {
            let c = pool.lock();

            for user in users {
                let user = user_id(&user);

                let filter = dsl::balances
                    .filter(dsl::channel.eq(channel.as_str()).and(dsl::user.eq(&user)));

                let b = filter.clone().first::<models::Balance>(&*c).optional()?;

                match b {
                    None => {
                        let balance = models::Balance {
                            channel: channel.to_string(),
                            user: user.clone(),
                            amount: amount,
                        };

                        diesel::insert_into(dsl::balances)
                            .values(&balance)
                            .execute(&*c)?;
                    }
                    Some(b) => {
                        let value = b.amount.saturating_add(amount);

                        diesel::update(filter)
                            .set(dsl::amount.eq(value))
                            .execute(&*c)?;
                    }
                }
            }

            Ok(())
        }))
    }
}

/// Convert a user display name into a user id.
fn user_id(user: &str) -> String {
    user.trim_start_matches('@').to_lowercase()
}

/// Common function to modify the balance for the given user.
fn modify_balance(
    c: &SqliteConnection,
    channel: &str,
    user: &str,
    amount: i64,
) -> Result<(), failure::Error> {
    use self::schema::balances::dsl;

    let filter = dsl::balances.filter(dsl::channel.eq(channel).and(dsl::user.eq(user)));

    match filter.clone().first::<models::Balance>(&*c).optional()? {
        None => {
            let balance = models::Balance {
                channel: channel.to_string(),
                user: user.to_string(),
                amount: amount,
            };

            diesel::insert_into(dsl::balances)
                .values(&balance)
                .execute(c)?;
        }
        Some(b) => {
            let amount = b.amount.saturating_add(amount);

            diesel::update(filter)
                .set(dsl::amount.eq(amount))
                .execute(c)?;
        }
    }

    Ok(())
}

impl words::Backend for Database {
    /// List all bad words.
    fn list(&self) -> Result<Vec<models::BadWord>, failure::Error> {
        use self::schema::bad_words::dsl;
        let c = self.pool.lock();
        Ok(dsl::bad_words.load::<models::BadWord>(&*c)?)
    }

    /// Insert a bad word into the database.
    fn edit(&self, word: &str, why: Option<&str>) -> Result<(), failure::Error> {
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

    fn delete(&self, word: &str) -> Result<bool, failure::Error> {
        use self::schema::bad_words::dsl;

        let c = self.pool.lock();

        let count = diesel::delete(dsl::bad_words.filter(dsl::word.eq(&word))).execute(&*c)?;
        Ok(count == 1)
    }
}

impl player::Backend for Database {
    fn list(&self) -> Result<Vec<models::Song>, failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();
        let songs = dsl::songs
            .filter(dsl::deleted.eq(false))
            .order((dsl::promoted_at.desc(), dsl::added_at.asc()))
            .load::<models::Song>(&*c)?;
        Ok(songs)
    }

    fn push_back(&self, song: &models::AddSong) -> Result<(), failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();
        diesel::insert_into(dsl::songs).values(song).execute(&*c)?;
        Ok(())
    }

    /// Purge the given channel from songs.
    fn song_purge(&self) -> Result<usize, failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();
        Ok(diesel::update(dsl::songs.filter(dsl::deleted.eq(false)))
            .set(dsl::deleted.eq(true))
            .execute(&*c)?)
    }

    /// Remove the song at the given location.
    fn remove_song(&self, track_id: &player::TrackId) -> Result<bool, failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();

        let track_id = track_id.to_base62();

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
    fn promote_song(&self, user: &str, track_id: &player::TrackId) -> Result<bool, failure::Error> {
        use self::schema::songs::dsl;
        let c = self.pool.lock();

        let track_id = track_id.to_base62();

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
}

#[cfg(tests)]
mod tests {
    use super::user_id;

    #[test]
    fn test_user_id() {
        assert_eq!("setmod", user_id("@SetMod"));
    }
}
