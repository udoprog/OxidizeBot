//! Stream currency configuration.
pub use crate::db::models::Balance;
use crate::{api, db::Database};
use failure::Error;
use hashbrown::HashSet;
use std::sync::Arc;
use tokio_threadpool::ThreadPool;

mod builtin;
mod mysql;

/// Helper struct to construct a currency.
pub struct CurrencyBuilder {
    pub ty: BackendType,
    pub enabled: bool,
    pub command_enabled: bool,
    pub name: Option<Arc<String>>,
    pub db: Database,
    pub twitch: api::Twitch,
    pub mysql_url: Option<String>,
    pub mysql_schema: mysql::Schema,
    pub thread_pool: Arc<ThreadPool>,
}

impl CurrencyBuilder {
    /// Construct a new currency builder.
    pub fn new(db: Database, twitch: api::Twitch, mysql_schema: mysql::Schema) -> Self {
        Self {
            ty: Default::default(),
            enabled: Default::default(),
            command_enabled: Default::default(),
            name: Default::default(),
            db,
            twitch,
            mysql_url: None,
            mysql_schema,
            thread_pool: Arc::new(ThreadPool::new()),
        }
    }

    /// Build a new currency.
    pub fn build(&self) -> Option<Currency> {
        use self::mysql::Schema;

        if !self.enabled {
            return None;
        }

        let backend = match self.ty {
            BackendType::BuiltIn => {
                let backend = self::builtin::Backend::new(self.db.clone());
                Backend::BuiltIn(backend)
            }
            BackendType::Mysql => {
                let channel = String::from("");
                let url = self.mysql_url.clone()?;
                let schema = self.mysql_schema.clone();
                let thread_pool = self.thread_pool.clone();

                let backend = match self::mysql::Backend::connect(channel, url, schema, thread_pool)
                {
                    Ok(backend) => backend,
                    Err(e) => {
                        log_err!(e, "failed to establish connection");
                        return None;
                    }
                };

                Backend::Honkos(backend)
            }
            BackendType::Honkos => {
                let channel = String::from("");
                let url = self.mysql_url.clone()?;
                let schema = Schema {
                    table: String::from("honkos"),
                    user_column: String::from("username"),
                    balance_column: String::from("honko_balance"),
                };
                let thread_pool = self.thread_pool.clone();

                let backend = match self::mysql::Backend::connect(channel, url, schema, thread_pool)
                {
                    Ok(backend) => backend,
                    Err(e) => {
                        log_err!(e, "failed to establish connection");
                        return None;
                    }
                };

                Backend::Honkos(backend)
            }
        };

        let name = Arc::new(self.name.as_ref()?.to_string());
        let twitch = self.twitch.clone();
        let command_enabled = self.command_enabled;

        Some(Currency {
            name,
            command_enabled,
            inner: Arc::new(Inner { backend, twitch }),
        })
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum BackendType {
    #[serde(rename = "builtin")]
    BuiltIn,
    #[serde(rename = "mysql")]
    Mysql,
    #[serde(rename = "honkos")]
    Honkos,
}

impl Default for BackendType {
    fn default() -> Self {
        BackendType::BuiltIn
    }
}

enum Backend {
    BuiltIn(self::builtin::Backend),
    Honkos(self::mysql::Backend),
}

impl Backend {
    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_transfer(
        &self,
        channel: String,
        giver: String,
        taker: String,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        use self::Backend::*;

        match *self {
            BuiltIn(ref backend) => {
                backend
                    .balance_transfer(channel, giver, taker, amount, override_balance)
                    .await
            }
            Honkos(ref backend) => {
                backend
                    .balance_transfer(channel, giver, taker, amount, override_balance)
                    .await
            }
        }
    }

    /// Get balances for all users.
    pub async fn export_balances(&self) -> Result<Vec<Balance>, Error> {
        use self::Backend::*;

        match *self {
            BuiltIn(ref backend) => backend.export_balances().await,
            Honkos(ref backend) => backend.export_balances().await,
        }
    }

    /// Import balances for all users.
    pub async fn import_balances(&self, balances: Vec<Balance>) -> Result<(), Error> {
        use self::Backend::*;

        match *self {
            BuiltIn(ref backend) => backend.import_balances(balances).await,
            Honkos(ref backend) => backend.import_balances(balances).await,
        }
    }

    /// Find user balance.
    pub async fn balance_of(&self, channel: String, user: String) -> Result<Option<i64>, Error> {
        use self::Backend::*;

        match *self {
            BuiltIn(ref backend) => backend.balance_of(channel, user).await,
            Honkos(ref backend) => backend.balance_of(channel, user).await,
        }
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_add(
        &self,
        channel: String,
        user: String,
        amount: i64,
    ) -> Result<(), Error> {
        use self::Backend::*;

        match *self {
            BuiltIn(ref backend) => backend.balance_add(channel, user, amount).await,
            Honkos(ref backend) => backend.balance_add(channel, user, amount).await,
        }
    }

    /// Add balance to users.
    pub async fn balances_increment(
        &self,
        channel: String,
        users: impl IntoIterator<Item = String> + Send + 'static,
        amount: i64,
    ) -> Result<(), Error> {
        use self::Backend::*;

        match *self {
            BuiltIn(ref backend) => backend.balances_increment(channel, users, amount).await,
            Honkos(ref backend) => backend.balances_increment(channel, users, amount).await,
        }
    }
}

struct Inner {
    backend: Backend,
    twitch: api::Twitch,
}

/// The currency being used.
#[derive(Clone)]
pub struct Currency {
    pub name: Arc<String>,
    pub command_enabled: bool,
    inner: Arc<Inner>,
}

impl Currency {
    /// Reward all users.
    pub async fn add_channel_all(
        &self,
        channel: String,
        reward: i64,
    ) -> Result<usize, failure::Error> {
        let chatters = self.inner.twitch.chatters(channel.clone()).await?;

        let mut users = HashSet::new();
        users.extend(chatters.viewers);
        users.extend(chatters.moderators);
        users.extend(chatters.broadcaster);

        let len = users.len();

        self.inner
            .backend
            .balances_increment(channel, users, reward)
            .await?;

        Ok(len)
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_transfer(
        &self,
        channel: String,
        giver: String,
        taker: String,
        amount: i64,
        override_balance: bool,
    ) -> Result<(), BalanceTransferError> {
        self.inner
            .backend
            .balance_transfer(channel, giver, taker, amount, override_balance)
            .await
    }

    /// Get balances for all users.
    pub async fn export_balances(&self) -> Result<Vec<Balance>, Error> {
        self.inner.backend.export_balances().await
    }

    /// Import balances for all users.
    pub async fn import_balances(&self, balances: Vec<Balance>) -> Result<(), Error> {
        self.inner.backend.import_balances(balances).await
    }

    /// Find user balance.
    pub async fn balance_of(&self, channel: String, user: String) -> Result<Option<i64>, Error> {
        self.inner.backend.balance_of(channel, user).await
    }

    /// Add (or subtract) from the balance for a single user.
    pub async fn balance_add(
        &self,
        channel: String,
        user: String,
        amount: i64,
    ) -> Result<(), Error> {
        self.inner.backend.balance_add(channel, user, amount).await
    }

    /// Add balance to users.
    pub async fn balances_increment(
        &self,
        channel: String,
        users: impl IntoIterator<Item = String> + Send + 'static,
        amount: i64,
    ) -> Result<(), Error> {
        self.inner
            .backend
            .balances_increment(channel, users, amount)
            .await
    }
}

#[derive(Debug, err_derive::Error)]
pub enum BalanceTransferError {
    #[error(display = "missing balance for transfer")]
    NoBalance,
    #[error(display = "other error: {}", _0)]
    Other(Error),
}

impl From<Error> for BalanceTransferError {
    fn from(value: Error) -> Self {
        BalanceTransferError::Other(value)
    }
}

impl From<diesel::result::Error> for BalanceTransferError {
    fn from(value: diesel::result::Error) -> Self {
        BalanceTransferError::Other(value.into())
    }
}

impl From<diesel::r2d2::Error> for BalanceTransferError {
    fn from(value: diesel::r2d2::Error) -> Self {
        BalanceTransferError::Other(value.into())
    }
}

impl From<std::num::TryFromIntError> for BalanceTransferError {
    fn from(value: std::num::TryFromIntError) -> Self {
        BalanceTransferError::Other(value.into())
    }
}
