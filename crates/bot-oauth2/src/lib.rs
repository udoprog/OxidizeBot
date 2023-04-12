use std::pin::pin;
use std::time::Duration as StdDuration;
use std::{fmt, mem};

use anyhow::Result;
use api::setbac::{Connection, ConnectionMeta};
use async_fuse::Fuse;
use async_injector::{Injector, Key};
use common::Duration;
use thiserror::Error;
use tokio::time::Instant;

/// Connection metadata.
pub struct ConnectionIntegrationMeta {
    pub id: String,
    pub title: String,
    pub description: String,
    pub hash: String,
}

impl ConnectionIntegrationMeta {
    #[inline]
    fn from_api(c: &Connection) -> Self {
        ConnectionIntegrationMeta {
            id: c.id.clone(),
            title: c.title.clone(),
            description: c.description.clone(),
            hash: c.hash.clone(),
        }
    }
}

pub trait ConnectionIntegration {
    /// Clear the given connection.
    fn clear_connection(&self, id: &str);

    /// Update connection metadata.
    fn update_connection(&self, id: &str, meta: ConnectionIntegrationMeta);
}

#[derive(Debug, Error)]
#[error("Missing OAuth 2.0 Connection: {0}")]
pub(crate) struct MissingTokenError(&'static str);

#[derive(Debug, Error)]
#[error("Connection receive was cancelled")]
pub(crate) struct CancelledToken(());

struct ConnectionFactory<I> {
    setbac: Option<api::Setbac>,
    id: &'static str,
    expires: StdDuration,
    force_refresh: bool,
    connection: Option<Box<Connection>>,
    settings: settings::Settings<::auth::Scope>,
    injector: Injector,
    key: Key<api::Token>,
    integration: I,
    token: api::Token,
    backoff: common::backoff::Exponential,
    backoff_deadline: Option<Instant>,
}

enum Validation {
    /// Everything is OK, keep the current connection.
    Keep,
    /// Connection needs to be updated.
    Updated(Box<Connection>),
    /// Remote connection no longer present.
    Cleared,
}

impl<I> ConnectionFactory<I>
where
    I: ConnectionIntegration,
{
    /// Reset backoff deadline.
    fn reset(&mut self) {
        tracing::trace!("Resetting backoff");
        self.backoff_deadline = None;
        self.backoff.reset();
    }

    /// Perform an update based on the existing state.
    async fn init(&mut self) {
        if let Some(c) = &self.connection {
            self.token.set(&c.token.access_token, &c.token.client_id);
            self.injector
                .update_key(&self.key, self.token.clone())
                .await;
            self.integration
                .update_connection(self.id, ConnectionIntegrationMeta::from_api(&c));
        } else {
            self.token.clear();
            self.injector.clear_key(&self.key).await;
            self.integration.clear_connection(self.id);
        }
    }

    /// Perform an update based on the existing state.
    async fn update(&mut self, from_setting: bool) -> Result<()> {
        let validation = match self.build().await {
            Ok(validation) => validation,
            Err(error) => {
                common::log_error!(error, "Failed to build connection");
                self.backoff_deadline = Some(Instant::now() + self.backoff.failed());
                return Ok(());
            }
        };

        self.reset();

        match validation {
            Validation::Keep => {
                tracing::trace!("Keeping currenct connection");
            }
            Validation::Updated(c) => {
                let old = self.connection.as_ref().map(|c| c.as_ref().hash.as_str());
                let new = Some(c.hash.as_str());

                tracing::info!(?old, ?new, "Connection updated");

                // Only update setting, if the update did not originate from settings.
                if !from_setting {
                    self.settings.set_silent("connection", Some(&c)).await?;
                }

                // Unconditionally update token, because access token is
                // frequently rotated and we want it to "just work" without
                // reconfiguring downstream components.
                self.token.set(&c.token.access_token, &c.token.client_id);

                // Perform a hash check before we update injection, to avoid
                // disrupting all downstream components unless we have to.
                //
                // A hash mismatch occurs for example if the configured set of
                // scopes have changed.
                if old != new {
                    tracing::trace!("Update sync token through injector");

                    self.injector
                        .update_key(&self.key, self.token.clone())
                        .await;
                }

                self.integration
                    .update_connection(self.id, ConnectionIntegrationMeta::from_api(&c));
                self.connection = Some(c);
            }
            Validation::Cleared => {
                tracing::info!("Connection cleared");

                // Only update setting, if the update did not originate from settings.
                if !from_setting {
                    self.settings
                        .set_silent("connection", None::<Connection>)
                        .await?;
                }

                self.token.clear();
                self.injector.clear_key(&self.key).await;
                self.integration.clear_connection(self.id);
                self.connection = None;
            }
        }

        Ok(())
    }

    /// Construct a new connection.
    #[tracing::instrument(skip_all)]
    async fn build(&mut self) -> Result<Validation> {
        let Some(setbac) = self.setbac.as_ref() else {
            tracing::trace!("No client to configure");
            return Ok(Validation::Keep);
        };

        tracing::trace!("Building connection");

        if mem::take(&mut self.force_refresh) {
            tracing::trace!("Forcing refresh of existing connection");

            return match setbac.refresh_connection(self.id).await? {
                Some(connection) => Ok(Validation::Updated(Box::new(connection))),
                None => Ok(Validation::Cleared),
            };
        }

        if let Some(connection) = &self.connection {
            if self.is_valid(setbac, connection).await? {
                return Ok(Validation::Keep);
            }
        }

        tracing::trace!("Requesting new connection");

        match setbac.get_connection(self.id).await? {
            Some(connection) => Ok(Validation::Updated(Box::new(connection))),
            None => Ok(Validation::Cleared),
        }
    }

    /// Validate a connection base on the current flow.
    #[tracing::instrument(skip_all)]
    async fn is_valid(&self, setbac: &api::Setbac, connection: &Connection) -> Result<bool> {
        tracing::trace!("Validating connection");

        let Some(meta) = setbac.get_connection_meta(self.id).await? else {
            tracing::trace!("Remote connection cleared");
            return Ok(false);
        };

        Ok(!self.is_outdated(connection, &meta)?)
    }

    /// Refresh a connection.
    #[tracing::instrument(skip_all)]
    async fn refresh(&self, setbac: &api::Setbac) -> Result<Option<Box<Connection>>> {
        tracing::trace!("Refreshing");

        let Some(connection) = setbac.refresh_connection(self.id).await? else {
            return Ok(None)
        };

        Ok(Some(Box::new(connection)))
    }

    /// Check if connection is outdated.
    fn is_outdated(&self, c: &Connection, meta: &ConnectionMeta) -> Result<bool> {
        if c.hash != meta.hash {
            return Ok(true);
        }

        c.token.expires_within(self.expires)
    }
}

impl<I> fmt::Debug for ConnectionFactory<I> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ConnectionFactory")
            .field("id", &self.id)
            .field("expires", &self.expires)
            .field("force_refresh", &self.force_refresh)
            .field("connection", &self.connection)
            .finish()
    }
}

/// Setup a synchronized token and the future necessary to keep it up-to-date.
#[tracing::instrument(skip_all, fields(id = id))]
pub async fn setup<I>(
    id: &'static str,
    parent: settings::Settings<::auth::Scope>,
    settings: settings::Settings<::auth::Scope>,
    injector: Injector,
    key: Key<api::Token>,
    integration: I,
) -> Result<()>
where
    I: ConnectionIntegration,
{
    // connection expires within 30 minutes.
    let expires = StdDuration::from_secs(30 * 60);

    let (mut connection_stream, connection) = settings
        .stream::<Connection>("connection")
        .optional()
        .await?;

    let (mut setbac_stream, setbac) = injector.stream::<api::Setbac>().await;

    let (mut check_interval_stream, check_interval) = parent
        .stream::<Duration>("remote/check-interval")
        .or_with(Duration::seconds(30))
        .await?;

    let mut builder = ConnectionFactory {
        setbac,
        id: id,
        expires,
        force_refresh: false,
        connection: connection.map(Box::new),
        settings,
        injector,
        key,
        integration,
        token: api::Token::new(),
        backoff: common::backoff::Exponential::new(StdDuration::from_millis(50)),
        backoff_deadline: None,
    };

    builder.init().await;

    // check for expirations.
    let mut check_interval = tokio::time::interval(check_interval.as_std());

    tracing::trace!("Starting loop");

    let mut backoff = pin!(Fuse::empty());

    loop {
        if backoff.is_empty() != builder.backoff_deadline.is_none() {
            backoff.set(match builder.backoff_deadline {
                Some(deadline) => Fuse::new(tokio::time::sleep_until(deadline)),
                None => Fuse::empty(),
            });
        }

        let backing_off = !backoff.is_empty();

        tokio::select! {
            setbac = setbac_stream.recv() => {
                tracing::trace!("Received new setbac client");
                builder.setbac = setbac;
                builder.reset();
                builder.update(false).await?;
            }
            connection = connection_stream.recv() => {
                tracing::trace!("New connection from settings");
                builder.connection = connection.map(Box::new);
                builder.reset();
                builder.update(true).await?;
            }
            _ = builder.token.wait_for_refresh(), if !backing_off => {
                tracing::trace!("Forced refresh");
                builder.force_refresh = true;
                builder.update(false).await?;
            }
            _ = check_interval.tick(), if !backing_off => {
                tracing::trace!("Check for expiration");
                builder.update(true).await?;
            }
            _ = backoff.as_mut() => {
                tracing::trace!("Backoff finished");
                builder.backoff_deadline = None;
                backoff.set(Fuse::empty());
                check_interval.reset();
            }
            update = check_interval_stream.recv() => {
                check_interval = tokio::time::interval(update.as_std());
            }
        }
    }
}
