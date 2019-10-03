use crate::{
    api::{
        setbac::{Connection, ConnectionMeta, Token},
        Setbac,
    },
    injector::{Injector, Key},
    prelude::*,
    settings::Settings,
};
use failure::Error;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use serde::Serialize;
use std::collections::VecDeque;
use std::fmt;
use std::{sync::Arc, time::Duration};

/// Connection identifier used for dependency injection.
#[derive(Debug, Clone, Serialize)]
pub enum TokenId {
    TwitchStreamer,
    TwitchBot,
    YouTube,
    NightBot,
    Spotify,
}

#[derive(Debug, err_derive::Error)]
#[error(display = "Missing OAuth 2.0 Connection: {0}", _0)]
pub struct MissingTokenError(&'static str);

#[derive(Debug, err_derive::Error)]
#[error(display = "Connection receive was cancelled")]
pub struct CancelledToken;

#[derive(Debug, Default)]
pub struct SyncTokenInner {
    /// Stored connection.
    connection: Option<Connection>,
    /// Queue to notify when a connection is available.
    ready_queue: VecDeque<oneshot::Sender<()>>,
}

#[derive(Clone, Debug)]
pub struct SyncToken {
    /// Name of the flow associated with connection.
    what: &'static str,
    inner: Arc<RwLock<SyncTokenInner>>,
    /// Channel to use to force a refresh.
    force_refresh: mpsc::UnboundedSender<Option<Connection>>,
}

impl SyncToken {
    /// Set the connection and notify all waiters.
    pub fn set(&self, update: Option<Connection>, key: &Key<SyncToken>, injector: &Injector) {
        let mut lock = self.inner.write();

        let SyncTokenInner {
            ref mut connection,
            ref mut ready_queue,
        } = *lock;

        let token_was_some = connection.is_some();
        let update_is_some = update.is_some();

        *connection = update;

        // send ready notifications if we updated the connection.
        if connection.is_some() {
            while let Some(front) = ready_queue.pop_front() {
                if let Err(()) = front.send(()) {
                    log::warn!("tried to send ready notification but failed");
                }
            }
        }

        match (token_was_some, update_is_some) {
            (true, false) => injector.clear_key(key),
            (false, true) => injector.update_key(key, self.clone()),
            _ => (),
        }
    }

    /// Force a connection refresh.
    pub fn force_refresh(&self) -> Result<(), Error> {
        log::warn!("Forcing connection refresh for: {}", self.what);
        let connection = self.inner.write().connection.take();
        self.force_refresh.unbounded_send(connection)?;
        Ok(())
    }

    /// Check if connection is ready.
    pub fn is_ready(&self) -> bool {
        self.inner.read().connection.is_some()
    }

    /// Wait until an underlying connection is available.
    pub async fn wait_until_ready(&self) -> Result<(), CancelledToken> {
        let rx = {
            let mut lock = self.inner.write();

            let SyncTokenInner {
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
            Err(oneshot::Canceled) => Err(CancelledToken),
        }
    }

    /// Read the synchronized connection.
    ///
    /// This results in an error if there is no connection to read.
    pub fn read<'a>(&'a self) -> Result<MappedRwLockReadGuard<'a, Token>, MissingTokenError> {
        match RwLockReadGuard::try_map(self.inner.read(), |i| {
            i.connection.as_ref().map(|c| &c.token)
        }) {
            Ok(guard) => Ok(guard),
            Err(_) => Err(MissingTokenError(self.what)),
        }
    }
}

struct ConnectionFactory {
    setbac: Option<Setbac>,
    flow_id: String,
    what: &'static str,
    expires: Duration,
    force_refresh: bool,
    new_connection: Option<Connection>,
    connection: Option<Connection>,
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
    /// Construct a new connection and log on failures.
    pub async fn log_build(&mut self) -> Option<Option<Connection>> {
        match self.build().await {
            Ok(connection) => connection,
            Err(e) => {
                log::error!("{}: Failed to build connection: {}", self.what, e);
                None
            }
        }
    }

    /// Construct a new connection.
    pub async fn build(&mut self) -> Result<Option<Option<Connection>>, Error> {
        let setbac = match self.setbac.as_ref() {
            Some(setbac) => setbac,
            _ => return Ok(None),
        };

        if let Some(connection) = self.new_connection.take() {
            log::trace!("{}: Validating new connection", self.what);

            match self.validate_connection(setbac, &connection).await? {
                Validation::Ok => {
                    self.connection = Some(connection.clone());
                    return Ok(Some(Some(connection)));
                }
                Validation::Cleared => {
                    self.connection = None;
                    return Ok(Some(None));
                }
                Validation::Updated(connection) => {
                    self.connection = Some(connection.clone());
                    return Ok(Some(Some(connection)));
                }
            }
        }

        if self.force_refresh {
            self.force_refresh = false;
            log::trace!("{}: Forcing refresh of existing connection", self.what);

            match self.refresh_connection(setbac).await? {
                Validation::Ok => (),
                Validation::Cleared => {
                    self.connection = None;
                    return Ok(Some(None));
                }
                Validation::Updated(connection) => {
                    self.connection = Some(connection.clone());
                    return Ok(Some(Some(connection)));
                }
            }

            return Ok(None);
        }

        match self.connection.as_ref() {
            // existing expired connection.
            Some(connection) => {
                let result = self.validate_connection(setbac, connection).await?;

                match result {
                    Validation::Ok => {
                        return Ok(None);
                    }
                    Validation::Cleared => {
                        self.connection = None;
                        return Ok(Some(None));
                    }
                    Validation::Updated(connection) => {
                        self.connection = Some(connection.clone());
                        return Ok(Some(Some(connection)));
                    }
                }
            }
            // No existing connection, request a new one.
            None => {
                if let Some(connection) = self.request_new_connection(setbac).await? {
                    match self.validate_connection(setbac, &connection).await? {
                        Validation::Ok => {
                            self.connection = Some(connection.clone());
                            return Ok(Some(Some(connection)));
                        }
                        Validation::Cleared => {
                            self.connection = None;
                            return Ok(Some(None));
                        }
                        Validation::Updated(connection) => {
                            self.connection = Some(connection.clone());
                            return Ok(Some(Some(connection)));
                        }
                    }
                }

                return Ok(None);
            }
        }
    }

    /// Request a new connection from the authentication flow.
    async fn request_new_connection(&self, setbac: &Setbac) -> Result<Option<Connection>, Error> {
        log::trace!("{}: Requesting new connection", self.what);
        Ok(setbac.get_connection(&self.flow_id).await?)
    }

    /// Validate a connection base on the current flow.
    async fn validate_connection(
        &self,
        setbac: &Setbac,
        connection: &Connection,
    ) -> Result<Validation, Error> {
        /// TODO: for some reason, this doesn't update :/
        let meta = match setbac.get_connection_meta(&self.flow_id).await? {
            Some(c) => c,
            None => return Ok(Validation::Ok),
        };

        if !self.is_outdated(connection, &meta)? {
            return Ok(Validation::Ok);
        }

        // try to refresh in case it has expired.
        self.refresh_connection(setbac).await
    }

    /// Refresh a connection.
    async fn refresh_connection(&self, setbac: &Setbac) -> Result<Validation, Error> {
        log::trace!("{}: Attempting to Refresh", self.what);

        let connection = match setbac.refresh_connection(&self.flow_id).await? {
            Some(connection) => connection,
            None => return Ok(Validation::Cleared),
        };

        Ok(Validation::Updated(connection))
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
            .field("new_connection", &self.new_connection)
            .field("connection", &self.connection)
            .finish()
    }
}

/// Setup a synchronized token and the future necessary to keep it up-to-date.
pub async fn build(
    flow_id: &str,
    what: &'static str,
    settings: Settings,
    injector: Injector,
    key: Key<SyncToken>,
) -> Result<(SyncToken, impl Future<Output = Result<(), Error>>), Error> {
    // connection expires within 30 minutes.
    let expires = Duration::from_secs(30 * 60);

    // queue used to force connection refreshes.
    let (force_refresh, mut force_refresh_rx) = mpsc::unbounded();

    let (mut connection_stream, connection) =
        settings.stream::<Connection>("connection").optional()?;
    let (mut setbac_stream, setbac) = injector.stream::<Setbac>();

    let mut builder = ConnectionFactory {
        setbac,
        flow_id: flow_id.to_string(),
        what,
        expires,
        force_refresh: false,
        new_connection: None,
        connection: None,
    };

    builder.new_connection = connection;

    // check for expirations.
    let mut check_interval = tokio::timer::Interval::new_interval(Duration::from_secs(30));

    let sync_token = SyncToken {
        what,
        inner: Default::default(),
        force_refresh,
    };

    let returned_sync_token = sync_token.clone();

    let update = move |connection| {
        sync_token.set(connection, &key, &injector);
    };

    if let Some(connection) = builder.log_build().await {
        log::trace!("{}: Storing new connection", what);
        settings.set_silent("connection", &connection)?;
        update(connection);
    }

    let future = async move {
        log::trace!("{}: Running loop", what);

        loop {
            futures::select! {
                setbac = setbac_stream.select_next_some() => {
                    builder.setbac = setbac;

                    if let Some(connection) = builder.log_build().await {
                        log::trace!("{}: New after client update", what);
                        settings.set_silent("connection", &connection)?;
                        update(connection);
                    }
                }
                connection = connection_stream.select_next_some() => {
                    log::trace!("{}: New from settings", what);

                    match connection {
                        Some(connection) => {
                            // force new connection to be validated.
                            builder.new_connection = Some(connection);

                            if let Some(connection) = builder.log_build().await {
                                log::trace!("{}: Updating in-memory connection", what);
                                update(connection);
                            }
                        }
                        None => {
                            // unset the existing connection to force a new authentication loop.
                            builder.connection = None;

                            if let Some(connection) = builder.log_build().await {
                                log::trace!("{}: Storing new connection", what);
                                settings.set_silent("connection", &connection)?;
                                update(connection);
                            }
                        }
                    }
                }
                current = force_refresh_rx.select_next_some() => {
                    log::trace!("{}: Forced refresh", what);

                    builder.force_refresh = true;

                    if let Some(connection) = builder.log_build().await {
                        log::trace!("{}: Storing new connection", what);
                        settings.set_silent("connection", &connection)?;
                        update(connection);
                    }
                }
                _ = check_interval.select_next_some() => {
                    log::trace!("{}: Check for expiration", what);

                    if let Some(connection) = builder.log_build().await {
                        log::trace!("{}: Storing new connection", what);
                        settings.set_silent("connection", &connection)?;
                        update(connection);
                    }
                }
            }
        }
    };

    Ok((returned_sync_token, future.boxed()))
}
