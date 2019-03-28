//! Twitch API helpers.

use crate::oauth2;
use chrono::{DateTime, Utc};
use futures::{future, Future, Stream as _};
use parking_lot::RwLock;
use reqwest::{
    header,
    r#async::{Body, Client, Decoder},
    Method, Url,
};
use std::{mem, sync::Arc};

pub const CLIPS_URL: &'static str = "http://clips.twitch.tv";
const TMI_TWITCH_URL: &'static str = "https://tmi.twitch.tv";
const API_TWITCH_URL: &'static str = "https://api.twitch.tv";
const GQL_TWITCH_URL: &'static str = "https://gql.twitch.tv/gql";

/// API integration.
#[derive(Clone, Debug)]
pub struct Twitch {
    client: Client,
    api_url: Url,
    gql_url: Url,
    token: Arc<RwLock<oauth2::Token>>,
}

impl Twitch {
    /// Create a new API integration.
    pub fn new(token: Arc<RwLock<oauth2::Token>>) -> Result<Twitch, failure::Error> {
        Ok(Twitch {
            client: Client::new(),
            api_url: str::parse::<Url>(API_TWITCH_URL)?,
            gql_url: str::parse::<Url>(GQL_TWITCH_URL)?,
            token,
        })
    }

    /// Get request against API.
    fn new_api(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.push("helix");
            url_path.extend(path);
        }

        RequestBuilder {
            token: Arc::clone(&self.token),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
            use_bearer: true,
        }
    }

    /// Get request against API.
    fn v5(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.push("kraken");
            url_path.extend(path);
        }

        RequestBuilder {
            token: Arc::clone(&self.token),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
            use_bearer: false,
        }
    }

    /// Build request against GQL api.
    fn gql(&self, method: Method) -> RequestBuilder {
        let mut url = self.gql_url.clone();
        url.path_segments_mut().expect("bad base").push("gql");

        RequestBuilder {
            token: Arc::clone(&self.token),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
            use_bearer: false,
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
            .v5(Method::PUT, &["channels", channel_id])
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

        self.new_api(Method::GET, &["users"])
            .query_param("login", login.as_str())
            .execute::<Data<User>>()
            .map(|data| data.data.into_iter().next())
    }

    /// Create a clip for the given broadcaster.
    pub fn create_clip(
        &self,
        broadcaster_id: &str,
    ) -> impl Future<Item = Option<Clip>, Error = failure::Error> {
        return self
            .new_api(Method::POST, &["clips"])
            .query_param("broadcaster_id", broadcaster_id)
            .execute::<Data<Clip>>()
            .map(|data| data.data.into_iter().next());
    }

    /// Update the title of a clip.
    pub fn update_clip_title(
        &self,
        clip_id: &str,
        title: &str,
    ) -> impl Future<Item = (), Error = failure::Error> {
        let body = vec![Request {
            operation_name: "ClipsTitleEdit_UpdateClip",
            variables: Variables {
                input: Input {
                    title: title.to_string(),
                    slug: clip_id.to_string(),
                },
            },
        }];

        let future = Self::serialize(&body);

        let req = self.gql(Method::POST);

        return future
            .and_then(move |body| req.body(body).execute::<serde_json::Value>())
            .map(|_| ());

        #[derive(serde::Serialize)]
        struct Request {
            #[serde(rename = "operationName")]
            operation_name: &'static str,
            variables: Variables,
        }

        #[derive(serde::Serialize)]
        struct Variables {
            input: Input,
        }

        #[derive(serde::Serialize)]
        struct Input {
            title: String,
            slug: String,
        }
    }

    /// Get the channela associated with the current authentication.
    pub fn channel(&self) -> impl Future<Item = Channel, Error = failure::Error> {
        self.v5(Method::GET, &["channel"]).execute::<Channel>()
    }

    /// Get the channela associated with the current authentication.
    pub fn channel_by_login(
        &self,
        login: &str,
    ) -> impl Future<Item = Channel, Error = failure::Error> {
        self.v5(Method::GET, &["channels", login])
            .execute::<Channel>()
    }

    /// Get stream information.
    pub fn stream_by_login(
        &self,
        login: &str,
    ) -> impl Future<Item = Option<Stream>, Error = failure::Error> {
        return self
            .new_api(Method::GET, &["streams"])
            .query_param("user_login", login)
            .execute::<Page<Stream>>()
            .map(|data| data.data.into_iter().next());
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
    /// Use Bearer header instead of OAuth for access tokens.
    use_bearer: bool,
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

        if self.use_bearer {
            r = r.header(header::AUTHORIZATION, format!("Bearer {}", access_token));
        } else {
            r = r.header(header::AUTHORIZATION, format!("OAuth {}", access_token));
        }

        r.header("Client-ID", token.client_id())
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
    pub login: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub broadcaster_type: String,
    pub description: String,
    pub profile_image_url: String,
    pub offline_image_url: String,
    pub view_count: u64,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct StreamInfo {
    pub started_at: DateTime<Utc>,
    pub title: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Stream {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub game_id: Option<String>,
    pub community_ids: Vec<String>,
    #[serde(rename = "type")]
    pub ty: String,
    pub title: String,
    pub viewer_count: u64,
    pub started_at: DateTime<Utc>,
    pub language: String,
    pub thumbnail_url: String,
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

#[derive(serde::Deserialize)]
pub struct Clip {
    pub id: String,
    pub edit_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Pagination {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Page<T> {
    data: Vec<T>,
    pagination: Pagination,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Data<T> {
    pub data: Vec<T>,
}
