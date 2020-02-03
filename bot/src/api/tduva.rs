//! tduva API Client.

use crate::api::RequestBuilder;
use anyhow::Error;
use reqwest::{header, Client, Method, Url};

const URL: &str = "https://tduva.com";

/// API integration.
#[derive(Clone, Debug)]
pub struct Tduva {
    client: Client,
    url: Url,
}

impl Tduva {
    /// Create a new API integration.
    pub fn new() -> Result<Tduva, Error> {
        Ok(Tduva {
            client: Client::new(),
            url: str::parse::<Url>(URL)?,
        })
    }

    /// Build a new request.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.extend(path);
        }

        let req = RequestBuilder::new(self.client.clone(), method, url);
        req.header(header::ACCEPT, "application/json")
    }

    /// Access resource badges.
    pub async fn res_badges(&self) -> Result<Vec<Badge>, Error> {
        let req = self.request(Method::GET, &["res", "badges"]);

        req.execute().await?.json()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Badge {
    pub id: String,
    pub version: String,
    pub image_url: String,
    pub color: Option<String>,
    pub meta_title: String,
    pub meta_url: Option<String>,
    pub usernames: Vec<String>,
}
