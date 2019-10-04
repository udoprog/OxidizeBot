//! Module for the built-in currency which uses the regular databse support.

use crate::{
    currency::BalanceTransferError,
    db::{models, schema, user_id, Database},
    prelude::*,
};

use diesel::prelude::*;
use failure::Error;

pub struct Backend {
    db: Database,
}

impl Backend {
    /// Construct a new built-in backend.
    pub fn new(db: Database) -> Self {
        Backend { db }
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_transfer(
        &self,
        channel: &str,
        giver: &str,
        taker: &str,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        use self::schema::balances::dsl;

        let channel = channel_id(channel);
        let giver = giver.to_string();
        let taker = taker.to_string();
        let taker = user_id(&taker);
        let giver = user_id(&giver);
        let pool = self.db.pool.clone();

        let future = async move {
            let c = pool.lock();
            let c = &*c;

            c.transaction(move || {
                let giver_filter = dsl::balances
                    .filter(dsl::channel.eq(channel.as_str()).and(dsl::user.eq(&giver)));

                let balance = giver_filter
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
        };

        let (future, remote) = future.remote_handle();
        tokio::spawn(future);
        remote.await
    }

    /// Get balances for all users.
    pub async fn export_balances(&self) -> Result<Vec<models::Balance>, Error> {
        use self::schema::balances::dsl;

        let pool = self.db.pool.clone();

        let future = async move {
            let c = pool.lock();
            let balances = dsl::balances.load::<models::Balance>(&*c)?;
            Ok(balances)
        };

        let (future, handle) = future.remote_handle();
        tokio::spawn(future);
        handle.await
    }

    /// Import balances for all users.
    pub async fn import_balances(&self, balances: Vec<models::Balance>) -> Result<(), Error> {
        use self::schema::balances::dsl;

        let pool = self.db.pool.clone();

        let future = async move {
            let c = pool.lock();

            for balance in balances {
                let balance = balance.checked();
                let channel = channel_id(&balance.channel);

                let filter = dsl::balances.filter(
                    dsl::channel
                        .eq(channel.as_str())
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
        };

        let (future, handle) = future.remote_handle();
        tokio::spawn(future);
        handle.await
    }

    /// Find user balance.
    pub async fn balance_of(&self, channel: &str, user: &str) -> Result<Option<i64>, Error> {
        use self::schema::balances::dsl;

        let channel = channel_id(channel);
        let user = user_id(&user);
        let pool = self.db.pool.clone();

        let future = async move {
            let c = pool.lock();

            let balance = dsl::balances
                .select(dsl::amount)
                .filter(dsl::channel.eq(channel).and(dsl::user.eq(user)))
                .first::<i64>(&*c)
                .optional()?;

            Ok(balance)
        };

        let (future, handle) = future.remote_handle();
        tokio::spawn(future);
        handle.await
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_add(&self, channel: &str, user: &str, amount: i64) -> Result<(), Error> {
        let channel = channel_id(channel);
        let user = user_id(user);
        let pool = self.db.pool.clone();

        let future = async move {
            let c = pool.lock();
            modify_balance(&*c, &channel, &user, amount)
        };

        let (future, handle) = future.remote_handle();
        tokio::spawn(future);
        handle.await
    }

    /// Add balance to users.
    pub async fn balances_increment(
        &self,
        channel: &str,
        users: impl IntoIterator<Item = String> + Send + 'static,
        amount: i64,
    ) -> Result<(), Error> {
        use self::schema::balances::dsl;

        // NB: for legacy reasons, channel is stored with a hash.
        let channel = format!("#{}", channel);
        let pool = self.db.pool.clone();

        let future = async move {
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
        };

        let (future, handle) = future.remote_handle();
        tokio::spawn(future);
        handle.await
    }
}

/// Common function to modify the balance for the given user.
fn modify_balance(
    c: &SqliteConnection,
    channel: &str,
    user: &str,
    amount: i64,
) -> Result<(), Error> {
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

/// Normalize channel.
fn channel_id(channel: &str) -> String {
    format!("#{}", channel.trim_start_matches('#'))
}
