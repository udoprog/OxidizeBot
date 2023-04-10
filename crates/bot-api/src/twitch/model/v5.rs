use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct UpdateChannelRequest {
    pub(crate) channel: UpdateChannel,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct UpdateChannel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) game: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) delay: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) channel_feed_enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct User {
    #[serde(rename = "_id")]
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: String,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    #[serde(default)]
    pub(crate) bio: Option<String>,
    pub(crate) email: String,
    pub(crate) email_verified: bool,
    #[serde(default)]
    pub(crate) logo: Option<String>,
    pub(crate) notifications: HashMap<String, bool>,
    pub(crate) partnered: bool,
    pub(crate) twitter_connected: bool,
    #[serde(rename = "type")]
    pub(crate) ty: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Channel {
    pub(crate) mature: bool,
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) broadcaster_language: Option<String>,
    #[serde(default)]
    pub(crate) display_name: Option<String>,
    #[serde(default)]
    pub(crate) game: Option<String>,
    #[serde(default)]
    pub(crate) language: Option<String>,
    #[serde(rename = "_id")]
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) partner: bool,
    #[serde(default)]
    pub(crate) logo: Option<String>,
    #[serde(default)]
    pub(crate) video_banner: Option<String>,
    #[serde(default)]
    pub(crate) profile_banner: Option<String>,
    #[serde(default)]
    pub(crate) profile_banner_background_color: Option<String>,
    pub(crate) url: String,
    pub(crate) views: u64,
    pub(crate) followers: u64,
    #[serde(default)]
    pub(crate) broadcaster_type: Option<String>,
    #[serde(default)]
    pub(crate) stream_key: Option<String>,
    #[serde(default)]
    pub(crate) email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Emote {
    pub(crate) code: String,
    pub(crate) id: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EmoticonSets {
    pub(crate) emoticon_sets: HashMap<String, Vec<Emote>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct BadgeTypes {
    #[serde(default)]
    pub(crate) alpha: Option<String>,
    #[serde(default)]
    pub(crate) image: Option<String>,
    #[serde(default)]
    pub(crate) svg: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ChatBadges {
    #[serde(flatten)]
    pub(crate) badges: HashMap<String, BadgeTypes>,
}
