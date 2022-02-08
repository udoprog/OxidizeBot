//! Twitch API helpers.

use crate::api::RequestBuilder;
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{Client, Method, Url};

const API_URL: &str = "https://api.github.com";

/// API integration.
#[derive(Clone, Debug)]
pub struct GitHub {
    client: Client,
    api_url: Url,
}

impl GitHub {
    /// Create a new API integration.
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            api_url: str::parse::<Url>(API_URL)?,
        })
    }

    /// Build request against v3 URL.
    fn request<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.api_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        RequestBuilder::new(&self.client, method, url)
    }

    /// Get all releases for the given repo.
    pub async fn releases(&self, user: String, repo: String) -> Result<Vec<Release>> {
        let req = self.request(
            Method::GET,
            &["repos", user.as_str(), repo.as_str(), "releases"],
        );

        Ok(req.execute().await?.json()?)
    }

    /// Get all releases for the given repo.
    pub async fn releases_latest(&self, user: String, repo: String) -> Result<Option<Release>> {
        let req = self.request(
            Method::GET,
            &["repos", user.as_str(), repo.as_str(), "releases", "latest"],
        );

        Ok(req.execute().await?.json()?)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Release {
    pub tag_name: String,
    pub prerelease: bool,
    pub created_at: DateTime<Utc>,
    pub published_at: DateTime<Utc>,
    pub assets: Vec<Asset>,
}
