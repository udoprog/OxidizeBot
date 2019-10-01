use crate::{
    api::{setbac::Token, Setbac},
    injector::{Injector, Key},
    prelude::*,
    settings::Settings,
};
use failure::Error;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use serde::Serialize;
use std::collections::VecDeque;
use std::fmt;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Token identifier used for dependency injection.
#[derive(Debug, Clone, Serialize)]
pub enum TokenId {
    TwitchStreamer,
    TwitchBot,
    YouTube,
    NightBot,
    Spotify,
}

#[derive(Debug, err_derive::Error)]
#[error(display = "Missing OAuth 2.0 Token: {0}", _0)]
pub struct MissingTokenError(&'static str);

#[derive(Debug, err_derive::Error)]
#[error(display = "Token receive was cancelled")]
pub struct CancelledToken;

#[derive(Debug, Default)]
pub struct SyncTokenInner {
    /// Stored token.
    token: Option<Token>,
    /// Queue to notify when a token is available.
    ready_queue: VecDeque<oneshot::Sender<()>>,
}

#[derive(Clone, Debug)]
pub struct SyncToken {
    /// Name of the flow associated with token.
    what: &'static str,
    inner: Arc<RwLock<SyncTokenInner>>,
    /// Channel to use to force a refresh.
    force_refresh: mpsc::UnboundedSender<Option<Token>>,
}

impl SyncToken {
    /// Set the token and notify all waiters.
    pub fn set(&self, update: Option<Token>, key: &Key<SyncToken>, injector: &Injector) {
        let mut lock = self.inner.write();

        let SyncTokenInner {
            ref mut token,
            ref mut ready_queue,
        } = *lock;

        let token_was_some = token.is_some();
        let update_is_some = update.is_some();

        *token = update;

        // send ready notifications if we updated the token.
        if token.is_some() {
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

    /// Force a token refresh.
    pub fn force_refresh(&self) -> Result<(), Error> {
        log::warn!("Forcing token refresh for: {}", self.what);
        let token = self.inner.write().token.take();
        self.force_refresh.unbounded_send(token)?;
        Ok(())
    }

    /// Check if token is ready.
    pub fn is_ready(&self) -> bool {
        self.inner.read().token.is_some()
    }

    /// Wait until an underlying token is available.
    pub async fn wait_until_ready(&self) -> Result<(), CancelledToken> {
        let rx = {
            let mut lock = self.inner.write();

            let SyncTokenInner {
                ref token,
                ref mut ready_queue,
            } = *lock;

            if token.is_some() {
                return Ok(());
            }

            let (tx, rx) = oneshot::channel();
            ready_queue.push_back(tx);
            rx
        };

        log::trace!("Waiting for token: {}", self.what);

        match rx.await {
            Ok(()) => Ok(()),
            Err(oneshot::Canceled) => Err(CancelledToken),
        }
    }

    /// Read the synchronized token.
    ///
    /// This results in an error if there is no token to read.
    pub fn read<'a>(&'a self) -> Result<MappedRwLockReadGuard<'a, Token>, MissingTokenError> {
        match RwLockReadGuard::try_map(self.inner.read(), |i| i.token.as_ref()) {
            Ok(guard) => Ok(guard),
            Err(_) => Err(MissingTokenError(self.what)),
        }
    }
}

struct TokenBuilder {
    setbac: Option<Setbac>,
    flow_id: String,
    what: &'static str,
    expires: Duration,
    force_refresh: bool,
    initial: bool,
    initial_token: Option<Token>,
    new_token: Option<Token>,
    token: Option<Token>,
}

impl TokenBuilder {
    /// Construct a new token and log on failures.
    pub async fn log_build(&mut self) -> Option<Option<Token>> {
        match self.build().await {
            Ok(token) => token,
            Err(e) => {
                log::error!("{}: Failed to build token: {}", self.what, e);
                None
            }
        }
    }

    /// Construct a new token.
    pub async fn build(&mut self) -> Result<Option<Option<Token>>, Error> {
        let setbac = match self.setbac.as_ref() {
            Some(setbac) => setbac,
            _ => return Ok(None),
        };

        if let Some(token) = self.initial_token.take() {
            log::trace!("{}: Validating initial token", self.what);

            let token = self.validate_token(setbac, token).await?;
            self.token = token.clone();

            if let Some(token) = token.as_ref() {
                return Ok(Some(Some(token.clone())));
            }
        }

        if let Some(token) = self.new_token.take() {
            log::trace!("{}: Validating new token", self.what);
            let new_token = self.validate_token(setbac, token).await?;

            if let Some(token) = new_token {
                self.token = Some(token);
            }
        }

        let expires = self.expires.clone();

        loop {
            let update = match self.token.as_ref() {
                // existing expired token.
                Some(token) if token.expires_within(expires)? || self.force_refresh => {
                    self.force_refresh = false;

                    let result = setbac.refresh_token(&self.flow_id).await;

                    let new_token = match result {
                        Ok(Some(new_token)) => new_token,
                        Ok(None) => {
                            log::warn!(
                                "{}: Failed to refresh token since no connection is available",
                                self.what,
                            );
                            self.token = None;
                            continue;
                        }
                        Err(e) => {
                            log::warn!("{}: Failed to refresh token: {}", self.what, e);
                            self.token = None;
                            continue;
                        }
                    };

                    let new_token = match self.validate_token(setbac, new_token).await? {
                        Some(new_token) => new_token,
                        None => {
                            self.token = None;
                            continue;
                        }
                    };

                    self.token = Some(new_token.clone());
                    Some(Some(new_token))
                }
                // Existing token is fine.
                Some(..) => None,
                // No existing token, request a new one.
                None => {
                    if let Some(new_token) = self.request_new_token(setbac).await? {
                        let new_token = self.validate_token(setbac, new_token).await?;
                        self.token = new_token.clone();
                        Some(new_token)
                    } else {
                        log::warn!("{}: No connection configured", self.what);
                        None
                    }
                }
            };

            self.initial = false;
            return Ok(update);
        }
    }

    /// Request a new token from the authentication flow.
    async fn request_new_token(&self, setbac: &Setbac) -> Result<Option<Token>, Error> {
        log::trace!("{}: Requesting new token", self.what);
        let token = setbac.get_token(&self.flow_id).await?;
        Ok(token)
    }

    /// Validate a token base on the current flow.
    async fn validate_token(&self, setbac: &Setbac, token: Token) -> Result<Option<Token>, Error> {
        let expired = match token.expires_within(Duration::from_secs(60 * 10)) {
            Ok(expired) => expired,
            Err(e) => return Err(e),
        };

        // try to refresh in case it has expired.
        if expired {
            log::trace!("{}: Attempting to Refresh", self.what);

            let future = setbac.refresh_token(&self.flow_id).await;

            return Ok(match future {
                Ok(Some(token)) => Some(token),
                Ok(None) => {
                    log::warn!(
                        "{}: Failed to refresh token since no connection is available",
                        self.what
                    );
                    None
                }
                Err(e) => {
                    log::warn!("{}: Failed to refresh saved token: {}", self.what, e);
                    None
                }
            });
        }

        Ok(Some(token))
    }
}

impl fmt::Debug for TokenBuilder {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("TokenBuilder")
            .field("flow_id", &self.flow_id)
            .field("what", &self.what)
            .field("expires", &self.expires)
            .field("force_refresh", &self.force_refresh)
            .field("initial", &self.initial)
            .field("initial_token", &self.initial_token)
            .field("new_token", &self.new_token)
            .field("token", &self.token)
            .finish()
    }
}

/// Convert the flow into a token.
pub fn new_token(
    flow_id: &str,
    what: &'static str,
    settings: Settings,
    injector: Injector,
    key: Key<SyncToken>,
) -> Result<(SyncToken, impl Future<Output = Result<(), Error>>), Error> {
    // token expires within 30 minutes.
    let expires = Duration::from_secs(30 * 60);

    // queue used to force token refreshes.
    let (force_refresh, mut force_refresh_rx) = mpsc::unbounded();

    let (mut token_stream, token) = settings.stream::<Token>("token").optional()?;
    let (mut setbac_stream, setbac) = injector.stream::<Setbac>();

    let mut builder = TokenBuilder {
        setbac,
        flow_id: flow_id.to_string(),
        what,
        expires,
        force_refresh: false,
        initial: true,
        initial_token: None,
        new_token: None,
        token: None,
    };

    builder.initial_token = token;

    // check interval.
    let mut interval = tokio::timer::Interval::new(Instant::now(), Duration::from_secs(10 * 60));

    let sync_token = SyncToken {
        what,
        inner: Default::default(),
        force_refresh,
    };

    let returned_sync_token = sync_token.clone();

    let future = async move {
        log::trace!("{}: Running loop", what);

        let update = move |token| {
            sync_token.set(token, &key, &injector);
        };

        if let Some(token) = builder.log_build().await {
            log::trace!("{}: Storing new token", what);
            settings.set_silent("token", &token)?;
            update(token);
        }

        loop {
            futures::select! {
                setbac = setbac_stream.select_next_some() => {
                    builder.setbac = setbac;

                    if let Some(token) = builder.log_build().await {
                        log::trace!("{}: New after client update", what);
                        settings.set_silent("token", &token)?;
                        update(token);
                    }
                }
                token = token_stream.select_next_some() => {
                    log::trace!("{}: New from settings", what);

                    match token {
                        Some(token) => {
                            // force new token to be validated.
                            builder.new_token = Some(token);

                            if let Some(token) = builder.log_build().await {
                                log::trace!("{}: Updating in-memory token", what);
                                update(token);
                            }
                        }
                        None => {
                            // unset the existing token to force a new authentication loop.
                            builder.token = None;

                            if let Some(token) = builder.log_build().await {
                                log::trace!("{}: Storing new token", what);
                                settings.set_silent("token", &token)?;
                                update(token);
                            }
                        }
                    }
                }
                current = force_refresh_rx.select_next_some() => {
                    log::trace!("{}: Forced refresh", what);

                    builder.force_refresh = true;

                    if let Some(token) = builder.log_build().await {
                        log::trace!("{}: Storing new token", what);
                        settings.set_silent("token", &token)?;
                        update(token);
                    }
                }
                _ = interval.select_next_some() => {
                    log::trace!("{}: Check for expiration", what);

                    if let Some(token) = builder.log_build().await {
                        log::trace!("{}: Storing new token", what);
                        settings.set_silent("token", &token)?;
                        update(token);
                    }
                }
            }
        }
    };

    Ok((returned_sync_token, future.boxed()))
}
