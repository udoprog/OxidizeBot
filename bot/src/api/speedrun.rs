//! Twitch API helpers.

use crate::{prelude::*, utils::PtDuration};
use bytes::Bytes;
use chrono::{DateTime, NaiveDate, Utc};
use failure::Error;
use hashbrown::HashMap;
use reqwest::{
    header,
    r#async::{Chunk, Client, Decoder},
    Method, StatusCode, Url,
};
use std::mem;

const V1_URL: &'static str = "https://speedrun.com/api/v1";

/// API integration.
#[derive(Clone, Debug)]
pub struct Speedrun {
    client: Client,
    v1_url: Url,
}

impl Speedrun {
    /// Create a new API integration.
    pub fn new() -> Result<Speedrun, Error> {
        Ok(Speedrun {
            client: Client::new(),
            v1_url: str::parse::<Url>(V1_URL)?,
        })
    }

    /// Build request against v3 URL.
    fn v1(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.v1_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.extend(path);
        }

        RequestBuilder {
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
        }
    }

    pub async fn user_by_id(&self, user: String) -> Result<Option<User>, Error> {
        let data = self
            .v1(Method::GET, &["users", user.as_str()])
            .json::<Data<User>>()
            .await?;
        Ok(data.map(|d| d.data))
    }

    /// Get a game by id.
    pub async fn game_by_id(&self, game: String) -> Result<Option<Game>, Error> {
        let data = self
            .v1(Method::GET, &["games", game.as_str()])
            .json::<Data<Game>>()
            .await?;
        Ok(data.map(|d| d.data))
    }

    /// Get game categories by game id.
    pub async fn game_categories_by_id(
        &self,
        game: String,
    ) -> Result<Option<Vec<Category>>, Error> {
        let data = self
            .v1(Method::GET, &["games", game.as_str(), "categories"])
            .json::<Data<Vec<Category>>>()
            .await?;
        Ok(data.map(|d| d.data))
    }

    pub async fn category_records_by_id(
        &self,
        category: String,
        top: u32,
    ) -> Result<Option<Page<GameRecord>>, Error> {
        let data = self
            .v1(Method::GET, &["categories", category.as_str(), "records"])
            .query_param("top", top.to_string().as_str())
            .json::<Page<GameRecord>>()
            .await?;
        Ok(data)
    }
}

struct RequestBuilder {
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Bytes>,
}

impl RequestBuilder {
    /// Execute the request, providing the raw body as a response.
    pub async fn raw(self) -> Result<Option<Chunk>, Error> {
        let mut req = self.client.request(self.method, self.url);

        if let Some(body) = self.body {
            req = req.body(body);
        }

        for (key, value) in self.headers {
            req = req.header(key, value);
        }

        req = req.header(header::ACCEPT, "application/json");
        let mut res = req.send().compat().await?;

        let status = res.status();

        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let body = mem::replace(res.body_mut(), Decoder::empty());
        let body = body.compat().try_concat().await?;

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
    }

    /// Execute the request expecting a JSON response.
    pub async fn json<T>(self) -> Result<Option<T>, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let body = self.raw().await?;

        let body = match body {
            Some(body) => body,
            None => return Ok(None),
        };

        serde_json::from_slice(body.as_ref()).map_err(Into::into)
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
pub struct Names {
    international: String,
    #[serde(default)]
    japanese: Option<String>,
    #[serde(default)]
    twitch: Option<String>,
}

impl Names {
    /// Get as printable name.
    pub fn name(&self) -> &str {
        match self.japanese.as_ref() {
            Some(name) => name,
            None => &self.international,
        }
    }

    /// Check if the given name matches any of the provided names.
    pub fn matches(&self, pattern: &str) -> bool {
        if self.international.to_lowercase().contains(pattern) {
            return true;
        }

        if let Some(japanese) = self.japanese.as_ref() {
            if japanese.to_lowercase().contains(pattern) {
                return true;
            }
        }

        if let Some(twitch) = self.twitch.as_ref() {
            if twitch.to_lowercase().contains(pattern) {
                return true;
            }
        }

        false
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case", tag = "style")]
pub struct Color {
    light: String,
    dark: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "style")]
pub enum NameStyle {
    #[serde(rename = "gradient", rename_all = "kebab-case")]
    Gradient { color_from: Color, color_to: Color },
    #[serde(rename = "solid", rename_all = "kebab-case")]
    Solid { color: Color },
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Country {
    pub code: String,
    pub names: Names,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Location {
    pub country: Country,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Uri {
    pub uri: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Link {
    pub rel: String,
    pub uri: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Asset {
    pub uri: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct User {
    pub id: String,
    pub names: Names,
    pub weblink: String,
    pub name_style: NameStyle,
    pub role: String,
    pub signup: DateTime<Utc>,
    #[serde(default)]
    pub location: Option<Location>,
    #[serde(default)]
    pub twitch: Option<Uri>,
    #[serde(default)]
    pub hitbox: Option<Uri>,
    #[serde(default)]
    pub youtube: Option<Uri>,
    #[serde(default)]
    pub twitter: Option<Uri>,
    #[serde(default)]
    pub speedrunslive: Option<Uri>,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Videos {
    #[serde(default)]
    pub links: Vec<Uri>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Status {
    pub status: String,
    #[serde(default)]
    pub examiner: Option<String>,
    #[serde(default)]
    pub verify_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Player {
    pub rel: String,
    pub id: String,
    pub uri: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Times {
    pub primary: PtDuration,
    pub primary_t: u64,
    pub realtime: Option<PtDuration>,
    pub realtime_t: u64,
    pub realtime_noloads: Option<PtDuration>,
    pub realtime_noloads_t: u64,
    pub ingame: Option<PtDuration>,
    pub ingame_t: u64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct System {
    pub platform: String,
    pub emulated: bool,
    pub region: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Splits {
    pub rel: String,
    pub uri: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RunInfo {
    pub id: String,
    pub weblink: String,
    pub game: String,
    #[serde(default)]
    pub level: serde_json::Value,
    pub category: String,
    pub videos: Videos,
    #[serde(default)]
    pub comment: Option<String>,
    pub status: Status,
    #[serde(default)]
    pub players: Vec<Player>,
    pub date: NaiveDate,
    #[serde(default)]
    pub submitted: Option<DateTime<Utc>>,
    pub times: Times,
    pub system: System,
    pub splits: Option<Splits>,
    #[serde(default)]
    pub values: serde_json::Value,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Run {
    pub place: u32,
    pub run: RunInfo,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct GameRecord {
    pub weblink: String,
    pub game: String,
    pub category: String,
    #[serde(default)]
    pub level: serde_json::Value,
    #[serde(default)]
    pub platform: serde_json::Value,
    #[serde(default)]
    pub region: serde_json::Value,
    #[serde(default)]
    pub emulators: serde_json::Value,
    pub video_only: bool,
    #[serde(default)]
    pub timing: serde_json::Value,
    #[serde(default)]
    pub values: serde_json::Value,
    #[serde(default)]
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RuleSet {
    pub show_milliseconds: bool,
    pub require_verification: bool,
    pub require_video: bool,
    pub run_times: Vec<String>,
    pub default_time: String,
    pub emulators_allowed: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    SuperModerator,
    Moderator,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Moderators {
    #[serde(flatten)]
    map: HashMap<String, Role>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Game {
    pub id: String,
    pub names: Names,
    pub abbreviation: String,
    pub weblink: String,
    pub released: u32,
    pub release_date: NaiveDate,
    pub ruleset: RuleSet,
    pub romhack: bool,
    pub gametypes: Vec<serde_json::Value>,
    pub platforms: Vec<String>,
    pub regions: Vec<String>,
    pub genres: Vec<String>,
    pub engines: Vec<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub moderators: Moderators,
    pub created: Option<DateTime<Utc>>,
    pub assets: HashMap<String, Option<Asset>>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CategoryPlayers {
    #[serde(rename = "exactly")]
    Exactly { value: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CategoryType {
    PerGame,
    PerLevel,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub weblink: String,
    #[serde(rename = "type")]
    pub ty: CategoryType,
    pub rules: String,
    pub players: CategoryPlayers,
    pub miscellaneous: bool,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Data<T> {
    pub data: T,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Pagination {
    pub offset: u64,
    pub max: u64,
    pub size: u64,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Page<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: Option<Pagination>,
}
