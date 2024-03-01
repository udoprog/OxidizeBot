//! All objects related to album defined by Spotify API
use chrono::prelude::*;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::artist::SimplifiedArtist;
use super::image::Image;
use super::page::Page;
use super::senum::{AlbumType, Type};
use super::track::SimplifiedTrack;

///[link to album object simplified](https://developer.spotify.com/web-api/object-model/#album-object-simplified)
/// Simplified Album Object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimplifiedAlbum {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_type: Option<String>,
    pub artists: Vec<SimplifiedArtist>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub available_markets: Vec<String>,
    pub external_urls: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub images: Vec<Image>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date_precision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restrictions: Option<Restrictions>,
    #[serde(rename = "type")]
    pub _type: Type,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Restrictions object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Restrictions {
    pub reason: String,
}

///[link to album object full](https://developer.spotify.com/web-api/object-model/#album-object-full)
/// Full Album Object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullAlbum {
    pub artists: Vec<SimplifiedArtist>,
    pub album_type: AlbumType,
    pub available_markets: Vec<String>,
    pub copyrights: Vec<HashMap<String, String>>,
    pub external_ids: HashMap<String, String>,
    pub external_urls: HashMap<String, String>,
    pub genres: Vec<String>,
    pub href: String,
    pub id: String,
    pub images: Vec<Image>,
    pub name: String,
    #[serde(deserialize_with = "super::deserialize_number")]
    pub popularity: u32,
    pub release_date: String,
    pub release_date_precision: String,
    pub tracks: Page<SimplifiedTrack>,
    #[serde(rename = "type")]
    pub _type: Type,
    pub uri: String,
}

/// Full Albums
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullAlbums {
    pub albums: Vec<FullAlbum>,
}

///[link to get list new releases](https://developer.spotify.com/web-api/get-list-new-releases/)
/// Simplified Albums wrapped by Page object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageSimpliedAlbums {
    pub albums: Page<SimplifiedAlbum>,
}

///[link to save album object](https://developer.spotify.com/web-api/object-model/#save-album-object)
/// Saved Album object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedAlbum {
    pub added_at: DateTime<Utc>,
    pub album: FullAlbum,
}
