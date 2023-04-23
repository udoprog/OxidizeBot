//! Module for the built-in currency which uses the regular databse support.

use anyhow::Result;
use common::Channel;
use db::{models, schema, user_id, Database};
use diesel::prelude::*;

use crate::{BalanceOf, BalanceTransferError};

pub(crate) struct Backend {
    db: Database,
}

impl Backend {
    /// Construct a new built-in backend.
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Add (or subtract) from the balance for a single user.
    pub(crate) async fn balance_transfer(
        &self,
        channel: &Channel,
        giver: &str,
        taker: &str,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        use self::schema::balances::dsl;

        let channel = channel.to_owned();
        let giver = giver.to_string();
        let taker = taker.to_string();
        let taker = user_id(&taker);
        let giver = user_id(&giver);

        self.db
            .asyncify(move |c| {
                c.transaction(move |c| {
                    let giver_filter =
                        dsl::balances.filter(dsl::channel.eq(&channel).and(dsl::user.eq(&giver)));

                    let balance = giver_filter
                        .select(dsl::amount)
                        .first::<i64>(c)
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
    pub(crate) async fn export_balances(&self) -> Result<Vec<models::Balance>> {
        use self::schema::balances::dsl;

        self.db
            .asyncify(move |c| {
                let balances = dsl::balances.load::<models::Balance>(c)?;
                Ok(balances)
            })
            .await
    }

    /// Import balances for all users.
    pub(crate) async fn import_balances(&self, balances: Vec<models::Balance>) -> Result<()> {
        use self::schema::balances::dsl;

        self.db
            .asyncify(move |c| {
                for balance in balances {
                    let balance = balance.checked();

                    let filter = dsl::balances.filter(
                        dsl::channel
                            .eq(&balance.channel)
                            .and(dsl::user.eq(&balance.user)),
                    );

                    let b = filter.first::<models::Balance>(c).optional()?;

                    match b {
                        None => {
                            diesel::insert_into(dsl::balances)
                                .values(&balance)
                                .execute(c)?;
                        }
                        Some(_) => {
                            diesel::update(filter)
                                .set((
                                    dsl::amount.eq(balance.amount),
                                    dsl::watch_time.eq(balance.watch_time),
                                ))
                                .execute(c)?;
                        }
                    }
                }

                Ok(())
            })
            .await
    }

    /// Find user balance.
    pub(crate) async fn balance_of(
        &self,
        channel: &Channel,
        user: &str,
    ) -> Result<Option<BalanceOf>> {
        use self::schema::balances::dsl;

        let channel = channel.to_owned();
        let user = user_id(user);

        self.db
            .asyncify(move |c| {
                let result = dsl::balances
                    .select((dsl::amount, dsl::watch_time))
                    .filter(dsl::channel.eq(channel).and(dsl::user.eq(user)))
                    .first::<(i64, i64)>(c)
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
    pub async fn balance_add(&self, channel: &Channel, user: &str, amount: i64) -> Result<()> {
        let channel = channel.to_owned();
        let user = user_id(user);

        self.db
            .asyncify(move |c| modify_balance(c, &channel, &user, amount))
            .await
    }

    /// Add balance to users.
    pub(crate) async fn balances_increment<I>(
        &self,
        channel: &Channel,
        users: I,
        amount: i64,
        watch_time: i64,
    ) -> Result<()>
    where
        I: IntoIterator<Item = String> + Send + 'static,
        I::IntoIter: Send,
    {
        use self::schema::balances::dsl;

        let channel = channel.to_owned();

        self.db
            .asyncify(move |c| {
                for user in users {
                    let user = user_id(&user);

                    let filter =
                        dsl::balances.filter(dsl::channel.eq(&channel).and(dsl::user.eq(&user)));

                    let b = filter.first::<models::Balance>(c).optional()?;

                    match b {
                        None => {
                            let balance = models::Balance {
                                channel: channel.to_owned(),
                                user: user.clone(),
                                amount,
                                watch_time,
                            };

                            diesel::insert_into(dsl::balances)
                                .values(&balance)
                                .execute(c)?;
                        }
                        Some(b) => {
                            let value = b.amount.saturating_add(amount);
                            let watch_time = b.watch_time.saturating_add(watch_time);

                            diesel::update(filter)
                                .set((dsl::amount.eq(value), dsl::watch_time.eq(watch_time)))
                                .execute(c)?;
                        }
                    }
                }

                Ok(())
            })
            .await
    }
}

/// Common function to modify the balance for the given user.
fn modify_balance(
    c: &mut SqliteConnection,
    channel: &Channel,
    user: &str,
    amount: i64,
) -> Result<()> {
    use self::schema::balances::dsl;

    let filter = dsl::balances.filter(dsl::channel.eq(&channel).and(dsl::user.eq(user)));

    match filter.first::<models::Balance>(c).optional()? {
        None => {
            let balance = models::Balance {
                channel: channel.to_owned(),
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
