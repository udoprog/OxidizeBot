//! Module for using a currency from FriendlyBaron's database.
//!
//! # TODO
//!
//! Migrate to a less static implementation which works for more general cases where you can:
//!
//! 1) Name the table to use.
//! 2) Name the fields holding channel, user, and amount.

use crate::{
    currency::BalanceTransferError,
    db::{models::Balance, user_id},
    prelude::*,
};

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use failure::Error;
use std::{convert::TryInto as _, sync::Arc};
use tokio_threadpool::ThreadPool;

mod schema {
    table! {
        honkos (username) {
            username -> Text,
            honkos_earned -> Integer,
            honko_balance -> Integer,
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, diesel::Queryable, diesel::Insertable)]
    pub struct Honko {
        pub username: String,
        pub honkos_earned: i32,
        pub honko_balance: i32,
    }
}

use self::schema::Honko;

pub struct Backend {
    channel: Arc<String>,
    pool: Pool<ConnectionManager<MysqlConnection>>,
    thread_pool: Arc<ThreadPool>,
}

impl Backend {
    /// Construct a new built-in backend.
    pub fn connect(
        channel: String,
        url: String,
        thread_pool: Arc<ThreadPool>,
    ) -> Result<Self, Error> {
        let channel = Arc::new(channel);
        let manager = ConnectionManager::<MysqlConnection>::new(url);
        let pool = Pool::builder().build(manager)?;

        Ok(Backend {
            channel,
            pool,
            thread_pool,
        })
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_transfer(
        &self,
        _channel: String,
        giver: String,
        taker: String,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        use self::schema::honkos::dsl;

        let amount: i32 = amount.try_into()?;

        let taker = user_id(&taker);
        let giver = user_id(&giver);
        let pool = self.pool.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool
                .get()
                .map_err(|e| BalanceTransferError::Other(e.into()))?;

            c.transaction(|| {
                let giver_filter = dsl::honkos.filter(dsl::username.eq(&giver));

                let balance = giver_filter
                    .clone()
                    .select(dsl::honko_balance)
                    .first::<i32>(&*c)
                    .optional()?
                    .unwrap_or_default();

                if balance < amount && !override_balance {
                    return Err(BalanceTransferError::NoBalance);
                }

                modify_balance(&c, &taker, amount)?;
                modify_balance(&c, &giver, -amount)?;
                Ok(())
            })
        }));

        future.compat().await
    }

    /// Get balances for all users.
    pub async fn export_balances(&self) -> Result<Vec<Balance>, Error> {
        use self::schema::honkos::dsl;

        let channel = self.channel.to_string();
        let pool = self.pool.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let mut balances = Vec::new();

            for h in dsl::honkos.load::<Honko>(&*pool.get()?)? {
                balances.push(Balance {
                    channel: channel.clone(),
                    user: h.username,
                    amount: h.honko_balance as i64,
                });
            }

            Ok(balances)
        }));

        future.compat().await
    }

    /// Import balances for all users.
    pub async fn import_balances(&self, balances: Vec<Balance>) -> Result<(), Error> {
        use self::schema::honkos::dsl;

        let pool = self.pool.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool.get()?;

            for balance in balances {
                let balance = balance.checked();
                let filter = dsl::honkos.filter(dsl::username.eq(&balance.user));
                let b = filter.clone().first::<Honko>(&*c).optional()?;

                match b {
                    None => {
                        let honko = Honko {
                            username: balance.user,
                            honkos_earned: 0,
                            honko_balance: balance.amount.try_into()?,
                        };

                        diesel::insert_into(dsl::honkos)
                            .values(&honko)
                            .execute(&*c)?;
                    }
                    Some(_) => {
                        let amount: i32 = balance.amount.try_into()?;
                        diesel::update(filter)
                            .set(dsl::honko_balance.eq(amount))
                            .execute(&*c)?;
                    }
                }
            }

            Ok(())
        }));

        future.compat().await
    }

    /// Find user balance.
    pub async fn balance_of(&self, _channel: String, user: String) -> Result<Option<i64>, Error> {
        use self::schema::honkos::dsl;

        let user = user_id(&user);
        let pool = self.pool.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool.get()?;

            let balance = dsl::honkos
                .select(dsl::honko_balance)
                .filter(dsl::username.eq(user))
                .first::<i32>(&*c)
                .optional()?;

            let balance = match balance {
                Some(b) => Some(b.try_into()?),
                None => None,
            };

            Ok(balance)
        }));

        future.compat().await
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_add(
        &self,
        _channel: String,
        user: String,
        amount: i64,
    ) -> Result<(), Error> {
        let user = user_id(&user);
        let amount = amount.try_into()?;
        let pool = self.pool.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool.get()?;
            modify_balance(&*c, &user, amount)
        }));

        future.compat().await
    }

    /// Add balance to users.
    pub async fn balances_increment(
        &self,
        _channel: String,
        users: impl IntoIterator<Item = String> + Send + 'static,
        amount: i64,
    ) -> Result<(), Error> {
        use self::schema::honkos::dsl;

        let amount = amount.try_into()?;

        let pool = self.pool.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool.get()?;

            for user in users {
                let user = user_id(&user);

                let filter = dsl::honkos.filter(dsl::username.eq(&user));

                let b = filter.clone().first::<Honko>(&*c).optional()?;

                match b {
                    None => {
                        let balance = Honko {
                            username: user.clone(),
                            honkos_earned: 0,
                            honko_balance: amount,
                        };

                        diesel::insert_into(dsl::honkos)
                            .values(&balance)
                            .execute(&*c)?;
                    }
                    Some(b) => {
                        let value = b.honko_balance.saturating_add(amount);

                        diesel::update(filter)
                            .set(dsl::honko_balance.eq(value))
                            .execute(&*c)?;
                    }
                }
            }

            Ok(())
        }));

        future.compat().await
    }
}

/// Common function to modify the balance for the given user.
fn modify_balance(c: &MysqlConnection, user: &str, amount: i32) -> Result<(), Error> {
    use self::schema::honkos::dsl;

    let filter = dsl::honkos.filter(dsl::username.eq(user));

    match filter.clone().first::<Honko>(&*c).optional()? {
        None => {
            let honko = Honko {
                username: user.to_string(),
                honko_balance: amount,
                honkos_earned: 0,
            };

            diesel::insert_into(dsl::honkos).values(&honko).execute(c)?;
        }
        Some(b) => {
            let amount = b.honko_balance.saturating_add(amount);

            diesel::update(filter)
                .set(dsl::honko_balance.eq(amount))
                .execute(c)?;
        }
    }

    Ok(())
}
