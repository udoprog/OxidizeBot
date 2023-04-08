//! All kinds of playlists objects
use chrono::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

use super::image::Image;
use super::page::Page;
use super::senum::Type;
use super::track::FullTrack;
use super::user::PublicUser;
use serde::{Deserialize, Serialize};
///[playlist object simplified](https://developer.spotify.com/web-api/object-model/#playlist-object-simplified)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SimplifiedPlaylist {
    pub(crate) collaborative: bool,
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) href: String,
    pub(crate) id: String,
    pub(crate) images: Vec<Image>,
    pub(crate) name: String,
    pub(crate) owner: PublicUser,
    pub(crate) public: Option<bool>,
    pub(crate) snapshot_id: String,
    pub(crate) tracks: HashMap<String, Value>,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FullPlaylist {
    pub(crate) collaborative: bool,
    pub(crate) description: String,
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) followers: Option<HashMap<String, Value>>,
    pub(crate) href: String,
    pub(crate) id: String,
    pub(crate) images: Vec<Image>,
    pub(crate) name: String,
    pub(crate) owner: PublicUser,
    pub(crate) public: Option<bool>,
    pub(crate) snapshot_id: String,
    pub(crate) tracks: Page<PlaylistTrack>,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}

///[playlist track object](https://developer.spotify.com/web-api/object-model/#playlist-track-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlaylistTrack {
    pub(crate) added_at: DateTime<Utc>,
    pub(crate) added_by: Option<PublicUser>,
    pub(crate) is_local: bool,
    pub(crate) track: FullTrack,
}
///[get list featured playlists](https://developer.spotify.com/web-api/get-list-featured-playlists/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FeaturedPlaylists {
    pub(crate) message: String,
    pub(crate) playlists: Page<SimplifiedPlaylist>,
}
