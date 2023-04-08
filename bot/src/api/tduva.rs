//! tduva API Client.

use crate::api::RequestBuilder;
use anyhow::Result;
use reqwest::{header, Client, Method, Url};

const URL: &str = "https://tduva.com";

/// API integration.
#[derive(Clone, Debug)]
pub(crate) struct Tduva {
    client: Client,
    url: Url,
}

impl Tduva {
    /// Create a new API integration.
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            url: str::parse::<Url>(URL)?,
        })
    }

    /// Build a new request.
    fn request<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, method, url);
        req.header(header::ACCEPT, "application/json");
        req
    }

    /// Access resource badges.
    pub(crate) async fn res_badges(&self) -> Result<Vec<Badge>> {
        let req = self.request(Method::GET, &["res", "badges"]);

        req.execute().await?.json()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Badge {
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) image_url: String,
    pub(crate) color: Option<String>,
    pub(crate) meta_title: String,
    pub(crate) meta_url: Option<String>,
    pub(crate) usernames: Vec<String>,
}
