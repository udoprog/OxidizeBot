use ::oauth2::{
    AccessToken, AuthorizationCode, Client, ClientSecret, RefreshToken, RequestTokenError, Scope,
    StandardToken, State, Token, TokenType,
};
use anyhow::{anyhow, bail, Error};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use url::Url;

/// The configuration for a single flow.
#[derive(Debug, Clone, Deserialize)]
pub struct FlowConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub ty: FlowType,
    pub title: String,
    pub description: String,
    client_id: String,
    client_secret: ClientSecret,
    auth_url: Url,
    token_url: Url,
    #[serde(default)]
    scopes: Vec<Scope>,
    #[serde(default)]
    extra_params: HashMap<String, String>,
}

impl FlowConfig {
    /// Convert configuration into Flow.
    pub fn as_flow(&self, base_url: &Url, config: &Config) -> Result<Flow, Error> {
        let http_client = reqwest::Client::new();

        let mut client = Client::new(
            self.client_id.clone(),
            self.auth_url.clone(),
            self.token_url.clone(),
        );

        client.set_client_secret(self.client_secret.clone());

        let mut url = base_url.clone();
        url.path_segments_mut()
            .expect("valid: base_url")
            .extend(config.redirect_path.split('/'));

        client.set_redirect_url(url);

        for scope in &self.scopes {
            client.add_scope(scope.clone());
        }

        let mut flow = Flow::new(http_client, client, self.clone());

        for (key, value) in &self.extra_params {
            flow.extra_param(key.clone(), value.clone());
        }

        Ok(flow)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub redirect_path: String,
    pub login: FlowConfig,
    pub flows: Vec<FlowConfig>,
}

/// A token that comes out of a token workflow.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedToken {
    /// The ID of the flow used for the token.
    pub flow_id: String,
    /// The client id that generated the token.
    pub client_id: String,
    /// Store the known refresh token.
    pub refresh_token: RefreshToken,
    /// Access token.
    pub access_token: AccessToken,
    /// When the token was refreshed.
    pub refreshed_at: DateTime<Utc>,
    /// Expires in seconds.
    pub expires_in: Option<u64>,
    /// Scopes associated with token.
    pub scopes: Vec<Scope>,
}

impl SavedToken {
    /// Convert into an exported variant.
    pub fn as_exported(&self) -> ExportedToken<'_> {
        ExportedToken {
            flow_id: &self.flow_id,
            client_id: &self.client_id,
            access_token: &self.access_token,
            refreshed_at: &self.refreshed_at,
            expires_in: self.expires_in,
            scopes: &self.scopes,
        }
    }

    /// Generate a unique hash corresponding to this token.
    pub fn hash(&self) -> Result<String, Error> {
        let bytes = serde_cbor::to_vec(&[&self.client_id, &*self.access_token])?;
        let digest = ring::digest::digest(&ring::digest::SHA256, &bytes);
        Ok(base64::encode(digest.as_ref()))
    }
}

/// A token that has been exported from this system.
#[derive(Clone, Debug, Serialize)]
pub struct ExportedToken<'a> {
    /// The ID of the flow used for the token.
    pub flow_id: &'a str,
    /// The exported client id.
    pub client_id: &'a str,
    /// Access token.
    pub access_token: &'a AccessToken,
    /// When the token was refreshed.
    pub refreshed_at: &'a DateTime<Utc>,
    /// Expires in seconds.
    pub expires_in: Option<u64>,
    /// Scopes associated with token.
    pub scopes: &'a [Scope],
}

type Flows = (Arc<Flow>, HashMap<String, Arc<Flow>>);

/// Setup all required flows.
pub fn setup_flows(base_url: &Url, config: &Config) -> Result<Flows, Error> {
    let mut out = HashMap::new();

    let login_flow = Arc::new(config.login.as_flow(base_url, config)?);

    for client_config in &config.flows {
        let flow = client_config.as_flow(base_url, config)?;
        out.insert(client_config.id.clone(), Arc::new(flow));
    }

    Ok((login_flow, out))
}

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    pub code: AuthorizationCode,
    pub state: State,
}

#[derive(Debug)]
pub struct ExchangeToken {
    pub state: State,
    pub auth_url: Url,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum FlowType {
    #[serde(rename = "twitch")]
    Twitch,
    #[serde(rename = "youtube")]
    YouTube,
    #[serde(rename = "nightbot")]
    Nightbot,
    #[serde(rename = "spotify")]
    Spotify,
}

#[derive(Debug)]
pub struct Flow {
    http_client: reqwest::Client,
    client: Client,
    extra_params: Vec<(String, String)>,
    pub config: FlowConfig,
}

impl Flow {
    /// Construct a new web integration.
    pub fn new(http_client: reqwest::Client, client: Client, config: FlowConfig) -> Self {
        Flow {
            http_client,
            client,
            extra_params: Vec::new(),
            config,
        }
    }

    /// Check if saved token is compatible with the specified token.
    pub fn is_compatible_with(&self, token: &SavedToken) -> bool {
        let mut scopes = HashSet::new();
        scopes.extend(self.config.scopes.iter().cloned());

        for s in &token.scopes {
            scopes.remove(&s);
        }

        if !scopes.is_empty() {
            return false;
        }

        if self.config.client_id != token.client_id {
            return false;
        }

        true
    }

    /// Access scopes configured for this flow.
    pub fn scopes(&self) -> &[Scope] {
        &self.config.scopes
    }

    /// Append an extra parameter to the given flow.
    pub fn extra_param(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) {
        self.extra_params
            .push((key.as_ref().to_string(), value.as_ref().to_string()));
    }

    /// Exchange token with the given client.
    pub fn exchange_token(&self) -> ExchangeToken {
        let state = State::new_random();
        let mut auth_url = self.client.authorize_url(&state);

        for (key, value) in self.extra_params.iter() {
            auth_url.query_pairs_mut().append_pair(key, value);
        }

        ExchangeToken { state, auth_url }
    }

    /// Handle a received token.
    pub async fn handle_received_token(
        &self,
        exchange: ExchangeToken,
        received_token: TokenQuery,
    ) -> Result<SavedToken, Error> {
        if exchange.state != received_token.state {
            bail!("CSRF Token Mismatch");
        }

        match self.config.ty {
            FlowType::Twitch => {
                self.exchange_received_code::<TwitchToken>(received_token.code)
                    .await
            }
            FlowType::YouTube => {
                self.exchange_received_code::<StandardToken>(received_token.code)
                    .await
            }
            FlowType::Nightbot => {
                self.exchange_received_code::<StandardToken>(received_token.code)
                    .await
            }
            FlowType::Spotify => {
                self.exchange_received_code::<StandardToken>(received_token.code)
                    .await
            }
        }
    }

    async fn exchange_received_code<T>(&self, code: AuthorizationCode) -> Result<SavedToken, Error>
    where
        T: Token,
    {
        let token_response = self
            .client
            .exchange_code(code)
            .param("client_id", self.config.client_id.as_str())
            .param("client_secret", &self.config.client_secret)
            .with_client(&self.http_client)
            .execute::<T>()
            .await;

        let token_response = match token_response {
            Ok(t) => t,
            Err(RequestTokenError::Parse(e, res)) => {
                log::error!(
                    "bad token response: {}: {}",
                    e,
                    String::from_utf8_lossy(&res)
                );
                return Err(anyhow!("bad response from server"));
            }
            Err(e) => return Err(Error::from(e)),
        };

        let refresh_token = match token_response.refresh_token() {
            Some(refresh_token) => refresh_token.clone(),
            None => bail!("did not receive a refresh token from the service"),
        };

        let refreshed_at = Utc::now();

        let token = SavedToken {
            flow_id: self.config.id.clone(),
            client_id: self.config.client_id.clone(),
            refresh_token,
            access_token: token_response.access_token().clone(),
            refreshed_at,
            expires_in: token_response.expires_in().map(|e| e.as_secs()),
            scopes: token_response.scopes().cloned().unwrap_or_default(),
        };

        Ok(token)
    }

    /// Refresh the specified token.
    pub async fn refresh_token(&self, refresh_token: &RefreshToken) -> Result<SavedToken, Error> {
        match self.config.ty {
            FlowType::Twitch => self.refresh_token_inner::<TwitchToken>(refresh_token).await,
            FlowType::YouTube => {
                self.refresh_token_inner::<StandardToken>(refresh_token)
                    .await
            }
            FlowType::Nightbot => {
                self.refresh_token_inner::<StandardToken>(refresh_token)
                    .await
            }
            FlowType::Spotify => {
                self.refresh_token_inner::<StandardToken>(refresh_token)
                    .await
            }
        }
    }

    /// Inner, typed implementation of executing a refresh.
    async fn refresh_token_inner<T>(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<SavedToken, Error>
    where
        T: Token,
    {
        let token_response = self
            .client
            .exchange_refresh_token(refresh_token)
            .param("client_id", self.config.client_id.as_str())
            .param("client_secret", &self.config.client_secret)
            .with_client(&self.http_client)
            .execute::<T>()
            .await;

        let token_response = match token_response {
            Ok(t) => t,
            Err(RequestTokenError::Parse(_, res)) => {
                log::error!("bad token response: {}", String::from_utf8_lossy(&res));
                return Err(anyhow!("bad response from server"));
            }
            Err(e) => return Err(Error::from(e)),
        };

        let refresh_token = token_response
            .refresh_token()
            .cloned()
            .unwrap_or_else(|| refresh_token.clone());

        let refreshed_at = Utc::now();

        let token = SavedToken {
            flow_id: self.config.id.clone(),
            client_id: self.config.client_id.clone(),
            refresh_token,
            access_token: token_response.access_token().clone(),
            refreshed_at,
            expires_in: token_response.expires_in().map(|e| e.as_secs()),
            scopes: token_response.scopes().cloned().unwrap_or_default(),
        };

        Ok(token)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TwitchToken {
    access_token: AccessToken,
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

impl Token for TwitchToken {
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
