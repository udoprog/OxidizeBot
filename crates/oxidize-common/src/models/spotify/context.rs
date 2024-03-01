//! All objects related to context

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::device::Device;
use super::senum::{RepeatState, Type};
use super::track::FullTrack;

/// Context object
///[get the users currently playing track](https://developer.spotify.com/web-api/get-the-users-currently-playing-track/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Context {
    pub uri: String,
    pub href: String,
    pub external_urls: HashMap<String, String>,
    #[serde(rename = "type")]
    pub _type: Type,
}

/// Full playing context
///[get information about the users current playback](https://developer.spotify.com/web-api/get-information-about-the-users-current-playback/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullPlayingContext {
    pub device: Device,
    pub repeat_state: RepeatState,
    pub shuffle_state: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
    #[serde(deserialize_with = "super::deserialize_number")]
    pub timestamp: u64,
    #[serde(default, deserialize_with = "super::deserialize_option_number")]
    pub progress_ms: Option<u32>,
    pub is_playing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<FullTrack>,
}

///[get the users currently playing track](https://developer.spotify.com/web-api/get-the-users-currently-playing-track/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimplifiedPlayingContext {
    pub context: Option<Context>,
    #[serde(deserialize_with = "super::deserialize_number")]
    pub timestamp: u64,
    #[serde(default, deserialize_with = "super::deserialize_option_number")]
    pub progress_ms: Option<u32>,
    pub is_playing: bool,
    pub item: Option<FullTrack>,
}
