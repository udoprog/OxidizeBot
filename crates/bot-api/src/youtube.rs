//! Twitch API helpers.

use std::collections::HashMap;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use common::models::youtube::{Video, Videos, SearchResults};
use reqwest::{Client, Method, Url};
use serde::{Deserialize, Serialize};

use crate::base::RequestBuilder;
use crate::token::Token;

const V3_URL: &str = "https://www.googleapis.com/youtube/v3";

/// API integration.
#[derive(Clone, Debug)]
pub struct YouTube {
    user_agent: &'static str,
    token: Token,
    client: Client,
    v3_url: Url,
}

impl YouTube {
    /// Create a new API integration.
    pub fn new(user_agent: &'static str, token: Token) -> Result<Self> {
        Ok(Self {
            user_agent,
            token,
            client: Client::new(),
            v3_url: str::parse::<Url>(V3_URL)?,
        })
    }

    /// Access the underlying bearer token for this client.
    pub fn token(&self) -> &Token {
        &self.token
    }

    /// Build request against v3 URL.
    fn v3<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.v3_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, self.user_agent, method, url);
        req.token(&self.token);
        req
    }

    /// Update the channel information.
    pub async fn videos_by_id(&self, video_id: &str, part: &str) -> Result<Option<Video>> {
        let mut req = self.v3(Method::GET, &["videos"]);

        req.query_param("part", part).query_param("id", video_id);

        Ok(req
            .execute()
            .await?
            .not_found()
            .json::<Videos>()?
            .and_then(|v| v.items.into_iter().next()))
    }

    /// Search YouTube.
    pub async fn search(&self, q: &str) -> Result<SearchResults> {
        let mut req = self.v3(Method::GET, &["search"]);

        req.query_param("part", "snippet").query_param("q", q);

        match req.execute().await?.not_found().json::<SearchResults>()? {
            Some(result) => Ok(result),
            None => bail!("got empty response"),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[non_exhaustive]
struct Empty;
