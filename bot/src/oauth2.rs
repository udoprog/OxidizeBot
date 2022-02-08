use crate::api::{
    setbac::{Connection, ConnectionMeta, Token},
    Setbac,
};
use crate::injector::{Injector, Key};
use crate::prelude::*;
use crate::utils::Duration;
use crate::web;
use anyhow::Error;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use std::time;
use thiserror::Error;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Debug, Error)]
#[error("Missing OAuth 2.0 Connection: {0}")]
pub struct MissingTokenError(&'static str);

#[derive(Debug, Error)]
#[error("Connection receive was cancelled")]
pub struct CancelledToken(());

#[derive(Debug, Default)]
pub struct InnerSyncToken {
    /// Stored connection.
    connection: Option<Connection>,
    /// Queue to notify when a connection is available.
    ready_queue: VecDeque<oneshot::Sender<()>>,
}

#[derive(Clone, Debug)]
pub struct SyncToken {
    /// Name of the flow associated with connection.
    what: &'static str,
    /// The interior reference to the token.
    inner: Arc<RwLock<InnerSyncToken>>,
    /// Channel to use to force a refresh.
    force_refresh: mpsc::UnboundedSender<Option<Connection>>,
}

impl SyncToken {
    /// Create a new SyncToken.
    pub fn new(
        what: &'static str,
        force_refresh: mpsc::UnboundedSender<Option<Connection>>,
    ) -> Self {
        Self {
            what,
            inner: Default::default(),
            force_refresh,
        }
    }

    /// Set the connection and notify all waiters.
    pub async fn update(&self, update: Connection) {
        let mut lock = self.inner.write().await;

        let InnerSyncToken {
            ref mut connection,
            ref mut ready_queue,
        } = *lock;

        *connection = Some(update);

        // send ready notifications if we updated the connection.
        while let Some(front) = ready_queue.pop_front() {
            if let Err(()) = front.send(()) {
                log::warn!("tried to send ready notification but failed");
            }
        }
    }

    /// Clear the current connection.
    pub async fn clear(&self) {
        self.inner.write().await.connection = None;
    }

    /// Force a connection refresh.
    pub async fn force_refresh(&self) -> Result<(), Error> {
        let connection = self.inner.write().await.connection.take();
        self.force_refresh.send(connection)?;
        Ok(())
    }

    /// Check if connection is ready.
    pub async fn is_ready(&self) -> bool {
        self.inner.read().await.connection.is_some()
    }

    /// Wait until an underlying connection is available.
    pub async fn wait_until_ready(&self) -> Result<(), CancelledToken> {
        let rx = {
            let mut lock = self.inner.write().await;

            let InnerSyncToken {
                ref connection,
                ref mut ready_queue,
            } = *lock;

            if connection.is_some() {
                return Ok(());
            }

            let (tx, rx) = oneshot::channel();
            ready_queue.push_back(tx);
            rx
        };

        log::trace!("Waiting for connection: {}", self.what);

        match rx.await {
            Ok(()) => Ok(()),
            Err(..) => Err(CancelledToken(())),
        }
    }

    /// Read the synchronized connection.
    ///
    /// This results in an error if there is no connection to read.
    pub async fn read(&self) -> Result<RwLockReadGuard<'_, Token>, MissingTokenError> {
        match RwLockReadGuard::try_map(self.inner.read().await, |i| {
            i.connection.as_ref().map(|c| &c.token)
        }) {
            Ok(guard) => Ok(guard),
            Err(_) => Err(MissingTokenError(self.what)),
        }
    }
}

struct ConnectionFactory {
    setbac: Option<Setbac>,
    flow_id: &'static str,
    what: &'static str,
    expires: time::Duration,
    force_refresh: bool,
    connection: Option<Connection>,
    sync_token: SyncToken,
    settings: crate::Settings,
    injector: Injector,
    key: Key<SyncToken>,
    server: web::Server,
    current_hash: Option<String>,
}

enum Validation {
    /// Everything is OK, do nothing.
    Ok,
    /// Remote connection no longer present.
    Cleared,
    /// Connection needs to be updated.
    Updated(Connection),
}

impl ConnectionFactory {
    /// Perform an update based on the existing state.
    pub async fn update(&mut self) -> Result<(), Error> {
        match self.log_build().await {
            Validation::Ok => (),
            Validation::Cleared => {
                self.settings
                    .set_silent("connection", None::<Connection>)
                    .await?;
                self.sync_token.clear().await;

                if self.current_hash.is_some() {
                    self.injector.clear_key(&self.key).await;
                }

                self.server.clear_connection(&self.flow_id).await;
            }
            Validation::Updated(connection) => {
                let meta = connection.as_meta();
                self.settings
                    .set_silent("connection", Some(&connection))
                    .await?;
                self.sync_token.update(connection).await;

                if self.current_hash.as_ref() != Some(&meta.hash) {
                    self.injector
                        .update_key(&self.key, self.sync_token.clone())
                        .await;
                    self.current_hash = Some(meta.hash.clone());
                }

                self.server.update_connection(&self.flow_id, meta).await;
            }
        }

        Ok(())
    }

    /// Set the connection from settings.
    pub async fn update_from_settings(
        &mut self,
        connection: Option<Connection>,
    ) -> Result<(), Error> {
        let was_none = self.connection.is_none();
        self.connection = connection.clone();

        let connection = match self.log_build().await {
            Validation::Ok => connection,
            // already cleared, nothing to do.
            Validation::Cleared if was_none => return Ok(()),
            Validation::Cleared => None,
            Validation::Updated(connection) => {
                self.settings
                    .set_silent("connection", Some(&connection))
                    .await?;
                Some(connection)
            }
        };

        if let Some(connection) = connection {
            let meta = connection.as_meta();
            self.sync_token.update(connection).await;

            if self.current_hash.as_ref() != Some(&meta.hash) {
                self.injector
                    .update_key(&self.key, self.sync_token.clone())
                    .await;
                self.current_hash = Some(meta.hash.clone());
            }

            self.server.update_connection(&self.flow_id, meta).await;
        } else {
            self.sync_token.clear().await;

            if self.current_hash.is_some() {
                self.injector.clear_key(&self.key).await;
                self.current_hash = None;
            }

            self.server.clear_connection(&self.flow_id).await;
        }

        Ok(())
    }

    /// Construct a new connection and log on failures.
    pub async fn log_build(&mut self) -> Validation {
        match self.build().await {
            Ok(connection) => connection,
            Err(e) => {
                log_error!(e, "{}: Failed to build connection", self.what);
                Validation::Ok
            }
        }
    }

    /// Construct a new connection.
    pub async fn build(&mut self) -> Result<Validation, Error> {
        let setbac = match self.setbac.as_ref() {
            Some(setbac) => setbac,
            _ => {
                log::trace!("{}: No client to configured", self.what);
                return Ok(Validation::Ok);
            }
        };

        if self.force_refresh {
            self.force_refresh = false;
            log::trace!("{}: Forcing refresh of existing connection", self.what);

            if let Some(connection) = self.refresh_connection(setbac).await? {
                self.connection = Some(connection.clone());
                return Ok(Validation::Updated(connection));
            } else {
                self.connection = None;
                return Ok(Validation::Cleared);
            }
        }

        match self.connection.as_ref() {
            // existing expired connection.
            Some(connection) => {
                let result = self.validate_connection(setbac, connection).await?;

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
                if let Some(connection) = self.request_new_connection(setbac).await? {
                    Ok(match self.validate_connection(setbac, &connection).await? {
                        Validation::Ok => {
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
                } else {
                    Ok(Validation::Ok)
                }
            }
        }
    }

    /// Request a new connection from the authentication flow.
    async fn request_new_connection(&self, setbac: &Setbac) -> Result<Option<Connection>, Error> {
        log::trace!("{}: Requesting new connection", self.what);
        Ok(setbac.get_connection(self.flow_id).await?)
    }

    /// Validate a connection base on the current flow.
    async fn validate_connection(
        &self,
        setbac: &Setbac,
        connection: &Connection,
    ) -> Result<Validation, Error> {
        log::trace!("{}: Validating connection", self.what);

        // TODO: for some reason, this doesn't update :/
        let meta = match setbac.get_connection_meta(self.flow_id).await? {
            Some(c) => c,
            None => {
                log::trace!("{}: Remote connection cleared", self.what);
                return Ok(Validation::Cleared);
            }
        };

        if !self.is_outdated(connection, &meta)? {
            log::trace!("{}: Connection OK", self.what);
            return Ok(Validation::Ok);
        }

        // try to refresh in case it has expired.
        Ok(match self.refresh_connection(setbac).await? {
            None => Validation::Cleared,
            Some(connection) => Validation::Updated(connection),
        })
    }

    /// Refresh a connection.
    async fn refresh_connection(&self, setbac: &Setbac) -> Result<Option<Connection>, Error> {
        log::trace!("{}: Refreshing connection", self.what);

        let connection = match setbac.refresh_connection(self.flow_id).await? {
            Some(connection) => connection,
            None => return Ok(None),
        };

        Ok(Some(connection))
    }

    /// Check if connection is outdated.
    fn is_outdated(&self, c: &Connection, meta: &ConnectionMeta) -> Result<bool, Error> {
        if c.hash != meta.hash {
            return Ok(true);
        }

        Ok(c.token.expires_within(self.expires)?)
    }
}

impl fmt::Debug for ConnectionFactory {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ConnectionFactory")
            .field("flow_id", &self.flow_id)
            .field("what", &self.what)
            .field("expires", &self.expires)
            .field("force_refresh", &self.force_refresh)
            .field("connection", &self.connection)
            .finish()
    }
}

/// Setup a synchronized token and the future necessary to keep it up-to-date.
pub async fn build(
    flow_id: &'static str,
    what: &'static str,
    parent: &crate::Settings,
    settings: crate::Settings,
    injector: Injector,
    key: Key<SyncToken>,
    server: web::Server,
) -> Result<(SyncToken, impl Future<Output = Result<(), Error>>), Error> {
    // connection expires within 30 minutes.
    let expires = time::Duration::from_secs(30 * 60);

    // queue used to force connection refreshes.
    let (force_refresh, mut force_refresh_rx) = mpsc::unbounded_channel();

    let (mut connection_stream, connection) = settings
        .stream::<Connection>("connection")
        .optional()
        .await?;
    let (mut setbac_stream, setbac) = injector.stream::<Setbac>().await;
    let (mut check_interval_stream, check_interval) = parent
        .stream::<Duration>("remote/check-interval")
        .or_with(Duration::seconds(30))
        .await?;

    let sync_token = SyncToken::new(what, force_refresh);

    let mut builder = ConnectionFactory {
        setbac,
        flow_id,
        what,
        expires,
        force_refresh: false,
        connection: None,
        sync_token: sync_token.clone(),
        settings,
        injector,
        key,
        server,
        current_hash: None,
    };

    // check for expirations.
    let mut check_interval = tokio::time::interval(check_interval.as_std());

    builder.update_from_settings(connection).await?;

    let future = async move {
        log::trace!("{}: Running loop", what);

        loop {
            tokio::select! {
                setbac = setbac_stream.recv() => {
                    builder.setbac = setbac;
                    builder.update().await?;
                }
                connection = connection_stream.recv() => {
                    log::trace!("{}: New from settings", what);
                    builder.update_from_settings(connection).await?;
                }
                _ = force_refresh_rx.recv() => {
                    log::trace!("{}: Forced refresh", what);

                    if !std::mem::take(&mut builder.force_refresh) {
                        log::warn!("Forcing connection refresh for: {}", builder.what);
                        builder.update().await?;
                    }
                }
                _ = check_interval.tick() => {
                    log::trace!("{}: Check for expiration", what);
                    builder.update().await?;
                }
                update = check_interval_stream.recv() => {
                    check_interval = tokio::time::interval(update.as_std());
                }
            }
        }
    };

    Ok((sync_token, future))
}
