//! Twitch API helpers.

use crate::prelude::*;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use failure::Error;
use reqwest::{
    header,
    r#async::{Chunk, Client, Decoder},
    Method, Url,
};
use std::mem;

const API_URL: &'static str = "https://api.github.com";

/// API integration.
#[derive(Clone, Debug)]
pub struct GitHub {
    client: Client,
    api_url: Url,
}

impl GitHub {
    /// Create a new API integration.
    pub fn new() -> Result<GitHub, Error> {
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

        RequestBuilder {
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
        }
    }

    /// Get all releases for the given repo.
    pub async fn releases(&self, user: String, repo: String) -> Result<Vec<Release>, Error> {
        let data = self
            .request(
                Method::GET,
                &["repos", user.as_str(), repo.as_str(), "releases"],
            )
            .json()
            .await?;
        Ok(data)
    }
}

struct RequestBuilder {
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Bytes>,
}

impl RequestBuilder {
    /// Execute the request, providing the raw body as a response.
    pub async fn raw(self) -> Result<Chunk, Error> {
        let mut req = self.client.request(self.method, self.url);

        if let Some(body) = self.body {
            req = req.body(body);
        }

        for (key, value) in self.headers {
            req = req.header(key, value);
        }

        req = req.header(header::ACCEPT, "application/json");
        req = req.header(
            header::USER_AGENT,
            concat!("OxidizeBot/", env!("CARGO_PKG_VERSION")),
        );
        let mut res = req.send().compat().await?;

        let status = res.status();

        let body = mem::replace(res.body_mut(), Decoder::empty());
        let body = body.compat().try_concat().await?;

        if !status.is_success() {
            failure::bail!(
                "bad response: {}: {}",
                status,
                String::from_utf8_lossy(&body)
            );
        }

        if log::log_enabled!(log::Level::Trace) {
            let response = String::from_utf8_lossy(body.as_ref());
            log::trace!("response: {}", response);
        }

        Ok(body)
    }

    /// Execute the request expecting a JSON response.
    pub async fn json<T>(self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let body = self.raw().await?;
        serde_json::from_slice(body.as_ref()).map_err(Into::into)
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
