//! speedrun.com API client.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use common::PtDuration;
use reqwest::{header, Client, Method, StatusCode, Url};
use serde::{de, ser, Deserialize, Serialize};

use crate::base::RequestBuilder;

const V1_URL: &str = "https://speedrun.com/api/v1";

/// API integration.
#[derive(Clone, Debug)]
pub struct Speedrun {
    user_agent: &'static str,
    client: Client,
    v1_url: Url,
}

impl Speedrun {
    /// Create a new API integration.
    pub fn new(user_agent: &'static str) -> Result<Speedrun> {
        Ok(Speedrun {
            user_agent,
            client: Client::new(),
            v1_url: str::parse::<Url>(V1_URL)?,
        })
    }

    /// Build request against v3 URL.
    fn v1<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.v1_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, self.user_agent, method, url);
        req.header(header::ACCEPT, "application/json");
        req
    }

    /// Fetch the user by id.
    pub(crate) async fn user_by_id(&self, user: &str) -> Result<Option<User>> {
        let req = self.v1(Method::GET, &["users", user]);
        let data: Option<Data<User>> = req
            .execute()
            .await?
            .empty_on_status(StatusCode::NO_CONTENT)
            .json()?;
        Ok(data.map(|d| d.data))
    }

    /// Fetch the user by id.
    pub(crate) async fn user_personal_bests(
        &self,
        user_id: &str,
        embeds: &Embeds,
    ) -> Result<Option<Vec<Run>>> {
        let mut request = self.v1(Method::GET, &["users", user_id, "personal-bests"]);

        if let Some(q) = embeds.to_query() {
            request.query_param("embed", q.as_str());
        }

        let data: Option<Data<Vec<Run>>> = request
            .execute()
            .await?
            .empty_on_status(StatusCode::NO_CONTENT)
            .json()?;
        Ok(data.map(|d| d.data))
    }

    /// Get a game by id.
    pub(crate) async fn game_by_id(&self, game: &str) -> Result<Option<Game>> {
        let req = self.v1(Method::GET, &["games", game]);
        let data: Option<Data<Game>> = req
            .execute()
            .await?
            .empty_on_status(StatusCode::NO_CONTENT)
            .json()?;
        Ok(data.map(|d| d.data))
    }

    /// Get game categories by game id.
    pub(crate) async fn game_categories_by_id(
        &self,
        game_id: &str,
        embeds: &Embeds,
    ) -> Result<Option<Vec<Category>>> {
        let mut request = self.v1(Method::GET, &["games", game_id, "categories"]);

        if let Some(q) = embeds.to_query() {
            request.query_param("embed", q.as_str());
        }

        let data: Option<Data<Vec<Category>>> = request
            .execute()
            .await?
            .empty_on_status(StatusCode::NO_CONTENT)
            .json()?;
        Ok(data.map(|d| d.data))
    }

    /// Get game levels.
    pub(crate) async fn game_levels(&self, game_id: &str) -> Result<Option<Vec<Level>>> {
        let request = self.v1(Method::GET, &["games", game_id, "levels"]);
        let data: Option<Data<Vec<Level>>> = request
            .execute()
            .await?
            .empty_on_status(StatusCode::NO_CONTENT)
            .json()?;
        Ok(data.map(|d| d.data))
    }

    /// Get all variables associated with a category.
    pub(crate) async fn category_variables(&self, category: &str) -> Result<Option<Vec<Variable>>> {
        let req = self.v1(Method::GET, &["categories", category, "variables"]);
        let data: Option<Data<Vec<Variable>>> = req
            .execute()
            .await?
            .empty_on_status(StatusCode::NO_CONTENT)
            .json()?;
        Ok(data.map(|d| d.data))
    }

    /// Get all records associated with a category.
    pub(crate) async fn leaderboard(
        &self,
        game_id: &str,
        category_id: &str,
        top: u32,
        variables: &Variables,
        embeds: &Embeds,
    ) -> Result<Option<GameRecord>> {
        let mut request = self.v1(
            Method::GET,
            &["leaderboards", game_id, "category", category_id],
        );

        request.query_param("top", top.to_string().as_str());

        if let Some(q) = embeds.to_query() {
            request.query_param("embed", q.as_str());
        }

        for (key, value) in &variables.0 {
            request.query_param(&format!("var-{}", key), value);
        }

        let data: Option<Data<GameRecord>> = request
            .execute()
            .await?
            .empty_on_status(StatusCode::NO_CONTENT)
            .json()?;
        Ok(data.map(|d| d.data))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Names {
    international: String,
    #[serde(default)]
    japanese: Option<String>,
    #[serde(default)]
    twitch: Option<String>,
}

impl Names {
    /// Get as printable name.
    pub(crate) fn name(&self) -> &str {
        match self.japanese.as_ref() {
            Some(name) => name,
            None => &self.international,
        }
    }

    /// Check if the given name matches any of the provided names.
    pub(crate) fn matches(&self, pattern: &str) -> bool {
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

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Variables(BTreeMap<String, String>);

impl Variables {
    /// Insert a variable to query for.
    pub(crate) fn insert(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) {
        self.0
            .insert(key.as_ref().to_string(), value.as_ref().to_string());
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize)]
pub(crate) enum Embed {
    Category,
    Game,
    Players,
    Variables,
}

impl Embed {
    /// Get the id of this embed.
    pub(crate) fn id(&self) -> &'static str {
        use self::Embed::*;

        match *self {
            Category => "category",
            Game => "game",
            Players => "players",
            Variables => "variables",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Embeds(BTreeSet<Embed>);

impl Embeds {
    /// Convert into a query.
    pub(crate) fn to_query(&self) -> Option<String> {
        let mut it = self.0.iter().peekable();

        it.peek()?;

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
    pub(crate) fn push(&mut self, embed: Embed) {
        self.0.insert(embed);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "style")]
pub(crate) struct Color {
    light: String,
    dark: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "style")]
pub(crate) enum NameStyle {
    #[serde(rename = "gradient", rename_all = "kebab-case")]
    Gradient { color_from: Color, color_to: Color },
    #[serde(rename = "solid", rename_all = "kebab-case")]
    Solid { color: Color },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Country {
    pub(crate) code: String,
    pub(crate) names: Names,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Location {
    pub(crate) country: Country,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Uri {
    pub(crate) uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Link {
    pub(crate) rel: String,
    pub(crate) uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Asset {
    pub(crate) uri: String,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct User {
    pub(crate) id: String,
    pub(crate) names: Names,
    pub(crate) weblink: String,
    pub(crate) name_style: NameStyle,
    pub(crate) role: String,
    pub(crate) signup: DateTime<Utc>,
    #[serde(default)]
    pub(crate) location: Option<Location>,
    #[serde(default)]
    pub(crate) twitch: Option<Uri>,
    #[serde(default)]
    pub(crate) hitbox: Option<Uri>,
    #[serde(default)]
    pub(crate) youtube: Option<Uri>,
    #[serde(default)]
    pub(crate) twitter: Option<Uri>,
    #[serde(default)]
    pub(crate) speedrunslive: Option<Uri>,
    #[serde(default)]
    pub(crate) links: Vec<Link>,
}

impl User {
    /// Check if the given user matches the given string.
    pub(crate) fn matches(&self, s: &str) -> bool {
        if self.names.matches(s) {
            return true;
        }

        if self.twitch_matches(s) {
            return true;
        }

        false
    }

    /// Test if Twitch matches.
    pub(crate) fn twitch_matches(&self, s: &str) -> bool {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Guest {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) links: Vec<Link>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "rel")]
pub(crate) enum Players {
    #[serde(rename = "user")]
    User(Box<User>),
    #[serde(rename = "guest")]
    Guest(Box<Guest>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Videos {
    #[serde(default)]
    pub(crate) links: Vec<Uri>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Status {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) examiner: Option<String>,
    #[serde(default)]
    pub(crate) verify_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "rel")]
pub(crate) enum RelatedPlayer {
    #[serde(rename = "user")]
    Player(RelatedUser),
    #[serde(rename = "guest")]
    Guest(RelatedGuest),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct RelatedUser {
    pub(crate) id: String,
    pub(crate) uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct RelatedGuest {
    pub(crate) name: String,
    pub(crate) uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Times {
    pub(crate) primary: PtDuration,
    pub(crate) primary_t: serde_json::Number,
    pub(crate) realtime: Option<PtDuration>,
    pub(crate) realtime_t: serde_json::Number,
    pub(crate) realtime_noloads: Option<PtDuration>,
    pub(crate) realtime_noloads_t: serde_json::Number,
    pub(crate) ingame: Option<PtDuration>,
    pub(crate) ingame_t: serde_json::Number,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct System {
    #[serde(default)]
    pub(crate) platform: Option<String>,
    pub(crate) emulated: bool,
    #[serde(default)]
    pub(crate) region: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Splits {
    pub(crate) rel: String,
    pub(crate) uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct RunInfo {
    pub(crate) id: String,
    pub(crate) weblink: String,
    pub(crate) game: String,
    #[serde(default)]
    pub(crate) level: Option<String>,
    pub(crate) category: String,
    #[serde(default)]
    pub(crate) videos: Option<Videos>,
    #[serde(default)]
    pub(crate) comment: Option<String>,
    pub(crate) status: Status,
    #[serde(default)]
    pub(crate) players: Vec<RelatedPlayer>,
    #[serde(default)]
    pub(crate) date: Option<NaiveDate>,
    #[serde(default)]
    pub(crate) submitted: Option<DateTime<Utc>>,
    pub(crate) times: Times,
    pub(crate) system: System,
    pub(crate) splits: Option<Splits>,
    #[serde(default)]
    pub(crate) values: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Run {
    pub(crate) place: u32,
    pub(crate) run: RunInfo,
    /// Annotated information on players, if embed=game was requested.
    #[serde(default)]
    pub(crate) game: Option<Data<Game>>,
    /// Annotated information on players, if embed=category was requested.
    #[serde(default)]
    pub(crate) category: Option<Data<Category>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct VariableFlags {
    #[serde(default)]
    pub(crate) miscellaneous: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct VariableValue {
    pub(crate) label: String,
    pub(crate) rule: Option<String>,
    #[serde(default)]
    pub(crate) flags: VariableFlags,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct VariableValues {
    #[serde(rename = "_note")]
    pub(crate) note: Option<String>,
    #[serde(default)]
    pub(crate) choices: HashMap<String, String>,
    #[serde(default)]
    pub(crate) values: HashMap<String, VariableValue>,
    #[serde(default)]
    pub(crate) default: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Variable {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) category: Option<String>,
    pub(crate) scope: Scope,
    pub(crate) mandatory: bool,
    pub(crate) user_defined: bool,
    pub(crate) obsoletes: bool,
    pub(crate) values: VariableValues,
    pub(crate) is_subcategory: bool,
    #[serde(default)]
    pub(crate) links: Vec<Link>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GameRecord {
    pub(crate) weblink: String,
    pub(crate) game: String,
    pub(crate) category: String,
    #[serde(default)]
    pub(crate) level: Option<String>,
    #[serde(default)]
    pub(crate) platform: Option<String>,
    #[serde(default)]
    pub(crate) region: Option<String>,
    #[serde(default)]
    pub(crate) emulators: serde_json::Value,
    pub(crate) video_only: bool,
    #[serde(default)]
    pub(crate) timing: serde_json::Value,
    #[serde(default)]
    pub(crate) values: serde_json::Value,
    #[serde(default)]
    pub(crate) runs: Vec<Run>,
    #[serde(default)]
    pub(crate) links: Vec<Link>,
    /// Annotated information on players, if embed=players was requested.
    #[serde(default)]
    pub(crate) players: Option<Data<Vec<Players>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct RuleSet {
    pub(crate) show_milliseconds: bool,
    pub(crate) require_verification: bool,
    pub(crate) require_video: bool,
    pub(crate) run_times: Vec<String>,
    pub(crate) default_time: String,
    pub(crate) emulators_allowed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Role {
    SuperModerator,
    Moderator,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Moderators {
    #[serde(flatten)]
    pub(crate) map: HashMap<String, Role>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Game {
    pub(crate) id: String,
    pub(crate) names: Names,
    pub(crate) abbreviation: String,
    pub(crate) weblink: String,
    pub(crate) released: u32,
    pub(crate) release_date: NaiveDate,
    pub(crate) ruleset: RuleSet,
    pub(crate) romhack: bool,
    pub(crate) gametypes: Vec<serde_json::Value>,
    pub(crate) platforms: Vec<String>,
    pub(crate) regions: Vec<String>,
    pub(crate) genres: Vec<String>,
    pub(crate) engines: Vec<String>,
    pub(crate) developers: Vec<String>,
    pub(crate) publishers: Vec<String>,
    pub(crate) moderators: Moderators,
    pub(crate) created: Option<DateTime<Utc>>,
    pub(crate) assets: HashMap<String, Option<Asset>>,
    pub(crate) links: Vec<Link>,
}

impl Game {
    /// Test if game matches the given identifying string.
    pub(crate) fn matches(&self, s: &str) -> bool {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum CategoryPlayers {
    #[serde(rename = "exactly")]
    Exactly { value: u32 },
    #[serde(rename = "up-to")]
    UpTo { value: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CategoryType {
    PerGame,
    PerLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum Scope {
    #[serde(rename_all = "kebab-case")]
    FullGame {},
    #[serde(rename_all = "kebab-case")]
    AllLevels {},
    #[serde(rename_all = "kebab-case")]
    Global {},
    #[serde(rename_all = "kebab-case")]
    SingleLevel { level: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Category {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) weblink: String,
    #[serde(rename = "type")]
    pub(crate) ty: CategoryType,
    #[serde(default)]
    pub(crate) rules: Option<String>,
    pub(crate) players: CategoryPlayers,
    pub(crate) miscellaneous: bool,
    #[serde(default)]
    pub(crate) links: Vec<Link>,
    /// This is included in case we have the `variables` embed.
    #[serde(default)]
    pub(crate) variables: Option<Data<Vec<Variable>>>,
    /// Annotated information on players, if embed=game was requested.
    #[serde(default)]
    pub(crate) game: Option<Data<Game>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Level {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) weblink: String,
    pub(crate) rules: Option<String>,
    pub(crate) links: Vec<Link>,
}

impl Level {
    /// Test if level matches the given identifying string.
    pub(crate) fn matches(&self, s: &str) -> bool {
        if self.id == s {
            return true;
        }

        if self.name.to_lowercase().contains(s) {
            return true;
        }

        false
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Data<T> {
    pub(crate) data: T,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Pagination {
    pub(crate) offset: u64,
    pub(crate) max: u64,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) links: Vec<Link>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Page<T> {
    pub(crate) data: Vec<T>,
    #[serde(default)]
    pub(crate) pagination: Option<Pagination>,
}
