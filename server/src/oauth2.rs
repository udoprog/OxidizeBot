use crate::web;
use chrono::{DateTime, Utc};
use failure::{format_err, ResultExt};
use futures::{future, Async, Future, Poll, Stream as _};
use oauth2::{
    basic::{BasicErrorResponseType, BasicTokenResponse, BasicTokenType},
    prelude::{NewType, SecretNewType},
    AccessToken, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, RefreshToken, RequestTokenError, Scope, TokenResponse, TokenUrl,
};
use std::{
    fs::{self, File},
    marker,
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

pub type TwitchToken = Token<TwitchTokenResponse>;
pub type SpotifyToken = Token<BasicTokenResponse>;

/// Setup a Twitch authentication flow.
pub fn twitch(
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
) -> Result<FlowBuilder<TwitchTokenResponse>, failure::Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
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
        marker: marker::PhantomData,
    })
}

/// Setup a Spotify AUTH flow.
pub fn spotify(
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
) -> Result<FlowBuilder<BasicTokenResponse>, failure::Error> {
    let redirect_url = format!("{}{}", web::URL, web::REDIRECT_URI);

    Ok(FlowBuilder {
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
        marker: marker::PhantomData,
    })
}

/// A token that comes out of a token workflow.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TokenData<T> {
    /// Store the known refresh token.
    refresh_token: RefreshToken,
    refreshed_at: DateTime<Utc>,
    token_response: T,
}

#[derive(Clone, Debug)]
pub struct Token<T> {
    /// Associated secrets configuration.
    secrets_config: Arc<SecretsConfig>,
    /// Serialized token data.
    data: TokenData<T>,
}

impl<T> Token<T> {
    /// Get the client ID that requested the token.
    pub fn client_id(&self) -> &str {
        self.secrets_config.client_id.as_str()
    }
}

impl<T> Token<T>
where
    T: TokenResponse<BasicTokenType>,
{
    /// Get the current access token.
    pub fn access_token(&self) -> &str {
        self.data.token_response.access_token().secret().as_str()
    }

    /// Return `true` if the token expires within 30 minutes.
    pub fn expires_within(&self, within: Duration) -> Result<bool, failure::Error> {
        let out = match self.data.token_response.expires_in() {
            Some(expires_in) => {
                let expires_in = chrono::Duration::from_std(expires_in)?;
                let diff = (self.data.refreshed_at + expires_in) - Utc::now();
                diff < chrono::Duration::from_std(within)?
            }
            None => true,
        };

        Ok(out)
    }

    /// Test that token has all the specified scopes.
    pub fn has_scopes(&self, scopes: &[String]) -> bool {
        use std::collections::HashSet;

        let mut scopes = scopes
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>();

        if let Some(existing) = self.data.token_response.scopes() {
            for s in existing {
                scopes.remove(s.as_ref());
            }
        };

        scopes.is_empty()
    }
}

pub struct FlowBuilder<T> {
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
    redirect_url: RedirectUrl,
    auth_url: AuthUrl,
    token_url: Option<TokenUrl>,
    scopes: Vec<String>,
    state_path: Option<PathBuf>,
    marker: marker::PhantomData<T>,
}

impl<T> FlowBuilder<T>
where
    T: TokenResponse<BasicTokenType>,
{
    /// Configure a local cache file for token.
    pub fn with_state_path(self, state_path: PathBuf) -> FlowBuilder<T> {
        FlowBuilder {
            state_path: Some(state_path),
            ..self
        }
    }

    /// Convert into an authentication flow.
    pub fn build(self) -> Result<Flow<T>, failure::Error> {
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
            web: self.web.clone(),
            secrets_config,
            client,
            state_path: self.state_path.map(|p| p.to_owned()),
            scopes: self.scopes,
        })
    }
}

pub struct Flow<T>
where
    T: TokenResponse<BasicTokenType>,
{
    web: web::Server,
    secrets_config: Arc<SecretsConfig>,
    client: Client<BasicErrorResponseType, T, BasicTokenType>,
    state_path: Option<PathBuf>,
    scopes: Vec<String>,
}

impl<T> Flow<T>
where
    T: TokenResponse<BasicTokenType>,
{
    /// Execute the flow.
    pub fn execute(
        self,
        what: &str,
        thread_pool: Arc<ThreadPool>,
    ) -> Result<(Arc<RwLock<Token<T>>>, TokenRefreshFuture<T>), failure::Error> {
        if let Some(path) = self.state_path.as_ref() {
            if path.is_file() {
                let token = self.token_from_path(path).with_context(|_| {
                    format_err!("failed to load token from: {}", path.display())
                })?;

                if !token.expires_within(Duration::from_secs(60 * 30))?
                    && token.has_scopes(&self.scopes)
                {
                    let token = Arc::new(RwLock::new(token));
                    return Ok((
                        Arc::clone(&token),
                        TokenRefreshFuture::new(self, token, thread_pool),
                    ));
                }
            }
        }

        let (auth_url, csrf_token) = self.client.authorize_url(CsrfToken::new_random);

        let mut runtime = tokio::runtime::current_thread::Runtime::new()?;
        let token = runtime.block_on(self.web.receive_token(
            auth_url,
            what.to_string(),
            csrf_token.secret().to_string(),
        ))?;

        if csrf_token != CsrfToken::new(token.state) {
            failure::bail!("CSRF Token Mismatch");
        }

        let token_response = self.client.exchange_code_extension(
            AuthorizationCode::new(token.code),
            &[
                ("client_id", self.secrets_config.client_id.as_str()),
                ("client_secret", self.secrets_config.client_secret.as_str()),
            ],
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

        let token = Arc::new(RwLock::new(self.save_token(refresh_token, token_response)?));
        Ok((
            Arc::clone(&token),
            TokenRefreshFuture::new(self, token, thread_pool),
        ))
    }

    /// Refresh the token.
    pub fn refresh(&self, token: &Token<T>) -> Result<Token<T>, failure::Error> {
        let refresh_token = token.data.refresh_token.clone();

        let token_response = self.client.exchange_refresh_token_extension(
            &refresh_token,
            &[
                ("client_id", self.secrets_config.client_id.as_str()),
                ("client_secret", self.secrets_config.client_secret.as_str()),
            ],
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
        self.save_token(refresh_token, token_response)
    }

    /// Save and return the given token.
    fn save_token(
        &self,
        refresh_token: RefreshToken,
        token_response: T,
    ) -> Result<Token<T>, failure::Error> {
        let refreshed_at = Utc::now();

        let data = TokenData {
            refresh_token,
            refreshed_at: refreshed_at.clone(),
            token_response: token_response.clone(),
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
    fn token_from_path(&self, path: &Path) -> Result<Token<T>, failure::Error> {
        let f = File::open(path)?;
        let data = serde_yaml::from_reader(f)?;

        Ok(Token {
            secrets_config: Arc::clone(&self.secrets_config),
            data,
        })
    }

    /// Write token to path.
    fn token_to_path(&self, path: &Path, data: &TokenData<T>) -> Result<(), failure::Error> {
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

impl TokenResponse<BasicTokenType> for TwitchTokenResponse {
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
pub struct TokenRefreshFuture<T>
where
    T: TokenResponse<BasicTokenType>,
{
    flow: Arc<Flow<T>>,
    token: Arc<RwLock<Token<T>>>,
    interval: timer::Interval,
    thread_pool: Arc<ThreadPool>,
    refresh_duration: Duration,
    refresh_future: Option<SpawnHandle<(), failure::Error>>,
}

impl<T> TokenRefreshFuture<T>
where
    T: TokenResponse<BasicTokenType>,
{
    pub fn new(flow: Flow<T>, token: Arc<RwLock<Token<T>>>, thread_pool: Arc<ThreadPool>) -> Self {
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

impl<T> Future for TokenRefreshFuture<T>
where
    T: 'static + Send + Sync + TokenResponse<BasicTokenType>,
{
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
