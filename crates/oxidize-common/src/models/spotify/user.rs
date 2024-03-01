//! All kinds of user object

use std::collections::HashMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::image::Image;
use super::senum::Type;

///[public user object](https://developer.spotify.com/web-api/object-model/#user-object-public)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicUser {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub external_urls: HashMap<String, String>,
    pub followers: Option<HashMap<String, Option<Value>>>,
    pub href: String,
    pub id: String,
    pub images: Option<Vec<Image>>,
    #[serde(rename = "type")]
    pub _type: Type,
    pub uri: String,
}

///[private user object](https://developer.spotify.com/web-api/object-model/#user-object-private)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivateUser {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birthdate: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub external_urls: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub followers: Option<HashMap<String, Option<Value>>>,
    pub href: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<Image>>,
    #[serde(rename = "type")]
    pub _type: Type,
    pub uri: String,
}
