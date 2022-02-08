//! nightbot.tv API helpers.

use crate::api::base::RequestBuilder;
use crate::injector::{Injector, Provider};
use crate::oauth2;
use crate::tags;
use anyhow::{Error, Result};
use reqwest::{header, Client, Method, Url};

static NIGHTBOT_URL_V1: &str = "https://api.nightbot.tv/1";

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
    pub fn new(token: oauth2::SyncToken) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            api_url: str::parse(NIGHTBOT_URL_V1)?,
            token,
        })
    }

    /// Run the stream that updates the nightbot client.
    pub async fn run(injector: Injector) -> Result<()> {
        #[derive(Provider)]
        struct Deps {
            #[dependency(tag = "tags::Token::NightBot")]
            token: oauth2::SyncToken,
        }

        let mut deps = Deps::provider(&injector).await?;

        loop {
            match deps.wait_for_update().await {
                Some(deps) => {
                    injector.update(NightBot::new(deps.token)?).await;
                }
                None => {
                    injector.clear::<NightBot>().await;
                }
            }
        }
    }

    /// Get request against API.
    fn request<'a>(&'a self, method: Method, path: &[&str]) -> RequestBuilder<'a> {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("bad base").extend(path);

        let mut req = RequestBuilder::new(&self.client, method, url);
        req.token(&self.token);
        req
    }

    /// Update the channel information.
    pub async fn channel_send(&self, message: String) -> Result<(), RequestError> {
        let message = Message { message };

        let message = serde_json::to_string(&message).map_err(|e| RequestError::Other(e.into()))?;

        let mut req = self.request(Method::POST, &["channel", "send"]);

        req.header(header::CONTENT_TYPE, "application/json")
            .body(message.into_bytes());

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
