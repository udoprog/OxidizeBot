//! FrankerFaceZ API Client.

use crate::api::RequestBuilder;
use anyhow::Result;
use reqwest::{header, Client, Method, Url};
use std::collections::HashMap;

const V1_URL: &str = "https://api.frankerfacez.com/v1";

/// API integration.
#[derive(Clone, Debug)]
pub(crate) struct FrankerFaceZ {
    client: Client,
    v1_url: Url,
}

impl FrankerFaceZ {
    /// Create a new API integration.
    pub(crate) fn new() -> Result<FrankerFaceZ> {
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
    pub(crate) async fn user(&self, user: &str) -> Result<Option<UserInfo>> {
        let req = self.v1(Method::GET, &["user", user]);
        let data = req.execute().await?.not_found().json()?;
        Ok(data)
    }

    /// Get the set associated with the room.
    pub(crate) async fn room(&self, room: &str) -> Result<Option<Room>> {
        let req = self.v1(Method::GET, &["room", room]);
        let data = req.execute().await?.not_found().json()?;
        Ok(data)
    }

    /// Get the global set.
    pub(crate) async fn set_global(&self) -> Result<Sets> {
        let req = self.v1(Method::GET, &["set", "global"]);
        let data = req.execute().await?.json()?;
        Ok(data)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct RoomInfo {
    #[serde(rename = "_id")]
    pub(crate) id: u64,
    #[serde(rename = "id")]
    pub(crate) name_id: String,
    pub(crate) css: serde_json::Value,
    pub(crate) display_name: String,
    pub(crate) is_group: bool,
    pub(crate) mod_urls: serde_json::Value,
    pub(crate) moderator_badge: serde_json::Value,
    pub(crate) set: u64,
    pub(crate) twitch_id: u64,
    pub(crate) user_badges: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct EmoticonUser {
    #[serde(rename = "_id")]
    pub(crate) id: u64,
    pub(crate) display_name: String,
    pub(crate) name: String,
}

/// URLs of different sizes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Urls {
    #[serde(rename = "1")]
    pub(crate) x1: Option<String>,
    #[serde(rename = "2")]
    pub(crate) x2: Option<String>,
    #[serde(rename = "4")]
    pub(crate) x4: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Emoticon {
    pub(crate) id: u64,
    pub(crate) css: serde_json::Value,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) hidden: bool,
    pub(crate) margins: serde_json::Value,
    pub(crate) modifier: bool,
    pub(crate) name: String,
    pub(crate) offset: serde_json::Value,
    pub(crate) owner: EmoticonUser,
    pub(crate) public: bool,
    pub(crate) urls: Urls,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Set {
    #[serde(rename = "_type")]
    pub(crate) ty: u32,
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) css: serde_json::Value,
    #[serde(default)]
    pub(crate) description: serde_json::Value,
    pub(crate) emoticons: Vec<Emoticon>,
    pub(crate) icon: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Room {
    pub(crate) room: RoomInfo,
    pub(crate) sets: HashMap<String, Set>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Sets {
    pub(crate) default_sets: Vec<u64>,
    pub(crate) sets: HashMap<String, Set>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct User {
    pub(crate) avatar: String,
    pub(crate) badges: Vec<u64>,
    pub(crate) display_name: String,
    pub(crate) emote_sets: Vec<u64>,
    pub(crate) id: u64,
    pub(crate) is_donor: bool,
    pub(crate) name: String,
    pub(crate) twitch_id: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Badge {
    pub(crate) color: String,
    pub(crate) css: serde_json::Value,
    pub(crate) id: u64,
    pub(crate) image: String,
    pub(crate) name: String,
    pub(crate) replaces: serde_json::Value,
    pub(crate) slot: u32,
    pub(crate) title: String,
    pub(crate) urls: Urls,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct UserInfo {
    pub(crate) badges: HashMap<String, Badge>,
    pub(crate) sets: HashMap<String, Set>,
    pub(crate) user: User,
}
