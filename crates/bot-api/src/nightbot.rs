//! nightbot.tv API helpers.

use anyhow::Result;
use async_injector::{Injector, Provider};
use common::tags;
use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};

use crate::base::RequestBuilder;
use crate::token::Token;

static NIGHTBOT_URL_V1: &str = "https://api.nightbot.tv/1";

/// API integration.
#[derive(Clone, Debug)]
pub struct NightBot {
    user_agent: &'static str,
    client: Client,
    api_url: Url,
    token: Token,
}

impl NightBot {
    /// Create a new API integration.
    pub fn new(user_agent: &'static str, token: Token) -> Result<Self> {
        Ok(Self {
            user_agent,
            client: Client::new(),
            api_url: str::parse(NIGHTBOT_URL_V1)?,
            token,
        })
    }

    /// Run the stream that updates the nightbot client.
    #[tracing::instrument(skip_all)]
    pub async fn run(user_agent: &'static str, injector: Injector) -> Result<()> {
        #[derive(Provider)]
        struct Deps {
            #[dependency(tag = "tags::Token::NightBot")]
            token: Token,
        }

        let mut deps = Deps::provider(&injector).await?;

        loop {
            match deps.wait_for_update().await {
                Some(deps) => {
                    injector.update(NightBot::new(user_agent, deps.token)?).await;
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

        let mut req = RequestBuilder::new(&self.client, self.user_agent, method, url);
        req.token(&self.token);
        req
    }

    /// Update the channel information.
    pub(crate) async fn channel_send(&self, message: String) -> Result<()> {
        let message = Message { message };
        let message = serde_json::to_string(&message)?;

        let mut req = self.request(Method::POST, &["channel", "send"]);

        req.header(header::CONTENT_TYPE, "application/json")
            .body(message.into_bytes());

        let _ = req.execute().await?.json::<Status>()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Message {
    message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Status {
    status: u32,
}
