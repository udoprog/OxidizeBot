//! Spotify API helpers.

pub use self::model::artist::SimplifiedArtist;
pub use self::model::context::FullPlayingContext;
pub use self::model::device::Device;
pub use self::model::page::Page;
pub use self::model::playlist::{FullPlaylist, SimplifiedPlaylist};
pub use self::model::search::SearchTracks;
pub use self::model::senum::DeviceType;
pub use self::model::track::{FullTrack, SavedTrack};
pub use self::model::user::PrivateUser;
use crate::api::RequestBuilder;
use crate::oauth2;
use crate::prelude::*;
use crate::spotify_id::SpotifyId;
use anyhow::Result;
use bytes::Bytes;
use reqwest::{header, Client, Method, StatusCode};
use std::pin::Pin;
use std::task::{Context, Poll};
use url::Url;

mod model;

const API_URL: &str = "https://api.spotify.com/v1";
const DEFAULT_LIMIT: usize = 50;

/// API integration.
#[derive(Clone, Debug)]
pub struct Spotify {
    client: Client,
    api_url: Url,
    pub token: oauth2::SyncToken,
}

impl Spotify {
    /// Create a new API integration.
    pub fn new(token: oauth2::SyncToken) -> Result<Spotify> {
        Ok(Spotify {
            client: Client::new(),
            api_url: str::parse::<Url>(API_URL)?,
            token,
        })
    }

    /// Get request against API.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("bad base").extend(path);
        RequestBuilder::new(self.client.clone(), method, url).token(self.token.clone())
    }

    /// Get user info.
    pub async fn me(&self) -> Result<PrivateUser> {
        let req = self.request(Method::GET, &["me"]);

        req.execute().await?.json()
    }

    /// Get my playlists.
    pub async fn playlist(&self, id: SpotifyId, market: Option<&str>) -> Result<FullPlaylist> {
        let req = self
            .request(Method::GET, &["playlists", id.to_string().as_str()])
            .query_param("limit", &DEFAULT_LIMIT.to_string())
            .optional_query_param("market", market);

        req.execute().await?.json()
    }

    /// Get my devices.
    pub async fn my_player_devices(&self) -> Result<Vec<Device>> {
        let req = self.request(Method::GET, &["me", "player", "devices"]);
        let r = req.execute().await?.json::<Response>()?;
        return Ok(r.devices);

        #[derive(serde::Deserialize)]
        struct Response {
            devices: Vec<Device>,
        }
    }

    /// Set player volume.
    pub async fn me_player_volume(&self, device_id: Option<&str>, volume: f32) -> Result<bool> {
        let volume = u32::min(100, (volume * 100f32).round() as u32).to_string();

        self.request(Method::PUT, &["me", "player", "volume"])
            .optional_query_param("device_id", device_id)
            .query_param("volume_percent", &volume)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_LENGTH, "0")
            .absent_body(true)
            .json_map(device_control)
            .await
    }

    /// Start playing a track.
    pub async fn me_player_pause(&self, device_id: Option<&str>) -> Result<bool> {
        self.request(Method::PUT, &["me", "player", "pause"])
            .optional_query_param("device_id", device_id)
            .header(header::CONTENT_LENGTH, "0")
            .header(header::ACCEPT, "application/json")
            .absent_body(true)
            .json_map(device_control)
            .await
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

        let body = Bytes::from(serde_json::to_vec(&request)?);

        let r = self
            .request(Method::PUT, &["me", "player", "play"])
            .optional_query_param("device_id", device_id)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .absent_body(true)
            .body(body);

        return r.json_map(device_control).await;

        #[derive(serde::Serialize)]
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
            T: serde::Serialize,
            S: serde::Serializer,
        {
            if let Some(value) = value {
                return serde::Serialize::serialize(&[value], serializer);
            }

            serializer.serialize_none()
        }
    }

    /// Enqueue the specified track.
    pub async fn me_player_queue(&self, device_id: Option<&str>, track_uri: &str) -> Result<bool> {
        let r = self
            .request(Method::POST, &["me", "player", "queue"])
            .query_param("uri", &track_uri)
            .optional_query_param("device_id", device_id)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json");

        r.json_map(device_control).await
    }

    /// Skip to the next song.
    pub async fn me_player_next(&self, device_id: Option<&str>) -> Result<bool> {
        let r = self
            .request(Method::POST, &["me", "player", "next"])
            .optional_query_param("device_id", device_id)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json");

        r.json_map(device_control).await
    }

    /// Get my playlists.
    pub async fn my_playlists(&self) -> Result<Page<SimplifiedPlaylist>> {
        let req = self.request(Method::GET, &["me", "playlists"]);
        req.execute().await?.json()
    }

    /// Get my songs.
    pub async fn my_tracks(&self) -> Result<Page<SavedTrack>> {
        let req = self
            .request(Method::GET, &["me", "tracks"])
            .query_param("limit", &DEFAULT_LIMIT.to_string());

        req.execute().await?.json()
    }

    /// Get the full track by ID.
    pub async fn track(&self, id: String, market: Option<&str>) -> Result<FullTrack> {
        let req = self
            .request(Method::GET, &["tracks", id.as_str()])
            .optional_query_param("market", market);

        req.execute().await?.json()
    }

    /// Search for tracks.
    pub async fn search_track(&self, q: &str) -> Result<Page<FullTrack>> {
        let req = self
            .request(Method::GET, &["search"])
            .query_param("type", "track")
            .query_param("q", q);

        req.execute()
            .await?
            .json::<SearchTracks>()
            .map(|r| r.tracks)
    }

    /// Convert a page object into a stream.
    pub fn page_as_stream<T>(&self, page: Page<T>) -> PageStream<T>
    where
        T: 'static + Send + serde::de::DeserializeOwned,
    {
        self.page_stream(std::future::ready(Ok(page)))
    }

    /// Create a streamed page request.
    fn page_stream<'a, T>(
        &self,
        future: impl Future<Output = Result<Page<T>>> + Send + 'static,
    ) -> PageStream<T> {
        PageStream {
            client: self.client.clone(),
            token: self.token.clone(),
            next: Some(Box::pin(future)),
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

/// A page converted into a stream which will perform pagination under the hood.
pub struct PageStream<T> {
    client: Client,
    token: oauth2::SyncToken,
    next: Option<BoxFuture<'static, Result<Page<T>>>>,
}

impl<T> PageStream<T>
where
    T: serde::de::DeserializeOwned,
{
    /// Get the next page for a type.
    pub fn next_page(&self, url: Url) -> impl Future<Output = Result<Page<T>>> {
        let req =
            RequestBuilder::new(self.client.clone(), Method::GET, url).token(self.token.clone());

        async move { req.execute().await?.json() }
    }
}

impl<T> stream::Stream for PageStream<T>
where
    T: 'static + Send + serde::de::DeserializeOwned,
{
    type Item = Result<Vec<T>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let future = match self.next.as_mut() {
            Some(future) => future,
            None => return Poll::Ready(None),
        };

        let page = match future.as_mut().poll(cx)? {
            Poll::Ready(page) => page,
            Poll::Pending => return Poll::Pending,
        };

        self.as_mut().next = match page.next.map(|s| str::parse(s.as_str())).transpose()? {
            Some(next) => Some(Box::pin(self.next_page(next))),
            None => None,
        };

        Poll::Ready(Some(Ok(page.items)))
    }
}
