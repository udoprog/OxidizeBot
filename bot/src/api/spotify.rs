//! Spotify API helpers.

use crate::{api::RequestBuilder, oauth2, prelude::*};
use bytes::Bytes;
use failure::Error;
use reqwest::{header, r#async::Client, Method, StatusCode, Url};
use rspotify::spotify::model::search;
pub use rspotify::spotify::{
    model::{
        artist::SimplifiedArtist,
        context::FullPlayingContext,
        device::Device,
        page::Page,
        playlist::{FullPlaylist, SimplifiedPlaylist},
        track::{FullTrack, SavedTrack},
    },
    senum::DeviceType,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

const API_URL: &'static str = "https://api.spotify.com/v1";

/// API integration.
#[derive(Clone, Debug)]
pub struct Spotify {
    client: Client,
    api_url: Url,
    pub token: oauth2::SyncToken,
}

impl Spotify {
    /// Create a new API integration.
    pub fn new(token: oauth2::SyncToken) -> Result<Spotify, Error> {
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

    /// Get my playlists.
    pub async fn playlist(&self, id: String) -> Result<FullPlaylist, Error> {
        self.request(Method::GET, &["playlists", id.as_str()])
            .json()
            .await
    }

    /// Get my devices.
    pub async fn my_player_devices(&self) -> Result<Vec<Device>, Error> {
        let r = self
            .request(Method::GET, &["me", "player", "devices"])
            .json::<Response>()
            .await?;

        return Ok(r.devices);

        #[derive(serde::Deserialize)]
        struct Response {
            devices: Vec<Device>,
        }
    }

    /// Set player volume.
    pub async fn me_player_volume(
        &self,
        device_id: Option<String>,
        volume: f32,
    ) -> Result<bool, Error> {
        let volume = u32::min(100, (volume * 100f32).round() as u32).to_string();

        self.request(Method::PUT, &["me", "player", "volume"])
            .optional_query_param("device_id", device_id)
            .query_param("volume_percent", &volume)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_LENGTH, "0")
            .json_map(device_control)
            .await
    }

    /// Start playing a track.
    pub async fn me_player_pause(&self, device_id: Option<String>) -> Result<bool, Error> {
        self.request(Method::PUT, &["me", "player", "pause"])
            .optional_query_param("device_id", device_id)
            .header(header::CONTENT_LENGTH, "0")
            .header(header::ACCEPT, "application/json")
            .json_map(device_control)
            .await
    }

    /// Information on the current playback.
    pub async fn me_player(&self) -> Result<Option<FullPlayingContext>, Error> {
        self.request(Method::GET, &["me", "player"])
            .json_option(not_found)
            .await
    }

    /// Start playing a track.
    pub async fn me_player_play(
        &self,
        device_id: Option<String>,
        track_uri: Option<String>,
        position_ms: Option<u64>,
    ) -> Result<bool, Error> {
        let request = Request {
            uris: track_uri.into_iter().collect(),
            position_ms,
        };

        let r = self
            .request(Method::PUT, &["me", "player", "play"])
            .optional_query_param("device_id", device_id)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json");

        let body = Bytes::from(serde_json::to_vec(&request)?);
        return r.body(body).json_map(device_control).await;

        #[derive(serde::Serialize)]
        struct Request {
            #[serde(skip_serializing_if = "Vec::is_empty")]
            uris: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            position_ms: Option<u64>,
        }
    }

    /// Get my playlists.
    pub async fn my_playlists(&self) -> Result<Page<SimplifiedPlaylist>, Error> {
        self.request(Method::GET, &["me", "playlists"]).json().await
    }

    /// Get my songs.
    pub async fn my_tracks(&self) -> Result<Page<SavedTrack>, Error> {
        self.request(Method::GET, &["me", "tracks"]).json().await
    }

    /// Get my songs.
    pub fn my_tracks_stream(&self) -> PageStream<SavedTrack> {
        self.page_stream(self.request(Method::GET, &["me", "tracks"]).json())
    }

    /// Get the full track by ID.
    pub async fn track(&self, id: String) -> Result<FullTrack, Error> {
        self.request(Method::GET, &["tracks", id.as_str()])
            .json()
            .await
    }

    /// Search for tracks.
    pub async fn search_track(&self, q: String) -> Result<Page<FullTrack>, Error> {
        self.request(Method::GET, &["search"])
            .query_param("type", "track")
            .query_param("q", &q)
            .json::<search::SearchTracks>()
            .await
            .map(|r| r.tracks)
    }

    /// Convert a page object into a stream.
    pub fn page_as_stream<T>(&self, page: Page<T>) -> PageStream<T>
    where
        T: 'static + Send + serde::de::DeserializeOwned,
    {
        self.page_stream(future::ok(page))
    }

    /// Create a streamed page request.
    fn page_stream<T>(
        &self,
        future: impl Future<Output = Result<Page<T>, Error>> + Send + 'static,
    ) -> PageStream<T> {
        PageStream {
            client: self.client.clone(),
            token: self.token.clone(),
            next: Some(future.boxed()),
        }
    }
}

/// Handle device control requests.
fn device_control<C>(status: &StatusCode, _: &C) -> Result<Option<bool>, Error> {
    match *status {
        StatusCode::NO_CONTENT => Ok(Some(true)),
        StatusCode::NOT_FOUND => Ok(Some(false)),
        _ => Ok(None),
    }
}

/// Handle not found as a missing body.
fn not_found(status: &StatusCode) -> bool {
    match *status {
        StatusCode::NOT_FOUND => true,
        StatusCode::NO_CONTENT => true,
        _ => false,
    }
}

pub struct PageStream<T> {
    client: Client,
    token: oauth2::SyncToken,
    next: Option<future::BoxFuture<'static, Result<Page<T>, Error>>>,
}

impl<T> PageStream<T>
where
    T: serde::de::DeserializeOwned,
{
    /// Get the next page for a type.
    pub fn next_page(&self, url: Url) -> impl Future<Output = Result<Page<T>, Error>> {
        RequestBuilder::new(self.client.clone(), Method::GET, url)
            .token(self.token.clone())
            .json()
    }
}

impl<T> TryStream for PageStream<T>
where
    T: 'static + Send + serde::de::DeserializeOwned,
{
    type Ok = Vec<T>;
    type Error = Error;

    fn try_poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Ok, Self::Error>>> {
        let future = match self.next.as_mut() {
            Some(future) => future,
            None => return Poll::Ready(None),
        };

        if let Poll::Ready(page) = future.as_mut().poll(cx)? {
            self.as_mut().next = match page.next.map(|s| str::parse(s.as_str())).transpose()? {
                Some(next) => Some(self.next_page(next).boxed()),
                None => None,
            };

            return Poll::Ready(Some(Ok(page.items)));
        }

        Poll::Pending
    }
}
