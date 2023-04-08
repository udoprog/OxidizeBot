//! Offset object
use serde::{Deserialize, Serialize};

///[offset object](https://developer.spotify.com/documentation/web-api/reference/player/start-a-users-playback/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Offset {
    pub(crate) position: Option<u32>,
    pub(crate) uri: Option<String>,
}

pub(crate) fn for_position(position: u32) -> Option<Offset> {
    Some(Offset {
        position: Some(position),
        uri: None,
    })
}

pub(crate) fn for_uri(uri: String) -> Option<Offset> {
    Some(Offset {
        position: None,
        uri: Some(uri),
    })
}
