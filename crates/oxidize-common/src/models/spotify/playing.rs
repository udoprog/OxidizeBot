//! All kinds of play object

use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::context::Context;
use super::track::FullTrack;
use super::track::SimplifiedTrack;

/// current playing track
///[get the users currently playing track](https://developer.spotify.com/web-api/get-the-users-currently-playing-track/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Playing {
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

/// playing history
///[play history object](https://developer.spotify.com/web-api/object-model/#play-history-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayHistory {
    pub track: SimplifiedTrack,
    pub played_at: DateTime<Utc>,
    pub context: Option<Context>,
}
