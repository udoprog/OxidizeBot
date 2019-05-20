use crate::{settings, utils::BoxFuture, web};
use chrono::{DateTime, Utc};
use failure::{format_err, ResultExt};
use futures::{future, sync::oneshot, Async, Future, Poll, Stream as _};
use oauth2::{
    basic::{BasicErrorField, BasicTokenResponse, BasicTokenType},
    AccessToken, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, RefreshToken, RequestTokenError, Scope, TokenResponse, TokenUrl,
};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::timer;
use url::Url;

static YOUTUBE_CLIENT_ID: &'static str =
    "520353465977-filfj4j326v5vvd4do07riej30ekin70.apps.googleusercontent.com";
static YOUTUBE_CLIENT_SECRET: &'static str = "8Rcs45nQEmruNey4-Egx7S7C";

pub type AuthPair = (SyncToken, TokenRefreshFuture);

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SecretsConfig {
    client_id: String,
    client_secret: String,
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
    pub fn refresh_and_save_token(
        self,
        flow: Arc<Flow>,
        token: &Token,
    ) -> BoxFuture<Token, failure::Error> {
        match self {
            Type::Twitch => self.refresh_and_save_token_impl::<TwitchTokenResponse>(flow, token),
            Type::Spotify => self.refresh_and_save_token_impl::<BasicTokenResponse>(flow, token),
            Type::YouTube => self.refresh_and_save_token_impl::<BasicTokenResponse>(flow, token),
        }
    }

    /// Exchange and save a token based on a code.
    pub fn exchange_and_save_token(
        self,
        flow: Arc<Flow>,
        received_token: web::ReceivedToken,
    ) -> BoxFuture<AuthPair, failure::Error> {
        match self {
            Type::Twitch => Box::new(
                self.exchange_and_save_token_impl::<TwitchTokenResponse>(flow, received_token),
            ),
            Type::Spotify => Box::new(
                self.exchange_and_save_token_impl::<BasicTokenResponse>(flow, received_token),
            ),
            Type::YouTube => Box::new(
                self.exchange_and_save_token_impl::<BasicTokenResponse>(flow, received_token),
            ),
        }
    }

    /// Inner, typed implementation of executing a refresh.
    fn refresh_and_save_token_impl<T>(
        self,
        flow: Arc<Flow>,
        token: &Token,
    ) -> BoxFuture<Token, failure::Error>
    where
        T: 'static + Send + TokenResponse,
    {
        let refresh_token = token.refresh_token.clone();

        let future = flow
            .client
            .exchange_refresh_token(&refresh_token)
            .param("client_id", flow.secrets_config.client_id.as_str())
            .param("client_secret", flow.secrets_config.client_secret.as_str())
            .execute::<T>();

        let future = future.then(|token_response| match token_response {
            Ok(t) => Ok(t),
            Err(RequestTokenError::Parse(_, res)) => {
                log::error!("bad token response: {}", String::from_utf8_lossy(&res));
                Err(format_err!("bad response from server"))
            }
            Err(e) => Err(failure::Error::from(e)),
        });

        let future = future.and_then({
            let flow = flow.clone();

            move |token_response| {
                let refresh_token = token_response
                    .refresh_token()
                    .map(|r| r.clone())
                    .unwrap_or(refresh_token);

                flow.save_token(refresh_token, token_response)
            }
        });

        Box::new(future)
    }

    fn exchange_and_save_token_impl<T>(
        self,
        flow: Arc<Flow>,
        received_token: web::ReceivedToken,
    ) -> impl Future<Item = AuthPair, Error = failure::Error>
    where
        T: TokenResponse,
    {
        let client_id = flow.secrets_config.client_id.to_string();

        let exchange = flow
            .client
            .exchange_code(AuthorizationCode::new(received_token.code))
            .param("client_id", client_id.as_str())
            .param("client_secret", flow.secrets_config.client_secret.as_str());

        exchange.execute::<T>().then(move |token_response| {
            let token_response = match token_response {
                Ok(t) => t,
                Err(RequestTokenError::Parse(_, res)) => {
                    log::error!("bad token response: {}", String::from_utf8_lossy(&res));
                    return Err(format_err!("bad response from server"));
                }
                Err(e) => return Err(failure::Error::from(e)),
            };

            let refresh_token = match token_response.refresh_token() {
                Some(refresh_token) => refresh_token.clone(),
                None => failure::bail!("did not receive a refresh token from the service"),
            };

            let token = flow.save_token(refresh_token, token_response)?;
            let sync_token = SyncToken {
                token: Arc::new(RwLock::new(Some(token))),
            };

            Ok((
                sync_token.clone(),
                TokenRefreshFuture::new(flow, sync_token),
            ))
        })
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
) -> Result<FlowBuilder, failure::Error> {
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
) -> Result<FlowBuilder, failure::Error> {
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
pub fn youtube(
    web: web::Server,
    settings: settings::ScopedSettings,
) -> Result<FlowBuilder, failure::Error> {
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
    pub fn expires_within(&self, within: Duration) -> Result<bool, failure::Error> {
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
#[error(display = "Missing OAuth 2.0 Token")]
pub struct MissingTokenError;

#[derive(Clone, Debug)]
pub struct SyncToken {
    /// Serialized token token.
    token: Arc<RwLock<Option<Token>>>,
}

impl SyncToken {
    /// Read the synchronized token.
    ///
    /// This results in an error if there is no token to read.
    pub fn read<'a>(&'a self) -> Result<MappedRwLockReadGuard<'a, Token>, MissingTokenError> {
        match RwLockReadGuard::try_map(self.token.read(), Option::as_ref) {
            Ok(guard) => Ok(guard),
            Err(_) => Err(MissingTokenError),
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
    pub fn build(self) -> Result<Arc<Flow>, failure::Error> {
        let secrets_config = match self.secrets {
            Secrets::Config(config) => config,
            Secrets::Static {
                client_id,
                client_secret,
            } => Arc::new(SecretsConfig {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
            }),
        };

        let mut client = Client::new(
            ClientId::new(secrets_config.client_id.to_string()),
            Some(ClientSecret::new(secrets_config.client_secret.to_string())),
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
}

impl Flow {
    /// Execute the flow.
    pub fn execute(self: Arc<Self>, what: &str) -> BoxFuture<AuthPair, failure::Error> {
        let future = self.clone().token_from_settings(what);

        let future = future.and_then::<_, BoxFuture<AuthPair, failure::Error>>({
            let what = what.to_string();
            let flow = self.clone();

            move |token| match token {
                Some(token) => {
                    let sync_token = SyncToken {
                        token: Arc::new(RwLock::new(Some(token))),
                    };

                    return Box::new(future::ok((
                        sync_token.clone(),
                        TokenRefreshFuture::new(flow, sync_token),
                    )));
                }
                None => Box::new(flow.request_new_token(what)),
            }
        });

        Box::new(future)
    }

    /// Request a new token.
    fn request_new_token(
        self: Arc<Self>,
        what: String,
    ) -> impl Future<Item = AuthPair, Error = failure::Error> {
        let (mut auth_url, csrf_token) = self.client.authorize_url(CsrfToken::new_random);

        for (key, value) in self.extra_params.iter() {
            auth_url.query_pairs_mut().append_pair(key, value);
        }

        let future =
            self.web
                .receive_token(auth_url, what.to_string(), csrf_token.secret().to_string());

        let future = future
            .map_err(|oneshot::Canceled| format_err!("token receive cancelled"))
            .and_then(move |received_token| {
                if *csrf_token.secret() != received_token.state {
                    failure::bail!("CSRF Token Mismatch");
                }

                Ok(received_token)
            });

        future.and_then({
            let flow = self.clone();
            move |received_token| flow.ty.exchange_and_save_token(flow, received_token)
        })
    }

    /// Load a token from settings.
    fn token_from_settings(
        self: Arc<Self>,
        what: &str,
    ) -> BoxFuture<Option<Token>, failure::Error> {
        let token = match self.settings.get::<Token>("token") {
            Ok(token) => token,
            Err(e) => {
                log::warn!("failed to load saved token: {}", e);
                return Box::new(future::ok(None));
            }
        };

        let token = match token {
            Some(token) => token,
            None => return Box::new(future::ok(None)),
        };

        self.stored_token(token, what)
    }

    /// Validate a token base on the current flow.
    fn stored_token(
        self: Arc<Self>,
        token: Token,
        what: &str,
    ) -> BoxFuture<Option<Token>, failure::Error> {
        if token.client_id == self.secrets_config.client_id {
            log::warn!("Not using stored token since it uses a different Client ID");
            return Box::new(future::ok(None));
        }

        if !token.has_scopes(&self.scopes) {
            return Box::new(future::ok(None));
        }

        let expired = match token.expires_within(Duration::from_secs(60 * 10)) {
            Ok(expired) => expired,
            Err(e) => return Box::new(future::err(e)),
        };

        // try to refresh in case it has expired.
        if expired {
            log::info!("Attempting to refresh: {}", what);

            return Box::new(self.refresh(&token).map(Some).or_else(|e| {
                log::warn!("Failed to refresh saved token: {}", e);
                Ok(None)
            }));
        }

        Box::new(future::ok(Some(token)))
    }

    /// Refresh the token.
    pub fn refresh(self: Arc<Self>, token: &Token) -> BoxFuture<Token, failure::Error> {
        self.ty.refresh_and_save_token(self, token)
    }

    /// Save and return the given token.
    fn save_token(
        &self,
        refresh_token: RefreshToken,
        token_response: impl TokenResponse,
    ) -> Result<Token, failure::Error> {
        let refreshed_at = Utc::now();

        let token = Token {
            client_id: self.secrets_config.client_id.to_string(),
            refresh_token,
            access_token: token_response.access_token().clone(),
            refreshed_at: refreshed_at.clone(),
            expires_in: token_response.expires_in().map(|e| e.as_secs()),
            scopes: token_response
                .scopes()
                .map(|s| s.clone())
                .unwrap_or_default(),
        };

        self.settings
            .set("token", &token)
            .with_context(|_| failure::format_err!("failed to write token to"))?;

        Ok(token)
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

/// Future used to drive token refreshes.
pub struct TokenRefreshFuture {
    flow: Arc<Flow>,
    sync_token: SyncToken,
    interval: timer::Interval,
    refresh_duration: Duration,
    refresh_future: Option<BoxFuture<(), failure::Error>>,
}

impl TokenRefreshFuture {
    /// Construct a new future for refreshing oauth tokens.
    pub fn new(flow: Arc<Flow>, sync_token: SyncToken) -> Self {
        // check for expiration every 10 minutes.
        let check_duration = Duration::from_secs(10 * 60);
        // refresh if token expires within 30 minutes.
        let refresh_duration = Duration::from_secs(30 * 60);

        Self {
            flow,
            sync_token,
            interval: timer::Interval::new(Instant::now(), check_duration),
            refresh_duration,
            refresh_future: None,
        }
    }
}

impl Future for TokenRefreshFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let mut not_ready = true;

            if let Some(mut future) = self.refresh_future.take() {
                match future.poll() {
                    Ok(Async::NotReady) => self.refresh_future = Some(future),
                    Ok(Async::Ready(())) => {
                        self.refresh_future = None;
                        not_ready = false;
                    }
                    Err(e) => {
                        log::warn!("failed to refresh token: {}", e);
                        self.refresh_future = None;
                        not_ready = false;
                    }
                }
            }

            if let Some(_) = try_infinite!(self.interval.poll()) {
                if self.refresh_future.is_some() {
                    return Err(format_err!("refresh already in progress"));
                }

                let refresh = match self.sync_token.token.read().as_ref() {
                    Some(current) if current.expires_within(self.refresh_duration.clone())? => {
                        Some(self.flow.clone().refresh(current))
                    }
                    _ => None,
                };

                if let Some(refresh) = refresh {
                    let sync_token = self.sync_token.clone();
                    self.refresh_future = Some(Box::new(refresh.map(move |token| {
                        *sync_token.token.write() = Some(token);
                    })));
                    not_ready = false;
                }
            }

            if not_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}
