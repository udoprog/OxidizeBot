//! nightbot.tv API helpers.

use crate::{oauth2, prelude::*};
use bytes::Bytes;
use failure::{format_err, Error};
use reqwest::{
    header,
    r#async::{Body, Client, Decoder},
    Method, StatusCode, Url,
};
use std::mem;

static NIGHTBOT_URL_V1: &'static str = "https://api.nightbot.tv/1";

pub enum RequestError {
    TooManyRequests,
    Other(Error),
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

        RequestBuilder {
            token: self.token.clone(),
            client: self.client.clone(),
            url,
            method,
            headers: Vec::new(),
            body: None,
        }
    }

    /// Update the channel information.
    pub async fn channel_send(&self, message: String) -> Result<(), RequestError> {
        let message = Message { message };

        let message = serde_json::to_string(&message).map_err(|e| RequestError::Other(e.into()))?;

        let req = self
            .request(Method::POST, &["channel", "send"])
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(Bytes::from(message.as_bytes())));

        let _ = req.execute::<Status>().await?;
        Ok(())
    }
}

struct RequestBuilder {
    token: oauth2::SyncToken,
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Body>,
}

impl RequestBuilder {
    /// Execute the request.
    pub async fn execute<T>(self) -> Result<T, RequestError>
    where
        T: serde::de::DeserializeOwned,
    {
        let req = {
            let token = self
                .token
                .read()
                .map_err(|e| RequestError::Other(e.into()))?;
            let access_token = token.access_token().to_string();

            let mut req = self.client.request(self.method, self.url);

            if let Some(body) = self.body {
                req = req.body(body);
            }

            for (key, value) in self.headers {
                req = req.header(key, value);
            }

            let req = req.header(header::AUTHORIZATION, format!("Bearer {}", access_token));
            req
        };

        let mut res = req
            .send()
            .compat()
            .await
            .map_err(|e| RequestError::Other(e.into()))?;

        let body = mem::replace(res.body_mut(), Decoder::empty()).compat();

        let body = body
            .try_concat()
            .await
            .map_err(|e| RequestError::Other(e.into()))?;

        let status = res.status();

        if status == StatusCode::UNAUTHORIZED {
            self.token
                .force_refresh()
                .map_err(|e| RequestError::Other(e.into()))?;
        }

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(RequestError::TooManyRequests);
        }

        if !status.is_success() {
            return Err(RequestError::Other(format_err!(
                "bad response: {}: {}",
                status,
                String::from_utf8_lossy(body.as_ref())
            )));
        }

        if log::log_enabled!(log::Level::Trace) {
            let response = String::from_utf8_lossy(body.as_ref());
            log::trace!("response: {}", response);
        }

        serde_json::from_slice(body.as_ref()).map_err(|e| RequestError::Other(e.into()))
    }

    /// Add a body to the request.
    pub fn body(mut self, body: Body) -> Self {
        self.body = Some(body);
        self
    }

    /// Push a header.
    pub fn header(mut self, key: header::HeaderName, value: &str) -> Self {
        self.headers.push((key, value.to_string()));
        self
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
