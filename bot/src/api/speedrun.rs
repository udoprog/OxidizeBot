//! Twitch API helpers.

use crate::{api::RequestBuilder, utils::PtDuration};
use chrono::{DateTime, NaiveDate, Utc};
use failure::Error;
use hashbrown::HashMap;
use reqwest::{header, r#async::Client, Method, Url};
use std::collections::BTreeMap;

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

        let req = RequestBuilder::new(self.client.clone(), method, url);
        req.header(header::ACCEPT, "application/json")
    }

    /// Fetch the user by id.
    pub async fn user_by_id(&self, user: String) -> Result<Option<User>, Error> {
        let data = self
            .v1(Method::GET, &["users", user.as_str()])
            .json_or::<Data<User>>()
            .await?;
        Ok(data.map(|d| d.data))
    }

    /// Fetch the user by id.
    pub async fn user_personal_bests(
        &self,
        user_id: String,
        embeds: Embeds,
    ) -> Result<Option<Vec<Run>>, Error> {
        let mut request = self.v1(Method::GET, &["users", user_id.as_str(), "personal-bests"]);

        if let Some(q) = embeds.to_query() {
            request = request.query_param("embed", q.as_str());
        }

        let data = request.json_or::<Data<Vec<Run>>>().await?;
        Ok(data.map(|d| d.data))
    }

    /// Get a game by id.
    pub async fn game_by_id(&self, game: String) -> Result<Option<Game>, Error> {
        let data = self
            .v1(Method::GET, &["games", game.as_str()])
            .json_or::<Data<Game>>()
            .await?;
        Ok(data.map(|d| d.data))
    }

    /// Get game categories by game id.
    pub async fn game_categories_by_id(
        &self,
        game: String,
        embeds: Embeds,
    ) -> Result<Option<Vec<Category>>, Error> {
        let mut request = self.v1(Method::GET, &["games", game.as_str(), "categories"]);

        if let Some(q) = embeds.to_query() {
            request = request.query_param("embed", q.as_str());
        }

        let data = request.json_or::<Data<Vec<Category>>>().await?;
        Ok(data.map(|d| d.data))
    }

    /// Get game levels.
    pub async fn game_levels(&self, game: String) -> Result<Option<Vec<Level>>, Error> {
        let request = self.v1(Method::GET, &["games", game.as_str(), "levels"]);
        let data = request.json_or::<Data<Vec<Level>>>().await?;
        Ok(data.map(|d| d.data))
    }

    /// Get all variables associated with a category.
    pub async fn category_variables(
        &self,
        category: String,
    ) -> Result<Option<Vec<Variable>>, Error> {
        let data = self
            .v1(Method::GET, &["categories", category.as_str(), "variables"])
            .json_or::<Data<Vec<Variable>>>()
            .await?;

        Ok(data.map(|d| d.data))
    }

    /// Get all records associated with a category.
    pub async fn category_records_by_id(
        &self,
        category: String,
        top: u32,
    ) -> Result<Option<Page<GameRecord>>, Error> {
        let data = self
            .v1(Method::GET, &["categories", category.as_str(), "records"])
            .query_param("top", top.to_string().as_str())
            .json_or::<Page<GameRecord>>()
            .await?;
        Ok(data)
    }

    /// Get all records associated with a category.
    pub async fn leaderboard(
        &self,
        game: String,
        category: String,
        top: u32,
        variables: Variables,
        embeds: Embeds,
    ) -> Result<Option<GameRecord>, Error> {
        let mut request = self
            .v1(
                Method::GET,
                &["leaderboards", game.as_str(), "category", category.as_str()],
            )
            .query_param("top", top.to_string().as_str());

        if let Some(q) = embeds.to_query() {
            request = request.query_param("embed", q.as_str());
        }

        for (key, value) in variables.variables {
            request = request.query_param(&format!("var-{}", key), &value);
        }

        let data = request.json_or::<Data<GameRecord>>().await?;
        Ok(data.map(|d| d.data))
    }
}

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

#[derive(Debug, Clone, Default)]
pub struct Variables {
    pub variables: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Embed {
    Players,
    Variables,
    Game,
    Category,
}

impl Embed {
    /// Get the id of this embed.
    pub fn id(&self) -> &'static str {
        use self::Embed::*;

        match *self {
            Players => "players",
            Variables => "variables",
            Game => "game",
            Category => "category",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Embeds {
    embeds: Vec<Embed>,
}

impl Embeds {
    /// Convert into a query.
    pub fn to_query(&self) -> Option<String> {
        let mut it = self.embeds.iter().peekable();

        if !it.peek().is_some() {
            return None;
        }

        let mut s = String::new();

        while let Some(e) = it.next() {
            s.push_str(e.id());

            if it.peek().is_some() {
                s.push(',');
            }
        }

        Some(s)
    }

    /// Add the given embed parameter.
    pub fn push(&mut self, embed: Embed) {
        self.embeds.push(embed);
    }
}

impl Variables {
    /// Generate a unique cache key for this collection of variables.
    pub fn cache_key(&self) -> String {
        self.variables
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join("/")
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

impl User {
    /// Check if the given user matches the given string.
    pub fn matches(&self, s: &str) -> bool {
        if self.names.matches(s) {
            return true;
        }

        if self.twitch_matches(s) {
            return true;
        }

        false
    }

    /// Test if Twitch matches.
    pub fn twitch_matches(&self, s: &str) -> bool {
        let twitch = match self.twitch.as_ref() {
            Some(twitch) => twitch,
            None => return false,
        };

        let url = match url::Url::parse(&twitch.uri) {
            Ok(url) => url,
            Err(_) => return false,
        };

        let mut segments = match url.path_segments() {
            Some(segments) => segments,
            None => return false,
        };

        let part = match segments.next() {
            Some(part) => part,
            None => return false,
        };

        part.contains(s)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Guest {
    pub name: String,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "rel")]
pub enum Players {
    #[serde(rename = "user")]
    User(User),
    #[serde(rename = "guest")]
    Guest(Guest),
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
#[serde(tag = "rel")]
pub enum RelatedPlayer {
    #[serde(rename = "user")]
    Player(RelatedUser),
    #[serde(rename = "guest")]
    Guest(RelatedGuest),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RelatedUser {
    pub id: String,
    pub uri: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RelatedGuest {
    pub name: String,
    pub uri: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Times {
    pub primary: PtDuration,
    pub primary_t: serde_json::Number,
    pub realtime: Option<PtDuration>,
    pub realtime_t: serde_json::Number,
    pub realtime_noloads: Option<PtDuration>,
    pub realtime_noloads_t: serde_json::Number,
    pub ingame: Option<PtDuration>,
    pub ingame_t: serde_json::Number,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct System {
    #[serde(default)]
    pub platform: Option<String>,
    pub emulated: bool,
    #[serde(default)]
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
    pub level: Option<String>,
    pub category: String,
    #[serde(default)]
    pub videos: Option<Videos>,
    #[serde(default)]
    pub comment: Option<String>,
    pub status: Status,
    #[serde(default)]
    pub players: Vec<RelatedPlayer>,
    #[serde(default)]
    pub date: Option<NaiveDate>,
    #[serde(default)]
    pub submitted: Option<DateTime<Utc>>,
    pub times: Times,
    pub system: System,
    pub splits: Option<Splits>,
    #[serde(default)]
    pub values: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Run {
    pub place: u32,
    pub run: RunInfo,
    /// Annotated information on players, if embed=game was requested.
    #[serde(default)]
    pub game: Option<Data<Game>>,
    /// Annotated information on players, if embed=category was requested.
    #[serde(default)]
    pub category: Option<Data<Category>>,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct VariableFlags {
    #[serde(default)]
    pub miscellaneous: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct VariableValue {
    pub label: String,
    pub rule: Option<String>,
    #[serde(default)]
    pub flags: VariableFlags,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct VariableValues {
    #[serde(rename = "_note")]
    pub note: Option<String>,
    #[serde(default)]
    pub choices: HashMap<String, String>,
    #[serde(default)]
    pub values: HashMap<String, VariableValue>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Variable {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: Option<String>,
    pub scope: Scope,
    pub mandatory: bool,
    pub user_defined: bool,
    pub obsoletes: bool,
    pub values: VariableValues,
    pub is_subcategory: bool,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct GameRecord {
    pub weblink: String,
    pub game: String,
    pub category: String,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub emulators: serde_json::Value,
    pub video_only: bool,
    #[serde(default)]
    pub timing: serde_json::Value,
    #[serde(default)]
    pub values: serde_json::Value,
    #[serde(default)]
    pub runs: Vec<Run>,
    #[serde(default)]
    pub links: Vec<Link>,
    /// Annotated information on players, if embed=players was requested.
    #[serde(default)]
    pub players: Option<Data<Vec<Players>>>,
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
    pub map: HashMap<String, Role>,
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

impl Game {
    /// Test if game matches the given identifying string.
    pub fn matches(&self, s: &str) -> bool {
        if self.id == s {
            return true;
        }

        if self.abbreviation == s {
            return true;
        }

        if self.names.matches(s) {
            return true;
        }

        false
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CategoryPlayers {
    #[serde(rename = "exactly")]
    Exactly { value: u32 },
    #[serde(rename = "up-to")]
    UpTo { value: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CategoryType {
    PerGame,
    PerLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Scope {
    #[serde(rename_all = "kebab-case")]
    FullGame {},
    #[serde(rename_all = "kebab-case")]
    AllLevels {},
    #[serde(rename_all = "kebab-case")]
    Global {},
    #[serde(rename_all = "kebab-case")]
    SingleLevel { level: String },
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub weblink: String,
    #[serde(rename = "type")]
    pub ty: CategoryType,
    #[serde(default)]
    pub rules: Option<String>,
    pub players: CategoryPlayers,
    pub miscellaneous: bool,
    #[serde(default)]
    pub links: Vec<Link>,
    /// This is included in case we have the `variables` embed.
    #[serde(default)]
    pub variables: Option<Data<Vec<Variable>>>,
    /// Annotated information on players, if embed=game was requested.
    #[serde(default)]
    pub game: Option<Data<Game>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Level {
    pub id: String,
    pub name: String,
    pub weblink: String,
    pub rules: Option<String>,
    pub links: Vec<Link>,
}

impl Level {
    /// Test if level matches the given identifying string.
    pub fn matches(&self, s: &str) -> bool {
        if self.id == s {
            return true;
        }

        if self.name.to_lowercase().contains(s) {
            return true;
        }

        false
    }
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
