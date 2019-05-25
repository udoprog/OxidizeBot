//! setbac.tv API helpers.

use crate::{oauth2, player, prelude::*, utils};
use reqwest::{
    header,
    r#async::{Body, Client, Decoder},
    Method, StatusCode, Url,
};
use std::{mem, sync::Arc};

/// Run update loop shipping information to the remote server.
pub fn run(
    api_url: &str,
    player: &player::Player,
    token: oauth2::SyncToken,
) -> Result<impl Future<Output = Result<(), failure::Error>>, failure::Error> {
    /* perform remote player update */
    let setbac = SetBac::new(token, api_url)?;
    let client = player.client();

    let mut rx = player.add_rx().compat();

    Ok(async move {
        while let Some(_) = rx.next().await {
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

        Ok(())
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
    pub fn new(token: oauth2::SyncToken, api_url: &str) -> Result<SetBac, failure::Error> {
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
            duration: utils::compact_duration(i.duration.clone()),
        }
    }
}
