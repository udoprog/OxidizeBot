use std::pin::pin;
use std::time::Duration as StdDuration;
use std::{fmt, mem};

use anyhow::Result;
use api::setbac::Connection;
use async_fuse::Fuse;
use async_injector::{Injector, Key};
use common::Duration;
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
        if self.backoff_deadline.is_some() {
            tracing::trace!("Resetting backoff");
            self.backoff_deadline = None;
            self.backoff.reset();
        }
    }

    /// Perform an update based on the existing state.
    async fn init(&mut self) {
        if let Some(c) = &self.connection {
            self.token.set(&c.token.access_token, &c.token.client_id);
            self.integration
                .update_connection(self.id, ConnectionIntegrationMeta::from_api(c));
        } else {
            self.token.clear();
            self.integration.clear_connection(self.id);
        }

        self.injector
            .update_key(&self.key, self.token.clone())
            .await;
    }

    /// Perform an update based on the existing state.
    async fn update(&mut self, from_setting: bool) -> Result<()> {
        let validation = match self.build().await {
            Ok(validation) => validation,
            Err(error) => {
                // We perform backoff here, since this is a remote operation
                // that *might* fail for reasons unrelated to a buggy bot.
                common::log_error!(error, "Failed to build connection");
                self.backoff_deadline = Some(Instant::now() + self.backoff.failed());
                return Ok(());
            }
        };

        self.reset();

        match validation {
            Validation::Keep => {
                tracing::trace!("Keeping current connection");
            }
            Validation::Updated(c) => {
                let old_hash = self.connection.as_ref().map(|c| c.as_ref().hash.as_str());
                let new_hash = Some(c.hash.as_str());
                let hash_mismatch = old_hash != new_hash;

                tracing::trace!(hash_mismatch, "Connection updated");

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
                if hash_mismatch {
                    tracing::trace!(?old_hash, ?new_hash, "Update sync token through injector");

                    self.injector
                        .update_key(&self.key, self.token.clone())
                        .await;
                }

                self.integration
                    .update_connection(self.id, ConnectionIntegrationMeta::from_api(&c));
                self.connection = Some(c);
            }
            Validation::Cleared => {
                tracing::trace!("Connection cleared");

                // Only update setting, if the update did not originate from settings.
                if !from_setting {
                    self.settings
                        .set_silent("connection", None::<Connection>)
                        .await?;
                }

                self.token.clear();
                self.injector
                    .update_key(&self.key, self.token.clone())
                    .await;
                self.integration.clear_connection(self.id);
                self.connection = None;
            }
        }

        Ok(())
    }

    /// Construct a new connection.
    #[tracing::instrument(skip_all)]
    async fn build(&mut self) -> Result<Validation> {
        tracing::trace!("Building connection");

        let Some(setbac) = self.setbac.as_ref() else {
            tracing::trace!("No client to configure");
            return Ok(Validation::Keep);
        };

        if !setbac.is_authorized() {
            tracing::warn!("Remote connection is not configured");
            return Ok(Validation::Keep);
        }

        if mem::take(&mut self.force_refresh) {
            tracing::trace!("Forcing refresh of existing connection");

            return match setbac.refresh_connection(self.id).await? {
                Some(connection) => Ok(Validation::Updated(Box::new(connection))),
                None => Ok(Validation::Cleared),
            };
        }

        let Some(c) = &self.connection else {
            tracing::trace!("Requesting new connection, local connection absent");

            return match setbac.get_connection(self.id).await? {
                Some(connection) => Ok(Validation::Updated(Box::new(connection))),
                None => Ok(Validation::Cleared),
            };
        };

        let Some(meta) = setbac.get_connection_meta(self.id).await? else {
            tracing::info!("Remote connection cleared");
            return Ok(Validation::Cleared);
        };

        // Test if the hash of the local connection matches that of the remote.
        // Hashes change either when the remote connection has a new access
        // token, or something else like scopes have changed.
        if meta.hash != c.hash {
            tracing::trace!(
                local_hash = c.hash,
                remote_hash = meta.hash,
                "Requesting new connection, hash mismatch"
            );

            return match setbac.get_connection(self.id).await? {
                Some(connection) => Ok(Validation::Updated(Box::new(connection))),
                None => Ok(Validation::Cleared),
            };
        }

        // If the location connection is expired, and matches the remote hash.
        // Force a remote refresh to get a new token.
        if c.token.expires_within(self.expires)? {
            tracing::info!("Remote connection outdated, forcing refresh");

            return match setbac.refresh_connection(self.id).await? {
                Some(connection) => Ok(Validation::Updated(Box::new(connection))),
                None => Ok(Validation::Cleared),
            };
        }

        Ok(Validation::Keep)
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

    let (mut check_interval_stream, mut check_interval_duration) = parent
        .stream::<Duration>("remote/check-interval")
        .or_with(Duration::seconds(30))
        .await?;

    let mut builder = ConnectionFactory {
        setbac,
        id,
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

    tracing::trace!("Starting loop");

    let mut check_interval = pin!(Fuse::empty());
    let mut backoff = pin!(Fuse::empty());

    let mut needs_interval = builder
        .setbac
        .as_ref()
        .map(|s| s.is_authorized())
        .unwrap_or_default();

    loop {
        if backoff.is_empty() != builder.backoff_deadline.is_none() {
            backoff.set(match builder.backoff_deadline {
                Some(deadline) => Fuse::new(tokio::time::sleep_until(deadline)),
                None => Fuse::empty(),
            });
        }

        if check_interval.as_ref().is_empty() == needs_interval {
            check_interval.set(if needs_interval {
                Fuse::new(tokio::time::interval(check_interval_duration.as_std()))
            } else {
                Fuse::empty()
            });
        }

        let backing_off = !backoff.is_empty();

        tokio::select! {
            setbac = setbac_stream.recv() => {
                tracing::trace!("Received new setbac client");
                needs_interval = setbac.as_ref().map(|s| s.is_authorized()).unwrap_or_default();
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
            _ = check_interval.as_mut().poll_inner(|mut i, ctx| i.poll_tick(ctx)), if !backing_off => {
                tracing::trace!("Check for expiration");
                builder.update(true).await?;
            }
            _ = backoff.as_mut() => {
                tracing::trace!("Backoff finished");
                builder.backoff_deadline = None;
                backoff.set(Fuse::empty());
                check_interval.set(Fuse::empty());
            }
            update = check_interval_stream.recv() => {
                check_interval_duration = update;
                check_interval.set(Fuse::empty());
            }
        }
    }
}
