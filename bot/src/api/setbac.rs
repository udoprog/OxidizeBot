//! setbac.tv API helpers.

use crate::{
    bus,
    config::Config,
    injector::Injector,
    oauth2,
    player::{self, Player},
    prelude::*,
    settings::Settings,
    utils,
};
use futures::compat::Compat01As03;
use reqwest::{
    header,
    r#async::{Body, Client, Decoder},
    Method, StatusCode, Url,
};
use std::{mem, sync::Arc};

static DEFAULT_API_URL: &'static str = "https://setbac.tv";

fn parse_url(url: &str) -> Option<Url> {
    match str::parse(url) {
        Ok(api_url) => Some(api_url),
        Err(e) => {
            log::warn!("bad api url: {}: {}", url, e);
            None
        }
    }
}

struct RemoteBuilder {
    token: oauth2::SyncToken,
    enabled: bool,
    player: Option<Player>,
    api_url: Option<Url>,
}

impl RemoteBuilder {
    fn init(&self, remote: &mut Remote) {
        if !self.enabled {
            remote.rx = None;
            remote.client = None;
            remote.setbac = None;
            return;
        }

        remote.rx = match self.player.as_ref() {
            Some(player) => Some(player.add_rx().compat()),
            None => None,
        };

        remote.client = match self.player.as_ref() {
            Some(player) => Some(player.clone()),
            None => None,
        };

        remote.setbac = match self.api_url.as_ref() {
            Some(api_url) => Some(SetBac::new(self.token.clone(), api_url.clone())),
            None => None,
        };
    }
}

#[derive(Default)]
struct Remote {
    rx: Option<Compat01As03<bus::Reader<player::Event>>>,
    client: Option<player::Player>,
    setbac: Option<SetBac>,
}

/// Run update loop shipping information to the remote server.
pub fn run(
    config: &Config,
    settings: &Settings,
    injector: &Injector,
    token: oauth2::SyncToken,
) -> Result<impl Future<Output = Result<(), failure::Error>>, failure::Error> {
    let settings = settings.scoped("remote");

    if config.api_url.is_some() {
        log::warn!("`api_url` configuration has been deprecated");
    }

    let default_api_url = Some(
        config
            .api_url
            .clone()
            .unwrap_or_else(|| String::from(DEFAULT_API_URL)),
    );

    let (mut api_url_stream, api_url) =
        settings.stream("api-url").or(default_api_url).optional()?;

    let (mut enabled_stream, enabled) = settings
        .stream("enabled")
        .or_with(config.api_url.is_some())?;

    let (mut player_stream, player) = injector.stream::<Player>();

    let mut remote_builder = RemoteBuilder {
        token,
        enabled: false,
        player: None,
        api_url: None,
    };

    remote_builder.enabled = enabled;
    remote_builder.player = player;
    remote_builder.api_url = match api_url.and_then(|s| parse_url(&s)) {
        Some(api_url) => Some(api_url),
        None => None,
    };

    let mut remote = Remote::default();
    remote_builder.init(&mut remote);

    Ok(async move {
        loop {
            futures::select! {
                update = player_stream.select_next_some() => {
                    remote_builder.player = update;
                    remote_builder.init(&mut remote);
                }
                update = api_url_stream.select_next_some() => {
                    remote_builder.api_url = match update.and_then(|s| parse_url(&s)) {
                        Some(api_url) => Some(api_url),
                        None => None,
                    };

                    remote_builder.init(&mut remote);
                }
                update = enabled_stream.select_next_some() => {
                    remote_builder.enabled = update;
                    remote_builder.init(&mut remote);
                }
                result = remote.rx.select_next_some() => {
                    let _ = result?;

                    let setbac = match remote.setbac.as_ref() {
                        Some(setbac) => setbac,
                        None => continue,
                    };

                    let client = match remote.client.as_ref() {
                        Some(client) => client,
                        None => continue,
                    };

                    log::trace!("pushing remote player update");

                    let mut update = PlayerUpdate::default();

                    update.current = client.current().map(|c| c.item.into());

                    for i in client.list() {
                        update.items.push(i.into());
                    }

                    if let Err(e) = setbac.player_update(update).await {
                        log::error!("failed to perform remote player update: {}", e);
                    }
                }
            }
        }
    })
}

/// API integration.
#[derive(Clone, Debug)]
pub struct SetBac {
    client: Client,
    api_url: Url,
    token: oauth2::SyncToken,
}

impl SetBac {
    /// Create a new API integration.
    pub fn new(token: oauth2::SyncToken, api_url: Url) -> Self {
        SetBac {
            client: Client::new(),
            api_url,
            token,
        }
    }

    /// Get request against API.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("bad base").extend(path);

        RequestBuilder {
            token: self.token.clone(),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
        }
    }

    /// Update the channel information.
    pub async fn player_update(&self, request: PlayerUpdate) -> Result<(), failure::Error> {
        let body = Body::from(serde_json::to_vec(&request)?);

        let req = self
            .request(Method::POST, &["api", "player"])
            .header(header::CONTENT_TYPE, "application/json")
            .body(body);

        let _ = req.execute::<serde_json::Value>().await?;
        Ok(())
    }
}

struct RequestBuilder {
    token: oauth2::SyncToken,
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Body>,
}

impl RequestBuilder {
    /// Execute the request.
    pub async fn execute<T>(self) -> Result<T, failure::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let req = {
            let token = self.token.read()?;
            let access_token = token.access_token().to_string();

            let mut req = self.client.request(self.method, self.url);

            if let Some(body) = self.body {
                req = req.body(body);
            }

            for (key, value) in self.headers {
                req = req.header(key, value);
            }

            let req = req.header(header::AUTHORIZATION, format!("OAuth {}", access_token));
            let req = req.header("Client-ID", token.client_id());
            req
        };

        let mut res = req.send().compat().await?;
        let body = mem::replace(res.body_mut(), Decoder::empty()).compat();
        let body = body.try_concat().await?;

        let status = res.status();

        if status == StatusCode::UNAUTHORIZED {
            self.token.force_refresh()?;
        }

        if !status.is_success() {
            failure::bail!(
                "bad response: {}: {}",
                status,
                String::from_utf8_lossy(body.as_ref())
            );
        }

        if log::log_enabled!(log::Level::Trace) {
            let response = String::from_utf8_lossy(body.as_ref());
            log::trace!("response: {}", response);
        }

        serde_json::from_slice(body.as_ref()).map_err(Into::into)
    }

    /// Add a body to the request.
    pub fn body(mut self, body: Body) -> Self {
        self.body = Some(body);
        self
    }

    /// Push a header.
    pub fn header(mut self, key: header::HeaderName, value: &str) -> Self {
        self.headers.push((key, value.to_string()));
        self
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PlayerUpdate {
    /// Current song.
    #[serde(default)]
    current: Option<Item>,
    /// Songs.
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
            duration: utils::compact_duration(&i.duration),
        }
    }
}
