//! Twitch API helpers.

use crate::{oauth2, prelude::*};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use failure::{Error, ResultExt};
use reqwest::{
    header,
    r#async::{Client, Decoder},
    Method, StatusCode, Url,
};
use std::mem;

pub const CLIPS_URL: &'static str = "http://clips.twitch.tv";
const TMI_TWITCH_URL: &'static str = "https://tmi.twitch.tv";
const API_TWITCH_URL: &'static str = "https://api.twitch.tv";
const ID_TWITCH_URL: &'static str = "https://id.twitch.tv";

/// API integration.
#[derive(Clone, Debug)]
pub struct Twitch {
    client: Client,
    api_url: Url,
    id_url: Url,
    pub token: oauth2::SyncToken,
}

impl Twitch {
    /// Create a new API integration.
    pub fn new(token: oauth2::SyncToken) -> Result<Twitch, Error> {
        Ok(Twitch {
            client: Client::new(),
            api_url: str::parse::<Url>(API_TWITCH_URL)?,
            id_url: str::parse::<Url>(ID_TWITCH_URL)?,
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
            token: self.token.clone(),
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
            token: self.token.clone(),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
            use_bearer: false,
        }
    }

    /// Update the channel information.
    pub fn update_channel(
        &self,
        channel_id: &str,
        request: UpdateChannelRequest,
    ) -> impl Future<Output = Result<(), Error>> {
        let req = self
            .v5(Method::PUT, &["channels", channel_id])
            .header(header::CONTENT_TYPE, "application/json");

        async move {
            let body = Bytes::from(serde_json::to_vec(&request)?);
            let _ = req.body(body).execute::<serde_json::Value>().await?;
            Ok(())
        }
    }

    /// Get information on a user.
    pub fn user_by_login(&self, login: &str) -> impl Future<Output = Result<Option<User>, Error>> {
        let req = self
            .new_api(Method::GET, &["users"])
            .query_param("login", login)
            .execute::<Data<User>>();

        async move { Ok(req.await?.data.into_iter().next()) }
    }

    /// Get information on a user.
    pub fn stream_subscriptions(
        &self,
        broadcaster_id: &str,
        user_ids: Vec<String>,
    ) -> Paged<Subscription> {
        let mut request = self
            .new_api(Method::GET, &["subscriptions"])
            .query_param("broadcaster_id", broadcaster_id);

        for user_id in user_ids {
            request = request.query_param("user_id", &user_id);
        }

        let copied = request.clone_without_body();
        let initial = request.execute::<Page<Subscription>>();

        Paged {
            request: copied,
            page: Some(initial.boxed()),
        }
    }

    /// Create a clip for the given broadcaster.
    pub fn create_clip(
        &self,
        broadcaster_id: &str,
    ) -> impl Future<Output = Result<Option<Clip>, Error>> {
        let req = self
            .new_api(Method::POST, &["clips"])
            .query_param("broadcaster_id", broadcaster_id)
            .execute::<Data<Clip>>();

        async move { Ok(req.await?.data.into_iter().next()) }
    }

    /// Get the channela associated with the current authentication.
    pub async fn channel(&self) -> Result<Channel, Error> {
        self.v5(Method::GET, &["channel"])
            .execute::<Channel>()
            .await
    }

    /// Get the channela associated with the current authentication.
    pub fn channel_by_login(&self, login: &str) -> impl Future<Output = Result<Channel, Error>> {
        self.v5(Method::GET, &["channels", login])
            .execute::<Channel>()
    }

    /// Get stream information.
    pub fn stream_by_login(
        &self,
        login: &str,
    ) -> impl Future<Output = Result<Option<Stream>, Error>> {
        let req = self
            .new_api(Method::GET, &["streams"])
            .query_param("user_login", login)
            .execute::<Page<Stream>>();

        async move { Ok(req.await?.data.into_iter().next()) }
    }

    /// Get chatters for the given channel using TMI.
    pub async fn chatters(&self, channel: String) -> Result<Chatters, Error> {
        let channel = channel.trim_start_matches('#');
        let url = format!("{}/group/user/{}/chatters", TMI_TWITCH_URL, channel);

        let mut res = self.client.get(&url).send().compat().await?;
        let body = mem::replace(res.body_mut(), Decoder::empty());
        let body = body.compat().try_concat().await?;

        return serde_json::from_slice::<Response>(body.as_ref())
            .map(|l| l.chatters)
            .map_err(Into::into);

        #[derive(serde::Deserialize)]
        struct Response {
            chatters: Chatters,
        }
    }

    // Validate the specified token through twitch validation API.
    pub async fn validate_token(&self) -> Result<ValidateToken, Error> {
        let mut url = self.id_url.clone();

        url.path_segments_mut()
            .expect("bad base")
            .extend(&["oauth2", "validate"]);

        let request = RequestBuilder {
            token: self.token.clone(),
            client: self.client.clone(),
            url,
            method: Method::GET,
            headers: Vec::new(),
            body: None,
            use_bearer: false,
        };

        Ok(request.execute().await.context("validate token error")?)
    }
}

/// A response that is paged as a stream of requests.
pub struct Paged<T> {
    request: RequestBuilder,
    page: Option<future::BoxFuture<'static, Result<Page<T>, Error>>>,
}

impl<T> futures::Stream for Paged<T>
where
    T: 'static + Send + serde::de::DeserializeOwned,
{
    type Item = Result<Vec<T>, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Some(page) = self.as_mut().page.as_mut() {
            match unsafe { Pin::new_unchecked(page) }.poll(cx) {
                Poll::Pending => {
                    return Poll::Pending;
                }
                Poll::Ready(result) => {
                    self.as_mut().page = None;

                    match result {
                        Err(e) => {
                            return Poll::Ready(Some(Err(e)));
                        }
                        Ok(page) => {
                            let Page { data, pagination } = page;

                            if data.is_empty() {
                                return Poll::Ready(None);
                            }

                            if let Some(cursor) = pagination.and_then(|p| p.cursor) {
                                let req = self
                                    .request
                                    .clone_without_body()
                                    .query_param("after", &cursor);
                                self.as_mut().page = Some(req.execute().boxed());
                            }

                            return Poll::Ready(Some(Ok(data)));
                        }
                    }
                }
            }
        }

        Poll::Ready(None)
    }
}

struct RequestBuilder {
    token: oauth2::SyncToken,
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Bytes>,
    /// Use Bearer header instead of OAuth for access tokens.
    use_bearer: bool,
}

impl RequestBuilder {
    /// Clone the request but discard the body since it's not cloneable.
    pub fn clone_without_body(&self) -> RequestBuilder {
        RequestBuilder {
            token: self.token.clone(),
            client: self.client.clone(),
            url: self.url.clone(),
            method: self.method.clone(),
            headers: self.headers.clone(),
            body: None,
            use_bearer: self.use_bearer,
        }
    }

    /// Execute the request.
    pub async fn execute<T>(self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        // NB: scope to only lock the token over the request setup.
        let req = {
            let token = self.token.read()?;
            let access_token = token.access_token().to_string();

            log::trace!("request: {}: {}", self.method, self.url);
            let mut req = self.client.request(self.method, self.url);

            if let Some(body) = self.body {
                req = req.body(body);
            }

            for (key, value) in self.headers {
                req = req.header(key, value);
            }

            if self.use_bearer {
                req = req.header(header::AUTHORIZATION, format!("Bearer {}", access_token));
            } else {
                req = req.header(header::AUTHORIZATION, format!("OAuth {}", access_token));
            }

            req.header("Client-ID", token.client_id())
        };

        let mut res = req.send().compat().await?;

        let body = mem::replace(res.body_mut(), Decoder::empty());
        let body = body.compat().try_concat().await?;

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
    pub fn body(mut self, body: Bytes) -> Self {
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
pub struct Subscription {
    pub broadcaster_id: String,
    pub broadcaster_name: String,
    pub is_gift: bool,
    pub tier: String,
    pub plan_name: String,
    pub user_id: String,
    pub user_name: String,
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
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Data<T> {
    pub data: Vec<T>,
}

/// Response from the validate token endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct ValidateToken {
    pub client_id: String,
    pub login: String,
    pub scopes: Vec<String>,
    pub user_id: String,
}
