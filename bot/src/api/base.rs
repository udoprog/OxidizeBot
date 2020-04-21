use crate::oauth2;
use anyhow::{bail, Error};
use bytes::Bytes;
use reqwest::{header, Client, Method, StatusCode, Url};

pub const USER_AGENT: &str = concat!("OxidizeBot/", version_str!());

/// Trait to deal with optional bodies.
///
/// Fix and replace once we get HRTB's or HRT's :cry:
pub trait BodyHelper {
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
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
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

    /// Send request and only return status.
    pub async fn json_map<T>(
        self,
        m: impl FnOnce(StatusCode, &Bytes) -> Result<Option<T>, Error>,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let Response { status, body, .. } = self.execute().await?;

        if let Some(output) = m(status, &body)? {
            return Ok(output);
        }

        let body = String::from_utf8_lossy(body.as_ref());

        bail!(
            "Bad response: {}: {}: {}: {}",
            self.method,
            self.url,
            status,
            body
        );
    }

    /// Execute the request.
    pub async fn execute(&self) -> Result<Response<'_, Bytes>, Error> {
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

            req = req.header(header::USER_AGENT, USER_AGENT);

            req
        };

        let res = req.send().await?;
        let status = res.status();
        let body = res.bytes().await?;

        if log::log_enabled!(log::Level::Trace) {
            let response = String::from_utf8_lossy(&body);
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

        Ok(Response {
            method: &self.method,
            url: &self.url,
            status,
            body,
        })
    }
}

pub struct Response<'a, B> {
    method: &'a Method,
    url: &'a Url,
    status: StatusCode,
    body: B,
}

impl Response<'_, Bytes> {
    /// Expect a successful response.
    pub fn ok(self) -> Result<(), Error> {
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
    pub fn json<T>(self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
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

impl Response<'_, Option<Bytes>> {
    /// Expect a JSON response of the given type.
    pub fn json<T>(self) -> Result<Option<T>, Error>
    where
        T: serde::de::DeserializeOwned,
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

    /// Access the underlying raw body.
    pub fn body(self) -> Result<Option<Bytes>, Error> {
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

        Ok(Some(body))
    }
}

impl<'a, B> Response<'a, B>
where
    B: BodyHelper,
{
    /// Handle as empty if we encounter the given status code.
    pub fn empty_on_status(self, status: StatusCode) -> Response<'a, Option<B::Value>> {
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
    pub fn not_found(self) -> Response<'a, Option<B::Value>> {
        self.empty_on_status(StatusCode::NOT_FOUND)
    }
}
