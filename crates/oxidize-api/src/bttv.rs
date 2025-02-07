//! BetterTTV API Client.

use std::collections::HashSet;

use anyhow::Result;
use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};

use crate::base::RequestBuilder;

const V2_URL: &str = "https://api.betterttv.net/2";

/// API integration.
#[derive(Clone, Debug)]
pub struct BetterTTV {
    user_agent: &'static str,
    client: Client,
    v2_url: Url,
}

impl BetterTTV {
    /// Create a new API integration.
    pub fn new(user_agent: &'static str) -> Result<Self> {
        Ok(Self {
            user_agent,
            client: Client::new(),
            v2_url: str::parse::<Url>(V2_URL)?,
        })
    }

    /// Get the set associated with the room.
    pub async fn channels(&self, channel: &str) -> Result<Option<Channel>> {
        let req = self.v2(Method::GET, &["channels", channel]);
        let data = req.execute().await?.not_found().json()?;
        Ok(data)
    }

    pub async fn emotes(&self) -> Result<Emotes> {
        let req = self.v2(Method::GET, &["emotes"]);
        let data = req.execute().await?.json()?;
        Ok(data)
    }

    /// Build request against v2 URL.
    fn v2<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator<Item: AsRef<str>>,
    {
        let mut url = self.v2_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, self.user_agent, method, url);
        req.header(header::ACCEPT, "application/json");
        req
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub url_template: String,
    pub bots: HashSet<String>,
    pub emotes: Vec<Emote>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Emotes {
    #[serde(default)]
    pub status: Option<u32>,
    pub url_template: String,
    pub emotes: Vec<Emote>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Emote {
    pub id: String,
    pub channel: Option<String>,
    pub code: String,
    pub image_type: String,
}
