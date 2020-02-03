use super::RequestBuilder;
use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
const URL: &str = "https://api.spotify.com";

#[derive(Debug, Serialize, Deserialize)]
pub struct Followers {
    #[serde(default)]
    href: Option<String>,
    total: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Image {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
    pub url: String,
}

/// Response from the validate token endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "type")]
    pub ty: String,
    pub uri: String,
    pub href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub external_urls: HashMap<String, String>,
    pub followers: Followers,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    pub images: Vec<Image>,
}

/// Client for api.spotify.com
#[derive(Clone, Debug)]
pub struct Spotify {
    client: Client,
    api_url: Url,
}

impl Spotify {
    /// Create a new Spotify client.
    pub fn new() -> Result<Spotify, anyhow::Error> {
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
    pub async fn v1_me(&self, token: &str) -> Result<User, anyhow::Error> {
        let request = self
            .request(Method::GET, &["v1", "me"])
            .header(header::AUTHORIZATION, &format!("Bearer {}", token));

        request.execute().await
    }
}
