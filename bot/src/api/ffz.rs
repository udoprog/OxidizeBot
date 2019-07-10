//! FrankerFaceZ API Client.

use crate::api::RequestBuilder;
use failure::Error;
use hashbrown::HashMap;
use reqwest::{header, r#async::Client, Method, StatusCode, Url};

const V1_URL: &'static str = "https://api.frankerfacez.com/v1";

/// API integration.
#[derive(Clone, Debug)]
pub struct FrankerFaceZ {
    client: Client,
    v1_url: Url,
}

impl FrankerFaceZ {
    /// Create a new API integration.
    pub fn new() -> Result<FrankerFaceZ, Error> {
        Ok(FrankerFaceZ {
            client: Client::new(),
            v1_url: str::parse::<Url>(V1_URL)?,
        })
    }

    /// Build request against v2 URL.
    fn v1(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.v1_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.extend(path);
        }

        let req = RequestBuilder::new(self.client.clone(), method, url);
        req.header(header::ACCEPT, "application/json")
    }

    /// Get the set associated with the room.
    pub async fn room(&self, room: &str) -> Result<Option<Room>, Error> {
        let req = self.v1(Method::GET, &["room", room]);
        let data = req.execute().await?.json_option(not_found)?;
        Ok(data)
    }

    /// Get the global set.
    pub async fn set_global(&self) -> Result<Sets, Error> {
        let req = self.v1(Method::GET, &["set", "global"]);
        let data = req.execute().await?.json()?;
        Ok(data)
    }
}

/// Handle as not found.
fn not_found(status: &StatusCode) -> bool {
    match *status {
        StatusCode::NOT_FOUND => true,
        _ => false,
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RoomInfo {
    #[serde(rename = "_id")]
    id: u64,
    #[serde(rename = "id")]
    name_id: String,
    css: serde_json::Value,
    display_name: String,
    is_group: bool,
    mod_urls: serde_json::Value,
    moderator_badge: serde_json::Value,
    set: u64,
    twitch_id: u64,
    user_badges: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: u64,
    pub display_name: String,
    pub name: String,
}

/// URLs of different sizes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Urls {
    #[serde(rename = "1")]
    pub x1: Option<String>,
    #[serde(rename = "2")]
    pub x2: Option<String>,
    #[serde(rename = "4")]
    pub x4: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Emoticon {
    pub id: u64,
    pub css: serde_json::Value,
    pub width: u32,
    pub height: u32,
    pub hidden: bool,
    pub margins: serde_json::Value,
    pub modifier: bool,
    pub name: String,
    pub offset: serde_json::Value,
    pub owner: User,
    pub public: bool,
    pub urls: Urls,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Set {
    #[serde(rename = "_type")]
    pub ty: u32,
    pub id: u64,
    pub title: String,
    pub css: serde_json::Value,
    pub description: serde_json::Value,
    pub emoticons: Vec<Emoticon>,
    pub icon: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Room {
    pub room: RoomInfo,
    pub sets: HashMap<String, Set>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Sets {
    pub default_sets: Vec<u64>,
    pub sets: HashMap<String, Set>,
}
