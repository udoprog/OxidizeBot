//! Twitch API helpers.

mod gql;

pub mod model;

pub mod pubsub;

use common::stream::Stream;
use anyhow::Result;
use common::stream;
use reqwest::{header, Client, Method, Url};
use serde::{de, Serialize};
use thiserror::Error;

use crate::base::RequestBuilder;
use crate::token::Token;

pub const CLIPS_URL: &str = "http://clips.twitch.tv";
const API_TWITCH_URL: &str = "https://api.twitch.tv";
const GQL_URL: &str = "https://gql.twitch.tv/gql";

const GQL_CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
/// Common header.
const BROADCASTER_ID: &str = "broadcaster_id";


#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("missing user")]
    MissingUser,
}

pub(crate) use self::model::*;

/// Twitch API client.
#[derive(Clone, Debug)]
pub struct Twitch {
    user_agent: &'static str,
    token: Token,
    client: Client,
    client_id_header: header::HeaderName,
    api_url: Url,
    gql_url: Url,
}

impl Twitch {
    /// Create a new Twitch API client.
    pub fn new(user_agent: &'static str, token: Token) -> Result<Self> {
        Ok(Self {
            user_agent,
            token,
            client: Client::new(),
            client_id_header: str::parse::<header::HeaderName>("Client-ID")?,
            api_url: str::parse::<Url>(API_TWITCH_URL)?,
            gql_url: str::parse::<Url>(GQL_URL)?,
        })
    }

    /// Access bearer token for the current Twitch client.
    pub fn token(&self) -> &Token {
        &self.token
    }

    /// Get display badges through GQL.
    pub async fn gql_display_badges(
        &self,
        login: &str,
        channel: &str,
    ) -> Result<Option<gql::badges::ResponseData>> {
        use graphql_client::{GraphQLQuery as _, Response};

        let body = gql::Badges::build_query(gql::badges::Variables {
            login: login.to_string(),
            channel_login: channel.to_string(),
        });

        let res = self
            .gql()
            .body(serde_json::to_vec(&body)?)
            .execute()
            .await?
            .json::<Response<gql::badges::ResponseData>>()?
            .data;

        Ok(res)
    }

    /// Get chatters for the given broadcaster.
    pub fn chatters(
        &self,
        moderator_id: &str,
        broadcaster_id: &str,
    ) -> impl Stream<Item = Result<Chatter>> + '_ {
        let mut req = self.new_api(Method::GET, &["chat", "chatters"]);

        req.query_param("moderator_id", moderator_id)
            .query_param("broadcaster_id", broadcaster_id);

        page(req)
    }

    /// Get moderators for the current broadcaster.
    pub fn moderators(
        &self,
        broadcaster_id: &str,
    ) -> impl Stream<Item = Result<Chatter>> + '_ {
        let mut req = self.new_api(Method::GET, &["moderation", "moderators"]);
        req.query_param("broadcaster_id", broadcaster_id);
        page(req)
    }

    /// Get VIPs for the current broadcaster.
    pub fn vips(&self, broadcaster_id: &str) -> impl Stream<Item = Result<Chatter>> + '_ {
        let mut req = self.new_api(Method::GET, &["channels", "vips"]);
        req.query_param("broadcaster_id", broadcaster_id);
        page(req)
    }

    /// Search for a category with the given name.
    pub fn categories<'a>(
        &'a self,
        query: &str,
    ) -> impl Stream<Item = Result<model::Category>> + 'a {
        let mut req = self.new_api(Method::GET, &["search", "categories"]);
        req.query_param("query", query);
        page(req)
    }

    /// Get information on a user.
    pub fn subscriptions<'a>(
        &'a self,
        broadcaster_id: &str,
        user_ids: Vec<String>,
    ) -> impl Stream<Item = Result<model::Subscription>> + 'a {
        let mut req = self.new_api(Method::GET, &["subscriptions"]);
        req.query_param(BROADCASTER_ID, broadcaster_id);

        for user_id in &user_ids {
            req.query_param("user_id", user_id);
        }

        page(req)
    }

    /// Create a clip for the given broadcaster.
    pub async fn create_clip(&self, broadcaster_id: &str) -> Result<Option<model::Clip>> {
        let res = self
            .new_api(Method::POST, &["clips"])
            .query_param(BROADCASTER_ID, broadcaster_id)
            .execute()
            .await?
            .json::<Data<Vec<model::Clip>>>()?;

        Ok(res.data.into_iter().next())
    }

    /// Get stream information.
    pub async fn streams(
        &self,
        user_id: &str,
    ) -> impl Stream<Item = Result<model::Stream>> + '_ {
        let mut req = self.new_api(Method::GET, &["streams"]);
        req.query_param("user_id", user_id);
        page(req)
    }

    /// Update the status of a redemption.
    pub async fn patch_redemptions(
        &self,
        broadcaster_id: &str,
        redemption: &pubsub::Redemption,
        status: pubsub::Status,
    ) -> Result<()> {
        let mut req = self.new_api(
            Method::PATCH,
            &["channel_points", "custom_rewards", "redemptions"],
        );

        req.header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .query_param(BROADCASTER_ID, broadcaster_id)
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

    /// Get the channel associated with the current authentication.
    pub async fn user(&self) -> Result<model::User> {
        let req = self.new_api(Method::GET, &["users"]);
        let data = req.execute().await?.json::<Data<Vec<model::User>>>()?;
        let user = data.data.into_iter().next().ok_or(Error::MissingUser)?;
        Ok(user)
    }

    /// Get the channel associated with the specified broadcaster id.
    pub async fn channels(&self, broadcaster_id: &str) -> Result<Option<model::Channel>> {
        let mut req = self.new_api(Method::GET, &["channels"]);
        req.query_param(BROADCASTER_ID, broadcaster_id);
        let result = req.execute().await?.json::<Data<Vec<model::Channel>>>()?;
        Ok(result.data.into_iter().next())
    }

    /// Get emotes by sets.
    pub async fn emote_set(&self, id: &str) -> Result<Vec<model::Emote>> {
        let mut req = self.new_api(Method::GET, &["chat", "emotes", "set"]);
        req.query_param("emote_set_id", id);
        Ok(req.execute().await?.json::<Data<Vec<model::Emote>>>()?.data)
    }

    /// Update the channel information.
    pub(crate) async fn patch_channel(
        &self,
        broadcaster_id: &str,
        request: model::ModifyChannelRequest<'_>,
    ) -> Result<()> {
        let body = serde_json::to_vec(&request)?;

        self.new_api(Method::PATCH, &["channels"])
            .query_param(BROADCASTER_ID, broadcaster_id)
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .execute()
            .await?
            .ok()
    }

    /// Get request against API.
    fn new_api<'a>(&'a self, method: Method, path: &[&str]) -> RequestBuilder<'a> {
        let mut url = self.api_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.push("helix");
            url_path.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, self.user_agent, method, url);
        req.token(&self.token)
            .client_id_header(&self.client_id_header);
        req
    }

    /// Access GQL client.
    fn gql(&self) -> RequestBuilder<'_> {
        let mut req = RequestBuilder::new(&self.client, self.user_agent, Method::POST, self.gql_url.clone());

        req.header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .header(self.client_id_header.clone(), GQL_CLIENT_ID);

        req
    }
}

/// Perform pagination over the given request.
fn page<'a, T: 'a>(request: RequestBuilder<'a>) -> impl Stream<Item = Result<T>> + 'a
where
    T: de::DeserializeOwned,
{
    async_stream::try_stream! {
        let initial = request.execute().await?.json::<model::Page<T>>()?;
        let mut page = initial.data.into_iter();
        let mut pagination = initial.pagination;

        loop {
            for item in page.by_ref() {
                yield item;
            }

            let cursor = match pagination.as_ref().and_then(|p| p.cursor.as_ref()) {
                Some(cursor) => cursor,
                None => break,
            };

            let mut next = request.clone();
            next.query_param("after", cursor);
            let next = next.execute().await?.json::<model::Page<T>>()?;
            page = next.data.into_iter();
            pagination = next.pagination;
        }
    }
}
