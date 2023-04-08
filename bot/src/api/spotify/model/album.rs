//! All objects related to album defined by Spotify API
use chrono::prelude::*;

use std::collections::HashMap;

use super::artist::SimplifiedArtist;
use super::image::Image;
use super::page::Page;
use super::senum::{AlbumType, Type};
use super::track::SimplifiedTrack;
use serde::{Deserialize, Serialize};

///[link to album object simplified](https://developer.spotify.com/web-api/object-model/#album-object-simplified)
/// Simplified Album Object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SimplifiedAlbum {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) album_group: Option<String>,
    pub(crate) album_type: Option<String>,
    pub(crate) artists: Vec<SimplifiedArtist>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) available_markets: Vec<String>,
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) href: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) images: Vec<Image>,
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) release_date_precision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) restrictions: Option<Restrictions>,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: Option<String>,
}

/// Restrictions object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Restrictions {
    pub(crate) reason: String,
}

///[link to album object full](https://developer.spotify.com/web-api/object-model/#album-object-full)
/// Full Album Object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FullAlbum {
    pub(crate) artists: Vec<SimplifiedArtist>,
    pub(crate) album_type: AlbumType,
    pub(crate) available_markets: Vec<String>,
    pub(crate) copyrights: Vec<HashMap<String, String>>,
    pub(crate) external_ids: HashMap<String, String>,
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) genres: Vec<String>,
    pub(crate) href: String,
    pub(crate) id: String,
    pub(crate) images: Vec<Image>,
    pub(crate) name: String,
    pub(crate) popularity: u32,
    pub(crate) release_date: String,
    pub(crate) release_date_precision: String,
    pub(crate) tracks: Page<SimplifiedTrack>,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}

/// Full Albums
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FullAlbums {
    pub(crate) albums: Vec<FullAlbum>,
}

///[link to get list new releases](https://developer.spotify.com/web-api/get-list-new-releases/)
/// Simplified Albums wrapped by Page object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PageSimpliedAlbums {
    pub(crate) albums: Page<SimplifiedAlbum>,
}

///[link to save album object](https://developer.spotify.com/web-api/object-model/#save-album-object)
/// Saved Album object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SavedAlbum {
    pub(crate) added_at: DateTime<Utc>,
    pub(crate) album: FullAlbum,
}
