//! Twitch API helpers.

use crate::oauth2;
use chrono::{DateTime, Utc};
use futures::{Future, Stream as _};
use hashbrown::HashMap;
use parking_lot::RwLock;
use reqwest::{
    header,
    r#async::{Body, Chunk, Client, Decoder},
    Method, StatusCode, Url,
};
use std::{mem, sync::Arc};

const V3_URL: &'static str = "https://www.googleapis.com/youtube/v3";
const GET_VIDEO_INFO_URL: &'static str = "https://www.youtube.com/get_video_info";

/// API integration.
#[derive(Clone, Debug)]
pub struct YouTube {
    client: Client,
    v3_url: Url,
    get_video_info_url: Url,
    token: Arc<RwLock<oauth2::Token>>,
}

impl YouTube {
    /// Create a new API integration.
    pub fn new(token: Arc<RwLock<oauth2::Token>>) -> Result<YouTube, failure::Error> {
        Ok(YouTube {
            client: Client::new(),
            v3_url: str::parse::<Url>(V3_URL)?,
            get_video_info_url: str::parse::<Url>(GET_VIDEO_INFO_URL)?,
            token,
        })
    }

    /// Build request against v3 URL.
    fn v3(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.v3_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.extend(path);
        }

        RequestBuilder {
            token: Arc::clone(&self.token),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
        }
    }

    /// Update the channel information.
    pub fn videos_by_id(
        &self,
        video_id: &str,
        part: &str,
    ) -> impl Future<Item = Option<Video>, Error = failure::Error> {
        self.v3(Method::GET, &["videos"])
            .query_param("part", part)
            .query_param("id", video_id)
            .json::<Videos>()
            .map(|videos| videos.and_then(|v| v.items.into_iter().next()))
    }

    /// Get video info of a video.
    pub fn get_video_info(
        &self,
        video_id: &str,
    ) -> impl Future<Item = Option<VideoInfo>, Error = failure::Error> {
        let mut url = self.get_video_info_url.clone();
        url.query_pairs_mut().append_pair("video_id", video_id);

        let request = RequestBuilder {
            token: Arc::clone(&self.token),
            client: self.client.clone(),
            url,
            method: Method::GET,
            headers: Vec::new(),
            body: None,
        };

        request.raw().and_then(|body| {
            let body = match body {
                Some(body) => body,
                None => return Ok(None),
            };

            let result: RawVideoInfo = serde_urlencoded::from_bytes(&body)?;
            let result = result.into_decoded()?;
            Ok(Some(result))
        })
    }
}

struct RequestBuilder {
    token: Arc<RwLock<oauth2::Token>>,
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Body>,
}

impl RequestBuilder {
    /// Execute the request, providing the raw body as a response.
    pub fn raw(self) -> impl Future<Item = Option<Chunk>, Error = failure::Error> {
        let token = self.token.read();
        let access_token = token.access_token().to_string();

        let mut r = self.client.request(self.method, self.url);

        if let Some(body) = self.body {
            r = r.body(body);
        }

        for (key, value) in self.headers {
            r = r.header(key, value);
        }

        r = r.header(header::ACCEPT, "application/json");
        r = r.header(header::AUTHORIZATION, format!("Bearer {}", access_token));

        r.send().map_err(Into::into).and_then(|mut res| {
            let body = mem::replace(res.body_mut(), Decoder::empty());

            body.concat2().map_err(Into::into).and_then(move |body| {
                let status = res.status();

                if status == StatusCode::NOT_FOUND {
                    return Ok(None);
                }

                if !status.is_success() {
                    failure::bail!(
                        "bad response: {}: {}",
                        status,
                        String::from_utf8_lossy(&body)
                    );
                }

                if log::log_enabled!(log::Level::Trace) {
                    let response = String::from_utf8_lossy(body.as_ref());
                    log::trace!("response: {}", response);
                }

                Ok(Some(body))
            })
        })
    }

    /// Execute the request expecting a JSON response.
    pub fn json<T>(self) -> impl Future<Item = Option<T>, Error = failure::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.raw().and_then(|body| {
            let body = match body {
                Some(body) => body,
                None => return Ok(None),
            };

            serde_json::from_slice(body.as_ref()).map_err(Into::into)
        })
    }

    /// Add a query parameter.
    pub fn query_param(mut self, key: &str, value: &str) -> Self {
        self.url.query_pairs_mut().append_pair(key, value);
        self
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Empty {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub total_results: u32,
    pub results_per_page: u32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Videos {
    pub kind: String,
    pub etag: String,
    pub page_info: PageInfo,
    #[serde(default)]
    pub items: Vec<Video>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RawVideoInfo {
    pub author: String,
    pub video_id: String,
    pub status: String,
    pub title: String,
    pub thumbnail_url: String,
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    pub loudness_db: f32,
    pub perceptual_loudness_db: f32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerConfig {
    #[serde(default)]
    pub audio_config: Option<AudioConfig>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResponse {
    #[serde(default)]
    pub player_config: Option<PlayerConfig>,
}

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub player_response: Option<PlayerResponse>,
}

impl RawVideoInfo {
    /// Convert into a decoded version.
    pub fn into_decoded(self) -> Result<VideoInfo, failure::Error> {
        let player_response = match self.player_response.as_ref() {
            Some(player_response) => Some(serde_json::from_str(player_response)?),
            None => None,
        };

        Ok(VideoInfo { player_response })
    }
}
