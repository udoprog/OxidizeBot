use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A Twitch category.
#[derive(Debug, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub box_art_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Emote {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pagination {
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: Option<Pagination>,
}

#[derive(Deserialize)]
pub struct Clip {
    pub id: String,
    pub edit_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub broadcaster_type: String,
    pub description: String,
    pub profile_image_url: String,
    pub offline_image_url: String,
    pub view_count: u64,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Channel {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub game_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Subscription {
    pub broadcaster_id: String,
    pub broadcaster_name: String,
    pub is_gift: bool,
    pub tier: String,
    pub plan_name: String,
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stream {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    #[serde(default)]
    pub game_id: Option<String>,
    #[serde(default)]
    pub community_ids: Vec<String>,
    #[serde(rename = "type")]
    pub ty: String,
    pub title: String,
    pub viewer_count: u64,
    pub started_at: DateTime<Utc>,
    pub language: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Badge {
    pub id: String,
    pub image_url_1x: String,
    pub image_url_2x: String,
    pub image_url_4x: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatBadge {
    pub set_id: String,
    pub versions: Vec<Badge>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ModifyChannelRequest<'a> {
    pub title: Option<&'a str>,
    pub game_id: Option<&'a str>,
}
