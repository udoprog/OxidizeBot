//! setbac.tv API helpers.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_injector::{Injector, Key};
use chrono::{DateTime, Utc};
use common::tags;
use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};
use tracing::Instrument;

use crate::base::RequestBuilder;
use crate::token;

const DEFAULT_API_URL: &str = "https://setbac.tv";

struct Inner {
    user_agent: &'static str,
    client: Client,
    api_url: Url,
    streamer_token: Option<crate::token::Token>,
    secret_key: Option<String>,
}

/// API integration.
#[derive(Clone)]
pub struct Setbac {
    inner: Arc<Inner>,
}

impl Setbac {
    /// Create a new API integration.
    pub fn new(
        user_agent: &'static str,
        streamer_token: Option<crate::token::Token>,
        secret_key: Option<String>,
        api_url: Url,
    ) -> Self {
        Setbac {
            inner: Arc::new(Inner {
                user_agent,
                client: Client::new(),
                api_url,
                streamer_token,
                secret_key,
            }),
        }
    }

    /// Get request against API.
    fn request<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.inner.api_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut request = RequestBuilder::new(&self.inner.client, self.inner.user_agent, method, url);

        if let Some(secret_key) = self.inner.secret_key.as_ref() {
            request.header(header::AUTHORIZATION, &format!("key:{}", secret_key));
        } else if let Some(streamer_token) = self.inner.streamer_token.as_ref() {
            request.token(streamer_token).use_oauth2_header();
        }

        request
    }

    /// Update the channel information.
    pub(crate) async fn player_update(&self, request: PlayerUpdate) -> Result<()> {
        let body = serde_json::to_vec(&request)?;

        let mut req = self.request(Method::POST, &["api", "player"]);

        req.header(header::CONTENT_TYPE, "application/json")
            .body(body);

        req.execute().await?.ok()?;
        Ok(())
    }

    /// Get the token corresponding to the given flow.
    pub(crate) async fn get_connection(&self, id: &str) -> Result<Option<Connection>> {
        let mut req = self.request(Method::GET, &["api", "connections", id]);

        req.header(header::CONTENT_TYPE, "application/json");

        let token = req.execute().await?.json::<Data<Connection>>()?;
        Ok(token.data)
    }

    /// Get the token corresponding to the given flow.
    pub(crate) async fn get_connection_meta(
        &self,
        flow_id: &str,
    ) -> Result<Option<ConnectionMeta>> {
        let mut req = self.request(Method::GET, &["api", "connections", flow_id]);

        req.query_param("format", "meta")
            .header(header::CONTENT_TYPE, "application/json");

        let token = req.execute().await?.json::<Data<ConnectionMeta>>()?;
        Ok(token.data)
    }

    /// Refresh the token corresponding to the given flow.
    pub(crate) async fn refresh_connection(&self, id: &str) -> Result<Option<Connection>> {
        let mut req = self.request(Method::POST, &["api", "connections", id, "refresh"]);

        req.header(header::CONTENT_TYPE, "application/json");

        let token = req.execute().await?.json::<Data<Connection>>()?;
        Ok(token.data)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Data<T> {
    data: Option<T>,
}

impl<T> Default for Data<T> {
    fn default() -> Self {
        Self { data: None }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct PlayerUpdate {
    /// Current song.
    #[serde(default)]
    current: Option<Item>,
    /// Songs.
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Item {
    /// Name of the song.
    name: String,
    /// Artists of the song.
    #[serde(default)]
    artists: Option<String>,
    /// Track ID of the song.
    track_id: String,
    /// URL of the song.
    track_url: String,
    /// User who requested the song.
    #[serde(default)]
    user: Option<String>,
    /// Length of the song.
    duration: String,
}

impl From<Arc<common::models::Item>> for Item {
    fn from(i: Arc<common::models::Item>) -> Self {
        Item {
            name: i.track().name(),
            artists: i.track().artists(),
            track_id: i.track_id().to_string(),
            track_url: i.track_id().url(),
            user: i.user().cloned(),
            duration: common::display::compact_duration(i.duration()),
        }
    }
}

/// A token that comes out of a token workflow.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Token {
    /// The client identifier that generated the token.
    pub client_id: String,
    /// Flow that generated the token.
    pub flow_id: String,
    /// Access token.
    pub access_token: String,
    /// When the token was refreshed.
    pub refreshed_at: DateTime<Utc>,
    /// Expires in seconds.
    pub expires_in: Option<u64>,
    /// Scopes associated with token.
    pub scopes: Vec<String>,
}

impl Token {
    /// Return `true` if the token expires within 30 minutes.
    pub fn expires_within(&self, within: Duration) -> Result<bool> {
        let out = match self.expires_in {
            Some(expires_in) => {
                let expires_in = chrono::Duration::seconds(expires_in as i64);
                let diff = (self.refreshed_at + expires_in) - Utc::now();
                diff < chrono::Duration::from_std(within)?
            }
            None => true,
        };

        Ok(out)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub struct ConnectionMeta {
    pub id: String,
    pub title: String,
    pub description: String,
    pub hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Connection {
    pub id: String,
    pub title: String,
    pub description: String,
    pub hash: String,
    pub token: Token,
}

impl Connection {
    /// Convert connection into its corresponding [`ConnectionMeta`].
    pub fn as_meta(&self) -> ConnectionMeta {
        ConnectionMeta {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            hash: self.hash.clone(),
        }
    }
}

fn parse_url(url: &str) -> Option<Url> {
    match str::parse(url) {
        Ok(api_url) => Some(api_url),
        Err(e) => {
            common::log_warn!(e, "bad api url: {}", url);
            None
        }
    }
}
