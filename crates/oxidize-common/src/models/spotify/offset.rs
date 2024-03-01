//! Offset object

use serde::{Deserialize, Serialize};

///[offset object](https://developer.spotify.com/documentation/web-api/reference/player/start-a-users-playback/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Offset {
    #[serde(deserialize_with = "super::deserialize_option_number")]
    pub position: Option<u32>,
    pub uri: Option<String>,
}
