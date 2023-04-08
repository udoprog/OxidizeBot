use super::RequestBuilder;
use anyhow::Error;
use chrono::{DateTime, Utc};
use reqwest::{Client, Method, Url};

const API_URL: &str = "https://api.github.com";

/// API integration.
#[derive(Clone, Debug)]
pub(crate) struct GitHub {
    client: Client,
    api_url: Url,
}

impl GitHub {
    /// Create a new API integration.
    pub(crate) fn new() -> Result<GitHub, Error> {
        Ok(GitHub {
            client: Client::new(),
            api_url: str::parse::<Url>(API_URL)?,
        })
    }

    /// Build request against v3 URL.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.extend(path);
        }

        RequestBuilder::new(self.client.clone(), method, url)
    }

    /// Get all releases for the given repo.
    pub(crate) async fn releases(&self, user: &str, repo: &str) -> Result<Vec<Release>, Error> {
        let req = self.request(Method::GET, &["repos", user, repo, "releases"]);
        req.execute().await
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct Asset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct Release {
    pub(crate) tag_name: String,
    pub(crate) prerelease: bool,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) published_at: DateTime<Utc>,
    pub(crate) assets: Vec<Asset>,
}
