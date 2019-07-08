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
use diesel::{
    expression::dsl::sql,
    sql_types::{Integer, Text},
};
use failure::Error;
use std::{convert::TryInto as _, sync::Arc};
use tokio_threadpool::ThreadPool;

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
    fn select_balances(&self, c: &MysqlConnection) -> Result<Vec<(String, i32)>, Error> {
        let sql = sql::<(Text, Integer)>(&format!(
            "SELECT ({user_column}, {balance_column}) \
             FROM {table}",
            table = self.schema.table,
            balance_column = self.schema.balance_column,
            user_column = self.schema.user_column,
        ));

        let balances = sql.load::<(String, i32)>(c)?;
        Ok(balances)
    }

    /// Select the given balance.
    fn select_balance(&self, c: &MysqlConnection, user: &str) -> Result<Option<i32>, Error> {
        let sql = sql::<Integer>(&format!(
            "SELECT {balance_column} \
             FROM {table} \
             WHERE {user_column} = ",
            table = self.schema.table,
            balance_column = self.schema.balance_column,
            user_column = self.schema.user_column,
        ))
        .bind::<Text, _>(user)
        .sql(" LIMIT 1");

        let balance = sql.load::<i32>(c)?.into_iter().next();
        Ok(balance)
    }

    /// Insert the given balance.
    fn insert_balance(&self, c: &MysqlConnection, user: &str, balance: i32) -> Result<(), Error> {
        let sql = sql::<()>(&format!(
            "INSERT INTO {table}({user_column}, {balance_column}) \
             VALUES(",
            table = self.schema.table,
            user_column = self.schema.user_column,
            balance_column = self.schema.balance_column,
        ))
        .bind::<Text, _>(user)
        .sql(", ")
        .bind::<Integer, _>(balance)
        .sql(")");

        sql.execute(c)?;
        Ok(())
    }

    /// Update the given balance.
    fn update_balance(&self, c: &MysqlConnection, user: &str, balance: i32) -> Result<(), Error> {
        let sql = sql::<()>(&format!(
            "UPDATE {table} SET {balance_column} = ",
            table = self.schema.table,
            balance_column = self.schema.balance_column,
        ))
        .bind::<Integer, _>(balance)
        .sql(&format!(
            " WHERE {user_column} = ",
            user_column = self.schema.user_column,
        ))
        .bind::<Text, _>(user);

        sql.execute(c)?;
        Ok(())
    }
}

pub struct Backend {
    channel: Arc<String>,
    pool: Pool<ConnectionManager<MysqlConnection>>,
    thread_pool: Arc<ThreadPool>,
    queries: Arc<Queries>,
}

impl Backend {
    /// Construct a new built-in backend.
    pub fn connect(
        channel: String,
        url: String,
        schema: Schema,
        thread_pool: Arc<ThreadPool>,
    ) -> Result<Self, Error> {
        let channel = Arc::new(channel);
        let manager = ConnectionManager::<MysqlConnection>::new(url);
        let pool = Pool::builder().build(manager)?;

        let queries = Arc::new(Queries { schema });

        Ok(Backend {
            channel,
            pool,
            thread_pool,
            queries,
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
        let amount: i32 = amount.try_into()?;

        let taker = user_id(&taker);
        let giver = user_id(&giver);
        let pool = self.pool.clone();
        let queries = self.queries.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool
                .get()
                .map_err(|e| BalanceTransferError::Other(e.into()))?;

            c.transaction(|| {
                let balance = queries
                    .select_balance(&*c, &giver)
                    .map_err(|e| BalanceTransferError::Other(e.into()))?
                    .unwrap_or_default();

                if balance < amount && !override_balance {
                    return Err(BalanceTransferError::NoBalance);
                }

                modify_balance(&c, &*queries, &taker, amount)?;
                modify_balance(&c, &*queries, &giver, -amount)?;
                Ok(())
            })
        }));

        future.compat().await
    }

    /// Get balances for all users.
    pub async fn export_balances(&self) -> Result<Vec<Balance>, Error> {
        let channel = self.channel.to_string();
        let pool = self.pool.clone();
        let queries = self.queries.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let mut balances = Vec::new();
            let c = pool.get()?;

            for (user, balance) in queries.select_balances(&*c)? {
                balances.push(Balance {
                    channel: channel.clone(),
                    user: user,
                    amount: balance as i64,
                });
            }

            Ok(balances)
        }));

        future.compat().await
    }

    /// Import balances for all users.
    pub async fn import_balances(&self, balances: Vec<Balance>) -> Result<(), Error> {
        let pool = self.pool.clone();
        let queries = self.queries.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool.get()?;

            for balance in balances {
                let user = user_id(&balance.user);

                match queries.select_balance(&*c, &user)? {
                    None => queries.insert_balance(&*c, &user, 0)?,
                    Some(_) => {
                        let amount: i32 = balance.amount.try_into()?;
                        queries.update_balance(&*c, &user, amount)?;
                    }
                }
            }

            Ok(())
        }));

        future.compat().await
    }

    /// Find user balance.
    pub async fn balance_of(&self, _channel: &str, user: &str) -> Result<Option<i64>, Error> {
        let user = user_id(&user);
        let pool = self.pool.clone();
        let queries = self.queries.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool.get()?;
            let balance = queries.select_balance(&*c, &user)?;

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
        let queries = self.queries.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool.get()?;
            modify_balance(&*c, &*queries, &user, amount)
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
        let amount = amount.try_into()?;

        let pool = self.pool.clone();
        let queries = self.queries.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            let c = pool.get()?;

            for user in users {
                let user = user_id(&user);

                match queries.select_balance(&*c, &user)? {
                    None => queries.insert_balance(&*c, &user, 0)?,
                    Some(balance) => {
                        let balance = balance.saturating_add(amount);
                        queries.update_balance(&*c, &user, balance)?;
                    }
                }
            }

            Ok(())
        }));

        future.compat().await
    }
}

/// Common function to modify the balance for the given user.
fn modify_balance(
    c: &MysqlConnection,
    queries: &Queries,
    user: &str,
    amount: i32,
) -> Result<(), Error> {
    match queries.select_balance(c, user)? {
        None => queries.insert_balance(c, user, amount)?,
        Some(balance) => {
            let amount = balance.saturating_add(amount);
            queries.update_balance(c, user, amount)?;
        }
    }

    Ok(())
}
