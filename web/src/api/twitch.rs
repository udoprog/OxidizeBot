use super::RequestBuilder;
use reqwest::{header, r#async::Client, Method, Url};
use serde::{Deserialize, Serialize};
const ID_TWITCH_URL: &'static str = "https://id.twitch.tv";

/// Response from the validate token endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateToken {
    pub client_id: String,
    pub login: String,
    pub scopes: Vec<String>,
    pub user_id: String,
}

/// Client for id.twitch.tv
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
        RequestBuilder::new(self.client.clone(), method, url)
    }

    // Validate the specified token through twitch validation API.
    pub async fn validate_token(&self, token: &str) -> Result<ValidateToken, failure::Error> {
        let request = self
            .request(Method::GET, &["oauth2", "validate"])
            .header(header::AUTHORIZATION, &format!("OAuth {}", token));

        request.execute().await
    }
}
