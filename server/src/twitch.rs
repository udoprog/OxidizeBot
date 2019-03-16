//! Twitch API helpers.

use crate::oauth2;
use chrono::{DateTime, Utc};
use futures::{future, Future, Stream as _};
use reqwest::{
    header,
    r#async::{Body, Client, Decoder},
    Method, Url,
};
use std::{
    mem,
    sync::{Arc, RwLock},
};

const TMI_TWITCH_URL: &'static str = "https://tmi.twitch.tv";
const API_TWITCH_URL: &'static str = "https://api.twitch.tv";

/// API integration.
#[derive(Clone, Debug)]
pub struct Twitch {
    client: Client,
    api_url: Url,
    token: Arc<RwLock<oauth2::Token>>,
}

impl Twitch {
    /// Create a new API integration.
    pub fn new(token: Arc<RwLock<oauth2::Token>>) -> Result<Twitch, failure::Error> {
        Ok(Twitch {
            client: Client::new(),
            api_url: str::parse::<Url>(API_TWITCH_URL)?,
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
    pub fn update_channel(
        &self,
        channel_id: &str,
        request: &UpdateChannelRequest,
    ) -> impl Future<Item = (), Error = failure::Error> {
        let req = self
            .request(Method::PUT, &["kraken", "channels", channel_id])
            .header(header::CONTENT_TYPE, "application/json");

        Self::serialize(request)
            .and_then(move |body| req.body(body).execute::<serde_json::Value>())
            .and_then(|_| Ok(()))
    }

    /// Get information on a user.
    pub fn user_by_login(
        &self,
        login: &str,
    ) -> impl Future<Item = Option<User>, Error = failure::Error> {
        let login = login.to_string();

        self.request(Method::GET, &["helix", "users"])
            .query_param("login", login.as_str())
            .execute::<Data<User>>()
            .map(|data| data.data.into_iter().next())
    }

    /// Get the channela associated with the current authentication.
    pub fn channel(&self) -> impl Future<Item = Channel, Error = failure::Error> {
        self.request(Method::GET, &["kraken", "channel"])
            .execute::<Channel>()
    }

    /// Get the channela associated with the current authentication.
    pub fn channel_by_id(&self, id: &str) -> impl Future<Item = Channel, Error = failure::Error> {
        self.request(Method::GET, &["kraken", "channels", id])
            .execute::<Channel>()
    }

    /// Get stream information.
    pub fn stream_by_id(
        &self,
        id: &str,
    ) -> impl Future<Item = Option<Stream>, Error = failure::Error> {
        return self
            .request(Method::GET, &["kraken", "streams", id])
            .execute::<Response>()
            .map(|data| data.stream);

        #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
        pub struct Response {
            stream: Option<Stream>,
        }
    }

    /// Get chatters for the given channel using TMI.
    pub fn chatters(&self, channel: &str) -> impl Future<Item = Chatters, Error = failure::Error> {
        let channel = channel.trim_start_matches('#');
        let url = format!("{}/group/user/{}/chatters", TMI_TWITCH_URL, channel);

        return self
            .client
            .get(&url)
            .send()
            .and_then(|mut res| mem::replace(res.body_mut(), Decoder::empty()).concat2())
            .map_err(Into::into)
            .and_then(|body| {
                serde_json::from_slice::<Response>(body.as_ref())
                    .map(|l| l.chatters)
                    .map_err(Into::into)
            });

        #[derive(serde::Deserialize)]
        struct Response {
            chatters: Chatters,
        }
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

    /// Add a query parameter.
    pub fn query_param(mut self, key: &str, value: &str) -> Self {
        self.url.query_pairs_mut().append_pair(key, value);
        self
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct UpdateChannelRequest {
    pub channel: UpdateChannel,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct UpdateChannel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_feed_enabled: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct User {
    pub id: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct StreamInfo {
    pub started_at: DateTime<Utc>,
    pub title: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Stream {
    #[serde(rename = "_id")]
    pub id: u64,
    pub viewers: u32,
    pub game: Option<String>,
    pub video_height: u32,
    pub average_fps: u32,
    pub delay: u32,
    pub created_at: DateTime<Utc>,
    pub is_playlist: bool,
    pub stream_type: String,
    pub channel: Channel,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Channel {
    pub mature: bool,
    pub status: String,
    pub broadcaster_language: Option<String>,
    pub display_name: Option<String>,
    pub game: Option<String>,
    pub language: Option<String>,
    #[serde(rename = "_id")]
    pub id: u64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub partner: bool,
    pub logo: Option<String>,
    pub video_banner: Option<String>,
    pub profile_banner: Option<String>,
    pub profile_banner_background_color: Option<String>,
    pub url: String,
    pub views: u64,
    pub followers: u64,
    pub broadcaster_type: Option<String>,
    pub stream_key: Option<String>,
    pub email: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct Chatters {
    pub broadcaster: Vec<String>,
    pub vips: Vec<String>,
    pub moderators: Vec<String>,
    pub staff: Vec<String>,
    pub admins: Vec<String>,
    pub global_mods: Vec<String>,
    pub viewers: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Data<T> {
    pub data: Vec<T>,
}
