//! Twitch API helpers.

use crate::{api::RequestBuilder, oauth2, prelude::*};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use failure::{Error, ResultExt};
use hashbrown::HashMap;
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
const BADGES_TWITCH_URL: &'static str = "https://badges.twitch.tv";
const GQL_URL: &'static str = "https://gql.twitch.tv/gql";

const GQL_CLIENT_ID: &'static str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

mod gql;

/// API integration.
#[derive(Clone, Debug)]
pub struct Twitch {
    client: Client,
    api_url: Url,
    id_url: Url,
    badges_url: Url,
    gql_url: Url,
    pub token: oauth2::SyncToken,
}

impl Twitch {
    /// Create a new API integration.
    pub fn new(token: oauth2::SyncToken) -> Result<Twitch, Error> {
        Ok(Twitch {
            client: Client::new(),
            api_url: str::parse::<Url>(API_TWITCH_URL)?,
            id_url: str::parse::<Url>(ID_TWITCH_URL)?,
            badges_url: str::parse::<Url>(BADGES_TWITCH_URL)?,
            gql_url: str::parse::<Url>(GQL_URL)?,
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

        RequestBuilder::new(self.client.clone(), method, url)
            .token(self.token.clone())
            .client_id_header("Client-ID")
    }

    /// Get request against API.
    fn v5(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.push("kraken");
            url_path.extend(path);
        }

        RequestBuilder::new(self.client.clone(), method, url)
            .header(header::ACCEPT, "application/vnd.twitchtv.v5+json")
            .token(self.token.clone())
            .client_id_header("Client-ID")
            .use_oauth2_header()
    }

    /// Get request against Badges API.
    fn badges_v1(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.badges_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.push("v1");
            url_path.extend(path);
        }

        RequestBuilder::new(self.client.clone(), method, url)
    }

    /// Access GQL client.
    fn gql(&self) -> Result<RequestBuilder, Error> {
        let req = RequestBuilder::new(self.client.clone(), Method::POST, self.gql_url.clone())
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .header(
                str::parse::<header::HeaderName>("Client-ID")?,
                GQL_CLIENT_ID,
            );

        Ok(req)
    }

    /// Update the channel information.
    pub async fn update_channel(
        &self,
        channel_id: &str,
        request: UpdateChannelRequest,
    ) -> Result<(), Error> {
        let body = Bytes::from(serde_json::to_vec(&request)?);

        let req = self
            .v5(Method::PUT, &["channels", channel_id])
            .header(header::CONTENT_TYPE, "application/json")
            .body(body);

        req.execute().await?.ok()
    }

    /// Get information on a user.
    pub async fn user_by_login(&self, login: &str) -> Result<Option<NewUser>, Error> {
        let req = self
            .new_api(Method::GET, &["users"])
            .query_param("login", login);

        let res = req.execute().await?.json::<Data<NewUser>>()?;

        Ok(res.data.into_iter().next())
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

        let req = request.clone();

        let initial = async move { req.execute().await?.json::<Page<Subscription>>() };

        Paged {
            request,
            page: Some(initial.boxed()),
        }
    }

    /// Create a clip for the given broadcaster.
    pub async fn create_clip(&self, broadcaster_id: &str) -> Result<Option<Clip>, Error> {
        let req = self
            .new_api(Method::POST, &["clips"])
            .query_param("broadcaster_id", broadcaster_id);

        let res = req.execute().await?.json::<Data<Clip>>()?;

        Ok(res.data.into_iter().next())
    }

    /// Get the channela associated with the current authentication.
    pub async fn user(&self) -> Result<User, Error> {
        let req = self.v5(Method::GET, &["user"]);
        req.execute().await?.json()
    }

    /// Get the channela associated with the current authentication.
    pub async fn channel(&self) -> Result<Channel, Error> {
        let req = self.v5(Method::GET, &["channel"]);

        req.execute().await?.json::<Channel>()
    }

    /// Get the channela associated with the current authentication.
    pub async fn channel_by_id(&self, channel_id: &str) -> Result<Channel, Error> {
        let req = self.v5(Method::GET, &["channels", channel_id]);
        req.execute().await?.json::<Channel>()
    }

    /// Get stream information.
    pub async fn stream_by_login(&self, login: &str) -> Result<Option<Stream>, Error> {
        let req = self
            .new_api(Method::GET, &["streams"])
            .query_param("user_login", login);

        let res = req.execute().await?.json::<Page<Stream>>()?;

        Ok(res.data.into_iter().next())
    }

    /// Get emotes by sets.
    pub async fn chat_emoticon_images(&self, emote_sets: &str) -> Result<EmoticonSets, Error> {
        let req = self
            .v5(Method::GET, &["chat", "emoticon_images"])
            .query_param("emotesets", emote_sets);
        req.execute().await?.json::<EmoticonSets>()
    }

    /// Get chatters for the given channel using TMI.
    pub async fn chatters(&self, channel: &str) -> Result<Chatters, Error> {
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
    pub async fn validate_token(&self) -> Result<Option<ValidateToken>, Error> {
        let mut url = self.id_url.clone();

        url.path_segments_mut()
            .expect("bad base")
            .extend(&["oauth2", "validate"]);

        let request = RequestBuilder::new(self.client.clone(), Method::GET, url)
            .token(self.token.clone())
            .client_id_header("Client-ID")
            .use_oauth2_header();

        return Ok(request
            .execute()
            .await?
            .json_option(unauthorized)
            .context("validate token error")?);
    }

    /// Get badge URLs for the specified channel.
    pub async fn badges_display(&self, channel_id: &str) -> Result<Option<BadgesDisplay>, Error> {
        let req = self.badges_v1(Method::GET, &["badges", "channels", &channel_id, "display"]);

        Ok(req
            .execute()
            .await?
            .json_option(not_found)
            .context("request badges")?)
    }

    /// Get all badge URLs for the given chat.
    pub async fn chat_badges(&self, channel_id: &str) -> Result<Option<ChatBadges>, Error> {
        let req = self.v5(Method::GET, &["chat", &channel_id, "badges"]);

        Ok(req
            .execute()
            .await?
            .json_option(not_found)
            .context("request chat badges")?)
    }

    /// Get display badges through GQL.
    pub async fn gql_display_badges(
        &self,
        login: &str,
        channel: &str,
    ) -> Result<Option<self::gql::badges::ResponseData>, Error> {
        use graphql_client::{GraphQLQuery as _, Response};

        let body = self::gql::Badges::build_query(self::gql::badges::Variables {
            login: login.to_string(),
            channel_login: channel.to_string(),
        });

        let req = self.gql()?.body(serde_json::to_vec(&body)?);

        let res = req
            .execute()
            .await?
            .json::<Response<self::gql::badges::ResponseData>>()?
            .data;

        Ok(res)
    }
}

/// Handle unahtorized as a missing body.
fn unauthorized(status: &StatusCode) -> bool {
    match *status {
        StatusCode::UNAUTHORIZED => true,
        _ => false,
    }
}

/// Handle not found as a missing body.
fn not_found(status: &StatusCode) -> bool {
    match *status {
        StatusCode::NOT_FOUND => true,
        _ => false,
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
                                let req = self.request.clone().query_param("after", &cursor);
                                self.as_mut().page =
                                    Some(async move { req.execute().await?.json() }.boxed());
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

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct UpdateChannelRequest {
    pub channel: UpdateChannel,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct UpdateChannel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_feed_enabled: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NewUser {
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
pub struct User {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub bio: Option<String>,
    pub email: String,
    pub email_verified: bool,
    #[serde(default)]
    pub logo: Option<String>,
    pub notifications: HashMap<String, bool>,
    pub partnered: bool,
    pub twitter_connected: bool,
    #[serde(rename = "type")]
    pub ty: String,
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
    #[serde(default)]
    pub game_id: Option<String>,
    #[serde(default)]
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
    #[serde(default)]
    pub broadcaster_language: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub game: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub partner: bool,
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default)]
    pub video_banner: Option<String>,
    #[serde(default)]
    pub profile_banner: Option<String>,
    #[serde(default)]
    pub profile_banner_background_color: Option<String>,
    pub url: String,
    pub views: u64,
    pub followers: u64,
    #[serde(default)]
    pub broadcaster_type: Option<String>,
    #[serde(default)]
    pub stream_key: Option<String>,
    #[serde(default)]
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
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    #[serde(default)]
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

#[derive(Debug, serde::Deserialize)]
pub struct Emote {
    pub code: String,
    pub id: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct EmoticonSets {
    pub emoticon_sets: HashMap<String, Vec<Emote>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Badge {
    pub image_url_1x: String,
    pub image_url_2x: String,
    pub image_url_4x: String,
    pub description: String,
    pub title: String,
    pub click_action: String,
    pub click_url: String,
    #[serde(default)]
    pub last_updated: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BadgeSet {
    pub versions: HashMap<String, Badge>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BadgesDisplay {
    pub badge_sets: HashMap<String, BadgeSet>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BadgeTypes {
    #[serde(default)]
    pub alpha: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub svg: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ChatBadges {
    #[serde(flatten)]
    pub badges: HashMap<String, BadgeTypes>,
}
