//! Spotify API helpers.

use crate::{oauth2, utils::BoxFuture};
use futures::{future, Async, Future, Poll, Stream};
use reqwest::{
    header,
    r#async::{Body, Client, Decoder},
    Method, Url,
};
use rspotify::spotify::model::search;
pub use rspotify::spotify::{
    model::{
        context::FullPlayingContext,
        device::Device,
        page::Page,
        playlist::{FullPlaylist, SimplifiedPlaylist},
        track::{FullTrack, SavedTrack},
    },
    senum::DeviceType,
};
use std::{
    mem,
    sync::{Arc, RwLock},
};

const API_URL: &'static str = "https://api.spotify.com/v1";

/// API integration.
#[derive(Clone, Debug)]
pub struct Spotify {
    client: Client,
    api_url: Url,
    pub token: Arc<RwLock<oauth2::Token>>,
}

impl Spotify {
    /// Create a new API integration.
    pub fn new(token: Arc<RwLock<oauth2::Token>>) -> Result<Spotify, failure::Error> {
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

        RequestBuilder {
            token: Arc::clone(&self.token),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
        }
    }

    /// Get my playlists.
    pub fn playlist(&self, id: &str) -> impl Future<Item = FullPlaylist, Error = failure::Error> {
        return self.request(Method::GET, &["playlists", id]).execute();
    }

    /// Get my devices.
    pub fn my_player_devices(&self) -> impl Future<Item = Vec<Device>, Error = failure::Error> {
        return self
            .request(Method::GET, &["me", "player", "devices"])
            .execute::<Response>()
            .map(|r| r.devices);

        #[derive(serde::Deserialize)]
        struct Response {
            devices: Vec<Device>,
        }
    }

    /// Set player volume.
    pub fn me_player_volume(
        &self,
        device_id: &str,
        volume: f32,
    ) -> impl Future<Item = (), Error = failure::Error> {
        let volume = u32::min(100, (volume * 100f32).round() as u32).to_string();

        self.request(Method::PUT, &["me", "player", "volume"])
            .query_param("device_id", device_id)
            .query_param("volume_percent", &volume)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_LENGTH, "0")
            .execute_empty()
    }

    /// Start playing a track.
    pub fn me_player_pause(
        &self,
        device_id: &str,
    ) -> impl Future<Item = (), Error = failure::Error> {
        self.request(Method::PUT, &["me", "player", "pause"])
            .query_param("device_id", device_id)
            .header(header::CONTENT_LENGTH, "0")
            .header(header::ACCEPT, "application/json")
            .execute_empty()
    }

    /// Information on the current playback.
    pub fn me_player(&self) -> impl Future<Item = FullPlayingContext, Error = failure::Error> {
        self.request(Method::GET, &["me", "player"]).execute()
    }

    /// Start playing a track.
    pub fn me_player_play(
        &self,
        device_id: &str,
        track_uri: Option<&str>,
        position_ms: Option<u64>,
    ) -> impl Future<Item = (), Error = failure::Error> {
        let request = Request {
            uris: track_uri.into_iter().map(|s| s.to_string()).collect(),
            position_ms,
        };

        let r = self
            .request(Method::PUT, &["me", "player", "play"])
            .query_param("device_id", device_id)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json");

        return serialize(&request).and_then(move |body| r.body(Body::from(body)).execute_empty());

        #[derive(serde::Serialize)]
        struct Request {
            #[serde(skip_serializing_if = "Vec::is_empty")]
            uris: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            position_ms: Option<u64>,
        }
    }

    /// Get my playlists.
    pub fn my_playlists(
        &self,
    ) -> impl Future<Item = Page<SimplifiedPlaylist>, Error = failure::Error> {
        return self.request(Method::GET, &["me", "playlists"]).execute();
    }

    /// Get my songs.
    pub fn my_tracks(&self) -> impl Future<Item = Page<SavedTrack>, Error = failure::Error> {
        return self.request(Method::GET, &["me", "tracks"]).execute();
    }

    /// Get my songs.
    pub fn my_tracks_stream(
        self: Arc<Self>,
    ) -> impl Stream<Item = Vec<SavedTrack>, Error = failure::Error> {
        PageStream {
            client: Arc::clone(&self),
            next: Some(Box::new(
                self.request(Method::GET, &["me", "tracks"]).execute(),
            )),
        }
    }

    /// Get the full track by ID.
    pub fn track(&self, id: &str) -> impl Future<Item = FullTrack, Error = failure::Error> {
        return self.request(Method::GET, &["tracks", id]).execute();
    }

    /// Search for tracks.
    pub fn search_track(
        &self,
        q: &str,
    ) -> impl Future<Item = Page<FullTrack>, Error = failure::Error> {
        return self
            .request(Method::GET, &["search"])
            .query_param("type", "track")
            .query_param("q", &q)
            .execute::<search::SearchTracks>()
            .map(|r| r.tracks);
    }

    /// Convert a page object into a stream.
    pub fn page_as_stream<T>(
        self: Arc<Self>,
        page: Page<T>,
    ) -> impl Stream<Item = Vec<T>, Error = failure::Error>
    where
        T: 'static + Send + serde::de::DeserializeOwned,
    {
        PageStream {
            client: Arc::clone(&self),
            next: Some(Box::new(future::ok(page))),
        }
    }

    /// Get the next page for a type.
    pub fn next_page<T>(
        &self,
        page: &Page<T>,
    ) -> Option<impl Future<Item = Page<T>, Error = failure::Error>>
    where
        T: serde::de::DeserializeOwned,
    {
        let next = match page.next.as_ref() {
            Some(next) => next,
            None => return None,
        };

        let url = match str::parse::<Url>(next) {
            Ok(url) => future::ok(url),
            Err(e) => future::err(failure::Error::from(e)),
        };

        let token = Arc::clone(&self.token);
        let client = self.client.clone();

        Some(url.and_then(move |url| {
            let request = RequestBuilder {
                token,
                client,
                url,
                method: Method::GET,
                headers: Vec::new(),
                body: None,
            };

            request.execute()
        }))
    }
}

struct PageStream<T> {
    client: Arc<Spotify>,
    next: Option<BoxFuture<Page<T>, failure::Error>>,
}

impl<T> Stream for PageStream<T>
where
    T: 'static + Send + serde::de::DeserializeOwned,
{
    type Item = Vec<T>;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Vec<T>>, failure::Error> {
        let future = match self.next.as_mut() {
            Some(future) => future,
            None => return Ok(Async::Ready(None)),
        };

        if let Async::Ready(page) = future.poll()? {
            self.next = match self.client.next_page(&page) {
                Some(future) => Some(Box::new(future)),
                None => None,
            };

            return Ok(Async::Ready(Some(page.items)));
        }

        Ok(Async::NotReady)
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
        let token = self.token.read().expect("lock poisoned");
        let access_token = token.access_token().to_string();

        let mut r = self.client.request(self.method, self.url);

        if let Some(body) = self.body {
            r = r.body(body);
        }

        for (key, value) in self.headers {
            r = r.header(key, value);
        }

        r.header(header::AUTHORIZATION, format!("Bearer {}", access_token))
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

    /// Execute the request, expecting nothing back.
    pub fn execute_empty(self) -> impl Future<Item = (), Error = failure::Error> {
        let token = self.token.read().expect("lock poisoned");
        let access_token = token.access_token().to_string();

        let mut r = self.client.request(self.method, self.url);

        if let Some(body) = self.body {
            r = r.body(body);
        }

        for (key, value) in self.headers {
            r = r.header(key, value);
        }

        r.header(header::AUTHORIZATION, format!("Bearer {}", access_token))
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
                    Ok(())
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

    /// Add a query parameter.
    pub fn query_param(mut self, key: &str, value: &str) -> Self {
        self.url.query_pairs_mut().append_pair(key, value);
        self
    }
}

/// Serialize the given argument into a future.
fn serialize<T: serde::Serialize>(value: &T) -> impl Future<Item = Body, Error = failure::Error> {
    match serde_json::to_vec(value) {
        Ok(body) => future::ok(Body::from(body)),
        Err(e) => future::err(failure::Error::from(e)),
    }
}
