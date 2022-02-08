//! FrankerFaceZ API Client.

use crate::api::RequestBuilder;
use anyhow::Result;
use reqwest::{header, Client, Method, Url};
use std::collections::HashMap;

const V1_URL: &str = "https://api.frankerfacez.com/v1";

/// API integration.
#[derive(Clone, Debug)]
pub struct FrankerFaceZ {
    client: Client,
    v1_url: Url,
}

impl FrankerFaceZ {
    /// Create a new API integration.
    pub fn new() -> Result<FrankerFaceZ> {
        Ok(FrankerFaceZ {
            client: Client::new(),
            v1_url: str::parse::<Url>(V1_URL)?,
        })
    }

    /// Build request against v2 URL.
    fn v1<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.v1_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, method, url);
        req.header(header::ACCEPT, "application/json");
        req
    }

    /// Get information on a single user.
    pub async fn user(&self, user: &str) -> Result<Option<UserInfo>> {
        let req = self.v1(Method::GET, &["user", user]);
        let data = req.execute().await?.not_found().json()?;
        Ok(data)
    }

    /// Get the set associated with the room.
    pub async fn room(&self, room: &str) -> Result<Option<Room>> {
        let req = self.v1(Method::GET, &["room", room]);
        let data = req.execute().await?.not_found().json()?;
        Ok(data)
    }

    /// Get the global set.
    pub async fn set_global(&self) -> Result<Sets> {
        let req = self.v1(Method::GET, &["set", "global"]);
        let data = req.execute().await?.json()?;
        Ok(data)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RoomInfo {
    #[serde(rename = "_id")]
    pub id: u64,
    #[serde(rename = "id")]
    pub name_id: String,
    pub css: serde_json::Value,
    pub display_name: String,
    pub is_group: bool,
    pub mod_urls: serde_json::Value,
    pub moderator_badge: serde_json::Value,
    pub set: u64,
    pub twitch_id: u64,
    pub user_badges: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EmoticonUser {
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
    pub owner: EmoticonUser,
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
    #[serde(default)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub avatar: String,
    pub badges: Vec<u64>,
    pub display_name: String,
    pub emote_sets: Vec<u64>,
    pub id: u64,
    pub is_donor: bool,
    pub name: String,
    pub twitch_id: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Badge {
    pub color: String,
    pub css: serde_json::Value,
    pub id: u64,
    pub image: String,
    pub name: String,
    pub replaces: serde_json::Value,
    pub slot: u32,
    pub title: String,
    pub urls: Urls,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
    pub badges: HashMap<String, Badge>,
    pub sets: HashMap<String, Set>,
    pub user: User,
}
