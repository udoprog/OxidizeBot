use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UpdateChannelRequest {
    pub channel: UpdateChannel,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UpdateChannel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_feed_enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub bio: Option<String>,
    pub email: String,
    pub email_verified: bool,
    #[serde(default)]
    pub logo: Option<String>,
    pub notifications: HashMap<String, bool>,
    pub partnered: bool,
    pub twitter_connected: bool,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Channel {
    pub mature: bool,
    pub status: Option<String>,
    #[serde(default)]
    pub broadcaster_language: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub game: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub partner: bool,
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default)]
    pub video_banner: Option<String>,
    #[serde(default)]
    pub profile_banner: Option<String>,
    #[serde(default)]
    pub profile_banner_background_color: Option<String>,
    pub url: String,
    pub views: u64,
    pub followers: u64,
    #[serde(default)]
    pub broadcaster_type: Option<String>,
    #[serde(default)]
    pub stream_key: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Emote {
    pub code: String,
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct EmoticonSets {
    pub emoticon_sets: HashMap<String, Vec<Emote>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BadgeTypes {
    #[serde(default)]
    pub alpha: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub svg: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatBadges {
    #[serde(flatten)]
    pub badges: HashMap<String, BadgeTypes>,
}
