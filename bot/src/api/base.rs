use crate::{oauth2, prelude::*};
use bytes::Bytes;
use failure::Error;
use reqwest::{
    header,
    r#async::{Chunk, Client, Decoder},
    Method, StatusCode,
};
use std::mem;
use url::Url;

#[derive(Clone)]
pub struct RequestBuilder {
    token: Option<oauth2::SyncToken>,
    client: Client,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Option<Bytes>,
    /// Use Bearer header instead of OAuth for access tokens.
    use_bearer: bool,
    /// Add the client id to the specified header if configured.
    client_id_header: Option<&'static str>,
}

impl RequestBuilder {
    /// Construct a new request builder.
    pub fn new(client: Client, method: Method, url: Url) -> RequestBuilder {
        RequestBuilder {
            token: None,
            client,
            url,
            method,
            headers: Vec::new(),
            body: None,
            use_bearer: true,
            client_id_header: None,
        }
    }

    /// Use the OAuth2 header instead of Bearer when sending authentication.
    pub fn use_oauth2_header(mut self) -> Self {
        self.use_bearer = false;
        self
    }

    /// Use the specified Client-ID header.
    pub fn client_id_header(mut self, header: &'static str) -> Self {
        self.client_id_header = Some(header);
        self
    }

    /// Set the token to use.
    pub fn token(self, token: oauth2::SyncToken) -> Self {
        Self {
            token: Some(token),
            ..self
        }
    }

    /// Change the body of the request.
    pub fn body(mut self, body: Bytes) -> Self {
        self.body = Some(body);
        self
    }

    /// Push a header.
    pub fn header(mut self, key: header::HeaderName, value: &str) -> Self {
        self.headers.push((key, value.to_string()));
        self
    }

    /// Add a query parameter.
    pub fn query_param(mut self, key: &str, value: &str) -> Self {
        self.url.query_pairs_mut().append_pair(key, value);
        self
    }

    /// Add a query parameter.
    pub fn optional_query_param(mut self, key: &str, value: Option<String>) -> Self {
        if let Some(value) = value {
            self.url.query_pairs_mut().append_pair(key, value.as_str());
        }

        self
    }

    /// Send request and expect a JSON response.
    pub async fn json<T>(self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let (status, body) = self.execute().await?;

        if !status.is_success() {
            let body = String::from_utf8_lossy(body.as_ref());
            failure::bail!(
                "Bad response: {}: {}: {}: {}",
                self.method,
                self.url,
                status,
                body
            );
        }

        match serde_json::from_slice(body.as_ref()) {
            Ok(body) => Ok(body),
            Err(e) => {
                let body = String::from_utf8_lossy(body.as_ref());
                failure::bail!(
                    "Bad response: {}: {}: {}: {}: {}",
                    self.method,
                    self.url,
                    status,
                    e,
                    body
                );
            }
        }
    }

    /// Send request and only return status.
    pub async fn json_map<T>(
        self,
        m: impl FnOnce(&StatusCode, &Chunk) -> Result<Option<T>, Error>,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let (status, body) = self.execute().await?;

        if let Some(output) = m(&status, &body)? {
            return Ok(output);
        }

        let body = String::from_utf8_lossy(body.as_ref());

        failure::bail!(
            "Bad response: {}: {}: {}: {}",
            self.method,
            self.url,
            status,
            body
        );
    }

    /// Send request and expect a JSON response.
    pub async fn json_or<T>(self) -> Result<Option<T>, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        return self
            .json_option(|status| match *status {
                StatusCode::NO_CONTENT => true,
                _ => false,
            })
            .await;
    }

    /// Send request and expect a JSON response.
    pub async fn json_option<T>(
        self,
        condition: impl FnOnce(&StatusCode) -> bool,
    ) -> Result<Option<T>, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let (status, body) = self.execute().await?;

        if !status.is_success() {
            let body = String::from_utf8_lossy(body.as_ref());
            failure::bail!(
                "Bad response: {}: {}: {}: {}",
                self.method,
                self.url,
                status,
                body
            );
        }

        if condition(&status) {
            return Ok(None);
        }

        match serde_json::from_slice(body.as_ref()) {
            Ok(body) => Ok(body),
            Err(e) => {
                let body = String::from_utf8_lossy(body.as_ref());
                failure::bail!(
                    "Bad response: {}: {}: {}: {}: {}",
                    self.method,
                    self.url,
                    status,
                    e,
                    body
                );
            }
        }
    }

    /// Execute the request.
    async fn execute(&self) -> Result<(StatusCode, Chunk), Error> {
        // NB: scope to only lock the token over the request setup.
        let req = {
            log::trace!("Request: {}: {}", self.method, self.url);
            let mut req = self.client.request(self.method.clone(), self.url.clone());

            if let Some(body) = self.body.as_ref() {
                req = req.body(body.clone());
            }

            for (key, value) in &self.headers {
                req = req.header(key.clone(), value.clone());
            }

            if let Some(token) = self.token.as_ref() {
                let token = token.read()?;
                let access_token = token.access_token().to_string();

                if self.use_bearer {
                    req = req.header(header::AUTHORIZATION, format!("Bearer {}", access_token));
                } else {
                    req = req.header(header::AUTHORIZATION, format!("OAuth {}", access_token));
                }

                if let Some(client_id_header) = self.client_id_header {
                    req = req.header(client_id_header, token.client_id())
                }
            }

            req = req.header(
                header::USER_AGENT,
                concat!("setmod/", env!("CARGO_PKG_VERSION")),
            );

            req
        };

        let mut res = req.send().compat().await?;

        let body = mem::replace(res.body_mut(), Decoder::empty());
        let body = body.compat().try_concat().await?;

        let status = res.status();

        if log::log_enabled!(log::Level::Trace) {
            let response = String::from_utf8_lossy(body.as_ref());
            log::trace!(
                "Response: {}: {}: {}: {}",
                self.method,
                self.url,
                status,
                response
            );
        }

        if let Some(token) = self.token.as_ref() {
            if status == StatusCode::UNAUTHORIZED {
                token.force_refresh()?;
            }
        }

        Ok((status, body))
    }
}
