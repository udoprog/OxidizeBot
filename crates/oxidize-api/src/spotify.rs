//! Spotify API helpers.

use anyhow::Result;
use common::models::spotify::context::FullPlayingContext;
use common::models::spotify::device::Device;
use common::models::spotify::page::Page;
use common::models::spotify::playlist::FullPlaylist;
use common::models::spotify::search::SearchTracks;
use common::models::spotify::track::{FullTrack, SavedTrack};
use common::models::spotify::user::PrivateUser;
use common::models::SpotifyId;
use common::stream::Stream;
use reqwest::{header, Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::{ser, Deserialize, Serialize};
use url::Url;

use crate::base::RequestBuilder;
use crate::token::Token;

const API_URL: &str = "https://api.spotify.com/v1";
const DEFAULT_LIMIT: usize = 50;

/// API integration.
#[derive(Clone, Debug)]
pub struct Spotify {
    user_agent: &'static str,
    token: Token,
    client: Client,
    api_url: Url,
}

impl Spotify {
    /// Create a new API integration.
    pub fn new(user_agent: &'static str, token: Token) -> Result<Spotify> {
        Ok(Spotify {
            user_agent,
            token,
            client: Client::new(),
            api_url: str::parse::<Url>(API_URL)?,
        })
    }

    /// Access the underlying bearer token for this client.
    pub fn token(&self) -> &Token {
        &self.token
    }

    /// Get request against API.
    fn request<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.api_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, self.user_agent, method, url);
        req.token(&self.token);
        req
    }

    /// Get user info.
    pub async fn me(&self) -> Result<PrivateUser> {
        let req = self.request(Method::GET, &["me"]);

        req.execute().await?.json()
    }

    /// Get my playlists.
    pub async fn playlist(&self, id: SpotifyId, market: Option<&str>) -> Result<FullPlaylist> {
        let mut req = self.request(Method::GET, &["playlists", id.to_string().as_str()]);

        req.query_param("limit", &DEFAULT_LIMIT.to_string());

        if let Some(market) = market {
            req.query_param("market", market);
        }

        req.execute().await?.json()
    }

    /// Get my devices.
    pub async fn my_player_devices(&self) -> Result<Vec<Device>> {
        let req = self.request(Method::GET, &["me", "player", "devices"]);
        let r = req.execute().await?.json::<Response>()?;
        return Ok(r.devices);

        #[derive(Deserialize)]
        struct Response {
            devices: Vec<Device>,
        }
    }

    /// Set player volume.
    pub async fn me_player_volume(&self, device_id: Option<&str>, volume: f32) -> Result<bool> {
        let volume = u32::min(100, (volume * 100f32).round() as u32).to_string();

        let mut req = self.request(Method::PUT, &["me", "player", "volume"]);

        if let Some(device_id) = device_id {
            req.query_param("device_id", device_id);
        }

        req.query_param("volume_percent", &volume)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_LENGTH, "0")
            .empty_body();

        req.json_map(device_control).await
    }

    /// Start playing a track.
    pub async fn me_player_pause(&self, device_id: Option<&str>) -> Result<bool> {
        let mut req = self.request(Method::PUT, &["me", "player", "pause"]);

        if let Some(device_id) = device_id {
            req.query_param("device_id", device_id);
        }

        req.header(header::CONTENT_LENGTH, "0")
            .header(header::ACCEPT, "application/json")
            .empty_body();

        req.json_map(device_control).await
    }

    /// Information on the current playback.
    pub async fn me_player(&self) -> Result<Option<FullPlayingContext>> {
        let req = self.request(Method::GET, &["me", "player"]);

        req.execute()
            .await?
            .not_found()
            .empty_on_status(StatusCode::NO_CONTENT)
            .json()
    }

    /// Start playing a track.
    pub async fn me_player_play(
        &self,
        device_id: Option<&str>,
        track_uri: Option<&str>,
        position_ms: Option<u64>,
    ) -> Result<bool> {
        let request = Request {
            uris: track_uri,
            position_ms,
        };

        let body = serde_json::to_vec(&request)?;

        let mut req = self.request(Method::PUT, &["me", "player", "play"]);

        if let Some(device_id) = device_id {
            req.query_param("device_id", device_id);
        }

        req.header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .empty_body()
            .body(body);

        return req.json_map(device_control).await;

        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(
                skip_serializing_if = "Option::is_none",
                serialize_with = "option_sequence"
            )]
            uris: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            position_ms: Option<u64>,
        }

        fn option_sequence<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
        where
            T: Serialize,
            S: ser::Serializer,
        {
            if let Some(value) = value {
                return Serialize::serialize(&[value], serializer);
            }

            serializer.serialize_none()
        }
    }

    /// Enqueue the specified track.
    pub async fn me_player_queue(&self, device_id: Option<&str>, track_uri: &str) -> Result<bool> {
        let mut r = self.request(Method::POST, &["me", "player", "queue"]);

        if let Some(device_id) = device_id {
            r.query_param("device_id", device_id);
        }

        r.query_param("uri", track_uri)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json");

        r.json_map(device_control).await
    }

    /// Skip to the next song.
    pub async fn me_player_next(&self, device_id: Option<&str>) -> Result<bool> {
        let mut r = self.request(Method::POST, &["me", "player", "next"]);

        if let Some(device_id) = device_id {
            r.query_param("device_id", device_id);
        }

        r.header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json");

        r.json_map(device_control).await
    }

    /// Get my songs.
    pub async fn my_tracks(&self) -> Result<Page<SavedTrack>> {
        let mut req = self.request(Method::GET, &["me", "tracks"]);
        req.query_param("limit", &DEFAULT_LIMIT.to_string());
        req.execute().await?.json()
    }

    /// Get the full track by ID.
    pub async fn track(&self, id: String, market: Option<&str>) -> Result<FullTrack> {
        let mut req = self.request(Method::GET, &["tracks", id.as_str()]);

        if let Some(market) = market {
            req.query_param("market", market);
        }

        req.execute().await?.json()
    }

    /// Search for tracks.
    pub async fn search_track(&self, q: &str, limit: u32) -> Result<SearchTracks> {
        self.request(Method::GET, &["search"])
            .query_param("type", "track")
            .query_param("q", q)
            .query_param("limit", limit.to_string().as_str())
            .execute()
            .await?
            .json::<SearchTracks>()
    }

    /// Convert a page object into a stream.
    pub fn page_as_stream<'a, T: 'a>(&'a self, page: Page<T>) -> impl Stream<Item = Result<T>> + 'a
    where
        T: Send + DeserializeOwned,
    {
        async_stream::try_stream! {
            let mut current = page.items.into_iter();
            let mut next_url = page.next;

            loop {
                for item in current.by_ref() {
                    yield item;
                }

                let url = match next_url.take() {
                    Some(next) => next,
                    None => break,
                };

                let mut req = RequestBuilder::new(&self.client, self.user_agent, Method::GET, str::parse(&url)?);
                req.token(&self.token);

                let Page { items, next, .. } = req.execute().await?.json::<Page<T>>()?;

                current = items.into_iter();
                next_url = next;
            }
        }
    }
}

/// Handle device control requests.
fn device_control<C>(status: StatusCode, _: &C) -> Result<Option<bool>> {
    match status {
        StatusCode::NO_CONTENT => Ok(Some(true)),
        StatusCode::NOT_FOUND => Ok(Some(false)),
        _ => Ok(None),
    }
}
