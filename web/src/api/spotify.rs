use std::collections::HashMap;

use anyhow::Result;
use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};

use super::RequestBuilder;

const URL: &str = "https://api.spotify.com";

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Followers {
    #[serde(default)]
    href: Option<String>,
    total: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Image {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) height: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) width: Option<u64>,
    pub(crate) url: String,
}

/// Response from the validate token endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct User {
    #[serde(rename = "type")]
    pub(crate) ty: String,
    pub(crate) uri: String,
    pub(crate) href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) country: Option<String>,
    pub(crate) display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) email: Option<String>,
    pub(crate) external_urls: HashMap<String, String>,
    pub(crate) followers: Followers,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) product: Option<String>,
    pub(crate) images: Vec<Image>,
}

/// Client for api.spotify.com
#[derive(Clone, Debug)]
pub(crate) struct Spotify {
    client: Client,
    api_url: Url,
}

impl Spotify {
    /// Create a new Spotify client.
    pub(crate) fn new() -> Result<Spotify> {
        Ok(Spotify {
            client: Client::new(),
            api_url: str::parse::<Url>(URL)?,
        })
    }

    /// Construct a new request.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("bad base").extend(path);
        RequestBuilder::new(self.client.clone(), method, url)
    }

    // Get user information about the token.
    pub(crate) async fn v1_me(&self, token: &str) -> Result<User> {
        let request = self
            .request(Method::GET, &["v1", "me"])
            .header(header::AUTHORIZATION, &format!("Bearer {}", token));

        request.execute().await
    }
}
