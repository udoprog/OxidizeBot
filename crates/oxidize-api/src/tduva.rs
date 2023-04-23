//! tduva API Client.

use anyhow::Result;
use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};

use crate::base::RequestBuilder;

const URL: &str = "https://tduva.com";

/// API integration.
#[derive(Clone, Debug)]
pub struct Tduva {
    user_agent: &'static str,
    client: Client,
    url: Url,
}

impl Tduva {
    /// Create a new API integration.
    pub fn new(user_agent: &'static str) -> Result<Self> {
        Ok(Self {
            user_agent,
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

        let mut req = RequestBuilder::new(&self.client, self.user_agent, method, url);
        req.header(header::ACCEPT, "application/json");
        req
    }

    /// Access resource badges.
    pub async fn res_badges(&self) -> Result<Vec<Badge>> {
        let req = self.request(Method::GET, &["res", "badges"]);
        req.execute().await?.json()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Badge {
    pub id: String,
    pub version: String,
    pub image_url: String,
    pub color: Option<String>,
    pub meta_title: String,
    pub meta_url: Option<String>,
    pub usernames: Vec<String>,
}
