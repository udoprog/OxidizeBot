//! Twitch API helpers.

use anyhow::bail;
use bytes::Bytes;
use reqwest::{header, Client, Method, Url};

pub(crate) mod github;
pub(crate) mod spotify;
pub(crate) mod twitch;

pub(crate) use self::github::GitHub;
pub(crate) use self::spotify::Spotify;
pub(crate) use self::twitch::IdTwitchClient;

const USER_AGENT: &str = "OxidizeWeb/0";

struct RequestBuilder {
    client: Client,
    method: Method,
    url: Url,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Bytes>,
}

impl RequestBuilder {
    /// Create a new request.
    pub(crate) fn new(client: Client, method: Method, url: Url) -> Self {
        RequestBuilder {
            client,
            method,
            url,
            headers: Vec::new(),
            body: None,
        }
    }

    /// Execute the request.
    #[tracing::instrument(skip_all, fields(method = ?self.method, url = ?self.url.to_string()))]
    pub(crate) async fn execute<T>(self) -> Result<T, anyhow::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut r = self.client.request(self.method, self.url);

        if let Some(body) = self.body {
            r = r.body(body);
        }

        r = r.header(header::USER_AGENT, USER_AGENT);

        for (key, value) in self.headers {
            r = r.header(key, value);
        }

        let res = r.send().await?;
        let status = res.status();
        let body = res.bytes().await?;

        if !status.is_success() {
            bail!(
                "bad response: {}: {}",
                status,
                String::from_utf8_lossy(body.as_ref())
            );
        }

        tracing::trace!("Response: {}", String::from_utf8_lossy(body.as_ref()));
        serde_json::from_slice(body.as_ref()).map_err(Into::into)
    }

    /// Push a header.
    pub(crate) fn header(mut self, key: header::HeaderName, value: &str) -> Self {
        self.headers.push((key, value.to_string()));
        self
    }
}
