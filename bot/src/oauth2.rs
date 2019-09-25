use crate::{
    injector::{Injector, Key},
    prelude::*,
    settings::Settings,
    web,
};
use chrono::{DateTime, Utc};
use failure::{bail, format_err, Error};
use oauth2::{
    AccessToken, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, RefreshToken, RequestTokenError, Scope, StandardTokenResponse, TokenResponse,
    TokenType, TokenUrl,
};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::collections::VecDeque;
use std::fmt;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use url::Url;

/// Token identifier used for dependency injection.
#[derive(Debug, Clone, serde::Serialize)]
pub enum TokenId {
    TwitchStreamer,
    TwitchBot,
    YouTube,
    NightBot,
    Spotify,
}

/// Note:
/// These values obviously aren't secret. But due to the nature of this project, it's not possible to keep them that way.
/// Anyone can effectively use this information to impersonate OxidizeBot.
///
/// We protect against abuse the following ways:
/// * We only permit closed redirects to http://localhost:12345/redirect to receive the token.
///   - This makes it effectively useless to use for online services trying to impersonate OxidizeBot.
/// * We assume that a user would pay attention when downloading and running an application.
///   - If they don't, they have bigger problems on their hands.
static YOUTUBE_CLIENT_ID: &'static str =
    "652738525380-q3568vdosqg0g5h7p7ea5nrodm5doih3.apps.googleusercontent.com";
static YOUTUBE_CLIENT_SECRET: &'static str = "NrBvCdjodPnMl4_dVytRKaCA";

static NIGHTBOT_CLIENT_ID: &'static str = "cb1295f19d5bf423b82705808cc7df67";
static NIGHTBOT_CLIENT_SECRET: &'static str = "92f0ba5e9e4efdaab8cec43fb265d91e";

static TWITCH_CLIENT_ID: &'static str = "0n6sfb4ucob1djsb1owk2pwy5hto19";
static TWITCH_CLIENT_SECRET: &'static str = "h7o98dbp8gnq2wmfwa00e1gc5w280e";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
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
    #[serde(rename = "nightbot")]
    NightBot,
}

impl Type {
    /// Refresh and save an updated version of the given token.
    pub async fn refresh_token<'a>(
        self,
        config: &'a Config,
        client: &'a Client,
        refresh_token: RefreshToken,
    ) -> Result<Token, Error> {
        match self {
            Type::Twitch => {
                self.refresh_token_impl::<TwitchTokenResponse>(config, client, refresh_token)
                    .await
            }
            Type::Spotify => {
                self.refresh_token_impl::<StandardTokenResponse>(config, client, refresh_token)
                    .await
            }
            Type::YouTube => {
                self.refresh_token_impl::<StandardTokenResponse>(config, client, refresh_token)
                    .await
            }
            Type::NightBot => {
                self.refresh_token_impl::<StandardTokenResponse>(config, client, refresh_token)
                    .await
            }
        }
    }

    /// Exchange and save a token based on a code.
    pub async fn exchange_token<'a>(
        self,
        config: &'a Config,
        client: &'a Client,
        received_token: web::ReceivedToken,
    ) -> Result<Token, Error> {
        match self {
            Type::Twitch => {
                self.exchange_token_impl::<TwitchTokenResponse>(config, client, received_token)
                    .await
            }
            Type::Spotify => {
                self.exchange_token_impl::<StandardTokenResponse>(config, client, received_token)
                    .await
            }
            Type::YouTube => {
                self.exchange_token_impl::<StandardTokenResponse>(config, client, received_token)
                    .await
            }
            Type::NightBot => {
                self.exchange_token_impl::<StandardTokenResponse>(config, client, received_token)
                    .await
            }
        }
    }

    /// Inner, typed implementation of executing a refresh.
    async fn refresh_token_impl<'a, T>(
        self,
        config: &'a Config,
        client: &'a Client,
        refresh_token: RefreshToken,
    ) -> Result<Token, Error>
    where
        T: 'static + Send + TokenResponse,
    {
        let token_response = client
            .exchange_refresh_token(&refresh_token)
            .param("client_id", config.client_id.as_str())
            .param("client_secret", config.client_secret.secret().as_str())
            .execute::<T>()
            .await;

        let token_response = match token_response {
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

        let token = new_token(&config.client_id, refresh_token, token_response);
        Ok(token)
    }

    async fn exchange_token_impl<'a, T>(
        self,
        config: &'a Config,
        client: &'a Client,
        received_token: web::ReceivedToken,
    ) -> Result<Token, Error>
    where
        T: TokenResponse,
    {
        let token_response = client
            .exchange_code(AuthorizationCode::new(received_token.code))
            .param("client_id", config.client_id.as_str())
            .param("client_secret", config.client_secret.secret().as_str())
            .execute::<T>()
            .await;

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

        let token = new_token(config.client_id.as_str(), refresh_token, token_response);
        Ok(token)
    }
}

/// Save and return the given token.
fn new_token(
    client_id: &str,
    refresh_token: RefreshToken,
    token_response: impl TokenResponse,
) -> Token {
    let refreshed_at = Utc::now();

    Token {
        client_id: client_id.to_string(),
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

enum Secrets {
    /// Get secrets from settings.
    Settings(Settings),
    /// Static secrets configuration.
    Static {
        client_id: &'static str,
        client_secret: &'static str,
    },
}

/// Setup a Twitch authentication flow.
pub fn twitch(web: web::Server, settings: Settings) -> Result<FlowBuilder, Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
        ty: Type::Twitch,
        web,
        secrets: Secrets::Static {
            client_id: TWITCH_CLIENT_ID,
            client_secret: TWITCH_CLIENT_SECRET,
        },
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
    settings: Settings,
    shared_settings: Settings,
) -> Result<FlowBuilder, Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
        ty: Type::Spotify,
        web,
        secrets: Secrets::Settings(shared_settings),
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
pub fn youtube(web: web::Server, settings: Settings) -> Result<FlowBuilder, Error> {
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

/// Setup a NightBot AUTH flow.
pub fn nightbot(web: web::Server, settings: Settings) -> Result<FlowBuilder, Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
        ty: Type::NightBot,
        web,
        secrets: Secrets::Static {
            client_id: NIGHTBOT_CLIENT_ID,
            client_secret: NIGHTBOT_CLIENT_SECRET,
        },
        redirect_url: RedirectUrl::new(Url::parse(&redirect_url)?),
        auth_url: AuthUrl::new(Url::parse("https://api.nightbot.tv/oauth2/authorize")?),
        token_url: Some(TokenUrl::new(Url::parse(
            "https://api.nightbot.tv/oauth2/token",
        )?)),
        scopes: Default::default(),
        settings,
        extra_params: vec![],
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
    /// Name of the flow associated with token.
    what: Arc<String>,
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
            Err(_) => Err(MissingTokenError(self.what.to_string())),
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
    settings: Settings,
    extra_params: Vec<(String, String)>,
}

impl FlowBuilder {
    /// Configure scopes for flow builder.
    pub fn with_scopes(self, scopes: Vec<String>) -> Self {
        Self { scopes, ..self }
    }

    /// Convert into an authentication flow.
    pub fn build(self, what: String) -> Result<Flow, Error> {
        Ok(Flow {
            ty: self.ty,
            web: self.web.clone(),
            redirect_url: self.redirect_url,
            auth_url: self.auth_url,
            token_url: self.token_url,
            secrets: self.secrets,
            settings: self.settings.clone(),
            scopes: self.scopes,
            extra_params: self.extra_params,
            what,
        })
    }
}

struct TokenBuilder {
    ty: Type,
    web: web::Server,
    what: Arc<String>,
    redirect_url: RedirectUrl,
    auth_url: AuthUrl,
    token_url: Option<TokenUrl>,
    scopes: Vec<String>,
    extra_params: Vec<(String, String)>,
    expires: Duration,
    force_refresh: bool,
    initial: bool,
    config: Option<Config>,
    new_config: Option<Config>,
    client: Option<Client>,
    initial_token: Option<Token>,
    new_token: Option<Token>,
    token: Option<Token>,
}

impl TokenBuilder {
    /// Construct a new token builder.
    pub fn new(
        ty: Type,
        web: web::Server,
        what: Arc<String>,
        redirect_url: RedirectUrl,
        auth_url: AuthUrl,
        token_url: Option<TokenUrl>,
        expires: Duration,
        scopes: Vec<String>,
        extra_params: Vec<(String, String)>,
    ) -> Self {
        Self {
            ty,
            web,
            what,
            redirect_url,
            auth_url,
            token_url,
            expires,
            scopes,
            extra_params,
            force_refresh: false,
            initial: true,
            config: None,
            new_config: None,
            client: None,
            initial_token: None,
            new_token: None,
            token: None,
        }
    }

    /// Construct a new client.
    pub fn client(&self, config: &Config) -> Result<Client, Error> {
        let mut client = Client::new(
            ClientId::new(config.client_id.to_string()),
            Some(config.client_secret.clone()),
            self.auth_url.clone(),
            self.token_url.clone(),
        )?;

        for scope in self.scopes.iter() {
            client = client.add_scope(Scope::new(scope.to_string()));
        }

        Ok(client.set_redirect_url(self.redirect_url.clone()))
    }

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
        if let Some(config) = self.new_config.take() {
            // on new secrets configuration we must invalidate the old token.
            self.token = None;
            self.client = Some(self.client(&config)?);
            self.config = Some(config);
        }

        let (client, config) = match (self.client.as_ref(), self.config.as_ref()) {
            (Some(client), Some(config)) => (client, config),
            _ => return Ok(None),
        };

        if let Some(token) = self.initial_token.take() {
            log::trace!("{}: Validating initial token", self.what);

            let token = self.validate_token(config, client, token).await?;
            self.token = token.clone();

            if let Some(token) = token.as_ref() {
                return Ok(Some(Some(token.clone())));
            }
        }

        if let Some(token) = self.new_token.take() {
            log::trace!("{}: Validating new token", self.what);
            let new_token = self.validate_token(config, client, token).await?;

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

                    let result = self
                        .ty
                        .refresh_token(config, client, token.refresh_token.clone())
                        .await;

                    let new_token = match result {
                        Ok(new_token) => new_token,
                        Err(e) => {
                            log::warn!("{}: Failed to refresh token: {}", self.what, e);
                            self.token = None;
                            continue;
                        }
                    };

                    let new_token = match self.validate_token(config, client, new_token).await? {
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
                    let new_token = self.request_new_token(config, client).await?;
                    let new_token = self.validate_token(config, client, new_token).await?;
                    self.token = new_token.clone();
                    Some(new_token)
                }
            };

            self.initial = false;
            return Ok(update);
        }
    }

    /// Request a new token from the authentication flow.
    async fn request_new_token<'a>(
        &'a self,
        config: &'a Config,
        client: &'a Client,
    ) -> Result<Token, Error> {
        log::trace!("{}: Requesting new token", self.what);

        let (mut auth_url, csrf_token) = client.authorize_url(CsrfToken::new_random);

        for (key, value) in self.extra_params.iter() {
            auth_url.query_pairs_mut().append_pair(key, value);
        }

        let received = self
            .web
            .receive_token(
                auth_url,
                self.what.to_string(),
                csrf_token.secret().to_string(),
            )
            .await;

        let received_token = match received {
            Ok(received_token) => received_token,
            Err(oneshot::Canceled) => bail!("token received cancelled"),
        };

        if *csrf_token.secret() != received_token.state {
            bail!("CSRF Token Mismatch");
        }

        self.ty.exchange_token(config, client, received_token).await
    }

    /// Validate a token base on the current flow.
    async fn validate_token<'a>(
        &'a self,
        config: &'a Config,
        client: &'a Client,
        token: Token,
    ) -> Result<Option<Token>, Error> {
        if token.client_id != config.client_id {
            log::warn!("Not using stored token since it uses a different Client ID");
            return Ok(None);
        }

        if !token.has_scopes(&self.scopes) {
            log::warn!("Rejecting new token since it doesn't have the appropriate scopes");
            return Ok(None);
        }

        let expired = match token.expires_within(Duration::from_secs(60 * 10)) {
            Ok(expired) => expired,
            Err(e) => return Err(e),
        };

        // try to refresh in case it has expired.
        if expired {
            log::trace!("{}: Attempting to Refresh", self.what);

            let future = self
                .ty
                .refresh_token(config, client, token.refresh_token.clone())
                .await;

            return Ok(match future {
                Ok(token) => Some(token),
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
            .field("ty", &self.ty)
            .field("what", &self.what)
            .field("redirect_url", &self.redirect_url)
            .field("auth_url", &self.auth_url)
            .field("token_url", &self.token_url)
            .field("scopes", &self.scopes)
            .field("extra_params", &self.extra_params)
            .field("expires", &self.expires)
            .field("force_refresh", &self.force_refresh)
            .field("initial", &self.initial)
            .field("config", &self.config)
            .field("new_config", &self.new_config)
            .field("client", &self.client)
            .field("initial_token", &self.initial_token)
            .field("new_token", &self.new_token)
            .field("token", &self.token)
            .finish()
    }
}

pub struct Flow {
    ty: Type,
    web: web::Server,
    redirect_url: RedirectUrl,
    auth_url: AuthUrl,
    token_url: Option<TokenUrl>,
    secrets: Secrets,
    settings: Settings,
    scopes: Vec<String>,
    extra_params: Vec<(String, String)>,
    what: String,
}

impl fmt::Debug for Flow {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Flow")
            .field("ty", &self.ty)
            .field("scopes", &self.scopes)
            .field("extra_params", &self.extra_params)
            .field("what", &self.what)
            .finish()
    }
}

impl Flow {
    /// Convert the flow into a token.
    pub fn into_token<'a>(
        self,
        key: Key<SyncToken>,
        injector: &'a Injector,
    ) -> Result<(SyncToken, impl Future<Output = Result<(), Error>> + 'a), Error> {
        // token expires within 30 minutes.
        let expires = Duration::from_secs(30 * 60);

        // queue used to force token refreshes.
        let (force_refresh, mut force_refresh_rx) = mpsc::unbounded();

        let (mut config_stream, config) = match self.secrets {
            Secrets::Settings(settings) => {
                let (config_stream, config) = settings.stream("config").optional()?;
                (Some(config_stream), config)
            }
            Secrets::Static {
                client_id,
                client_secret,
            } => {
                let config = Config {
                    client_id: client_id.to_string(),
                    client_secret: ClientSecret::new(client_secret.to_string()),
                };

                (None, Some(config))
            }
        };

        let (mut token_stream, token) = self.settings.stream::<Token>("token").optional()?;

        let what = Arc::new(self.what.clone());

        let mut builder = TokenBuilder::new(
            self.ty,
            self.web,
            what.clone(),
            self.redirect_url,
            self.auth_url,
            self.token_url,
            expires.clone(),
            self.scopes.clone(),
            self.extra_params,
        );
        builder.new_config = config;
        builder.initial_token = token;

        // check interval.
        let mut interval =
            tokio::timer::Interval::new(Instant::now(), Duration::from_secs(10 * 60));

        let sync_token = SyncToken {
            what: what.clone(),
            inner: Default::default(),
            force_refresh,
        };

        let returned_sync_token = sync_token.clone();
        let settings = self.settings;

        let future = async move {
            log::trace!("{}: Running loop", what);

            let update = move |token| {
                sync_token.set(token, &key, injector);
            };

            if let Some(token) = builder.log_build().await {
                log::trace!("{}: Storing new token", what);
                settings.set_silent("token", &token)?;
                update(token);
            }

            loop {
                futures::select! {
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
                    config = config_stream.select_next_some() => {
                        log::trace!("{}: New configuration", what);

                        builder.new_config = config;

                        if let Some(token) = builder.log_build().await {
                            log::trace!("{}: Storing new token", what);
                            settings.set_silent("token", &token)?;
                            update(token);
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
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TwitchTokenResponse {
    access_token: AccessToken,
    #[serde(deserialize_with = "oauth2::helpers::deserialize_untagged_enum_case_insensitive")]
    token_type: TokenType,
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
    fn access_token(&self) -> &AccessToken {
        &self.access_token
    }

    fn token_type(&self) -> &TokenType {
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
