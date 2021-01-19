//! Twitch API helpers.

use crate::api::RequestBuilder;
use crate::oauth2;
use anyhow::{Context as _, Result};
use bytes::Bytes;
use reqwest::{header, Client, Method, StatusCode, Url};

pub const CLIPS_URL: &str = "http://clips.twitch.tv";
const TMI_TWITCH_URL: &str = "https://tmi.twitch.tv";
const API_TWITCH_URL: &str = "https://api.twitch.tv";
const ID_TWITCH_URL: &str = "https://id.twitch.tv";
const BADGES_TWITCH_URL: &str = "https://badges.twitch.tv";
const GQL_URL: &str = "https://gql.twitch.tv/gql";

const GQL_CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

mod gql;
mod model;
pub mod pubsub;

pub use self::model::*;

/// Twitch API client.
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
    /// Create a new Twitch API client.
    pub fn new(token: oauth2::SyncToken) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            api_url: str::parse::<Url>(API_TWITCH_URL)?,
            id_url: str::parse::<Url>(ID_TWITCH_URL)?,
            badges_url: str::parse::<Url>(BADGES_TWITCH_URL)?,
            gql_url: str::parse::<Url>(GQL_URL)?,
            token,
        })
    }

    /// Get chatters for the given channel using TMI.
    pub async fn chatters(&self, channel: &str) -> Result<Chatters> {
        let channel = channel.trim_start_matches('#');

        let url = Url::parse(&format!(
            "{}/group/user/{}/chatters",
            TMI_TWITCH_URL, channel
        ))?;

        let req = RequestBuilder::new(self.client.clone(), Method::GET, url)
            .header(header::ACCEPT, "application/json");

        let body = req.execute().await?.json::<Response>()?;

        return Ok(body.chatters);

        #[derive(serde::Deserialize)]
        struct Response {
            chatters: Chatters,
        }
    }

    // Validate the specified token through twitch validation API.
    pub async fn validate_token(&self) -> Result<Option<ValidateToken>> {
        let mut url = self.id_url.clone();

        url.path_segments_mut()
            .expect("bad base")
            .extend(&["oauth2", "validate"]);

        let request = RequestBuilder::new(self.client.clone(), Method::GET, url)
            .token(self.token.clone())
            .client_id_header("Client-ID")
            .use_oauth2_header();

        Ok(request
            .execute()
            .await?
            .empty_on_status(StatusCode::UNAUTHORIZED)
            .json()
            .context("validate token error")?)
    }

    /// Get badge URLs for the specified channel.
    pub async fn badges_v1_display(
        &self,
        channel_id: &str,
    ) -> Result<Option<badges_v1::BadgesDisplay>> {
        let req = self.badges_v1(Method::GET, &["badges", "channels", &channel_id, "display"]);

        Ok(req
            .execute()
            .await?
            .not_found()
            .json()
            .context("request badges")?)
    }

    /// Get display badges through GQL.
    pub async fn gql_display_badges(
        &self,
        login: &str,
        channel: &str,
    ) -> Result<Option<self::gql::badges::ResponseData>> {
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

    /// Get information on a user.
    pub async fn new_user_by_login(&self, login: &str) -> Result<Option<new::User>> {
        let req = self
            .new_api(Method::GET, &["users"])
            .query_param("login", login);

        let res = req.execute().await?.json::<Data<Vec<new::User>>>()?;
        Ok(res.data.into_iter().next())
    }

    /// Get information on a user.
    pub fn new_stream_subscriptions(
        &self,
        broadcaster_id: &str,
        user_ids: Vec<String>,
    ) -> new::Paged<new::Subscription> {
        let mut request = self
            .new_api(Method::GET, &["subscriptions"])
            .query_param("broadcaster_id", broadcaster_id);

        for user_id in &user_ids {
            request = request.query_param("user_id", user_id);
        }

        let req = request.clone();

        let initial = async move { req.execute().await?.json::<new::Page<new::Subscription>>() };

        new::Paged {
            request,
            page: Some(Box::pin(initial)),
        }
    }

    /// Create a clip for the given broadcaster.
    pub async fn new_create_clip(&self, broadcaster_id: &str) -> Result<Option<new::Clip>> {
        let req = self
            .new_api(Method::POST, &["clips"])
            .query_param("broadcaster_id", broadcaster_id);

        let res = req.execute().await?.json::<Data<Vec<new::Clip>>>()?;
        Ok(res.data.into_iter().next())
    }

    /// Get stream information.
    pub async fn new_stream_by_id(&self, id: &str) -> Result<Option<new::Stream>> {
        let req = self
            .new_api(Method::GET, &["streams"])
            .query_param("user_id", id);

        let res = req.execute().await?.json::<new::Page<new::Stream>>()?;
        Ok(res.data.into_iter().next())
    }

    /// Update the status of a redemption.
    pub async fn new_update_redemption_status(
        &self,
        broadcaster_id: &str,
        redemption: &pubsub::Redemption,
        status: pubsub::Status,
    ) -> Result<()> {
        use serde::Serialize;

        let req = self
            .new_api(
                Method::PATCH,
                &["channel_points", "custom_rewards", "redemptions"],
            )
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .query_param("broadcaster_id", broadcaster_id)
            .query_param("id", &redemption.id)
            .query_param("reward_id", &redemption.reward.id)
            .body(serde_json::to_vec(&UpdateRedemption { status })?);

        req.execute()
            .await?
            .json::<Data<Vec<serde_json::Value>>>()?;

        return Ok(());

        #[derive(Serialize)]
        struct UpdateRedemption {
            status: pubsub::Status,
        }
    }

    /// Update the channel information.
    pub async fn v5_update_channel(
        &self,
        channel_id: &str,
        request: v5::UpdateChannelRequest,
    ) -> Result<()> {
        let body = Bytes::from(serde_json::to_vec(&request)?);

        let req = self
            .v5(Method::PUT, &["channels", channel_id])
            .header(header::CONTENT_TYPE, "application/json")
            .body(body);

        req.execute().await?.ok()
    }

    /// Get the channela associated with the current authentication.
    pub async fn v5_user(&self) -> Result<v5::User> {
        let req = self.v5(Method::GET, &["user"]);
        req.execute().await?.json()
    }

    /// Get the channela associated with the current authentication.
    pub async fn v5_channel(&self) -> Result<v5::Channel> {
        let req = self.v5(Method::GET, &["channel"]);
        req.execute().await?.json::<v5::Channel>()
    }

    /// Get the channela associated with the current authentication.
    pub async fn v5_channel_by_id(&self, channel_id: &str) -> Result<v5::Channel> {
        let req = self.v5(Method::GET, &["channels", channel_id]);
        req.execute().await?.json::<v5::Channel>()
    }

    /// Get emotes by sets.
    pub async fn v5_chat_emoticon_images(&self, emote_sets: &str) -> Result<v5::EmoticonSets> {
        let req = self
            .v5(Method::GET, &["chat", "emoticon_images"])
            .query_param("emotesets", emote_sets);
        req.execute().await?.json::<v5::EmoticonSets>()
    }

    /// Get all badge URLs for the given chat.
    pub async fn v5_chat_badges(&self, channel_id: &str) -> Result<Option<v5::ChatBadges>> {
        let req = self.v5(Method::GET, &["chat", &channel_id, "badges"]);

        Ok(req
            .execute()
            .await?
            .not_found()
            .json()
            .context("request chat badges")?)
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
    fn gql(&self) -> Result<RequestBuilder> {
        let req = RequestBuilder::new(self.client.clone(), Method::POST, self.gql_url.clone())
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .header(
                str::parse::<header::HeaderName>("Client-ID")?,
                GQL_CLIENT_ID,
            );

        Ok(req)
    }
}
