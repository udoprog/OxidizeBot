//! BetterTTV API Client.

use crate::api::RequestBuilder;
use anyhow::Result;
use reqwest::{header, Client, Method, Url};
use std::collections::HashSet;

const V2_URL: &str = "https://api.betterttv.net/2";

/// API integration.
#[derive(Clone, Debug)]
pub(crate) struct BetterTTV {
    client: Client,
    v2_url: Url,
}

impl BetterTTV {
    /// Create a new API integration.
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            v2_url: str::parse::<Url>(V2_URL)?,
        })
    }

    /// Build request against v2 URL.
    fn v2<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.v2_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, method, url);
        req.header(header::ACCEPT, "application/json");
        req
    }

    /// Get the set associated with the room.
    pub(crate) async fn channels(&self, channel: &str) -> Result<Option<Channel>> {
        let req = self.v2(Method::GET, &["channels", channel]);
        let data = req.execute().await?.not_found().json()?;
        Ok(data)
    }

    pub(crate) async fn emotes(&self) -> Result<Emotes> {
        let req = self.v2(Method::GET, &["emotes"]);
        let data = req.execute().await?.json()?;
        Ok(data)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Channel {
    pub(crate) url_template: String,
    pub(crate) bots: HashSet<String>,
    pub(crate) emotes: Vec<Emote>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Emotes {
    #[serde(default)]
    pub(crate) status: Option<u32>,
    pub(crate) url_template: String,
    pub(crate) emotes: Vec<Emote>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Emote {
    pub(crate) id: String,
    pub(crate) channel: Option<String>,
    pub(crate) code: String,
    pub(crate) image_type: String,
}
