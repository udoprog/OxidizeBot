//! All objects related to artist defined by Spotify API

use super::image::Image;
use super::page::CursorBasedPage;
use super::senum::Type;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

///[artist object simplified](https://developer.spotify.com/web-api/object-model/#artist-object-simplified)
/// Simplified Artist Object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SimplifiedArtist {
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) href: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: Option<String>,
}

///[artist object full](https://developer.spotify.com/web-api/object-model/#artist-object-full)
/// Full Artist Object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FullArtist {
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) followers: HashMap<String, Option<Value>>,
    pub(crate) genres: Vec<String>,
    pub(crate) href: String,
    pub(crate) id: String,
    pub(crate) images: Vec<Image>,
    pub(crate) name: String,
    pub(crate) popularity: u32,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}

/// Full artist vector
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FullArtists {
    pub(crate) artists: Vec<FullArtist>,
}

/// Full Artists vector wrapped by cursor-based-page object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CursorPageFullArtists {
    pub(crate) artists: CursorBasedPage<FullArtist>,
}
