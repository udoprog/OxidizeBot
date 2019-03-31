//! SetBac API helpers.

use crate::{oauth2, player, utils};
use failure::format_err;
use futures::{future, Future, Stream as _};
use parking_lot::RwLock;
use reqwest::{
    header,
    r#async::{Body, Client, Decoder},
    Method, Url,
};
use std::{mem, sync::Arc};

/// Run update loop shipping information to the remote server.
pub fn run_update(
    api_url: &str,
    player: &player::Player,
    token: Arc<RwLock<oauth2::Token>>,
) -> Result<impl Future<Item = (), Error = failure::Error>, failure::Error> {
    /* perform remote player update */
    let setbac = SetBac::new(token, api_url)?;

    let client = player.client();

    Ok(player
        .add_rx()
        .map_err(|e| format_err!("setbac.tv update loop received error: {}", e))
        .for_each(move |_| {
            log::trace!("pushing remote player update");

            let mut update = PlayerUpdate::default();

            update.current = client.current().map(|c| c.item.into());

            for i in client.list() {
                update.items.push(i.into());
            }

            setbac.player_update(&update).or_else(|e| {
                log::error!("failed to perform remote player update: {}", e);
                Ok(())
            })
        }))
}

/// API integration.
#[derive(Clone, Debug)]
pub struct SetBac {
    client: Client,
    api_url: Url,
    token: Arc<RwLock<oauth2::Token>>,
}

impl SetBac {
    /// Create a new API integration.
    pub fn new(token: Arc<RwLock<oauth2::Token>>, api_url: &str) -> Result<SetBac, failure::Error> {
        Ok(SetBac {
            client: Client::new(),
            api_url: str::parse::<Url>(api_url)?,
            token,
        })
    }

    /// Get request against API.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("bad base").extend(path);

        RequestBuilder {
            token: Arc::clone(&self.token),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
        }
    }

    /// Serialize the given argument into a future.
    fn serialize<T: serde::Serialize>(
        value: &T,
    ) -> impl Future<Item = Body, Error = failure::Error> {
        match serde_json::to_vec(value) {
            Ok(body) => future::ok(Body::from(body)),
            Err(e) => future::err(failure::Error::from(e)),
        }
    }

    /// Update the channel information.
    pub fn player_update(
        &self,
        request: &PlayerUpdate,
    ) -> impl Future<Item = (), Error = failure::Error> {
        let req = self
            .request(Method::POST, &["api", "player"])
            .header(header::CONTENT_TYPE, "application/json");

        Self::serialize(request)
            .and_then(move |body| req.body(body).execute::<serde_json::Value>())
            .and_then(|_| Ok(()))
    }
}

struct RequestBuilder {
    token: Arc<RwLock<oauth2::Token>>,
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Body>,
}

impl RequestBuilder {
    /// Execute the request.
    pub fn execute<T>(self) -> impl Future<Item = T, Error = failure::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let token = self.token.read();
        let access_token = token.access_token().to_string();

        let mut r = self.client.request(self.method, self.url);

        if let Some(body) = self.body {
            r = r.body(body);
        }

        for (key, value) in self.headers {
            r = r.header(key, value);
        }

        r.header(header::AUTHORIZATION, format!("OAuth {}", access_token))
            .header("Client-ID", token.client_id())
            .send()
            .map_err(Into::into)
            .and_then(|mut res| {
                let body = mem::replace(res.body_mut(), Decoder::empty());

                body.concat2().map_err(Into::into).and_then(move |body| {
                    let status = res.status();

                    if !status.is_success() {
                        failure::bail!(
                            "bad response: {}: {}",
                            status,
                            String::from_utf8_lossy(body.as_ref())
                        );
                    }

                    log::trace!("response: {}", String::from_utf8_lossy(body.as_ref()));
                    serde_json::from_slice(body.as_ref()).map_err(Into::into)
                })
            })
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
    /// Spotify ID of the song.
    track_id: String,
    /// User who requested the song.
    #[serde(default)]
    user: Option<String>,
    /// Length of the song.
    duration: String,
}

impl From<Arc<player::Item>> for Item {
    fn from(i: Arc<player::Item>) -> Self {
        Item {
            name: i.track.name.clone(),
            artists: utils::human_artists(&i.track.artists),
            track_id: i.track_id.to_base62(),
            user: i.user.clone(),
            duration: utils::compact_duration(i.duration.clone()),
        }
    }
}
