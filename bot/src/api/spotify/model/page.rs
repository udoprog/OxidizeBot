//! All kinds of page object
use serde::{Deserialize, Serialize};

/// [Basic page object](https://developer.spotify.com/web-api/object-model/#paging-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Page<T> {
    pub(crate) href: String,
    pub(crate) items: Vec<T>,
    pub(crate) limit: u32,
    pub(crate) next: Option<String>,
    pub(crate) offset: u32,
    pub(crate) previous: Option<String>,
    pub(crate) total: u32,
}
/// cursor based page
/// [cursor based paging object](https://developer.spotify.com/web-api/object-model/#cursor-based-paging-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CursorBasedPage<T> {
    pub(crate) href: String,
    pub(crate) items: Vec<T>,
    pub(crate) limit: u32,
    pub(crate) next: Option<String>,
    pub(crate) cursors: Cursor,
    ///absent if it has read all data items. This field doesn't match what
    /// Spotify document says
    pub(crate) total: Option<u32>,
}
///Cursor object
///[cursor object](https://developer.spotify.com/web-api/object-model/#cursor-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Cursor {
    pub(crate) after: Option<String>,
}
