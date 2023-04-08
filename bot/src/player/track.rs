use crate::api;
use crate::utils;

/// Information on a single track.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub(crate) enum Track {
    #[serde(rename = "spotify")]
    Spotify { track: Box<api::spotify::FullTrack> },
    #[serde(rename = "youtube")]
    YouTube { video: Box<api::youtube::Video> },
}

impl Track {
    /// Get artists involved as a string.
    pub(crate) fn artists(&self) -> Option<String> {
        match *self {
            Self::Spotify { ref track } => utils::human_artists(&track.artists),
            Self::YouTube { ref video } => {
                video.snippet.as_ref().and_then(|s| s.channel_title.clone())
            }
        }
    }

    /// Get name of the track.
    pub(crate) fn name(&self) -> String {
        match *self {
            Self::Spotify { ref track } => track.name.to_string(),
            Self::YouTube { ref video } => video
                .snippet
                .as_ref()
                .map(|s| s.title.as_str())
                .unwrap_or("no name")
                .to_string(),
        }
    }
}
