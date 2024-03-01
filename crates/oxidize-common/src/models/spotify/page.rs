//! All kinds of page object

use serde::{Deserialize, Serialize};

/// [Basic page object](https://developer.spotify.com/web-api/object-model/#paging-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page<T> {
    pub href: String,
    pub items: Vec<T>,
    #[serde(deserialize_with = "super::deserialize_number")]
    pub limit: u32,
    pub next: Option<String>,
    #[serde(deserialize_with = "super::deserialize_number")]
    pub offset: u32,
    pub previous: Option<String>,
    #[serde(deserialize_with = "super::deserialize_number")]
    pub total: u32,
}

/// cursor based page
/// [cursor based paging object](https://developer.spotify.com/web-api/object-model/#cursor-based-paging-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorBasedPage<T> {
    pub href: String,
    pub items: Vec<T>,
    #[serde(deserialize_with = "super::deserialize_number")]
    pub limit: u32,
    pub next: Option<String>,
    pub cursors: Cursor,
    ///absent if it has read all data items. This field doesn't match what
    /// Spotify document says
    #[serde(deserialize_with = "super::deserialize_option_number")]
    pub total: Option<u32>,
}

///Cursor object
///[cursor object](https://developer.spotify.com/web-api/object-model/#cursor-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cursor {
    pub after: Option<String>,
}
