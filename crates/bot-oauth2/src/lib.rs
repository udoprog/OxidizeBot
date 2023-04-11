use std::fmt;
use std::time;

use anyhow::Result;
use api::setbac::{Connection, ConnectionMeta};
use async_injector::{Injector, Key};
use common::Duration;
use thiserror::Error;

/// Connection metadata.
pub struct ConnectionIntegrationMeta {
    pub id: String,
    pub title: String,
    pub description: String,
    pub hash: String,
}

impl ConnectionIntegrationMeta {
    #[inline]
    fn from_api(meta: ConnectionMeta) -> Self {
        ConnectionIntegrationMeta {
            id: meta.id,
            title: meta.title,
            description: meta.description,
            hash: meta.hash,
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
    flow_id: &'static str,
    expires: time::Duration,
    force_refresh: bool,
    connection: Option<Box<Connection>>,
    settings: settings::Settings<::auth::Scope>,
    injector: Injector,
    key: Key<api::Token>,
    integration: I,
    current_hash: Option<String>,
    token: api::Token,
}

enum Validation {
    /// Everything is OK, do nothing.
    Ok,
    /// Remote connection no longer present.
    Cleared,
    /// Connection needs to be updated.
    Updated(Box<Connection>),
}

impl<I> ConnectionFactory<I>
where
    I: ConnectionIntegration,
{
    /// Perform an update based on the existing state.
    pub(crate) async fn update(&mut self) -> Result<()> {
        match self.log_build().await {
            Validation::Ok => {
                tracing::trace!("Connection ok")
            }
            Validation::Cleared => {
                tracing::info!("Connection cleared");

                self.settings
                    .set_silent("connection", None::<Connection>)
                    .await?;

                self.token.clear();

                if self.current_hash.is_some() {
                    self.injector.clear_key(&self.key).await;
                }

                self.integration.clear_connection(self.flow_id);
            }
            Validation::Updated(connection) => {
                tracing::info!("Connection updated");

                let meta = connection.as_meta();

                self.settings
                    .set_silent("connection", Some(&connection))
                    .await?;

                self.token
                    .set(&connection.token.access_token, &connection.token.client_id);

                if self.current_hash.as_ref() != Some(&meta.hash) {
                    tracing::trace!("Update sync token through injector");

                    self.injector
                        .update_key(&self.key, self.token.clone())
                        .await;

                    self.current_hash = Some(meta.hash.clone());
                }

                self.integration
                    .update_connection(self.flow_id, ConnectionIntegrationMeta::from_api(meta));
            }
        }

        Ok(())
    }

    /// Set the connection from settings.
    #[tracing::instrument(skip_all)]
    pub(crate) async fn update_from_settings(
        &mut self,
        connection: Option<Box<Connection>>,
    ) -> Result<()> {
        let was_none = self.connection.is_none();
        self.connection = connection.clone();

        let connection = match self.log_build().await {
            Validation::Ok => connection,
            // already cleared, nothing to do.
            Validation::Cleared if was_none => return Ok(()),
            Validation::Cleared => None,
            Validation::Updated(connection) => {
                self.settings
                    .set_silent("connection", Some(connection.as_ref()))
                    .await?;
                Some(connection)
            }
        };

        if let Some(connection) = connection {
            let meta = connection.as_meta();
            self.token
                .set(&connection.token.access_token, &connection.token.client_id);

            if self.current_hash.as_ref() != Some(&meta.hash) {
                self.injector
                    .update_key(&self.key, self.token.clone())
                    .await;
                self.current_hash = Some(meta.hash.clone());
            }

            self.integration
                .update_connection(self.flow_id, ConnectionIntegrationMeta::from_api(meta));
        } else {
            self.token.clear();

            if self.current_hash.is_some() {
                self.injector.clear_key(&self.key).await;
                self.current_hash = None;
            }

            self.integration.clear_connection(self.flow_id);
        }

        Ok(())
    }

    /// Construct a new connection and log on failures.
    pub(crate) async fn log_build(&mut self) -> Validation {
        match self.build().await {
            Ok(connection) => connection,
            Err(e) => {
                common::log_error!(e, "Failed to build connection");
                Validation::Ok
            }
        }
    }

    /// Construct a new connection.
    #[tracing::instrument(skip_all)]
    pub(crate) async fn build(&mut self) -> Result<Validation> {
        let Some(setbac) = self.setbac.as_ref() else {
            tracing::trace!("No client to configure");
            return Ok(Validation::Ok);
        };

        tracing::trace!("Building connection");

        if self.force_refresh {
            self.force_refresh = false;
            tracing::trace!("Forcing refresh of existing connection");

            return Ok(if let Some(connection) = self.refresh(setbac).await? {
                self.connection = Some(connection.clone());
                Validation::Updated(connection)
            } else {
                self.connection = None;
                Validation::Cleared
            });
        }

        match self.connection.as_ref() {
            // existing expired connection.
            Some(connection) => {
                let result = self.validate(setbac, connection).await?;

                Ok(match result {
                    Validation::Ok => Validation::Ok,
                    Validation::Cleared => {
                        self.connection = None;
                        Validation::Cleared
                    }
                    Validation::Updated(connection) => {
                        self.connection = Some(connection.clone());
                        Validation::Updated(connection)
                    }
                })
            }
            // No existing connection, request a new one.
            None => {
                tracing::trace!("Requesting new connection");

                let Some(connection) = setbac.get_connection(self.flow_id).await? else {
                    tracing::trace!("No remote connection configured");
                    return Ok(Validation::Ok);
                };

                Ok(match self.validate(setbac, &connection).await? {
                    Validation::Ok => {
                        let connection = Box::new(connection);
                        self.connection = Some(connection.clone());
                        Validation::Updated(connection)
                    }
                    Validation::Cleared => {
                        self.connection = None;
                        Validation::Cleared
                    }
                    Validation::Updated(connection) => {
                        self.connection = Some(connection.clone());
                        Validation::Updated(connection)
                    }
                })
            }
        }
    }

    /// Validate a connection base on the current flow.
    #[tracing::instrument(skip_all)]
    async fn validate(&self, setbac: &api::Setbac, connection: &Connection) -> Result<Validation> {
        tracing::trace!("Validating connection");

        // TODO: for some reason, this doesn't update :/
        let Some(meta) = setbac.get_connection_meta(self.flow_id).await? else {
            tracing::trace!("Remote connection cleared");
            return Ok(Validation::Cleared);
        };

        if !self.is_outdated(connection, &meta)? {
            tracing::trace!("Connection not outdated");
            return Ok(Validation::Ok);
        }

        // try to refresh in case it has expired.
        Ok(match self.refresh(setbac).await? {
            None => Validation::Cleared,
            Some(connection) => Validation::Updated(connection),
        })
    }

    /// Refresh a connection.
    #[tracing::instrument(skip_all)]
    async fn refresh(&self, setbac: &api::Setbac) -> Result<Option<Box<Connection>>> {
        tracing::trace!("Refreshing");

        let Some(connection) = setbac.refresh_connection(self.flow_id).await? else {
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
            .field("flow_id", &self.flow_id)
            .field("expires", &self.expires)
            .field("force_refresh", &self.force_refresh)
            .field("connection", &self.connection)
            .finish()
    }
}

/// Setup a synchronized token and the future necessary to keep it up-to-date.
#[tracing::instrument(skip_all, fields(id = flow_id))]
pub async fn setup<I>(
    flow_id: &'static str,
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
    let expires = time::Duration::from_secs(30 * 60);

    let (mut connection_stream, connection) = settings
        .stream::<Connection>("connection")
        .optional()
        .await?;

    let (mut setbac_stream, setbac) = injector.stream::<api::Setbac>().await;

    let (mut check_interval_stream, check_interval) = parent
        .stream::<Duration>("remote/check-interval")
        .or_with(Duration::seconds(30))
        .await?;

    let token = api::Token::new();

    let mut builder = ConnectionFactory {
        setbac,
        flow_id,
        expires,
        force_refresh: false,
        connection: None,
        settings,
        injector,
        key,
        integration,
        current_hash: None,
        token: token.clone(),
    };

    // check for expirations.
    let mut check_interval = tokio::time::interval(check_interval.as_std());

    builder
        .update_from_settings(connection.map(Box::new))
        .await?;

    tracing::trace!("Starting loop");

    loop {
        tokio::select! {
            setbac = setbac_stream.recv() => {
                builder.setbac = setbac;
                builder.update().await?;
            }
            connection = connection_stream.recv() => {
                tracing::trace!("New from settings");
                builder.update_from_settings(connection.map(Box::new)).await?;
            }
            _ = builder.token.wait_for_refresh() => {
                tracing::trace!("Forced refresh");

                if !std::mem::take(&mut builder.force_refresh) {
                    tracing::warn!("Forcing connection refresh");
                    builder.update().await?;
                }
            }
            _ = check_interval.tick() => {
                tracing::trace!("Check for expiration");
                builder.update().await?;
            }
            update = check_interval_stream.recv() => {
                check_interval = tokio::time::interval(update.as_std());
            }
        }
    }
}
