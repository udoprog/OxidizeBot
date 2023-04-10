//! Module for using a currency from FriendlyBaron's database.
//!
//! # TODO
//!
//! Migrate to a less static implementation which works for more general cases where you can:
//!
//! 1) Name the table to use.
//! 2) Name the fields holding channel, user, and amount.

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use common::{Channel, OwnedChannel};
use db::models::Balance;
use db::user_id;
use mysql::prelude::*;
use mysql_async as mysql;
use serde::{Deserialize, Serialize};

use crate::{BalanceOf, BalanceTransferError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub table: String,
    pub balance_column: String,
    pub user_column: String,
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            table: String::from("balances"),
            balance_column: String::from("balance"),
            user_column: String::from("user"),
        }
    }
}

struct Queries {
    schema: Schema,
}

impl Queries {
    /// Select all balances.
    #[tracing::instrument(skip(self, tx))]
    async fn select_balances<Tx>(&self, tx: &mut Tx) -> Result<Vec<(String, i32)>>
    where
        Tx: Queryable,
    {
        tracing::trace!("Select balances");

        let query = format!(
            "SELECT (`{user_column}`, `{balance_column}`) FROM `{table}`",
            table = self.schema.table,
            balance_column = self.schema.balance_column,
            user_column = self.schema.user_column,
        );

        let results = tx
            .exec_map(query.as_str(), (), mysql::from_row::<(String, i32)>)
            .await?;
        Ok(results)
    }

    /// Select the given balance.
    #[tracing::instrument(skip(self, tx))]
    async fn select_balance<Tx>(&self, tx: &mut Tx, user: &str) -> Result<Option<i32>>
    where
        Tx: Queryable,
    {
        tracing::trace!("Select balance");

        let query = format!(
            "SELECT `{balance_column}` \
             FROM `{table}` \
             WHERE `{user_column}` = :user \
             LIMIT 1",
            table = self.schema.table,
            balance_column = self.schema.balance_column,
            user_column = self.schema.user_column,
        );

        let params = params! {
            "user" => user,
        };

        Ok(tx.exec_first(query.as_str(), params).await?)
    }

    /// Helper to insert or update a single balance.
    async fn modify_balance<Tx>(&self, tx: &mut Tx, user: &str, amount: i32) -> Result<()>
    where
        Tx: Queryable,
    {
        // TODO: when mysql_async moves to async/await we can probably remove this budged ownership.
        self.upsert_balances(tx, vec![user.to_string()], amount)
            .await?;
        Ok(())
    }

    /// Update or insert a batch of balances.
    #[tracing::instrument(skip_all)]
    async fn upsert_balances<Tx, I>(&self, tx: &mut Tx, users: I, amount: i32) -> Result<()>
    where
        Tx: Queryable,
        I: IntoIterator<Item = String> + Send,
        I::IntoIter: Send,
    {
        tracing::trace!("Upsert balances");

        let query = format! {
            "INSERT INTO `{table}` (`{user_column}`, `{balance_column}`) \
            VALUES (:user, :amount) \
            ON DUPLICATE KEY UPDATE  `{balance_column}` = `{balance_column}` + :amount",
            table = self.schema.table,
            user_column = self.schema.user_column,
            balance_column = self.schema.balance_column,
        };

        let params = users.into_iter().map(move |user| {
            params! {
                "user" => user,
                "amount" => amount,
            }
        });

        tx.exec_batch(query.as_str(), params).await?;
        Ok(())
    }

    /// Insert the given balance.
    #[tracing::instrument(skip(self, tx))]
    async fn insert_balance<Tx>(&self, tx: &mut Tx, user: &str, balance: i32) -> Result<()>
    where
        Tx: Queryable,
    {
        tracing::trace!("Insert balance");

        let query = format!(
            "INSERT INTO `{table}` (`{user_column}`, `{balance_column}`) \
             VALUES (:user, :balance)",
            table = self.schema.table,
            user_column = self.schema.user_column,
            balance_column = self.schema.balance_column,
        );

        let params = params! {
            "user" => user,
            "balance" => balance,
        };

        tx.exec_drop(query.as_str(), params).await?;
        Ok(())
    }

    /// Update the given balance.
    #[tracing::instrument(skip(self, tx))]
    async fn update_balance<Tx>(&self, tx: &mut Tx, user: &str, balance: i32) -> Result<()>
    where
        Tx: Queryable,
    {
        tracing::trace!("Update balance");

        let query = format!(
            "UPDATE `{table}` SET `{balance_column}` = :balance WHERE `{user_column}` = :user",
            table = self.schema.table,
            balance_column = self.schema.balance_column,
            user_column = self.schema.user_column,
        );

        let params = params! {
            "balance" => balance,
            "user" => user,
        };

        tx.exec_drop(query.as_str(), params).await?;
        Ok(())
    }
}

pub(crate) struct Backend {
    channel: Arc<OwnedChannel>,
    pool: mysql::Pool,
    queries: Arc<Queries>,
}

impl Backend {
    /// Construct a new built-in backend.
    pub(crate) fn connect(channel: OwnedChannel, url: String, schema: Schema) -> Result<Self> {
        let channel = Arc::new(channel);
        let opts = mysql::Opts::from_url(&url)?;
        let pool = mysql::Pool::new(opts);
        let queries = Arc::new(Queries { schema });

        Ok(Backend {
            channel,
            pool,
            queries,
        })
    }

    /// Add (or subtract) from the balance for a single user.
    pub(crate) async fn balance_transfer(
        &self,
        _channel: &Channel,
        giver: &str,
        taker: &str,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        let amount: i32 = amount.try_into().with_context(|| anyhow!("Unsupported amount `{amount}`"))?;
        let taker = user_id(taker);
        let giver = user_id(giver);

        let opts = mysql::TxOpts::new();
        let mut tx = self.pool.start_transaction(opts).await?;

        let balance = self.queries.select_balance(&mut tx, &giver).await?;

        let balance = balance.unwrap_or_default();

        if balance < amount && !override_balance {
            return Err(BalanceTransferError::NoBalance);
        }

        self.queries.modify_balance(&mut tx, &taker, amount).await?;
        self.queries
            .modify_balance(&mut tx, &giver, -amount)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Get balances for all users.
    pub(crate) async fn export_balances(&self) -> Result<Vec<Balance>> {
        let channel = self.channel.to_owned();
        let mut output = Vec::new();

        let opts = mysql::TxOpts::new();
        let mut tx = self.pool.start_transaction(opts).await?;

        let balances = self.queries.select_balances(&mut tx).await?;

        for (user, balance) in balances {
            output.push(Balance {
                channel: (*channel).to_owned(),
                user,
                amount: balance as i64,
                watch_time: 0,
            });
        }

        Ok(output)
    }

    /// Import balances for all users.
    pub(crate) async fn import_balances(&self, balances: Vec<Balance>) -> Result<()> {
        let opts = mysql::TxOpts::new();
        let mut tx = self.pool.start_transaction(opts).await?;

        for balance in balances {
            let amount: i32 = balance.amount.try_into()?;
            let user = user_id(&balance.user);

            let balance = self.queries.select_balance(&mut tx, &user).await?;

            match balance {
                None => self.queries.insert_balance(&mut tx, &user, 0).await?,
                Some(_) => self.queries.update_balance(&mut tx, &user, amount).await?,
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Find user balance.
    pub(crate) async fn balance_of(
        &self,
        _channel: &Channel,
        user: &str,
    ) -> Result<Option<BalanceOf>> {
        let user = user_id(user);
        let opts = mysql::TxOpts::new();
        let mut tx = self.pool.start_transaction(opts).await?;

        let balance = self.queries.select_balance(&mut tx, &user).await?;

        let balance = match balance {
            Some(b) => b.try_into()?,
            None => return Ok(None),
        };

        Ok(Some(BalanceOf {
            balance,
            watch_time: 0,
        }))
    }

    /// Add (or subtract) from the balance for a single user.
    pub(crate) async fn balance_add(
        &self,
        _channel: &Channel,
        user: &str,
        amount: i64,
    ) -> Result<()> {
        let user = user_id(user);
        let amount = amount.try_into()?;

        let opts = mysql::TxOpts::new();
        let mut tx = self.pool.start_transaction(opts).await?;
        self.queries.modify_balance(&mut tx, &user, amount).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Add balance to users.
    pub(crate) async fn balances_increment<I>(
        &self,
        _channel: &Channel,
        users: I,
        amount: i64,
    ) -> Result<()>
    where
        I: IntoIterator<Item = String> + Send,
        I::IntoIter: Send,
    {
        let amount = amount.try_into()?;
        let opts = mysql::TxOpts::new();
        let mut tx = self.pool.start_transaction(opts).await?;
        let users = users.into_iter().map(|u| user_id(&u));
        self.queries.upsert_balances(&mut tx, users, amount).await?;
        tx.commit().await?;
        Ok(())
    }
}
