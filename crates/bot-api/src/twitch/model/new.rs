use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A Twitch category.
#[derive(Debug, Deserialize)]
pub(crate) struct Category {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub struct Emote {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Pagination {
    #[serde(default)]
    pub(crate) cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Page<T> {
    pub(crate) data: Vec<T>,
    #[serde(default)]
    pub(crate) pagination: Option<Pagination>,
}

#[derive(Deserialize)]
pub(crate) struct Clip {
    pub(crate) id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct User {
    pub(crate) id: String,
    pub(crate) login: String,
    pub(crate) display_name: String,
    #[serde(rename = "type")]
    pub(crate) ty: String,
    pub(crate) broadcaster_type: String,
    pub(crate) description: String,
    pub(crate) profile_image_url: String,
    pub(crate) offline_image_url: String,
    pub(crate) view_count: u64,
    #[serde(default)]
    pub(crate) email: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Channel {
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) game_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Subscription {
    pub(crate) broadcaster_id: String,
    pub(crate) broadcaster_name: String,
    pub(crate) is_gift: bool,
    pub(crate) tier: String,
    pub(crate) plan_name: String,
    pub(crate) user_id: String,
    pub(crate) user_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct Stream {
    pub(crate) id: String,
    pub(crate) user_id: String,
    pub(crate) user_name: String,
    #[serde(default)]
    pub(crate) game_id: Option<String>,
    #[serde(default)]
    pub(crate) community_ids: Vec<String>,
    #[serde(rename = "type")]
    pub(crate) ty: String,
    pub(crate) title: String,
    pub(crate) viewer_count: u64,
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) language: String,
    pub(crate) thumbnail_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Badge {
    pub(crate) id: String,
    pub(crate) image_url_1x: String,
    pub(crate) image_url_2x: String,
    pub(crate) image_url_4x: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ChatBadge {
    pub(crate) set_id: String,
    pub(crate) versions: Vec<Badge>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ModifyChannelRequest<'a> {
    pub(crate) title: Option<&'a str>,
    pub(crate) game_id: Option<&'a str>,
}
