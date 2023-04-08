//! All object related to search
use super::album::SimplifiedAlbum;
use super::artist::FullArtist;
use super::page::Page;
use super::playlist::SimplifiedPlaylist;
use super::track::FullTrack;
use serde::{Deserialize, Serialize};

///[search item](https://developer.spotify.com/web-api/search-item/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SearchPlaylists {
    pub(crate) playlists: Page<SimplifiedPlaylist>,
}

///[search item](https://developer.spotify.com/web-api/search-item/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SearchAlbums {
    pub(crate) albums: Page<SimplifiedAlbum>,
}

///[search item](https://developer.spotify.com/web-api/search-item/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SearchArtists {
    pub(crate) artists: Page<FullArtist>,
}

///[search item](https://developer.spotify.com/web-api/search-item/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SearchTracks {
    pub(crate) tracks: Page<FullTrack>,
}
