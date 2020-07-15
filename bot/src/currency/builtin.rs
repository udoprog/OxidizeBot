//! Module for the built-in currency which uses the regular databse support.

use crate::currency::{BalanceOf, BalanceTransferError};
use crate::db::{models, schema, user_id, Database};

use anyhow::Result;
use diesel::prelude::*;

pub struct Backend {
    db: Database,
}

impl Backend {
    /// Construct a new built-in backend.
    pub fn new(db: Database) -> Self {
        Self { db }
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

        self.db
            .asyncify(move |c| {
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
            })
            .await
    }

    /// Get balances for all users.
    pub async fn export_balances(&self) -> Result<Vec<models::Balance>> {
        use self::schema::balances::dsl;

        self.db
            .asyncify(move |c| {
                let balances = dsl::balances.load::<models::Balance>(&*c)?;
                Ok(balances)
            })
            .await
    }

    /// Import balances for all users.
    pub async fn import_balances(&self, balances: Vec<models::Balance>) -> Result<()> {
        use self::schema::balances::dsl;

        self.db
            .asyncify(move |c| {
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
                                .set((
                                    dsl::amount.eq(balance.amount),
                                    dsl::watch_time.eq(balance.watch_time),
                                ))
                                .execute(&*c)?;
                        }
                    }
                }

                Ok(())
            })
            .await
    }

    /// Find user balance.
    pub async fn balance_of(&self, channel: &str, user: &str) -> Result<Option<BalanceOf>> {
        use self::schema::balances::dsl;

        let channel = channel_id(channel);
        let user = user_id(&user);

        self.db
            .asyncify(move |c| {
                let result = dsl::balances
                    .select((dsl::amount, dsl::watch_time))
                    .filter(dsl::channel.eq(channel).and(dsl::user.eq(user)))
                    .first::<(i64, i64)>(&*c)
                    .optional()?;

                let (balance, watch_time) = match result {
                    Some((balance, watch_time)) => (balance, watch_time),
                    None => return Ok(None),
                };

                Ok(Some(BalanceOf {
                    balance,
                    watch_time,
                }))
            })
            .await
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_add(&self, channel: &str, user: &str, amount: i64) -> Result<()> {
        let channel = channel_id(channel);
        let user = user_id(user);

        self.db
            .asyncify(move |c| modify_balance(&*c, &channel, &user, amount))
            .await
    }

    /// Add balance to users.
    pub async fn balances_increment(
        &self,
        channel: &str,
        users: impl IntoIterator<Item = String> + Send + 'static,
        amount: i64,
        watch_time: i64,
    ) -> Result<()> {
        use self::schema::balances::dsl;

        // NB: for legacy reasons, channel is stored with a hash.
        let channel = format!("#{}", channel);

        self.db
            .asyncify(move |c| {
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
                                amount,
                                watch_time,
                            };

                            diesel::insert_into(dsl::balances)
                                .values(&balance)
                                .execute(&*c)?;
                        }
                        Some(b) => {
                            let value = b.amount.saturating_add(amount);
                            let watch_time = b.watch_time.saturating_add(watch_time);

                            diesel::update(filter)
                                .set((dsl::amount.eq(value), dsl::watch_time.eq(watch_time)))
                                .execute(&*c)?;
                        }
                    }
                }

                Ok(())
            })
            .await
    }
}

/// Common function to modify the balance for the given user.
fn modify_balance(c: &SqliteConnection, channel: &str, user: &str, amount: i64) -> Result<()> {
    use self::schema::balances::dsl;

    let filter = dsl::balances.filter(dsl::channel.eq(channel).and(dsl::user.eq(user)));

    match filter.clone().first::<models::Balance>(&*c).optional()? {
        None => {
            let balance = models::Balance {
                channel: channel.to_string(),
                user: user.to_string(),
                amount,
                watch_time: 0,
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
