//! Twitch API helpers.

use crate::api::RequestBuilder;
use crate::oauth2;
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, Method, Url};
use std::collections::HashMap;

const V3_URL: &str = "https://www.googleapis.com/youtube/v3";

/// API integration.
#[derive(Clone, Debug)]
pub(crate) struct YouTube {
    client: Client,
    v3_url: Url,
    pub(crate) token: oauth2::SyncToken,
}

impl YouTube {
    /// Create a new API integration.
    pub(crate) fn new(token: oauth2::SyncToken) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            v3_url: str::parse::<Url>(V3_URL)?,
            token,
        })
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

        let mut req = RequestBuilder::new(&self.client, method, url);
        req.token(&self.token);
        req
    }

    /// Update the channel information.
    pub(crate) async fn videos_by_id(&self, video_id: &str, part: &str) -> Result<Option<Video>> {
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
    pub(crate) async fn search(&self, q: &str) -> Result<SearchResults> {
        let mut req = self.v3(Method::GET, &["search"]);

        req.query_param("part", "snippet").query_param("q", q);

        match req.execute().await?.not_found().json::<SearchResults>()? {
            Some(result) => Ok(result),
            None => bail!("got empty response"),
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub(crate) struct Empty {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageInfo {
    pub(crate) total_results: u32,
    pub(crate) results_per_page: u32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Videos {
    pub(crate) kind: String,
    pub(crate) etag: String,
    pub(crate) page_info: PageInfo,
    #[serde(default)]
    pub(crate) items: Vec<Video>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) enum Kind {
    #[serde(rename = "youtube#channel")]
    Channel,
    #[serde(rename = "youtube#video")]
    Video,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Id {
    pub(crate) kind: Kind,
    pub(crate) video_id: Option<String>,
    pub(crate) channel_id: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    pub(crate) kind: String,
    pub(crate) etag: String,
    pub(crate) id: Id,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResults {
    pub(crate) kind: String,
    pub(crate) etag: String,
    pub(crate) next_page_token: Option<String>,
    pub(crate) region_code: Option<String>,
    pub(crate) page_info: PageInfo,
    #[serde(default)]
    pub(crate) items: Vec<SearchResult>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Thumbnail {
    pub(crate) url: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snippet {
    #[serde(default)]
    pub(crate) published_at: Option<DateTime<Utc>>,
    pub(crate) channel_id: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) thumbnails: HashMap<String, Thumbnail>,
    #[serde(default)]
    pub(crate) channel_title: Option<String>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) category_id: Option<String>,
    #[serde(default)]
    pub(crate) live_broadcast_content: Option<String>,
    #[serde(default)]
    pub(crate) default_language: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContentDetails {
    #[serde(default)]
    pub(crate) published_at: Option<DateTime<Utc>>,
    pub(crate) duration: String,
    #[serde(default)]
    pub(crate) dimension: Option<String>,
    #[serde(default)]
    pub(crate) definition: Option<String>,
    pub(crate) caption: Option<String>,
    #[serde(default)]
    pub(crate) licensed_content: bool,
    #[serde(default)]
    pub(crate) projection: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Video {
    pub(crate) kind: String,
    pub(crate) etag: String,
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) snippet: Option<Snippet>,
    #[serde(default)]
    pub(crate) content_details: Option<ContentDetails>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct RawVideoInfo {
    pub(crate) author: Option<String>,
    pub(crate) video_id: String,
    pub(crate) status: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) thumbnail_url: Option<String>,
    pub(crate) url_encoded_fmt_stream_map: String,
    #[serde(default)]
    pub(crate) view_count: Option<usize>,
    #[serde(default)]
    pub(crate) adaptive_fmts: Option<String>,
    #[serde(default)]
    pub(crate) hlsvp: Option<String>,
    #[serde(default)]
    pub(crate) player_response: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioConfig {
    pub(crate) loudness_db: f32,
    pub(crate) perceptual_loudness_db: f32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlayerConfig {
    #[serde(default)]
    pub(crate) audio_config: Option<AudioConfig>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlayerResponse {
    #[serde(default)]
    pub(crate) player_config: Option<PlayerConfig>,
}
