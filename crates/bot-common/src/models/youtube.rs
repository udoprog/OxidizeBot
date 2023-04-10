use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub total_results: u32,
    pub results_per_page: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Videos {
    pub kind: String,
    pub etag: String,
    pub page_info: PageInfo,
    #[serde(default)]
    pub items: Vec<Video>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Kind {
    #[serde(rename = "youtube#channel")]
    Channel,
    #[serde(rename = "youtube#video")]
    Video,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Id {
    pub kind: Kind,
    pub video_id: Option<String>,
    pub channel_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub kind: String,
    pub etag: String,
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub kind: String,
    pub etag: String,
    pub next_page_token: Option<String>,
    pub region_code: Option<String>,
    pub page_info: PageInfo,
    #[serde(default)]
    pub items: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    #[serde(default)]
    pub published_at: Option<DateTime<Utc>>,
    pub channel_id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub thumbnails: HashMap<String, Thumbnail>,
    #[serde(default)]
    pub channel_title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category_id: Option<String>,
    #[serde(default)]
    pub live_broadcast_content: Option<String>,
    #[serde(default)]
    pub default_language: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDetails {
    #[serde(default)]
    pub published_at: Option<DateTime<Utc>>,
    pub duration: String,
    #[serde(default)]
    pub dimension: Option<String>,
    #[serde(default)]
    pub definition: Option<String>,
    pub caption: Option<String>,
    #[serde(default)]
    pub licensed_content: bool,
    #[serde(default)]
    pub projection: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub kind: String,
    pub etag: String,
    pub id: String,
    #[serde(default)]
    pub snippet: Option<Snippet>,
    #[serde(default)]
    pub content_details: Option<ContentDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawVideoInfo {
    pub author: Option<String>,
    pub video_id: String,
    pub status: String,
    pub title: String,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    pub url_encoded_fmt_stream_map: String,
    #[serde(default)]
    pub view_count: Option<usize>,
    #[serde(default)]
    pub adaptive_fmts: Option<String>,
    #[serde(default)]
    pub hlsvp: Option<String>,
    #[serde(default)]
    pub player_response: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    pub loudness_db: f32,
    pub perceptual_loudness_db: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerConfig {
    #[serde(default)]
    pub audio_config: Option<AudioConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResponse {
    #[serde(default)]
    pub player_config: Option<PlayerConfig>,
}
