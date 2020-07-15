use crate::api;
use crate::utils;
use anyhow::Result;

/// Information on a single track.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum Track {
    #[serde(rename = "spotify")]
    Spotify { track: api::spotify::FullTrack },
    #[serde(rename = "youtube")]
    YouTube { video: api::youtube::Video },
}

impl Track {
    /// Get artists involved as a string.
    pub fn artists(&self) -> Option<String> {
        match *self {
            Self::Spotify { ref track } => utils::human_artists(&track.artists),
            Self::YouTube { ref video } => {
                video.snippet.as_ref().and_then(|s| s.channel_title.clone())
            }
        }
    }

    /// Get name of the track.
    pub fn name(&self) -> String {
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

    /// Convert into JSON.
    /// TODO: this is a hack to avoid breaking web API.
    pub fn to_json(&self) -> Result<serde_json::Value> {
        let json = match *self {
            Self::Spotify { ref track } => serde_json::to_value(&track)?,
            Self::YouTube { ref video } => serde_json::to_value(&video)?,
        };

        Ok(json)
    }
}
