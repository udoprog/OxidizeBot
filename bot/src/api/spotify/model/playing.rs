//! All kinds of play object
use chrono::prelude::*;

use super::context::Context;
use super::track::FullTrack;
use super::track::SimplifiedTrack;
use serde::{Deserialize, Serialize};
/// current playing track
///[get the users currently playing track](https://developer.spotify.com/web-api/get-the-users-currently-playing-track/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Playing {
    pub(crate) context: Option<Context>,
    pub(crate) timestamp: u64,
    pub(crate) progress_ms: Option<u32>,
    pub(crate) is_playing: bool,
    pub(crate) item: Option<FullTrack>,
}

/// playing history
///[play history object](https://developer.spotify.com/web-api/object-model/#play-history-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlayHistory {
    pub(crate) track: SimplifiedTrack,
    pub(crate) played_at: DateTime<Utc>,
    pub(crate) context: Option<Context>,
}
