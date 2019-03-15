//! Twitch API helpers.

use futures::{Future, Stream as _};
use reqwest::{
    header,
    r#async::{Body, Client, Decoder},
    Method, Url,
};
use std::mem;

const ID_TWITCH_URL: &'static str = "https://id.twitch.tv";

/// API integration.
#[derive(Clone, Debug)]
pub struct IdTwitchClient {
    client: Client,
    api_url: Url,
}

impl IdTwitchClient {
    /// Create a new API integration.
    pub fn new() -> Result<IdTwitchClient, failure::Error> {
        Ok(IdTwitchClient {
            client: Client::new(),
            api_url: str::parse::<Url>(ID_TWITCH_URL)?,
        })
    }

    /// Get request against API.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("bad base").extend(path);

        RequestBuilder {
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
        }
    }

    // Validate the specified token through twitch validation API.
    pub fn validate_token(
        &self,
        token: &str,
    ) -> impl Future<Item = ValidateToken, Error = failure::Error> {
        self.request(Method::GET, &["oauth2", "validate"])
            .header(header::AUTHORIZATION, &format!("OAuth {}", token))
            .execute()
    }
}

/// Response from the validate token endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct ValidateToken {
    pub client_id: String,
    pub login: String,
    pub scopes: Vec<String>,
    pub user_id: String,
}

struct RequestBuilder {
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Body>,
}

impl RequestBuilder {
    /// Execute the request.
    pub fn execute<T>(self) -> impl Future<Item = T, Error = failure::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut r = self.client.request(self.method, self.url);

        if let Some(body) = self.body {
            r = r.body(body);
        }

        for (key, value) in self.headers {
            r = r.header(key, value);
        }

        r.send().map_err(Into::into).and_then(|mut res| {
            let body = mem::replace(res.body_mut(), Decoder::empty());

            body.concat2().map_err(Into::into).and_then(move |body| {
                let status = res.status();

                if !status.is_success() {
                    failure::bail!(
                        "bad response: {}: {}",
                        status,
                        String::from_utf8_lossy(body.as_ref())
                    );
                }

                log::trace!("response: {}", String::from_utf8_lossy(body.as_ref()));
                serde_json::from_slice(body.as_ref()).map_err(Into::into)
            })
        })
    }

    /// Push a header.
    pub fn header(mut self, key: header::HeaderName, value: &str) -> Self {
        self.headers.push((key, value.to_string()));
        self
    }
}
