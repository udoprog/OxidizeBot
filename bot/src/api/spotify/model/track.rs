//! All kinds of tracks object
use chrono::prelude::*;

use std::collections::HashMap;

use super::album::Restrictions;
use super::album::SimplifiedAlbum;
use super::artist::SimplifiedArtist;
use super::senum::Type;
use serde::{Deserialize, Serialize};

///[track object full](https://developer.spotify.com/web-api/object-model/#track-object-full)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FullTrack {
    pub(crate) album: SimplifiedAlbum,
    pub(crate) artists: Vec<SimplifiedArtist>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) available_markets: Vec<String>,
    pub(crate) disc_number: i32,
    pub(crate) duration_ms: u32,
    pub(crate) explicit: bool,
    pub(crate) external_ids: HashMap<String, String>,
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) href: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) is_local: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_playable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) linked_from: Option<TrackLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) restrictions: Option<Restrictions>,
    pub(crate) name: String,
    pub(crate) popularity: u32,
    pub(crate) preview_url: Option<String>,
    pub(crate) track_number: u32,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}

/// [link to track link] https://developer.spotify.com/documentation/web-api/reference/object-model/#track-link
/// Track Link

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TrackLink {
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) href: String,
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FullTracks {
    pub(crate) tracks: Vec<FullTrack>,
}

///[track object simplified](https://developer.spotify.com/web-api/object-model/#track-object-simplified)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SimplifiedTrack {
    pub(crate) artists: Vec<SimplifiedArtist>,
    pub(crate) available_markets: Option<Vec<String>>,
    pub(crate) disc_number: i32,
    pub(crate) duration_ms: u32,
    pub(crate) explicit: bool,
    pub(crate) external_urls: HashMap<String, String>,
    #[serde(default)]
    pub(crate) href: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) is_local: bool,
    pub(crate) name: String,
    pub(crate) preview_url: Option<String>,
    pub(crate) track_number: u32,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}

///[saved track object](https://developer.spotify.com/web-api/object-model/#saved-track-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SavedTrack {
    pub(crate) added_at: DateTime<Utc>,
    pub(crate) track: FullTrack,
}
