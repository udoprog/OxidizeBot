use crate::{prelude::*, settings, timer, web};
use chrono::{DateTime, Utc};
use failure::{bail, format_err, Error};
use oauth2::{
    basic::{BasicErrorField, BasicTokenResponse, BasicTokenType},
    AccessToken, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, RefreshToken, RequestTokenError, Scope, TokenResponse, TokenUrl,
};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::collections::VecDeque;
use std::fmt;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use url::Url;

static YOUTUBE_CLIENT_ID: &'static str =
    "520353465977-filfj4j326v5vvd4do07riej30ekin70.apps.googleusercontent.com";
static YOUTUBE_CLIENT_SECRET: &'static str = "8Rcs45nQEmruNey4-Egx7S7C";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SecretsConfig {
    client_id: String,
    client_secret: ClientSecret,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Type {
    #[serde(rename = "twitch")]
    Twitch,
    #[serde(rename = "spotify")]
    Spotify,
    #[serde(rename = "youtube")]
    YouTube,
}

impl Type {
    /// Refresh and save an updated version of the given token.
    pub async fn refresh_token(
        self,
        flow: &Arc<Flow>,
        refresh_token: RefreshToken,
    ) -> Result<Token, Error> {
        match self {
            Type::Twitch => {
                self.refresh_token_impl::<TwitchTokenResponse>(flow, refresh_token)
                    .await
            }
            Type::Spotify => {
                self.refresh_token_impl::<BasicTokenResponse>(flow, refresh_token)
                    .await
            }
            Type::YouTube => {
                self.refresh_token_impl::<BasicTokenResponse>(flow, refresh_token)
                    .await
            }
        }
    }

    /// Exchange and save a token based on a code.
    pub async fn exchange_token(
        self,
        flow: &Arc<Flow>,
        received_token: web::ReceivedToken,
    ) -> Result<Token, Error> {
        match self {
            Type::Twitch => {
                self.exchange_token_impl::<TwitchTokenResponse>(flow, received_token)
                    .await
            }
            Type::Spotify => {
                self.exchange_token_impl::<BasicTokenResponse>(flow, received_token)
                    .await
            }
            Type::YouTube => {
                self.exchange_token_impl::<BasicTokenResponse>(flow, received_token)
                    .await
            }
        }
    }

    /// Inner, typed implementation of executing a refresh.
    async fn refresh_token_impl<T>(
        self,
        flow: &Arc<Flow>,
        refresh_token: RefreshToken,
    ) -> Result<Token, Error>
    where
        T: 'static + Send + TokenResponse,
    {
        let future = flow
            .client
            .exchange_refresh_token(&refresh_token)
            .param("client_id", flow.secrets_config.client_id.as_str())
            .param(
                "client_secret",
                flow.secrets_config.client_secret.secret().as_str(),
            )
            .execute::<T>()
            .compat();

        let token_response = match future.await {
            Ok(t) => t,
            Err(RequestTokenError::Parse(_, res)) => {
                log::error!("bad token response: {}", String::from_utf8_lossy(&res));
                return Err(format_err!("bad response from server"));
            }
            Err(e) => return Err(Error::from(e)),
        };

        let refresh_token = token_response
            .refresh_token()
            .map(|r| r.clone())
            .unwrap_or(refresh_token);

        Ok(flow.new_token(refresh_token, token_response))
    }

    async fn exchange_token_impl<T>(
        self,
        flow: &Arc<Flow>,
        received_token: web::ReceivedToken,
    ) -> Result<Token, Error>
    where
        T: TokenResponse,
    {
        let client_id = flow.secrets_config.client_id.to_string();

        let exchange = flow
            .client
            .exchange_code(AuthorizationCode::new(received_token.code))
            .param("client_id", client_id.as_str())
            .param(
                "client_secret",
                flow.secrets_config.client_secret.secret().as_str(),
            );

        let token_response = exchange.execute::<T>().compat().await;

        let token_response = match token_response {
            Ok(t) => t,
            Err(RequestTokenError::Parse(_, res)) => {
                log::error!("bad token response: {}", String::from_utf8_lossy(&res));
                return Err(format_err!("bad response from server"));
            }
            Err(e) => return Err(Error::from(e)),
        };

        let refresh_token = match token_response.refresh_token() {
            Some(refresh_token) => refresh_token.clone(),
            None => bail!("did not receive a refresh token from the service"),
        };

        Ok(flow.new_token(refresh_token, token_response))
    }
}

enum Secrets {
    /// Dynamic secrets configuration.
    Config(Arc<SecretsConfig>),
    /// Static secrets configuration.
    Static {
        client_id: &'static str,
        client_secret: &'static str,
    },
}

/// Setup a Twitch authentication flow.
pub fn twitch(
    web: web::Server,
    settings: settings::ScopedSettings,
    secrets_config: Arc<SecretsConfig>,
) -> Result<FlowBuilder, Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
        ty: Type::Twitch,
        web,
        secrets: Secrets::Config(secrets_config),
        redirect_url: RedirectUrl::new(Url::parse(&redirect_url)?),
        auth_url: AuthUrl::new(Url::parse("https://id.twitch.tv/oauth2/authorize")?),
        token_url: Some(TokenUrl::new(Url::parse(
            "https://id.twitch.tv/oauth2/token",
        )?)),
        scopes: Default::default(),
        settings,
        extra_params: Default::default(),
    })
}

/// Setup a Spotify AUTH flow.
pub fn spotify(
    web: web::Server,
    settings: settings::ScopedSettings,
    secrets_config: Arc<SecretsConfig>,
) -> Result<FlowBuilder, Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
        ty: Type::Spotify,
        web,
        secrets: Secrets::Config(secrets_config),
        redirect_url: RedirectUrl::new(Url::parse(&redirect_url)?),
        auth_url: AuthUrl::new(Url::parse("https://accounts.spotify.com/authorize")?),
        token_url: Some(TokenUrl::new(Url::parse(
            "https://accounts.spotify.com/api/token",
        )?)),
        scopes: Default::default(),
        settings,
        extra_params: Default::default(),
    })
}

/// Setup a YouTube AUTH flow.
pub fn youtube(web: web::Server, settings: settings::ScopedSettings) -> Result<FlowBuilder, Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
        ty: Type::YouTube,
        web,
        secrets: Secrets::Static {
            client_id: YOUTUBE_CLIENT_ID,
            client_secret: YOUTUBE_CLIENT_SECRET,
        },
        redirect_url: RedirectUrl::new(Url::parse(&redirect_url)?),
        auth_url: AuthUrl::new(Url::parse("https://accounts.google.com/o/oauth2/v2/auth")?),
        token_url: Some(TokenUrl::new(Url::parse(
            "https://www.googleapis.com/oauth2/v4/token",
        )?)),
        scopes: Default::default(),
        settings,
        extra_params: vec![(String::from("access_type"), String::from("offline"))],
    })
}

/// A token that comes out of a token workflow.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Token {
    /// Client ID that requested the token.
    client_id: String,
    /// Store the known refresh token.
    refresh_token: RefreshToken,
    /// Access token.
    access_token: AccessToken,
    /// When the token was refreshed.
    refreshed_at: DateTime<Utc>,
    /// Expires in seconds.
    expires_in: Option<u64>,
    /// Scopes associated with token.
    scopes: Vec<Scope>,
}

impl Token {
    /// Get the client ID that requested the token.
    pub fn client_id(&self) -> &str {
        self.client_id.as_str()
    }

    /// Get the current access token.
    pub fn access_token(&self) -> &str {
        self.access_token.secret().as_str()
    }

    /// Return `true` if the token expires within 30 minutes.
    pub fn expires_within(&self, within: Duration) -> Result<bool, Error> {
        let out = match self.expires_in.clone() {
            Some(expires_in) => {
                let expires_in = chrono::Duration::seconds(expires_in as i64);
                let diff = (self.refreshed_at + expires_in) - Utc::now();
                diff < chrono::Duration::from_std(within)?
            }
            None => true,
        };

        Ok(out)
    }

    /// Test that token has all the specified scopes.
    pub fn has_scopes(&self, scopes: &[String]) -> bool {
        use hashbrown::HashSet;

        let mut scopes = scopes
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>();

        for s in &self.scopes {
            scopes.remove(s.as_ref());
        }

        scopes.is_empty()
    }
}

#[derive(Debug, err_derive::Error)]
#[error(display = "Missing OAuth 2.0 Token: {0}", _0)]
pub struct MissingTokenError(String);

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
    /// Flow associated with token.
    flow: Arc<Flow>,
    inner: Arc<RwLock<SyncTokenInner>>,
    /// Channel to use to force a refresh.
    force_refresh: mpsc::UnboundedSender<Option<Token>>,
}

impl SyncToken {
    /// Set the token and notify all waiters.
    pub fn set(&self, update: Option<Token>) {
        let mut lock = self.inner.write();

        let SyncTokenInner {
            ref mut token,
            ref mut ready_queue,
        } = *lock;

        *token = update;

        // send ready notifications if we updated the token.
        if token.is_some() {
            while let Some(front) = ready_queue.pop_front() {
                if let Err(()) = front.send(()) {
                    log::warn!("tried to send ready notification but failed");
                }
            }
        }
    }

    /// Force a token refresh.
    pub fn force_refresh(&self) -> Result<(), Error> {
        log::warn!("Forcing token refresh for: {}", self.flow.what);
        let token = self.inner.write().token.take();
        self.force_refresh.unbounded_send(token)?;
        Ok(())
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

        log::trace!("Waiting for token: {}", self.flow.what);

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
            Err(_) => Err(MissingTokenError(self.flow.what.clone())),
        }
    }
}

pub struct FlowBuilder {
    ty: Type,
    web: web::Server,
    secrets: Secrets,
    redirect_url: RedirectUrl,
    auth_url: AuthUrl,
    token_url: Option<TokenUrl>,
    scopes: Vec<String>,
    settings: settings::ScopedSettings,
    extra_params: Vec<(String, String)>,
}

impl FlowBuilder {
    /// Configure scopes for flow builder.
    pub fn with_scopes(self, scopes: Vec<String>) -> Self {
        Self { scopes, ..self }
    }

    /// Convert into an authentication flow.
    pub fn build(self, what: String) -> Result<Arc<Flow>, Error> {
        let secrets_config = match self.secrets {
            Secrets::Config(config) => config,
            Secrets::Static {
                client_id,
                client_secret,
            } => Arc::new(SecretsConfig {
                client_id: client_id.to_string(),
                client_secret: ClientSecret::new(client_secret.to_string()),
            }),
        };

        let mut client = Client::new(
            ClientId::new(secrets_config.client_id.to_string()),
            Some(secrets_config.client_secret.clone()),
            self.auth_url,
            self.token_url,
        );

        for scope in &self.scopes {
            client = client.add_scope(Scope::new(scope.to_string()));
        }

        client = client.set_redirect_url(self.redirect_url);

        Ok(Arc::new(Flow {
            ty: self.ty,
            web: self.web.clone(),
            secrets_config,
            client: Arc::new(client),
            settings: self.settings.clone(),
            scopes: Arc::new(self.scopes),
            extra_params: Arc::new(self.extra_params),
            what,
        }))
    }
}

pub struct Flow {
    ty: Type,
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
    client: Arc<Client>,
    settings: settings::ScopedSettings,
    scopes: Arc<Vec<String>>,
    extra_params: Arc<Vec<(String, String)>>,
    what: String,
}

impl fmt::Debug for Flow {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Flow")
            .field("ty", &self.ty)
            .field("secrets_config", &self.secrets_config)
            .field("scopes", &self.scopes)
            .field("extra_params", &self.extra_params)
            .field("what", &self.what)
            .finish()
    }
}

impl Flow {
    /// Convert the flow into a token.
    pub fn into_token(
        self: Arc<Flow>,
    ) -> Result<(SyncToken, impl Future<Output = Result<(), Error>>), Error> {
        // token expires within 30 minutes.
        let expires = Duration::from_secs(30 * 60);

        // queue used to force token refreshes.
        let (force_refresh, mut force_refresh_rx) = mpsc::unbounded();

        let sync_token = SyncToken {
            flow: self.clone(),
            inner: Default::default(),
            force_refresh,
        };

        let (mut token_stream, token) = self.settings.init_and_option_stream::<Token>("token")?;

        // check interval.
        let mut interval = timer::Interval::new(Instant::now(), Duration::from_secs(10 * 60));

        let returned_sync_token = sync_token.clone();
        let flow = self;

        let future = async move {
            log::trace!("Running loop for token: {}", flow.what);

            // Initial token request.
            let mut new_token = match token {
                // Existing but expired token, refresh.
                Some(ref token) if token.expires_within(expires.clone())? => {
                    Some(flow.refresh(token.refresh_token.clone()).boxed())
                }
                // No existing token, request a new one.
                None => Some(flow.request_new_token().boxed()),
                other => {
                    // set the SyncToken to its initial value.
                    sync_token.set(other);
                    None
                }
            };

            loop {
                futures::select! {
                    current = force_refresh_rx.select_next_some() => {
                        if new_token.is_some() {
                            log::warn!("ignoring refresh request since a token request is already in progress");
                            continue;
                        }

                        new_token = Some(match current {
                            Some(ref current) => {
                                flow.refresh(current.refresh_token.clone()).boxed()
                            }
                            _ => {
                                flow.request_new_token().boxed()
                            }
                        });
                    }
                    token = token_stream.select_next_some() => {
                        let token = match token {
                            Some(token) => flow.validate_token(token).await?,
                            None => None,
                        };

                        match (&token, &new_token) {
                            // a token, and a pending request.
                            (Some(..), Some(..)) => new_token = None,
                            // no token, and no pending request.
                            (None, None) => {
                                new_token = Some(flow.request_new_token().boxed());
                            }
                            _ => (),
                        };

                        sync_token.set(token);
                    }
                    _ = interval.select_next_some() => {
                        if new_token.as_ref().is_some() {
                            continue;
                        }

                        new_token = match sync_token.inner.read().token.as_ref() {
                            Some(current) if current.expires_within(expires.clone())? => {
                                Some(flow.refresh(current.refresh_token.clone()).boxed())
                            }
                            _ => None,
                        };
                    }
                    token = new_token.current() => {
                        match token {
                            Ok(token) => {
                                // NB: will invoke token_stream.
                                flow.settings.set("token", &token)?;
                            }
                            Err(e) => {
                                log_err!(e, "failed to request new token");
                                new_token = Some(flow.request_new_token().boxed());
                            }
                        }
                    }
                }
            }
        };

        Ok((returned_sync_token, future))
    }

    /// Request a new token from the authentication flow.
    async fn request_new_token(self: &Arc<Flow>) -> Result<Token, Error> {
        log::trace!("Requesting new token: {}", self.what);

        let (mut auth_url, csrf_token) = self.client.authorize_url(CsrfToken::new_random);

        for (key, value) in self.extra_params.iter() {
            auth_url.query_pairs_mut().append_pair(key, value);
        }

        let received = self
            .web
            .receive_token(auth_url, self.what.clone(), csrf_token.secret().to_string())
            .await;

        let received_token = match received {
            Ok(received_token) => received_token,
            Err(oneshot::Canceled) => bail!("token received cancelled"),
        };

        if *csrf_token.secret() != received_token.state {
            bail!("CSRF Token Mismatch");
        }

        self.ty.exchange_token(self, received_token).await
    }

    /// Validate a token base on the current flow.
    async fn validate_token(self: &Arc<Flow>, token: Token) -> Result<Option<Token>, Error> {
        if token.client_id != self.secrets_config.client_id {
            log::warn!("Not using stored token since it uses a different Client ID");
            return Ok(None);
        }

        if !token.has_scopes(&self.scopes) {
            return Ok(None);
        }

        let expired = match token.expires_within(Duration::from_secs(60 * 10)) {
            Ok(expired) => expired,
            Err(e) => return Err(e),
        };

        // try to refresh in case it has expired.
        if expired {
            log::info!("Attempting to refresh: {}", self.what);

            return Ok(match self.refresh(token.refresh_token.clone()).await {
                Ok(token) => Some(token),
                Err(e) => {
                    log::warn!("Failed to refresh saved token: {}", e);
                    None
                }
            });
        }

        Ok(Some(token))
    }

    /// Refresh the token.
    pub async fn refresh(self: &Arc<Flow>, refresh_token: RefreshToken) -> Result<Token, Error> {
        self.ty.refresh_token(self, refresh_token).await
    }

    /// Save and return the given token.
    fn new_token(
        self: &Arc<Flow>,
        refresh_token: RefreshToken,
        token_response: impl TokenResponse,
    ) -> Token {
        let refreshed_at = Utc::now();

        Token {
            client_id: self.secrets_config.client_id.to_string(),
            refresh_token,
            access_token: token_response.access_token().clone(),
            refreshed_at: refreshed_at.clone(),
            expires_in: token_response.expires_in().map(|e| e.as_secs()),
            scopes: token_response
                .scopes()
                .map(|s| s.clone())
                .unwrap_or_default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TwitchTokenResponse {
    access_token: AccessToken,
    #[serde(deserialize_with = "oauth2::helpers::deserialize_untagged_enum_case_insensitive")]
    token_type: BasicTokenType,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<RefreshToken>,
    #[serde(rename = "scope")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    scopes: Option<Vec<Scope>>,
}

impl TokenResponse for TwitchTokenResponse {
    type TokenType = BasicTokenType;
    type ErrorField = BasicErrorField;

    fn access_token(&self) -> &AccessToken {
        &self.access_token
    }

    fn token_type(&self) -> &BasicTokenType {
        &self.token_type
    }

    fn expires_in(&self) -> Option<Duration> {
        self.expires_in.map(Duration::from_secs)
    }

    fn refresh_token(&self) -> Option<&RefreshToken> {
        self.refresh_token.as_ref()
    }

    fn scopes(&self) -> Option<&Vec<Scope>> {
        self.scopes.as_ref()
    }
}
