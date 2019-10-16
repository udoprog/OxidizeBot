//! nightbot.tv API helpers.

use crate::{api::base::RequestBuilder, oauth2};
use anyhow::Error;
use reqwest::{header, Client, Method, Url};

static NIGHTBOT_URL_V1: &'static str = "https://api.nightbot.tv/1";

pub enum RequestError {
    TooManyRequests,
    Other(Error),
}

impl From<Error> for RequestError {
    fn from(value: Error) -> Self {
        RequestError::Other(value)
    }
}

/// API integration.
#[derive(Clone, Debug)]
pub struct NightBot {
    client: Client,
    api_url: Url,
    token: oauth2::SyncToken,
}

impl NightBot {
    /// Create a new API integration.
    pub fn new(token: oauth2::SyncToken) -> Result<Self, Error> {
        Ok(NightBot {
            client: Client::new(),
            api_url: str::parse(NIGHTBOT_URL_V1)?,
            token,
        })
    }

    /// Get request against API.
    fn request(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("bad base").extend(path);

        RequestBuilder::new(self.client.clone(), method, url).token(self.token.clone())
    }

    /// Update the channel information.
    pub async fn channel_send(&self, message: String) -> Result<(), RequestError> {
        let message = Message { message };

        let message = serde_json::to_string(&message).map_err(|e| RequestError::Other(e.into()))?;

        let req = self
            .request(Method::POST, &["channel", "send"])
            .header(header::CONTENT_TYPE, "application/json")
            .body(message.as_bytes());

        let _ = req.execute().await?.json::<Status>()?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Message {
    message: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Status {
    status: u32,
}
