//! All objects related to context
use std::collections::HashMap;

use super::device::Device;
use super::senum::{RepeatState, Type};
use super::track::FullTrack;
use serde::{Deserialize, Serialize};
/// Context object
///[get the users currently playing track](https://developer.spotify.com/web-api/get-the-users-currently-playing-track/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Context {
    pub(crate) uri: String,
    pub(crate) href: String,
    pub(crate) external_urls: HashMap<String, String>,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
}

/// Full playing context
///[get information about the users current playback](https://developer.spotify.com/web-api/get-information-about-the-users-current-playback/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FullPlayingContext {
    pub(crate) device: Device,
    pub(crate) repeat_state: RepeatState,
    pub(crate) shuffle_state: bool,
    pub(crate) context: Option<Context>,
    pub(crate) timestamp: u64,
    pub(crate) progress_ms: Option<u32>,
    pub(crate) is_playing: bool,
    pub(crate) item: Option<FullTrack>,
}

///[get the users currently playing track](https://developer.spotify.com/web-api/get-the-users-currently-playing-track/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SimplifiedPlayingContext {
    pub(crate) context: Option<Context>,
    pub(crate) timestamp: u64,
    pub(crate) progress_ms: Option<u32>,
    pub(crate) is_playing: bool,
    pub(crate) item: Option<FullTrack>,
}
