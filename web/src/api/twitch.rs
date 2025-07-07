use super::RequestBuilder;
use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};
const ID_TWITCH_URL: &str = "https://id.twitch.tv";

/// Response from the validate token endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ValidateToken {
    pub(crate) client_id: String,
    pub(crate) login: String,
    #[serde(default)]
    pub(crate) scopes: Option<Vec<String>>,
    pub(crate) user_id: String,
}

/// Client for id.twitch.tv
#[derive(Clone, Debug)]
pub(crate) struct IdTwitchClient {
    client: Client,
    api_url: Url,
}

impl IdTwitchClient {
    /// Create a new API integration.
    pub(crate) fn new() -> Result<IdTwitchClient, anyhow::Error> {
        Ok(IdTwitchClient {
            client: Client::new(),
            api_url: str::parse::<Url>(ID_TWITCH_URL)?,
        })
    }

    /// Get request against API.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("bad base").extend(path);
        RequestBuilder::new(self.client.clone(), method, url)
    }

    // Validate the specified token through twitch validation API.
    pub(crate) async fn validate_token(&self, token: &str) -> Result<ValidateToken, anyhow::Error> {
        let request = self
            .request(Method::GET, &["oauth2", "validate"])
            .header(header::AUTHORIZATION, &format!("OAuth {token}"));

        request.execute().await
    }
}
