use anyhow::{bail, Result};
use bytes::Bytes;
use reqwest::{header, Client, Method, StatusCode, Url};
use serde::de;
use thiserror::Error;

use crate::token::Token;

#[derive(Debug, Error)]
#[error("error when sending request")]
struct SendRequestError(#[source] reqwest::Error);

#[derive(Debug, Error)]
#[error("error when receiving response")]
struct ReceiveResponseError(#[source] reqwest::Error);

/// Trait to deal with optional bodies.
///
/// Fix and replace once we get HRTB's or HRT's :cry:
pub(crate) trait BodyHelper {
    type Value;

    /// Get a present body as an option.
    fn some(self) -> Option<Self::Value>;
}

impl BodyHelper for Bytes {
    type Value = Bytes;

    fn some(self) -> Option<Self::Value> {
        Some(self)
    }
}

impl BodyHelper for Option<Bytes> {
    type Value = Bytes;

    fn some(self) -> Option<Self::Value> {
        self
    }
}

#[derive(Clone)]
pub(crate) struct RequestBuilder<'a> {
    token: Option<&'a Token>,
    client: &'a Client,
    user_agent: &'static str,
    url: Url,
    method: Method,
    headers: Vec<(header::HeaderName, String)>,
    body: Bytes,
    /// Use Bearer header instead of OAuth for access tokens.
    use_bearer: bool,
    /// Add the client id to the specified header if configured.
    client_id_header: Option<&'a header::HeaderName>,
    empty_body: bool,
}

impl<'a> RequestBuilder<'a> {
    /// Construct a new request builder.
    pub(crate) fn new(
        client: &'a Client,
        user_agent: &'static str,
        method: Method,
        url: Url,
    ) -> Self {
        Self {
            token: None,
            client,
            user_agent,
            url,
            method,
            headers: Vec::new(),
            body: Bytes::new(),
            use_bearer: true,
            client_id_header: None,
            empty_body: false,
        }
    }

    /// Use the OAuth2 header instead of Bearer when sending authentication.
    pub(crate) fn use_oauth2_header(&mut self) -> &mut Self {
        self.use_bearer = false;
        self
    }

    /// Configure if empty bodies should have a Content-Type or not.
    pub(crate) fn empty_body(&mut self) -> &mut Self {
        self.empty_body = true;
        self
    }

    /// Use the specified Client-ID header.
    pub(crate) fn client_id_header(&mut self, header: &'a header::HeaderName) -> &mut Self {
        self.client_id_header = Some(header);
        self
    }

    /// Set the token to use.
    pub(crate) fn token(&mut self, token: &'a Token) -> &mut Self {
        self.token = Some(token);
        self
    }

    /// Change the body of the request.
    pub(crate) fn body(&mut self, body: impl Into<Bytes>) -> &mut Self {
        self.body = body.into();
        self
    }

    /// Push a header.
    pub(crate) fn header(&mut self, key: header::HeaderName, value: &str) -> &mut Self {
        self.headers.push((key, value.to_owned()));
        self
    }

    /// Add a query parameter.
    pub(crate) fn query_param<S>(&mut self, key: &str, value: S) -> &mut Self
    where
        S: AsRef<str>,
    {
        self.url.query_pairs_mut().append_pair(key, value.as_ref());
        self
    }

    /// Send request and only return status.
    pub(crate) async fn json_map<T>(
        &self,
        m: impl FnOnce(StatusCode, &Bytes) -> Result<Option<T>>,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let Response {
            method,
            url,
            status,
            body,
            ..
        } = self.execute().await?;

        if let Some(output) = m(status, &body)? {
            return Ok(output);
        }

        let body = String::from_utf8_lossy(body.as_ref());

        bail!("Bad response: {}: {}: {}: {}", method, url, status, body);
    }

    /// Execute the request.
    #[tracing::instrument(skip_all, fields(method = ?self.method, url = ?self.url.to_string()))]
    pub(crate) async fn execute(&self) -> Result<Response<Bytes>> {
        // NB: scope to only lock the token over the request setup.
        tracing::trace!("Executing");

        let mut req = self.client.request(self.method.clone(), self.url.clone());

        req = match self.method {
            Method::GET => req,
            Method::HEAD => req,
            _ => {
                if self.body.is_empty() && self.empty_body {
                    req
                } else {
                    req.header(header::CONTENT_LENGTH, self.body.len())
                        .body(self.body.clone())
                }
            }
        };

        for (key, value) in &self.headers {
            req = req.header(key.clone(), value);
        }

        if let Some((token, client_id)) = self.token.and_then(|t| t.read()) {
            if self.use_bearer {
                req = req.header(header::AUTHORIZATION, format!("Bearer {}", token.as_str()));
            } else {
                req = req.header(header::AUTHORIZATION, format!("OAuth {}", token.as_str()));
            }

            if let Some(client_id_header) = &self.client_id_header {
                req = req.header(&**client_id_header, client_id.as_ref());
            }
        }

        req = req.header(header::USER_AGENT, self.user_agent);

        let res = req.send().await.map_err(SendRequestError)?;
        let status = res.status();
        let body = res.bytes().await.map_err(ReceiveResponseError)?;

        if tracing::enabled!(tracing::Level::TRACE) {
            let response = String::from_utf8_lossy(&body);
            tracing::trace!(?status, "Response: {response}");
        }

        if let Some(token) = self.token {
            if status == StatusCode::UNAUTHORIZED {
                token.force_refresh();
            }
        }

        Ok(Response {
            method: self.method.clone(),
            url: self.url.clone(),
            status,
            body,
        })
    }
}

pub(crate) struct Response<B> {
    method: Method,
    url: Url,
    status: StatusCode,
    body: B,
}

impl Response<Bytes> {
    /// Expect a successful response.
    pub(crate) fn ok(self) -> Result<()> {
        if self.status.is_success() {
            return Ok(());
        }

        let body = String::from_utf8_lossy(self.body.as_ref());

        bail!(
            "Bad response: {}: {}: {}: {}",
            self.method,
            self.url,
            self.status,
            body
        );
    }

    /// Expect a JSON response of the given type.
    pub(crate) fn json<T>(self) -> Result<T>
    where
        T: de::DeserializeOwned,
    {
        if !self.status.is_success() {
            let body = String::from_utf8_lossy(self.body.as_ref());
            bail!(
                "Bad response: {}: {}: {}: {}",
                self.method,
                self.url,
                self.status,
                body
            );
        }

        match serde_json::from_slice(self.body.as_ref()) {
            Ok(body) => Ok(body),
            Err(e) => {
                let body = String::from_utf8_lossy(self.body.as_ref());
                bail!(
                    "Bad response: {}: {}: {}: {}: {}",
                    self.method,
                    self.url,
                    self.status,
                    e,
                    body
                );
            }
        }
    }
}

impl<B> Response<Option<B>>
where
    B: AsRef<[u8]>,
{
    /// Expect a JSON response of the given type.
    pub(crate) fn json<T>(self) -> Result<Option<T>>
    where
        T: de::DeserializeOwned,
    {
        let body = match self.body {
            Some(body) => body,
            None => return Ok(None),
        };

        if !self.status.is_success() {
            let body = String::from_utf8_lossy(body.as_ref());
            bail!(
                "Bad response: {}: {}: {}: {}",
                self.method,
                self.url,
                self.status,
                body
            );
        }

        match serde_json::from_slice(body.as_ref()) {
            Ok(body) => Ok(Some(body)),
            Err(e) => {
                let body = String::from_utf8_lossy(body.as_ref());
                bail!(
                    "Bad response: {}: {}: {}: {}: {}",
                    self.method,
                    self.url,
                    self.status,
                    e,
                    body
                );
            }
        }
    }
}

impl<B> Response<B>
where
    B: BodyHelper,
{
    /// Handle as empty if we encounter the given status code.
    pub(crate) fn empty_on_status(self, status: StatusCode) -> Response<Option<B::Value>> {
        let body = if self.status == status {
            None
        } else {
            self.body.some()
        };

        Response {
            method: self.method,
            url: self.url,
            status: self.status,
            body,
        }
    }

    /// Test if the underlying status is not found.
    pub(crate) fn not_found(self) -> Response<Option<B::Value>> {
        self.empty_on_status(StatusCode::NOT_FOUND)
    }
}
