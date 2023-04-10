//! Stream currency configuration.

use std::collections::HashSet;
use std::pin::pin;
use std::sync::Arc;

use anyhow::{Error, Result};
use async_injector::Injector;
use common::stream::StreamExt;
use common::{Channel, Duration};
use db::models::Balance;
use db::Database;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod builtin;
mod mysql;

/// Balance of a single user.
#[derive(Default)]
pub struct BalanceOf {
    balance: i64,
    watch_time: i64,
}

impl BalanceOf {
    /// Get the current watch time for the specified balance as a duration.
    pub fn watch_time(&self) -> Duration {
        if self.watch_time < 0 {
            return Duration::default();
        }

        Duration::seconds(self.watch_time as u64)
    }
}

/// Helper struct to construct a currency.
pub struct CurrencyBuilder {
    streamer: api::TwitchAndUser,
    pub mysql_schema: mysql::Schema,
    injector: Injector,
    pub ty: BackendType,
    pub enabled: bool,
    pub command_enabled: bool,
    pub name: Option<Arc<String>>,
    pub db: Option<Database>,
    pub mysql_url: Option<String>,
}

impl CurrencyBuilder {
    /// Construct a new currency builder.
    pub fn new(
        streamer: api::TwitchAndUser,
        mysql_schema: mysql::Schema,
        injector: Injector,
    ) -> Self {
        Self {
            streamer,
            mysql_schema,
            injector,
            ty: Default::default(),
            enabled: Default::default(),
            command_enabled: Default::default(),
            name: Default::default(),
            db: None,
            mysql_url: None,
        }
    }

    /// Inject the newly built value and return the result.
    async fn build_and_inject(&self) -> Option<Currency> {
        match self.build() {
            Some(currency) => {
                self.injector.update(currency.clone()).await;
                Some(currency)
            }
            None => {
                self.injector.clear::<Currency>().await;
                None
            }
        }
    }

    /// Build a new currency.
    fn build(&self) -> Option<Currency> {
        use self::mysql::Schema;

        if !self.enabled {
            return None;
        }

        let backend = match self.ty {
            BackendType::BuiltIn => {
                let db = self.db.as_ref()?;
                let backend = self::builtin::Backend::new(db.clone());
                Backend::BuiltIn(backend)
            }
            BackendType::Mysql => {
                let channel = Channel::new("#").to_owned();
                let url = self.mysql_url.clone()?;
                let schema = self.mysql_schema.clone();

                let backend = match self::mysql::Backend::connect(channel, url, schema) {
                    Ok(backend) => backend,
                    Err(e) => {
                        common::log_error!(e, "Failed to establish connection");
                        return None;
                    }
                };

                Backend::MySql(backend)
            }
            BackendType::Honkos => {
                let channel = Channel::new("#").to_owned();
                let url = self.mysql_url.clone()?;
                let schema = Schema {
                    table: String::from("honkos"),
                    user_column: String::from("username"),
                    balance_column: String::from("honko_balance"),
                };

                let backend = match self::mysql::Backend::connect(channel, url, schema) {
                    Ok(backend) => backend,
                    Err(e) => {
                        common::log_error!(e, "Failed to establish connection");
                        return None;
                    }
                };

                Backend::MySql(backend)
            }
        };

        Some(Currency {
            name: Arc::new(self.name.as_ref()?.to_string()),
            command_enabled: self.command_enabled,
            inner: Arc::new(Inner {
                backend,
                streamer: self.streamer.clone(),
            }),
        })
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub enum BackendType {
    #[serde(rename = "builtin")]
    #[default]
    BuiltIn,
    #[serde(rename = "mysql")]
    Mysql,
    #[serde(rename = "honkos")]
    Honkos,
}

enum Backend {
    BuiltIn(self::builtin::Backend),
    MySql(self::mysql::Backend),
}

impl Backend {
    /// Add (or subtract) from the balance for a single user.
    async fn balance_transfer(
        &self,
        channel: &Channel,
        giver: &str,
        taker: &str,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        use self::Backend::*;

        match self {
            BuiltIn(backend) => {
                backend
                    .balance_transfer(channel, giver, taker, amount, override_balance)
                    .await
            }
            MySql(backend) => {
                backend
                    .balance_transfer(channel, giver, taker, amount, override_balance)
                    .await
            }
        }
    }

    /// Get balances for all users.
    async fn export_balances(&self) -> Result<Vec<Balance>> {
        use self::Backend::*;

        match self {
            BuiltIn(backend) => backend.export_balances().await,
            MySql(backend) => backend.export_balances().await,
        }
    }

    /// Import balances for all users.
    async fn import_balances(&self, balances: Vec<Balance>) -> Result<()> {
        use self::Backend::*;

        match self {
            BuiltIn(backend) => backend.import_balances(balances).await,
            MySql(backend) => backend.import_balances(balances).await,
        }
    }

    /// Find user balance.
    async fn balance_of(&self, channel: &Channel, user: &str) -> Result<Option<BalanceOf>> {
        use self::Backend::*;

        match self {
            BuiltIn(backend) => backend.balance_of(channel, user).await,
            MySql(backend) => backend.balance_of(channel, user).await,
        }
    }

    /// Add (or subtract) from the balance for a single user.
    async fn balance_add(&self, channel: &Channel, user: &str, amount: i64) -> Result<()> {
        use self::Backend::*;

        match self {
            BuiltIn(backend) => backend.balance_add(channel, user, amount).await,
            MySql(backend) => backend.balance_add(channel, user, amount).await,
        }
    }

    /// Add balance to users.
    #[tracing::instrument(skip(self, users))]
    async fn balances_increment<I>(
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
        use self::Backend::*;

        match self {
            BuiltIn(backend) => {
                backend
                    .balances_increment(channel, users, amount, watch_time)
                    .await
            }
            MySql(backend) => backend.balances_increment(channel, users, amount).await,
        }
    }
}

struct Inner {
    backend: Backend,
    streamer: api::TwitchAndUser,
}

/// The currency being used.
#[derive(Clone)]
pub struct Currency {
    name: Arc<String>,
    command_enabled: bool,
    inner: Arc<Inner>,
}

impl Currency {
    /// Reward all users.
    #[tracing::instrument(skip(self))]
    async fn add_channel_all(
        &self,
        channel: &Channel,
        reward: i64,
        watch_time: i64,
    ) -> Result<usize, anyhow::Error> {
        tracing::trace!("Getting chatters");

        let mut chatters = pin!(self
            .inner
            .streamer
            .client
            .chatters(&self.inner.streamer.user.id, &self.inner.streamer.user.id));

        let mut users = HashSet::new();

        while let Some(chatter) = chatters.next().await.transpose()? {
            users.insert(chatter.user_login);
        }

        let len = users.len();

        self.inner
            .backend
            .balances_increment(channel, users, reward, watch_time)
            .await?;

        Ok(len)
    }

    /// Add (or subtract) from the balance for a single user.
    async fn balance_transfer(
        &self,
        channel: &Channel,
        giver: &str,
        taker: &str,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        self.inner
            .backend
            .balance_transfer(channel, giver, taker, amount, override_balance)
            .await
    }

    /// Get balances for all users.
    pub async fn export_balances(&self) -> Result<Vec<Balance>> {
        self.inner.backend.export_balances().await
    }

    /// Import balances for all users.
    pub async fn import_balances(&self, balances: Vec<Balance>) -> Result<()> {
        self.inner.backend.import_balances(balances).await
    }

    /// Find user balance.
    async fn balance_of(&self, channel: &Channel, user: &str) -> Result<Option<BalanceOf>> {
        self.inner.backend.balance_of(channel, user).await
    }

    /// Add (or subtract) from the balance for a single user.
    async fn balance_add(&self, channel: &Channel, user: &str, amount: i64) -> Result<()> {
        self.inner.backend.balance_add(channel, user, amount).await
    }

    /// Add balance to users.
    async fn balances_increment<I>(
        &self,
        channel: &Channel,
        users: I,
        amount: i64,
        watch_time: i64,
    ) -> Result<()>
    where
        I: IntoIterator<Item = String> + Send + 'static,
        I::IntoIter: Send + 'static,
    {
        self.inner
            .backend
            .balances_increment(channel, users, amount, watch_time)
            .await
    }
}

#[derive(Debug, Error)]
enum BalanceTransferError {
    #[error("missing balance for transfer")]
    NoBalance,
    #[error("database error: {0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("mysql error: {0}")]
    MysqlError(#[from] mysql_async::Error),
    #[error("error joining: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("other error: {}", _0)]
    Other(
        #[from]
        #[source]
        Error,
    ),
}
