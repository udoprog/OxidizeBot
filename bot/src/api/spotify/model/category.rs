//! All object related to category
use super::image::Image;
use super::page::Page;
use serde::{Deserialize, Serialize};
/// category object
///[category object](https://developer.spotify.com/web-api/get-list-categories/#categoryobject)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Category {
    pub(crate) href: String,
    pub(crate) icons: Vec<Image>,
    pub(crate) id: String,
    pub(crate) name: String,
}

/// Categories wrapped by page object
///[get list categories](https://developer.spotify.com/web-api/get-list-categories/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PageCategory {
    pub(crate) categories: Page<Category>,
}
