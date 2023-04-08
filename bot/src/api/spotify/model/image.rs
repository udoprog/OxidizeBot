//! Image object
use serde::{Deserialize, Serialize};

///[image object](https://developer.spotify.com/web-api/object-model/#image-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Image {
    pub(crate) height: Option<u32>,
    pub(crate) url: String,
    pub(crate) width: Option<u32>,
}
