//! All kinds of user object
use chrono::NaiveDate;
use serde_json::Value;

use std::collections::HashMap;

use super::image::Image;
use super::senum::Type;
use serde::{Deserialize, Serialize};

///[public user object](https://developer.spotify.com/web-api/object-model/#user-object-public)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PublicUser {
    pub(crate) display_name: Option<String>,
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) followers: Option<HashMap<String, Option<Value>>>,
    pub(crate) href: String,
    pub(crate) id: String,
    pub(crate) images: Option<Vec<Image>>,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}

///[private user object](https://developer.spotify.com/web-api/object-model/#user-object-private)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PrivateUser {
    pub(crate) birthdate: Option<NaiveDate>,
    pub(crate) country: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) followers: Option<HashMap<String, Option<Value>>>,
    pub(crate) href: String,
    pub(crate) id: String,
    pub(crate) images: Option<Vec<Image>>,
    #[serde(rename = "type")]
    pub(crate) _type: Type,
    pub(crate) uri: String,
}
