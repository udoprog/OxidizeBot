use crate::web;
use chrono::{DateTime, Utc};
use failure::{format_err, ResultExt};
use futures::{future, Async, Future, Poll, Stream as _};
use oauth2::{
    basic::{BasicErrorField, BasicTokenResponse, BasicTokenType},
    prelude::{NewType, SecretNewType},
    AccessToken, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, RefreshToken, RequestTokenError, Scope, TokenResponse, TokenUrl,
};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::timer;
use tokio_threadpool::{SpawnHandle, ThreadPool};
use url::Url;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SecretsConfig {
    pub client_id: Arc<String>,
    client_secret: String,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Type {
    #[serde(rename = "twitch")]
    Twitch,
    #[serde(rename = "spotify")]
    Spotify,
}

impl Type {
    /// Refresh and save an updated version of the given token.
    pub fn refresh_and_save_token(
        self,
        flow: &Flow,
        token: &Token,
    ) -> Result<Token, failure::Error> {
        match self {
            Type::Twitch => self.refresh_and_save_token_impl::<TwitchTokenResponse>(flow, token),
            Type::Spotify => self.refresh_and_save_token_impl::<BasicTokenResponse>(flow, token),
        }
    }

    /// Exchange and save a token based on a code.
    pub fn exchange_and_save_token(
        self,
        flow: Flow,
        received_token: web::ReceivedToken,
        thread_pool: Arc<ThreadPool>,
    ) -> Result<(Arc<RwLock<Token>>, TokenRefreshFuture), failure::Error> {
        match self {
            Type::Twitch => self.exchange_and_save_token_impl::<TwitchTokenResponse>(
                flow,
                received_token,
                thread_pool,
            ),
            Type::Spotify => self.exchange_and_save_token_impl::<BasicTokenResponse>(
                flow,
                received_token,
                thread_pool,
            ),
        }
    }

    pub fn refresh_and_save_token_impl<T>(
        self,
        flow: &Flow,
        token: &Token,
    ) -> Result<Token, failure::Error>
    where
        T: TokenResponse,
    {
        let mut runtime = tokio::runtime::current_thread::Runtime::new()?;

        let refresh_token = token.data.refresh_token.clone();

        let token_response = runtime.block_on(
            flow.client
                .exchange_refresh_token(&refresh_token)
                .param("client_id", flow.secrets_config.client_id.as_str())
                .param("client_secret", flow.secrets_config.client_secret.as_str())
                .execute::<T>(),
        );

        let token_response = match token_response {
            Ok(t) => t,
            Err(RequestTokenError::Parse(_, res)) => {
                log::error!("bad token response: {}", String::from_utf8_lossy(&res));
                return Err(format_err!("bad response from server"));
            }
            Err(e) => return Err(failure::Error::from(e)),
        };

        let refresh_token = token_response
            .refresh_token()
            .map(|r| r.clone())
            .unwrap_or(refresh_token);

        flow.save_token(refresh_token, token_response)
    }

    pub fn exchange_and_save_token_impl<T>(
        self,
        flow: Flow,
        received_token: web::ReceivedToken,
        thread_pool: Arc<ThreadPool>,
    ) -> Result<(Arc<RwLock<Token>>, TokenRefreshFuture), failure::Error>
    where
        T: TokenResponse,
    {
        let mut runtime = tokio::runtime::current_thread::Runtime::new()?;

        let token_response = runtime.block_on(
            flow.client
                .exchange_code(AuthorizationCode::new(received_token.code))
                .param("client_id", flow.secrets_config.client_id.as_str())
                .param("client_secret", flow.secrets_config.client_secret.as_str())
                .execute::<T>(),
        );

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

        let token = Arc::new(RwLock::new(flow.save_token(refresh_token, token_response)?));

        Ok((
            token.clone(),
            TokenRefreshFuture::new(flow, token, thread_pool),
        ))
    }
}

/// Setup a Twitch authentication flow.
pub fn twitch(
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
) -> Result<FlowBuilder, failure::Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
        ty: Type::Twitch,
        web,
        secrets_config,
        redirect_url: RedirectUrl::new(Url::parse(&redirect_url)?),
        auth_url: AuthUrl::new(Url::parse("https://id.twitch.tv/oauth2/authorize")?),
        token_url: Some(TokenUrl::new(Url::parse(
            "https://id.twitch.tv/oauth2/token",
        )?)),
        scopes: vec![
            String::from("channel:moderate"),
            String::from("chat:read"),
            String::from("chat:edit"),
            String::from("channel_read"),
            String::from("channel_editor"),
        ],
        state_path: None,
    })
}

/// Setup a Spotify AUTH flow.
pub fn spotify(
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
) -> Result<FlowBuilder, failure::Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
        ty: Type::Spotify,
        web,
        secrets_config,
        redirect_url: RedirectUrl::new(Url::parse(&redirect_url)?),
        auth_url: AuthUrl::new(Url::parse("https://accounts.spotify.com/authorize")?),
        token_url: Some(TokenUrl::new(Url::parse(
            "https://accounts.spotify.com/api/token",
        )?)),
        scopes: vec![
            String::from("user-read-private"),
            String::from("playlist-read-private"),
            String::from("playlist-read-collaborative"),
            String::from("playlist-modify-public"),
            String::from("playlist-modify-private"),
            String::from("user-follow-modify"),
            String::from("user-follow-read"),
            String::from("user-library-read"),
            String::from("user-library-modify"),
            String::from("user-top-read"),
            String::from("user-read-recently-played"),
            String::from("user-read-playback-state"),
            String::from("user-modify-playback-state"),
        ],
        state_path: None,
    })
}

/// A token that comes out of a token workflow.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TokenData {
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

#[derive(Clone, Debug)]
pub struct Token {
    /// Associated secrets configuration.
    secrets_config: Arc<SecretsConfig>,
    /// Serialized token data.
    data: TokenData,
}

impl Token {
    /// Get the client ID that requested the token.
    pub fn client_id(&self) -> &str {
        self.secrets_config.client_id.as_str()
    }

    /// Get the current access token.
    pub fn access_token(&self) -> &str {
        self.data.access_token.secret().as_str()
    }

    /// Return `true` if the token expires within 30 minutes.
    pub fn expires_within(&self, within: Duration) -> Result<bool, failure::Error> {
        let out = match self.data.expires_in.clone() {
            Some(expires_in) => {
                let expires_in = chrono::Duration::seconds(expires_in as i64);
                let diff = (self.data.refreshed_at + expires_in) - Utc::now();
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

        for s in &self.data.scopes {
            scopes.remove(s.as_ref());
        }

        scopes.is_empty()
    }
}

pub struct FlowBuilder {
    ty: Type,
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
    redirect_url: RedirectUrl,
    auth_url: AuthUrl,
    token_url: Option<TokenUrl>,
    scopes: Vec<String>,
    state_path: Option<PathBuf>,
}

impl FlowBuilder {
    /// Configure a local cache file for token.
    pub fn with_state_path(self, state_path: PathBuf) -> FlowBuilder {
        FlowBuilder {
            state_path: Some(state_path),
            ..self
        }
    }

    /// Convert into an authentication flow.
    pub fn build(self) -> Result<Flow, failure::Error> {
        let secrets_config = self.secrets_config;

        let mut client = Client::new(
            ClientId::new(secrets_config.client_id.as_str().to_string()),
            Some(ClientSecret::new(secrets_config.client_secret.clone())),
            self.auth_url,
            self.token_url,
        );

        for scope in &self.scopes {
            client = client.add_scope(Scope::new(scope.to_string()));
        }

        client = client.set_redirect_url(self.redirect_url);

        Ok(Flow {
            ty: self.ty,
            web: self.web.clone(),
            secrets_config,
            client,
            state_path: self.state_path.map(|p| p.to_owned()),
            scopes: self.scopes,
        })
    }
}

pub struct Flow {
    ty: Type,
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
    client: Client,
    state_path: Option<PathBuf>,
    scopes: Vec<String>,
}

impl Flow {
    /// Execute the flow.
    pub fn execute(
        self,
        what: &str,
        thread_pool: Arc<ThreadPool>,
    ) -> Result<(Arc<RwLock<Token>>, TokenRefreshFuture), failure::Error> {
        if let Some(token) = self.token_from_state_path()? {
            let token = Arc::new(RwLock::new(token));

            return Ok((
                Arc::clone(&token),
                TokenRefreshFuture::new(self, token, thread_pool),
            ));
        }

        let (auth_url, csrf_token) = self.client.authorize_url(CsrfToken::new_random);

        let mut runtime = tokio::runtime::current_thread::Runtime::new()?;

        let received_token = runtime.block_on(self.web.receive_token(
            auth_url,
            what.to_string(),
            csrf_token.secret().to_string(),
        ))?;

        if *csrf_token.secret() != received_token.state {
            failure::bail!("CSRF Token Mismatch");
        }

        self.ty
            .exchange_and_save_token(self, received_token, thread_pool)
    }

    /// Load a token from the current state path.
    fn token_from_state_path(&self) -> Result<Option<Token>, failure::Error> {
        let path = match self.state_path.as_ref() {
            Some(path) => path,
            None => return Ok(None),
        };

        if !path.is_file() {
            return Ok(None);
        }

        let token = match self.token_from_path(path) {
            Ok(token) => token,
            Err(e) => {
                log::warn!("failed to load saved token: {}: {}", path.display(), e);
                return Ok(None);
            }
        };

        if token.expires_within(Duration::from_secs(60 * 30))? || !token.has_scopes(&self.scopes) {
            return Ok(None);
        }

        Ok(Some(token))
    }

    /// Refresh the token.
    pub fn refresh(&self, token: &Token) -> Result<Token, failure::Error> {
        self.ty.refresh_and_save_token(self, token)
    }

    /// Save and return the given token.
    fn save_token(
        &self,
        refresh_token: RefreshToken,
        token_response: impl TokenResponse,
    ) -> Result<Token, failure::Error> {
        let refreshed_at = Utc::now();

        let data = TokenData {
            refresh_token,
            access_token: token_response.access_token().clone(),
            refreshed_at: refreshed_at.clone(),
            expires_in: token_response.expires_in().map(|e| e.as_secs()),
            scopes: token_response
                .scopes()
                .map(|s| s.clone())
                .unwrap_or_default(),
        };

        if let Some(path) = self.state_path.as_ref() {
            if let Some(parent) = path.parent() {
                if !parent.is_dir() {
                    fs::create_dir_all(parent).with_context(|_| {
                        format_err!("failed to create directory: {}", parent.display())
                    })?;
                }
            }

            self.token_to_path(path, &data).with_context(|_| {
                failure::format_err!("failed to write token to: {}", path.display())
            })?;
        }

        Ok(Token {
            secrets_config: Arc::clone(&self.secrets_config),
            data,
        })
    }

    /// Read token data from path.
    fn token_from_path(&self, path: &Path) -> Result<Token, failure::Error> {
        let f = File::open(path)?;
        let data = serde_yaml::from_reader(f)?;

        Ok(Token {
            secrets_config: Arc::clone(&self.secrets_config),
            data,
        })
    }

    /// Write token to path.
    fn token_to_path(&self, path: &Path, data: &TokenData) -> Result<(), failure::Error> {
        let f = File::create(path)?;
        log::info!("Writing: {}", path.display());
        serde_yaml::to_writer(f, data)?;
        Ok(())
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
    token: Arc<RwLock<Token>>,
    interval: timer::Interval,
    thread_pool: Arc<ThreadPool>,
    refresh_duration: Duration,
    refresh_future: Option<SpawnHandle<(), failure::Error>>,
}

impl TokenRefreshFuture {
    pub fn new(flow: Flow, token: Arc<RwLock<Token>>, thread_pool: Arc<ThreadPool>) -> Self {
        // check for expiration every 10 minutes.
        let duration = Duration::from_secs(10 * 60);
        // refresh if token expires within 30 minutes.
        let refresh_duration = Duration::from_secs(30 * 60);

        Self {
            flow: Arc::new(flow),
            token,
            interval: timer::Interval::new_interval(duration.clone()),
            thread_pool,
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
            if let Some(future) = self.refresh_future.as_mut() {
                if let Async::Ready(()) = future.poll()? {
                    self.refresh_future = None;
                    continue;
                }
            }

            let result = self
                .interval
                .poll()
                .map_err(|_| format_err!("failed to poll interval"))?;

            if let Async::Ready(result) = result {
                result.ok_or_else(|| format_err!("end of interval stream"))?;

                if self.refresh_future.is_some() {
                    return Err(format_err!("refresh already in progress"));
                }

                let flow = Arc::clone(&self.flow);
                let token = Arc::clone(&self.token);

                {
                    let token = token.read().expect("lock poisoned");

                    if !token.expires_within(self.refresh_duration.clone())? {
                        return Ok(Async::NotReady);
                    }
                }

                let handle = self.thread_pool.spawn_handle(future::lazy(move || {
                    let mut token = token.write().expect("lock poisoned");
                    *token = flow.refresh(&token)?;
                    Ok(())
                }));

                self.refresh_future = Some(handle);
                continue;
            }

            return Ok(Async::NotReady);
        }
    }
}
