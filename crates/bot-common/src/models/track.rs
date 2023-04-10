use serde::{Deserialize, Serialize};

use crate::display;

/// Information on a single track.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Track {
    #[serde(rename = "spotify")]
    Spotify {
        track: Box<crate::models::spotify::track::FullTrack>,
    },
    #[serde(rename = "youtube")]
    YouTube {
        video: Box<crate::models::youtube::Video>,
    },
}

impl Track {
    /// Get artists involved as a string.
    pub fn artists(&self) -> Option<String> {
        match self {
            Self::Spotify { track } => display::human_artists(&track.artists),
            Self::YouTube { video } => video.snippet.as_ref().and_then(|s| s.channel_title.clone()),
        }
    }

    /// Get name of the track.
    pub fn name(&self) -> String {
        match self {
            Self::Spotify { track } => track.name.to_string(),
            Self::YouTube { video } => video
                .snippet
                .as_ref()
                .map(|s| s.title.as_str())
                .unwrap_or("no name")
                .to_string(),
        }
    }
}
