//! Twitch API helpers.

use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{Client, Method, Url};
use serde::{Deserialize, Serialize};

use crate::base::RequestBuilder;

const API_URL: &str = "https://api.github.com";

/// API integration.
#[derive(Clone, Debug)]
pub struct GitHub {
    user_agent: &'static str,
    client: Client,
    api_url: Url,
}

impl GitHub {
    /// Create a new API integration.
    pub fn new(user_agent: &'static str) -> Result<Self> {
        Ok(Self {
            user_agent,
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

        RequestBuilder::new(&self.client, self.user_agent, method, url)
    }

    /// Get all releases for the given repo.
    pub(crate) async fn releases(&self, user: String, repo: String) -> Result<Vec<Release>> {
        let req = self.request(
            Method::GET,
            &["repos", user.as_str(), repo.as_str(), "releases"],
        );

        req.execute().await?.json()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Release {
    pub tag_name: String,
    pub prerelease: bool,
    pub created_at: DateTime<Utc>,
    pub published_at: DateTime<Utc>,
    pub assets: Vec<Asset>,
}
