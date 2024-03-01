//! All objects related to artist defined by Spotify API

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::image::Image;
use super::page::CursorBasedPage;
use super::senum::Type;

///[artist object simplified](https://developer.spotify.com/web-api/object-model/#artist-object-simplified)
/// Simplified Artist Object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimplifiedArtist {
    pub external_urls: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub _type: Type,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

///[artist object full](https://developer.spotify.com/web-api/object-model/#artist-object-full)
/// Full Artist Object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullArtist {
    pub external_urls: HashMap<String, String>,
    pub followers: HashMap<String, Option<Value>>,
    pub genres: Vec<String>,
    pub href: String,
    pub id: String,
    pub images: Vec<Image>,
    pub name: String,
    #[serde(deserialize_with = "super::deserialize_number")]
    pub popularity: u32,
    #[serde(rename = "type")]
    pub _type: Type,
    pub uri: String,
}

/// Full artist vector
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullArtists {
    pub artists: Vec<FullArtist>,
}

/// Full Artists vector wrapped by cursor-based-page object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorPageFullArtists {
    pub artists: CursorBasedPage<FullArtist>,
}
