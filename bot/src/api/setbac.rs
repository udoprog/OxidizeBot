//! setbac.tv API helpers.

use crate::api::base::RequestBuilder;
use crate::bus;
use crate::injector::{Injector, Key};
use crate::oauth2;
use crate::player::{self, Player};
use crate::prelude::*;
use crate::tags;
use crate::utils;
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_API_URL: &str = "https://setbac.tv";

/// A token that comes out of a token workflow.
#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct ConnectionMeta {
    pub id: String,
    pub title: String,
    pub description: String,
    pub hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Connection {
    pub id: String,
    pub title: String,
    pub description: String,
    pub hash: String,
    pub token: Token,
}

impl Connection {
    pub fn as_meta(&self) -> ConnectionMeta {
        ConnectionMeta {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            hash: self.hash.clone(),
        }
    }
}

impl Token {
    /// The client id that generated the token.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Get the current access token.
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

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

    /// Test that token has all the specified scopes.
    pub fn has_scopes(&self, scopes: &[String]) -> bool {
        use std::collections::HashSet;

        let mut scopes = scopes
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>();

        for s in &self.scopes {
            scopes.remove(s);
        }

        scopes.is_empty()
    }
}

fn parse_url(url: &str) -> Option<Url> {
    match str::parse(url) {
        Ok(api_url) => Some(api_url),
        Err(e) => {
            log_warn!(e, "bad api url: {}", url);
            None
        }
    }
}

struct RemoteBuilder {
    streamer_token: Option<oauth2::SyncToken>,
    injector: Injector,
    global_bus: bus::Bus<bus::Global>,
    player: Option<Player>,
    enabled: bool,
    api_url: Option<Url>,
    secret_key: Option<String>,
}

impl RemoteBuilder {
    async fn init(&self, remote: &mut Remote) {
        if self.enabled {
            remote.rx = Some(self.global_bus.subscribe());

            remote.player = match self.player.as_ref() {
                Some(player) => Some(player.clone()),
                None => None,
            };
        } else {
            remote.rx = None;
            remote.player = None;
        }

        remote.setbac = match self.api_url.as_ref() {
            Some(api_url) => {
                let setbac = Setbac::new(
                    self.streamer_token.clone(),
                    self.secret_key.clone(),
                    api_url.clone(),
                );

                self.injector.update(setbac.clone()).await;
                Some(setbac)
            }
            None => {
                self.injector.clear::<Setbac>().await;
                None
            }
        };
    }
}

#[derive(Default)]
struct Remote {
    rx: Option<bus::Reader<bus::Global>>,
    player: Option<player::Player>,
    setbac: Option<Setbac>,
}

/// Run update loop shipping information to the remote server.
pub async fn run(
    settings: &crate::Settings,
    injector: &Injector,
    global_bus: bus::Bus<bus::Global>,
) -> Result<impl Future<Output = Result<()>>> {
    let settings = settings.scoped("remote");

    let (mut api_url_stream, api_url) = settings
        .stream("api-url")
        .or(Some(String::from(DEFAULT_API_URL)))
        .optional()
        .await?;

    let (mut secret_key_stream, secret_key) = settings.stream("secret-key").optional().await?;
    let (mut enabled_stream, enabled) = settings.stream("enabled").or_with(false).await?;
    let (mut player_stream, player) = injector.stream::<Player>().await;
    let (mut streamer_token_stream, streamer_token) = injector
        .stream_key(Key::<oauth2::SyncToken>::tagged(tags::Token::Twitch(
            tags::Twitch::Streamer,
        ))?)
        .await;

    let mut remote_builder = RemoteBuilder {
        streamer_token,
        injector: injector.clone(),
        global_bus,
        player,
        enabled,
        api_url: None,
        secret_key,
    };

    remote_builder.api_url = match api_url.and_then(|s| parse_url(&s)) {
        Some(api_url) => Some(api_url),
        None => None,
    };

    let mut remote = Remote::default();
    remote_builder.init(&mut remote).await;

    Ok(async move {
        loop {
            tokio::select! {
                update = streamer_token_stream.recv() => {
                    remote_builder.streamer_token = update;
                    remote_builder.init(&mut remote).await;
                }
                secret_key = secret_key_stream.recv() => {
                    remote_builder.secret_key = secret_key;
                    remote_builder.init(&mut remote).await;
                }
                update = player_stream.recv() => {
                    remote_builder.player = update;
                    remote_builder.init(&mut remote).await;
                }
                api_url = api_url_stream.recv() => {
                    remote_builder.api_url = match api_url.and_then(|s| parse_url(&s)) {
                        Some(api_url) => Some(api_url),
                        None => None,
                    };

                    remote_builder.init(&mut remote).await;
                }
                enabled = enabled_stream.recv() => {
                    remote_builder.enabled = enabled;
                    remote_builder.init(&mut remote).await;
                }
                event = async { remote.rx.as_mut().unwrap().recv().await }, if remote.rx.is_some() => {
                    let event = event?;

                    // Only update on switches to current song.
                    match event {
                        bus::Global::SongModified => (),
                        _ => continue,
                    };

                    let setbac = match remote.setbac.as_ref() {
                        Some(setbac) => setbac,
                        None => continue,
                    };

                    let player = match remote.player.as_ref() {
                        Some(player) => player,
                        None => continue,
                    };

                    log::trace!("pushing remote player update");

                    let mut update = PlayerUpdate::default();

                    update.current = player.current().await.map(|c| c.item.into());

                    for i in player.list().await {
                        update.items.push(i.into());
                    }

                    if let Err(e) = setbac.player_update(update).await {
                        log_error!(e, "Failed to perform remote player update");
                    }
                }
            }
        }
    })
}

pub struct Inner {
    client: Client,
    api_url: Url,
    streamer_token: Option<oauth2::SyncToken>,
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
        streamer_token: Option<oauth2::SyncToken>,
        secret_key: Option<String>,
        api_url: Url,
    ) -> Self {
        Setbac {
            inner: Arc::new(Inner {
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

        let mut request = RequestBuilder::new(&self.inner.client, method, url);

        if let Some(secret_key) = self.inner.secret_key.as_ref() {
            request.header(header::AUTHORIZATION, &format!("key:{}", secret_key));
        } else if let Some(streamer_token) = self.inner.streamer_token.as_ref() {
            request.token(streamer_token).use_oauth2_header();
        }

        request
    }

    /// Update the channel information.
    pub async fn player_update(&self, request: PlayerUpdate) -> Result<()> {
        let body = serde_json::to_vec(&request)?;

        let mut req = self.request(Method::POST, &["api", "player"]);

        req.header(header::CONTENT_TYPE, "application/json")
            .body(body);

        req.execute().await?.ok()?;
        Ok(())
    }

    /// Get the token corresponding to the given flow.
    pub async fn get_connection(&self, id: &str) -> Result<Option<Connection>> {
        let mut req = self.request(Method::GET, &["api", "connections", id]);

        req.header(header::CONTENT_TYPE, "application/json");

        let token = req.execute().await?.json::<Data<Connection>>()?;
        Ok(token.data)
    }

    /// Get the token corresponding to the given flow.
    pub async fn get_connection_meta(&self, flow_id: &str) -> Result<Option<ConnectionMeta>> {
        let mut req = self.request(Method::GET, &["api", "connections", flow_id]);

        req.query_param("format", "meta")
            .header(header::CONTENT_TYPE, "application/json");

        let token = req.execute().await?.json::<Data<ConnectionMeta>>()?;
        Ok(token.data)
    }

    /// Get meta for all available connections.
    pub async fn get_connections_meta(&self) -> Result<Vec<ConnectionMeta>> {
        let mut req = self.request(Method::GET, &["api", "connections"]);

        req.query_param("format", "meta")
            .header(header::CONTENT_TYPE, "application/json");

        let data = req.execute().await?.json::<Vec<ConnectionMeta>>()?;
        Ok(data)
    }

    /// Refresh the token corresponding to the given flow.
    pub async fn refresh_connection(&self, id: &str) -> Result<Option<Connection>> {
        let mut req = self.request(Method::POST, &["api", "connections", id, "refresh"]);

        req.header(header::CONTENT_TYPE, "application/json");

        let token = req.execute().await?.json::<Data<Connection>>()?;
        Ok(token.data)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data<T> {
    data: Option<T>,
}

impl<T> Default for Data<T> {
    fn default() -> Self {
        Self { data: None }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PlayerUpdate {
    /// Current song.
    #[serde(default)]
    current: Option<Item>,
    /// Songs.
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
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

impl From<Arc<player::Item>> for Item {
    fn from(i: Arc<player::Item>) -> Self {
        Item {
            name: i.track.name(),
            artists: i.track.artists(),
            track_id: i.track_id.to_string(),
            track_url: i.track_id.url(),
            user: i.user.clone(),
            duration: utils::compact_duration(i.duration),
        }
    }
}
