use std::collections::HashMap;
use std::error::Error as _;
use std::fmt;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time;

use ::oauth2::State;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use hyper::body::Body;
use hyper::header;
use hyper::server::conn::AddrStream;
use hyper::server::Server;
use hyper::service;
use hyper::{Request, Response, StatusCode};
use parking_lot::Mutex;
use relative_path::RelativePathBuf;
use rust_embed::EmbeddedFile;
use serde::{de, Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;
use url::Url;

use crate::api;
use crate::db;
use crate::oauth2;
use crate::session;
use crate::stream::StreamExt as _;

macro_rules! log_error {
    ($e:expr, $fmt:expr $(, $($tt:tt)*)?) => {{
        tracing::error!($fmt $(, $($tt)*)*);
        tracing::error!("caused by: {}", $e);

        let mut last = $e.source();

        while let Some(e) = last {
            tracing::error!("caused by: {}", e);
            last = e.source();
        }
    }}
}

static SPOTIFY_TRACK_URL: &str = "https://open.spotify.com/track";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub verify_connection: bool,
    pub database: RelativePathBuf,
    pub base_url: Url,
    #[serde(default)]
    pub session: session::Config,
    pub oauth2: oauth2::Config,
}

mod assets {
    #[derive(rust_embed::RustEmbed)]
    #[folder = "$CARGO_MANIFEST_DIR/../web-ui/dist"]
    pub struct Asset;
}

pub fn setup(
    db: db::Database,
    host: String,
    port: u32,
    config: Config,
) -> Result<impl Future<Output = Result<(), hyper::Error>>, anyhow::Error> {
    let fallback =
        assets::Asset::get("index.html").ok_or_else(|| anyhow!("missing index.html in assets"))?;

    let pending_tokens = Arc::new(Mutex::new(HashMap::new()));

    let handler = Arc::new(Handler::new(fallback, db, config, pending_tokens.clone())?);

    let bind = format!("{}:{}", host, port);
    tracing::info!("Listening on: http://{}", bind);

    let addr: SocketAddr = str::parse(&bind)?;

    // TODO: add graceful shutdown.
    let mut server_future =
        Server::bind(&addr).serve(service::make_service_fn(move |s: &AddrStream| {
            let handler = handler.clone();
            let address = s.remote_addr();
            let service = service::service_fn(move |req| handler.clone().call(address, req));
            async move { Ok::<_, hyper::Error>(service) }
        }));

    let future = async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(30));
        let expires = chrono::Duration::minutes(5);

        #[allow(clippy::unnecessary_mut_passed)]
        loop {
            tokio::select! {
                result = &mut server_future => {
                    return result;
                }
                _ = interval.tick() => {
                    let now = Utc::now();
                    let mut tokens = pending_tokens.lock();
                    let mut to_remove = smallvec::SmallVec::<[State; 16]>::new();

                    for (key, pending) in &*tokens {
                        if now > pending.created_at + expires {
                            to_remove.push(*key);
                        }
                    }

                    if !to_remove.is_empty() {
                        tracing::info!("Removing {} expired tokens", to_remove.len());
                    }

                    for remove in to_remove {
                        tokens.remove(&remove);
                    }
                }
            }
        }
    };

    Ok(future)
}

#[derive(Debug, Error)]
pub enum Error {
    /// Client performed a bad request.
    #[error("bad request")]
    BadRequest(String),
    /// The resource could not be found.
    #[error("not found")]
    NotFound,
    /// User unauthorized to perform the given request.
    #[error("unauthorized")]
    Unauthorized,
    /// Generic error.
    #[error("other error")]
    Error(#[source] anyhow::Error),
}

impl Error {
    /// Construct a new bad request error.
    pub fn bad_request(s: impl AsRef<str>) -> Self {
        Self::BadRequest(s.as_ref().to_string())
    }
}

impl From<serde_cbor::error::Error> for Error {
    fn from(value: serde_cbor::error::Error) -> Error {
        Error::Error(value.into())
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Error {
        Error::Error(value)
    }
}
#[derive(Serialize)]
struct Data<T> {
    data: Option<T>,
}

impl Data<()> {
    fn empty() -> Data<()> {
        Data { data: None }
    }
}

impl<T> From<T> for Data<T> {
    fn from(data: T) -> Self {
        Self { data: Some(data) }
    }
}

#[derive(Debug)]
pub struct RegisterOrLogin {
    return_to: Option<Url>,
}

#[derive(Debug)]
pub struct Connect {
    user_id: String,
    id: String,
    return_to: Option<Url>,
}

#[derive(Debug)]
pub enum Action {
    /// Register or login an existing user.
    RegisterOrLogin(RegisterOrLogin),
    /// Create a connection.
    Connect(Connect),
}

#[derive(Debug)]
pub struct PendingToken {
    /// When the pending request was created.
    pub created_at: DateTime<Utc>,
    /// The flow for the pending token.
    pub flow: Arc<oauth2::Flow>,
    /// The exchange token used.
    pub exchange_token: oauth2::ExchangeToken,
    /// The action to take when the pending token resolved.
    pub action: Action,
}

pub struct Handler {
    db: db::Database,
    config: Config,
    session: session::Session,
    fallback: EmbeddedFile,
    id_twitch_client: api::IdTwitchClient,
    spotify: api::Spotify,
    login_flow: Arc<oauth2::Flow>,
    flows: HashMap<String, Arc<oauth2::Flow>>,
    pending_tokens: Arc<Mutex<HashMap<State, PendingToken>>>,
    random: ring::rand::SystemRandom,
    week: chrono::Duration,
}

impl Handler {
    /// Construct a new server.
    pub fn new(
        fallback: EmbeddedFile,
        db: db::Database,
        config: Config,
        pending_tokens: Arc<Mutex<HashMap<State, PendingToken>>>,
    ) -> Result<Self, anyhow::Error> {
        let (login_flow, flows) = oauth2::setup_flows(&config.base_url, &config.oauth2)?;
        let session = session::Session::new(db.clone(), &config.session)?;

        Ok(Self {
            db,
            config,
            session,
            fallback,
            id_twitch_client: api::IdTwitchClient::new()?,
            spotify: api::Spotify::new()?,
            login_flow,
            flows,
            pending_tokens,
            random: ring::rand::SystemRandom::new(),
            week: chrono::Duration::days(7),
        })
    }

    /// Try to access static asset.
    fn static_asset(&self, path: &str) -> Response<Body> {
        let path = path.trim_start_matches('/');

        let now = Utc::now();

        let mut r = Response::new(Body::empty());

        let (mime, asset) = match assets::Asset::get(path) {
            Some(asset) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();

                if let Ok(cache_control) = "public, max-age=604800".parse() {
                    r.headers_mut().insert(header::CACHE_CONTROL, cache_control);
                }

                if let Ok(expires) = (now + self.week).to_rfc2822().parse() {
                    r.headers_mut().insert(header::EXPIRES, expires);
                }

                (mime, asset.data)
            }
            None => (mime::TEXT_HTML_UTF_8, self.fallback.data.clone()),
        };

        *r.body_mut() = Body::from(asset);

        if let Ok(mime) = mime.as_ref().parse() {
            r.headers_mut().insert(header::CONTENT_TYPE, mime);
        }

        r
    }

    async fn call(
        self: Arc<Self>,
        remote_address: SocketAddr,
        mut req: Request<Body>,
    ) -> Result<Response<Body>, anyhow::Error> {
        use hyper::{body::HttpBody, Method};

        let path = req
            .uri()
            .path()
            .split('/')
            .skip(1)
            .collect::<SmallVec<[_; 8]>>();

        let request_size = HttpBody::size_hint(req.body()).upper();

        let result = match (req.method(), &*path) {
            (m, ["api", rest @ ..]) => match (m, rest) {
                (&Method::GET, &["auth", "redirect"]) => self.auth_redirect(&req).await,
                (&Method::POST, &["auth", "login"]) => self.auth_login(&req).await,
                (&Method::POST, &["auth", "logout"]) => self.auth_logout(&req).await,
                (&Method::GET, &["auth", "current"]) => self.get_auth_current(&req).await,
                (&Method::GET, &["connection-types"]) => self.get_connection_types(&req).await,
                (&Method::GET, &["connections"]) => self.list_connections(&req).await,
                (&Method::GET, &["connections", id]) => self.get_connection(&req, id).await,
                (&Method::POST, &["connections", id, "refresh"]) => {
                    self.refresh_connection(&req, id).await
                }
                (&Method::DELETE, &["connections", id]) => self.delete_connection(&req, id).await,
                (&Method::POST, &["connections", id]) => self.create_connection(&req, id).await,
                (&Method::POST, &["key"]) => self.create_key(&req).await,
                (&Method::DELETE, &["key"]) => self.delete_key(&req).await,
                (&Method::GET, &["key"]) => self.get_key(&req).await,
                (&Method::GET, &["players"]) => self.list_players().await,
                (&Method::GET, &["player", id]) => self.get_player(id).await,
                (&Method::POST, &["player"]) => {
                    drop(path);
                    self.update_player(&mut req).await
                }
                (&Method::GET, &["github-releases", user, repo]) => {
                    self.get_github_releases(user, repo).await
                }
                _ => Err(Error::NotFound),
            },
            (&Method::GET, _) => return Ok(self.static_asset(req.uri().path())),
            _ => Err(Error::NotFound),
        };

        let response = match result {
            Ok(mut response) => {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
                response
            }
            Err(e) => {
                let result = match e {
                    Error::BadRequest(message) => http_error(StatusCode::BAD_REQUEST, &message),
                    Error::NotFound => http_error(StatusCode::NOT_FOUND, "Not Found"),
                    Error::Unauthorized => http_error(StatusCode::UNAUTHORIZED, "Unauthorized"),
                    Error::Error(e) => {
                        log_error!(e, "internal server error");
                        http_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
                    }
                };

                match result {
                    Ok(result) => result,
                    Err(Error::Error(e)) => {
                        log_error!(e, "failed to build error response");
                        let mut r = Response::new(Body::from("Internal Server Error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        r
                    }
                    Err(e) => {
                        tracing::error!("failed to build response: {}", e);
                        let mut r = Response::new(Body::from("Internal Server Error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        r
                    }
                }
            }
        };

        let response_size = HttpBody::size_hint(response.body()).upper();

        let request_size = request_size
            .map(ContentLengthFmt::Some)
            .unwrap_or(ContentLengthFmt::None);

        let response_size = response_size
            .map(ContentLengthFmt::Some)
            .unwrap_or(ContentLengthFmt::None);

        let user_agent = match req.headers().get(header::USER_AGENT) {
            Some(header) => header.to_str().unwrap_or("?"),
            None => "?",
        };

        let x_real_ip = req.headers().get("X-Real-IP").and_then(|h| h.to_str().ok());

        let x_forwarded_for = req
            .headers()
            .get("X-Forwarded-For")
            .and_then(|h| h.to_str().ok());

        let remote_address = RemoteAddressFmt {
            remote_address,
            x_real_ip,
            x_forwarded_for,
        };

        tracing::info!(
            target: "request",
            "{remote_address} {method} {uri} (User Agent: {user_agent}) ({request_size}) => {status} ({response_size})",
            remote_address = remote_address,
            method = req.method(),
            uri = req.uri(),
            user_agent = user_agent,
            status = response.status(),
            request_size = request_size,
            response_size = response_size,
        );

        Ok(response)
    }
}

#[derive(Clone, Copy, Debug)]
enum ContentLengthFmt {
    Some(u64),
    None,
}

impl fmt::Display for ContentLengthFmt {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::None => "?".fmt(fmt),
            Self::Some(len) => write!(fmt, "{} bytes", len),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RemoteAddressFmt<'a> {
    remote_address: SocketAddr,
    x_real_ip: Option<&'a str>,
    x_forwarded_for: Option<&'a str>,
}

impl fmt::Display for RemoteAddressFmt<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.remote_address)?;

        if let Some(x_real_ip) = self.x_real_ip {
            write!(fmt, " (X-Real-IP: {})", x_real_ip)?;
        }

        if let Some(x_forwarded_for) = self.x_forwarded_for {
            write!(fmt, " (X-Forwarded-For: {})", x_forwarded_for)?;
        }

        Ok(())
    }
}

#[derive(Serialize)]
pub struct Connection<'a> {
    #[serde(rename = "type")]
    ty: oauth2::FlowType,
    id: &'a str,
    title: &'a str,
    description: &'a str,
    #[serde(skip_serializing_if = "db::meta_is_null")]
    meta: &'a serde_cbor::Value,
    hash: String,
    outdated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<oauth2::ExportedToken<'a>>,
}

/// Only contains metadata for a specific connection.
#[derive(Serialize)]
pub struct ConnectionMeta<'a> {
    #[serde(rename = "type")]
    ty: oauth2::FlowType,
    id: &'a str,
    title: &'a str,
    description: &'a str,
}

impl Handler {
    const MAX_BYTES: usize = 10_000;

    /// Get the token with the given ID.
    async fn get_connection(&self, req: &Request<Body>, id: &str) -> Result<Response<Body>, Error> {
        let user = self.verify(req)?;

        let c = self.db.get_connection(&user.user_id, id)?;
        let query = self.decode_query::<Query>(req)?;

        match c {
            Some(c) => {
                let flow = match self.flows.get(&c.id) {
                    Some(flow) => flow,
                    None => return json_ok(Data::empty()),
                };

                let outdated = !flow.is_compatible_with(&c.token);

                match query.format.as_deref() {
                    Some("meta") => {
                        return json_ok(Data::from(Connection {
                            ty: flow.config.ty,
                            id: &c.id,
                            title: &flow.config.title,
                            description: &flow.config.description,
                            meta: &c.meta,
                            hash: c.token.hash()?,
                            outdated,
                            token: None,
                        }));
                    }
                    _ => {
                        return json_ok(Data::from(Connection {
                            ty: flow.config.ty,
                            id: &c.id,
                            title: &flow.config.title,
                            description: &flow.config.description,
                            meta: &c.meta,
                            hash: c.token.hash()?,
                            outdated,
                            token: Some(c.token.as_exported()),
                        }));
                    }
                }
            }
            _ => return json_ok(Data::empty()),
        }

        #[derive(Deserialize)]
        pub struct Query {
            format: Option<String>,
        }
    }

    /// Get the token with the given ID.
    async fn refresh_connection(
        &self,
        req: &Request<Body>,
        id: &str,
    ) -> Result<Response<Body>, Error> {
        let user = self.verify(req)?;

        let c = self.db.get_connection(&user.user_id, id)?;

        let c = match c {
            Some(c) => c,
            None => return json_ok(Data::empty()),
        };

        let flow = match self.flows.get(&c.token.flow_id) {
            Some(flow) => flow,
            None => return json_ok(Data::empty()),
        };

        let token = flow.refresh_token(&c.token.refresh_token).await;

        let token = match token {
            Ok(token) => token,
            Err(e) => match e.status() {
                Some(status) if status.is_client_error() => {
                    self.db.delete_connection(&user.user_id, id)?;

                    log_error!(e, "failed to refresh token");

                    return http_error(
                        StatusCode::BAD_REQUEST,
                        "Token refresh failed, server responded with client error",
                    );
                }
                _ => return Err(Error::Error(e.into())),
            },
        };

        let meta = self.token_meta(flow, &token).await?;

        self.db.add_connection(
            &user.user_id,
            &db::Connection {
                id: id.to_string(),
                meta,
                token: token.clone(),
            },
        )?;

        let outdated = !flow.is_compatible_with(&token);

        let connection = Connection {
            ty: flow.config.ty,
            id: &c.id,
            title: &flow.config.title,
            description: &flow.config.description,
            meta: &c.meta,
            hash: token.hash()?,
            outdated,
            token: Some(token.as_exported()),
        };

        json_ok(Data::from(connection))
    }

    /// Get a list of connection types.
    async fn get_connection_types(&self, _: &Request<Body>) -> Result<Response<Body>, Error> {
        let mut out = Vec::new();

        for client_config in &self.config.oauth2.flows {
            out.push(ConnectionMeta {
                ty: client_config.ty,
                id: &client_config.id,
                title: &client_config.title,
                description: &client_config.description,
            });
        }

        json_ok(&out)
    }

    /// List connections for the current user.
    async fn list_connections(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let user = self.verify(req)?;
        let connections = self.db.connections_by_user(&user.user_id)?;

        let query = self.decode_query::<Query>(req)?;
        let meta = matches!(query.format.as_deref(), Some("meta"));

        let mut out = Vec::new();

        for c in &connections {
            let flow = match self.flows.get(&c.id) {
                Some(flow) => flow,
                None => continue,
            };

            let outdated = !flow.is_compatible_with(&c.token);

            out.push(Connection {
                ty: flow.config.ty,
                id: &c.id,
                title: &flow.config.title,
                description: &flow.config.description,
                meta: &c.meta,
                hash: c.token.hash()?,
                outdated,
                token: if meta {
                    None
                } else {
                    Some(c.token.as_exported())
                },
            });
        }

        return json_ok(&out);

        #[derive(Deserialize)]
        pub struct Query {
            format: Option<String>,
        }
    }

    /// Delete the specified connection.
    async fn delete_connection(
        &self,
        req: &Request<Body>,
        id: &str,
    ) -> Result<Response<Body>, Error> {
        let user = self.verify(req)?;

        self.db.delete_connection(&user.user_id, id)?;
        json_empty()
    }

    /// List connections for the current user.
    async fn create_connection(
        &self,
        req: &Request<Body>,
        id: &str,
    ) -> Result<Response<Body>, Error> {
        let user = self.verify(req)?;

        let flow = match self.flows.get(id).cloned() {
            Some(flow) => flow,
            None => {
                return Err(Error::bad_request(format!(
                    "unsupported connection: {}",
                    id
                )));
            }
        };

        let exchange_token = flow.exchange_token();

        let r = json_ok(&Login {
            auth_url: &exchange_token.auth_url,
        })?;

        self.pending_tokens.lock().insert(
            exchange_token.state,
            PendingToken {
                created_at: Utc::now(),
                flow,
                exchange_token,
                action: Action::Connect(Connect {
                    user_id: user.user_id,
                    id: String::from(id),
                    return_to: referer(req)?,
                }),
            },
        );

        return Ok(r);

        #[derive(Serialize)]
        pub struct Login<'a> {
            auth_url: &'a Url,
        }
    }

    /// Generate a new key.
    async fn create_key(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        use base64::prelude::*;

        let user = self.verify(req)?;

        use ring::rand::SecureRandom as _;

        let mut buf = [0u8; 32];
        self.random
            .fill(&mut buf)
            .map_err(|_| anyhow!("failed to generate random key"))?;
        let key = BASE64_STANDARD.encode(buf);
        self.db.insert_key(&user.user_id, &key)?;

        return json_ok(&KeyInfo { key });

        #[derive(Serialize)]
        struct KeyInfo {
            key: String,
        }
    }

    /// Delete the current key.
    async fn delete_key(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let user = self.verify(req)?;
        self.db.delete_key(&user.user_id)?;
        json_empty()
    }

    /// Get the current key.
    async fn get_key(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let user = self.verify(req)?;
        let key = self.db.get_key(&user.user_id)?;
        return json_ok(&KeyInfo { key });

        #[derive(Serialize)]
        struct KeyInfo {
            key: Option<String>,
        }
    }

    /// Handle listing players.
    async fn list_players(&self) -> Result<Response<Body>, Error> {
        let keys = self.db.list_players()?;
        json_ok(keys)
    }

    /// Get information for a single player.
    async fn get_player(&self, id: &str) -> Result<Response<Body>, Error> {
        let player = self.db.get_player(id)?;
        let player = player.ok_or_else(|| Error::NotFound)?;
        json_ok(player)
    }

    /// Verify the specified request.
    fn verify<B>(&self, req: &Request<B>) -> Result<db::User, Error> {
        let user = self
            .session
            .verify(req)?
            .ok_or_else(|| Error::Unauthorized)?;
        Ok(user)
    }

    /// Handle auth redirect coming back.
    async fn auth_redirect(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let mut r = json_empty()?;
        *r.status_mut() = StatusCode::TEMPORARY_REDIRECT;
        let query = self.decode_query::<oauth2::TokenQuery>(req)?;

        let removed = self.pending_tokens.lock().remove(&query.state);

        let PendingToken {
            flow,
            exchange_token,
            action,
            ..
        } = match removed {
            Some(removed) => removed,
            None => {
                return Err(Error::bad_request(
                    "no such session waiting to be authenticated",
                ));
            }
        };

        let token = flow.handle_received_token(exchange_token, query).await?;

        let (return_to, connected) = match action {
            Action::RegisterOrLogin(action) => {
                let result = self.auth_twitch_token(&token.access_token).await?;

                self.db.add_connection(
                    &result.user_id,
                    &db::Connection {
                        id: "login".to_string(),
                        meta: serde_cbor::Value::Null,
                        token,
                    },
                )?;

                self.db.insert_user(
                    &result.user_id,
                    db::User {
                        user_id: result.user_id.to_string(),
                        login: result.login.to_string(),
                    },
                )?;

                self.session.set_cookie(
                    r.headers_mut(),
                    "session",
                    session::SessionData {
                        user_id: result.user_id,
                    },
                )?;

                (action.return_to, None)
            }
            Action::Connect(action) => {
                if self.config.verify_connection {
                    let user = self.verify(req)?;

                    // NB: wrong user received the redirect.
                    if action.user_id != user.user_id {
                        return Err(Error::Unauthorized);
                    }
                }

                let meta = self.token_meta(&flow, &token).await?;

                self.db.add_connection(
                    &action.user_id,
                    &db::Connection {
                        id: action.id.clone(),
                        meta,
                        token,
                    },
                )?;

                (action.return_to, Some(action.id))
            }
        };

        let mut return_to = match return_to {
            Some(url) => url,
            None => self.config.base_url.clone(),
        };

        if let Some(id) = connected {
            return_to.set_query(Some(&format!("connected={}", id)));
        }

        let return_to = return_to.to_string();

        r.headers_mut().insert(
            header::LOCATION,
            return_to.parse().map_err(anyhow::Error::from)?,
        );

        Ok(r)
    }

    /// Handle login or registration.
    async fn auth_login(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let exchange_token = self.login_flow.exchange_token();

        let r = json_ok(&Login {
            auth_url: &exchange_token.auth_url,
        })?;

        self.pending_tokens.lock().insert(
            exchange_token.state,
            PendingToken {
                created_at: Utc::now(),
                flow: self.login_flow.clone(),
                exchange_token,
                action: Action::RegisterOrLogin(RegisterOrLogin {
                    return_to: referer(req)?,
                }),
            },
        );

        return Ok(r);
        #[derive(Serialize)]
        struct Login<'a> {
            auth_url: &'a Url,
        }
    }

    /// Handle clearing cookies for logging out.
    async fn auth_logout(&self, _: &Request<Body>) -> Result<Response<Body>, Error> {
        let mut r = json_empty()?;
        self.session.delete_cookie(r.headers_mut(), "session")?;
        Ok(r)
    }

    /// Show the current session.
    async fn get_auth_current(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        json_ok(self.session.verify(req)?)
    }

    /// Handle a playlist update.
    async fn update_player(&self, req: &mut Request<Body>) -> Result<Response<Body>, Error> {
        let user = self.session.verify(req)?;
        let twitch_token = extract_twitch_token(req);
        let update = receive_json::<PlayerUpdate>(req, Self::MAX_BYTES);

        // NB: need to support special case for backwards compatibility.
        let (login, update) = match user {
            Some(user) => (user.login, update.await?),
            None => {
                let token = twitch_token.ok_or_else(|| Error::Unauthorized)?;
                let (auth, update) = tokio::try_join!(self.auth_twitch_token(&token), update)?;
                (auth.login, update)
            }
        };

        {
            let player = db::Player {
                current: update.current.map(Item::into_player_item),
                items: update
                    .items
                    .into_iter()
                    .map(Item::into_player_item)
                    .collect(),
                last_update: Some(Utc::now()),
            };

            self.db.insert_player(&login, player)?;
        }

        return json_empty();

        #[derive(Debug, Deserialize)]
        struct PlayerUpdate {
            /// Current song.
            #[serde(default)]
            current: Option<Item>,
            /// Songs.
            #[serde(default)]
            items: Vec<Item>,
        }

        fn extract_twitch_token<B>(req: &Request<B>) -> Option<String> {
            let header = match req.headers().get(header::AUTHORIZATION) {
                Some(auth) => auth,
                None => return None,
            };

            let string = match header.to_str() {
                Ok(string) => string,
                _ => return None,
            };

            let mut it = string.splitn(2, ' ');

            match (it.next(), it.next()) {
                (Some("OAuth"), Some(token)) => Some(token.to_string()),
                _ => None,
            }
        }
    }

    /// Get the latest known github releases for the specified user/repo combo.
    async fn get_github_releases(&self, user: &str, repo: &str) -> Result<Response<Body>, Error> {
        let releases = match self.db.get_github_releases(user, repo)? {
            Some(releases) => releases,
            None => return Err(Error::NotFound),
        };

        json_ok(releases)
    }

    /// Test for authentication, if enabled.
    async fn auth_twitch_token(&self, token: &str) -> Result<api::twitch::ValidateToken, Error> {
        self.id_twitch_client
            .validate_token(token)
            .await
            .map_err(Error::Error)
    }

    /// Get token meta-information.
    async fn token_meta(
        &self,
        flow: &oauth2::Flow,
        token: &oauth2::SavedToken,
    ) -> Result<serde_cbor::Value, anyhow::Error> {
        return match flow.config.ty {
            oauth2::FlowType::Twitch => {
                let result = self
                    .id_twitch_client
                    .validate_token(&token.access_token)
                    .await?;

                Ok(serde_cbor::value::to_value(result)?)
            }
            oauth2::FlowType::Spotify => {
                let result = self.spotify.v1_me(&token.access_token).await?;
                Ok(serde_cbor::value::to_value(result)?)
            }
            _ => Ok(serde_cbor::value::to_value(&Empty {})?),
        };

        #[derive(Serialize)]
        pub struct Empty {}
    }

    /// Decode query parameters using the specified model.
    fn decode_query<'a, T>(&self, req: &'a Request<Body>) -> Result<T, Error>
    where
        T: Deserialize<'a>,
    {
        let query = req.uri().query().unwrap_or("");

        let query =
            serde_urlencoded::from_str::<T>(query).map_err(|_| Error::bad_request("bad query"))?;

        Ok(query)
    }
}

/// Token meta-information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TokenMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    login: Option<String>,
}

/// Extract referer.
fn referer<B>(req: &Request<B>) -> Result<Option<Url>, Error> {
    let referer = match req.headers().get(header::REFERER) {
        Some(referer) => referer,
        None => return Ok(None),
    };

    let referer = match std::str::from_utf8(referer.as_ref()) {
        Ok(referer) => referer,
        Err(_) => return Ok(None),
    };

    let referer = match referer.parse() {
        Ok(referer) => referer,
        Err(_) => return Ok(None),
    };

    Ok(Some(referer))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    /// Name of the song.
    name: String,
    /// Artists of the song.
    #[serde(default)]
    artists: Option<String>,
    /// The URL of a track.
    track_url: Option<String>,
    /// Spotify ID of the song.
    track_id: String,
    /// User who requested the song.
    #[serde(default)]
    user: Option<String>,
    /// Length of the song.
    duration: String,
}

impl Item {
    pub fn into_player_item(self) -> db::PlayerItem {
        let track_id = self.track_id;

        db::PlayerItem {
            name: self.name,
            artists: self.artists,
            track_url: self
                .track_url
                .unwrap_or_else(|| format!("{}/{}", SPOTIFY_TRACK_URL, track_id)),
            user: self.user,
            duration: self.duration,
        }
    }
}

/// Construct a JSON OK response.
pub fn http_error(status: StatusCode, message: &str) -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(&Error {
        status: status.as_u16(),
        message,
    })
    .map_err(anyhow::Error::from)?;

    let mut r = Response::new(Body::from(body));

    *r.status_mut() = status;

    r.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/json".parse().expect("valid header value"),
    );

    return Ok(r);

    #[derive(Debug, Default, Serialize)]
    struct Error<'a> {
        status: u16,
        message: &'a str,
    }
}

/// Concats the body and makes sure the request is not too large.
async fn receive_json<T>(req: &mut Request<Body>, max_bytes: usize) -> Result<T, Error>
where
    T: de::DeserializeOwned,
{
    let body = req.body_mut();

    let mut bytes = Vec::new();
    let mut received = 0;

    while let Some(chunk) = body.next().await.transpose().map_err(anyhow::Error::from)? {
        received += chunk.len();

        if received > max_bytes {
            return Err(Error::bad_request("request too large"));
        }

        bytes.extend(chunk);
    }

    match serde_json::from_slice::<T>(&bytes) {
        Ok(body) => Ok(body),
        Err(e) => Err(Error::bad_request(format!("malformed body: {}", e))),
    }
}

/// Construct a HTML response.
pub fn html(body: String) -> Result<Response<Body>, Error> {
    let mut r = Response::new(Body::from(body));

    r.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/html; charset=utf-8"
            .parse()
            .map_err(anyhow::Error::from)?,
    );

    Ok(r)
}

/// Construct a JSON OK response.
pub fn json_ok(body: impl Serialize) -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(&body).map_err(anyhow::Error::from)?;

    let mut r = Response::new(Body::from(body));

    r.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/json".parse().map_err(anyhow::Error::from)?,
    );

    Ok(r)
}

/// Construct an empty json response.
fn json_empty() -> Result<Response<Body>, Error> {
    return json_ok(&Empty {});

    #[derive(Debug, Serialize, Deserialize)]
    struct Empty {}
}

#[derive(Debug)]
pub struct ReceivedToken {
    pub code: String,
    pub state: String,
}
