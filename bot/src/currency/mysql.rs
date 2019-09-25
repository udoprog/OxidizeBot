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
};

use failure::Error;
use mysql_async as mysql;
use std::{convert::TryInto as _, sync::Arc};

use mysql::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    async fn select_balances<Tx>(&self, tx: Tx) -> Result<(Tx, Vec<(String, i32)>), Error>
    where
        Tx: Queryable,
    {
        let query = format!(
            "SELECT (`{user_column}`, `{balance_column}`) FROM `{table}`",
            table = self.schema.table,
            balance_column = self.schema.balance_column,
            user_column = self.schema.user_column,
        );

        log::trace!("select_balances: {}", query);

        let rows = tx.prep_exec(query, ()).await?;

        let (tx, result) = rows
            .map_and_drop(|row| mysql::from_row::<(String, i32)>(row))
            .await?;

        Ok((tx, result))
    }

    /// Select the given balance.
    async fn select_balance<Tx>(&self, tx: Tx, user: &str) -> Result<(Tx, Option<i32>), Error>
    where
        Tx: Queryable,
    {
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

        log::trace!("select_balance: {} {:?}", query, params);
        let result = tx.prep_exec(query, params).await?;

        let (tx, results) = result
            .map_and_drop(|row| mysql::from_row::<(i32,)>(row))
            .await?;

        Ok((tx, results.into_iter().map(|(c, ..)| c).next()))
    }

    /// Helper to insert or update a single balance.
    async fn modify_balance<Tx>(&self, tx: Tx, user: &str, amount: i32) -> Result<Tx, Error>
    where
        Tx: Queryable,
    {
        // TODO: when mysql_async moves to async/await we can probably remove this budged ownership.
        self.upsert_balances(tx, vec![user.to_string()], amount)
            .await
    }

    /// Update or insert a batch of balances.
    async fn upsert_balances<Tx, I>(&self, tx: Tx, users: I, amount: i32) -> Result<Tx, Error>
    where
        Tx: Queryable,
        I: IntoIterator<Item = String> + Send + 'static,
        I::IntoIter: Send + 'static,
    {
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

        log::trace!("upsert_balances: {}", query);
        Ok(tx.batch_exec(query, params).await?)
    }

    /// Insert the given balance.
    async fn insert_balance<Tx>(&self, tx: Tx, user: &str, balance: i32) -> Result<Tx, Error>
    where
        Tx: Queryable,
    {
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

        log::trace!("insert_balance: {} {:?}", query, params);
        Ok(tx.drop_exec(query, params).await?)
    }

    /// Update the given balance.
    async fn update_balance<Tx>(&self, tx: Tx, user: &str, balance: i32) -> Result<Tx, Error>
    where
        Tx: Queryable,
    {
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

        log::trace!("update_balance: {} {:?}", query, params);
        Ok(tx.drop_exec(query, params).await?)
    }
}

pub struct Backend {
    channel: Arc<String>,
    pool: mysql::Pool,
    queries: Arc<Queries>,
}

impl Backend {
    /// Construct a new built-in backend.
    pub fn connect(channel: String, url: String, schema: Schema) -> Result<Self, Error> {
        let channel = Arc::new(channel);
        let pool = mysql::Pool::new(url);
        let queries = Arc::new(Queries { schema });

        Ok(Backend {
            channel,
            pool,
            queries,
        })
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_transfer(
        &self,
        _channel: &str,
        giver: &str,
        taker: &str,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        let amount: i32 = amount.try_into()?;
        let taker = user_id(taker);
        let giver = user_id(giver);

        let opts = mysql::TransactionOptions::new();
        let tx = self.pool.start_transaction(opts).await?;

        let (tx, balance) = self.queries.select_balance(tx, &giver).await?;

        let balance = balance.unwrap_or_default();

        if balance < amount && !override_balance {
            return Err(BalanceTransferError::NoBalance);
        }

        let tx = self.queries.modify_balance(tx, &taker, amount).await?;
        let tx = self.queries.modify_balance(tx, &giver, -amount).await?;

        tx.commit().await?;
        Ok(())
    }

    /// Get balances for all users.
    pub async fn export_balances(&self) -> Result<Vec<Balance>, Error> {
        let channel = self.channel.to_string();
        let mut output = Vec::new();

        let opts = mysql::TransactionOptions::new();
        let tx = self.pool.start_transaction(opts).await?;

        let (_, balances) = self.queries.select_balances(tx).await?;

        for (user, balance) in balances {
            output.push(Balance {
                channel: channel.clone(),
                user: user,
                amount: balance as i64,
            });
        }

        Ok(output)
    }

    /// Import balances for all users.
    pub async fn import_balances(&self, balances: Vec<Balance>) -> Result<(), Error> {
        let opts = mysql::TransactionOptions::new();
        let mut tx = self.pool.start_transaction(opts).await?;

        for balance in balances {
            let amount: i32 = balance.amount.try_into()?;
            let user = user_id(&balance.user);

            let (new_tx, results) = self.queries.select_balance(tx, &user).await?;

            tx = match results {
                None => self.queries.insert_balance(new_tx, &user, 0).await?,
                Some(_) => self.queries.update_balance(new_tx, &user, amount).await?,
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Find user balance.
    pub async fn balance_of(&self, _channel: &str, user: &str) -> Result<Option<i64>, Error> {
        let user = user_id(&user);
        let opts = mysql::TransactionOptions::new();
        let tx = self.pool.start_transaction(opts).await?;

        let (_, balance) = self.queries.select_balance(tx, &user).await?;

        let balance = match balance {
            Some(b) => Some(b.try_into()?),
            None => None,
        };

        Ok(balance)
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_add(&self, _channel: &str, user: &str, amount: i64) -> Result<(), Error> {
        let user = user_id(&user);
        let amount = amount.try_into()?;

        let opts = mysql::TransactionOptions::new();
        let tx = self.pool.start_transaction(opts).await?;

        let tx = self.queries.modify_balance(tx, &user, amount).await?;

        tx.commit().await?;
        Ok(())
    }

    /// Add balance to users.
    pub async fn balances_increment<I>(
        &self,
        _channel: &str,
        users: I,
        amount: i64,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = String> + Send + 'static,
        I::IntoIter: Send + 'static,
    {
        let amount = amount.try_into()?;
        let opts = mysql::TransactionOptions::new();
        let tx = self.pool.start_transaction(opts).await?;

        let users = users.into_iter().map(|u| user_id(&u));
        let tx = self.queries.upsert_balances(tx, users, amount).await?;

        tx.commit().await?;
        Ok(())
    }
}
