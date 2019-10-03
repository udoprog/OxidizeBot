use crate::{db, oauth2, session, twitch};
use chrono::{DateTime, Utc};
use failure::format_err;
use futures::prelude::*;
use hyper::{body::Body, error, header, server, Method, Request, Response, StatusCode};
use parking_lot::Mutex;
use relative_path::RelativePathBuf;
use serde::{de, Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    borrow::Cow,
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::{Arc, RwLock},
    task::{Context, Poll},
    time,
};
use tower_service::Service;
use url::Url;

static SPOTIFY_TRACK_URL: &'static str = "https://open.spotify.com/track";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    database: RelativePathBuf,
    base_url: Url,
    #[serde(default)]
    session: session::Config,
    oauth2: oauth2::Config,
}

mod assets {
    #[derive(rust_embed::RustEmbed)]
    #[folder = "$CARGO_MANIFEST_DIR/ui/dist"]
    pub struct Asset;
}

pub struct MakeSvc(Server);

impl<T> Service<T> for MakeSvc {
    type Response = Server;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(self.0.clone())
    }
}

pub fn setup(
    no_auth: bool,
    root: &Path,
    config: Config,
) -> Result<impl Future<Output = Result<(), error::Error>>, failure::Error> {
    let fallback = assets::Asset::get("index.html")
        .ok_or_else(|| format_err!("missing index.html in assets"))?;

    let db = sled::Db::open(config.database.to_path(root))?;
    let tree = Arc::new(db.open_tree("storage")?);

    let pending_tokens = Arc::new(Mutex::new(HashMap::new()));

    let server = Server {
        handler: Arc::new(Handler::new(
            no_auth,
            fallback,
            tree,
            config,
            pending_tokens.clone(),
        )?),
    };

    let addr: SocketAddr = str::parse(&format!("0.0.0.0:8000"))?;

    // TODO: add graceful shutdown.
    let server_future = server::Server::bind(&addr).serve(MakeSvc(server));

    let future = async move {
        let mut interval = tokio::timer::Interval::new_interval(time::Duration::from_secs(30));
        let expires = chrono::Duration::minutes(5);

        let mut server_future = server_future.fuse();

        loop {
            futures::select! {
                result = server_future => {
                    return result;
                }
                _ = interval.select_next_some() => {
                    log::info!("Checking for expires pending tokens");

                    let now = Utc::now();
                    let mut tokens = pending_tokens.lock();
                    let mut to_remove = smallvec::SmallVec::<[String; 16]>::new();

                    for (key, pending) in &*tokens {
                        if pending.created_at + expires > now {
                            to_remove.push(key.to_string());
                        }
                    }

                    if !to_remove.is_empty() {
                        log::info!("Removing {} expired tokens", to_remove.len());
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

pub enum Error {
    /// Client performed a bad request.
    BadRequest(String),
    /// The resource could not be found.
    NotFound,
    /// User unauthorized to perform the given request.
    Unauthorized,
    /// Generic error.
    Error(failure::Error),
}

impl Error {
    /// Construct a new bad request error.
    pub fn bad_request(s: impl AsRef<str>) -> Self {
        Self::BadRequest(s.as_ref().to_string())
    }
}

impl From<failure::Error> for Error {
    fn from(value: failure::Error) -> Error {
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
    user: String,
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
    fallback: Cow<'static, [u8]>,
    players: Arc<RwLock<HashMap<String, Player>>>,
    id_twitch_client: twitch::IdTwitchClient,
    no_auth: bool,
    login_flow: Arc<oauth2::Flow>,
    flows: HashMap<String, Arc<oauth2::Flow>>,
    pending_tokens: Arc<Mutex<HashMap<String, PendingToken>>>,
    random: ring::rand::SystemRandom,
    week: chrono::Duration,
}

impl Handler {
    /// Construct a new server.
    pub fn new(
        no_auth: bool,
        fallback: Cow<'static, [u8]>,
        tree: Arc<sled::Tree>,
        config: Config,
        pending_tokens: Arc<Mutex<HashMap<String, PendingToken>>>,
    ) -> Result<Self, failure::Error> {
        let db = db::Database::load(tree)?;
        let (login_flow, flows) = oauth2::setup_flows(&config.base_url, &config.oauth2)?;
        let session = session::Session::new(db.clone(), &config.session)?;

        Ok(Self {
            db,
            config,
            session,
            fallback,
            players: Arc::new(RwLock::new(Default::default())),
            id_twitch_client: twitch::IdTwitchClient::new()?,
            no_auth,
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

                (mime, asset)
            }
            None => (mime::TEXT_HTML_UTF_8, self.fallback.clone()),
        };

        *r.body_mut() = Body::from(asset);

        if let Ok(mime) = mime.as_ref().parse() {
            r.headers_mut().insert(header::CONTENT_TYPE, mime);
        }

        r
    }

    async fn handle_call(&self, req: Request<Body>) -> Response<Body> {
        let uri = req.uri();

        log::info!("{} {}", req.method(), uri.path());

        let mut it = uri.path().split("/");
        it.next();

        let path = it.collect::<SmallVec<[_; 8]>>();

        let result = match (req.method(), &*path) {
            (&Method::GET, &["api", "auth", "redirect"]) => self.handle_auth_redirect(&req).await,
            (&Method::POST, &["api", "auth", "login"]) => self.handle_login(&req).await,
            (&Method::POST, &["api", "auth", "logout"]) => self.handle_logout(&req).await,
            (&Method::GET, &["api", "auth", "current"]) => self.handle_current(&req).await,
            (&Method::GET, &["api", "connection-types"]) => self.connection_types_list(&req).await,
            (&Method::GET, &["api", "connections"]) => self.connections_list(&req).await,
            (&Method::GET, &["api", "connections", id]) => self.connections_get(&req, id).await,
            (&Method::POST, &["api", "connections", id, "refresh"]) => {
                self.connection_refresh(&req, id).await
            }
            (&Method::DELETE, &["api", "connections", id]) => {
                self.connections_delete(&req, id).await
            }
            (&Method::POST, &["api", "connections", id]) => self.connections_create(&req, id).await,
            (&Method::POST, &["api", "key"]) => self.create_key(&req).await,
            (&Method::DELETE, &["api", "key"]) => self.delete_key(&req).await,
            (&Method::GET, &["api", "key"]) => self.get_key(&req).await,
            (&Method::GET, &["api", "players"]) => self.player_list().await,
            (&Method::GET, &["api", "player", id]) => self.player_get(id).await,
            (&Method::POST, &["api", "player"]) => {
                drop(path);
                self.player_update(req).await
            }
            (&Method::GET, _) => return self.static_asset(uri.path()),
            _ => Err(Error::NotFound),
        };

        match result {
            Ok(mut response) => {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
                return response;
            }
            Err(e) => {
                let result = match e {
                    Error::BadRequest(message) => http_error(StatusCode::BAD_REQUEST, &message),
                    Error::NotFound => http_error(StatusCode::NOT_FOUND, "Not Found"),
                    Error::Unauthorized => http_error(StatusCode::UNAUTHORIZED, "Unauthorized"),
                    Error::Error(e) => {
                        log::error!("Internal Server Error: {}", e);
                        http_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
                    }
                };

                match result {
                    Ok(result) => result,
                    Err(Error::Error(e)) => {
                        log::error!("error: {}", e);
                        let mut r = Response::new(Body::from("Internal Server Error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return r;
                    }
                    Err(_) => {
                        log::error!("unknown error :(");
                        let mut r = Response::new(Body::from("Internal Server Error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return r;
                    }
                }
            }
        }
    }
}

/// Interface to the server.
#[derive(Clone)]
pub struct Server {
    handler: Arc<Handler>,
}

impl Service<Request<Body>> for Server {
    type Response = Response<Body>;
    type Error = failure::Error;
    type Future = future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let handler = self.handler.clone();

        let future = async move { Ok(handler.handle_call(req).await) };

        future.boxed()
    }
}

#[derive(Serialize)]
pub struct Connection<'a> {
    #[serde(rename = "type")]
    ty: oauth2::FlowType,
    id: &'a str,
    title: &'a str,
    description: &'a str,
    hash: String,
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
    async fn connections_get(
        &self,
        req: &Request<Body>,
        id: &str,
    ) -> Result<Response<Body>, Error> {
        let session = self.verify(req)?;

        let c = self.db.get_connection(&session.user, id)?;
        let query = self.decode_query::<Query>(req)?;

        match c {
            Some(c) => {
                let flow = match self.flows.get(&c.id) {
                    Some(flow) => flow,
                    None => return json_ok(Data::empty()),
                };

                match query.format.as_ref().map(String::as_str) {
                    Some("meta") => {
                        return json_ok(Data::from(Connection {
                            ty: flow.ty,
                            id: &c.id,
                            title: &flow.config.title,
                            description: &flow.config.description,
                            hash: c.token.hash()?,
                            token: None,
                        }));
                    }
                    _ => {
                        return json_ok(Data::from(Connection {
                            ty: flow.ty,
                            id: &c.id,
                            title: &flow.config.title,
                            description: &flow.config.description,
                            hash: c.token.hash()?,
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
    async fn connection_refresh(
        &self,
        req: &Request<Body>,
        id: &str,
    ) -> Result<Response<Body>, Error> {
        let session = self.verify(req)?;

        let c = self.db.get_connection(&session.user, id)?;

        let c = match c {
            Some(c) => c,
            None => return json_ok(Data::empty()),
        };

        let flow = match self.flows.get(&c.token.flow_id) {
            Some(flow) => flow,
            None => return json_ok(Data::empty()),
        };

        let token = flow.refresh_token(&c.token.refresh_token).await?;

        self.db.add_connection(
            &session.user,
            &db::Connection {
                id: id.to_string(),
                token: token.clone(),
            },
        )?;

        let connection = Connection {
            ty: flow.ty,
            id: &c.id,
            title: &flow.config.title,
            description: &flow.config.description,
            hash: token.hash()?,
            token: Some(token.as_exported()),
        };

        json_ok(Data::from(connection))
    }

    /// Get a list of connection types.
    async fn connection_types_list(&self, _: &Request<Body>) -> Result<Response<Body>, Error> {
        let mut out = Vec::new();

        for client_config in &self.config.oauth2.flows {
            out.push(ConnectionMeta {
                ty: client_config.ty,
                id: &client_config.id,
                title: &client_config.title,
                description: &client_config.description,
            });
        }

        return json_ok(&out);
    }

    /// List connections for the current user.
    async fn connections_list(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let session = self.verify(req)?;
        let connections = self.db.connections_by_user(&session.user)?;

        let query = self.decode_query::<Query>(req)?;

        let meta = match query.format.as_ref().map(String::as_str) {
            Some("meta") => true,
            _ => false,
        };

        let mut out = Vec::new();

        for c in &connections {
            let flow = match self.flows.get(&c.id) {
                Some(flow) => flow,
                None => continue,
            };

            out.push(Connection {
                ty: flow.ty,
                id: &c.id,
                title: &flow.config.title,
                description: &flow.config.description,
                hash: c.token.hash()?,
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
    async fn connections_delete(
        &self,
        req: &Request<Body>,
        id: &str,
    ) -> Result<Response<Body>, Error> {
        let session = self.verify(req)?;

        self.db.delete_connection(&session.user, id)?;
        json_empty()
    }

    /// List connections for the current user.
    async fn connections_create(
        &self,
        req: &Request<Body>,
        id: &str,
    ) -> Result<Response<Body>, Error> {
        let session = self.verify(req)?;

        let flow = match self.flows.get(id).cloned() {
            Some(flow) => flow,
            None => {
                return Err(Error::bad_request(format!(
                    "unsupported connection: {}",
                    id
                )))
            }
        };

        let exchange_token = flow.exchange_token();

        let r = json_ok(&Login {
            auth_url: &exchange_token.auth_url,
        })?;

        self.pending_tokens.lock().insert(
            exchange_token.csrf_token.secret().to_string(),
            PendingToken {
                created_at: Utc::now(),
                flow,
                exchange_token,
                action: Action::Connect(Connect {
                    user: session.user.to_string(),
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
        let session = self.verify(req)?;

        use ring::rand::SecureRandom as _;

        let mut buf = [0u8; 32];
        self.random
            .fill(&mut buf)
            .map_err(|_| format_err!("failed to generate random key"))?;
        let key = base64::encode(&buf);
        self.db.insert_key(&session.user, &key)?;

        return json_ok(&KeyInfo { key });

        #[derive(Serialize)]
        struct KeyInfo {
            key: String,
        }
    }

    /// Delete the current key.
    async fn delete_key(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let session = self.verify(req)?;

        self.db.delete_key(&session.user)?;
        return json_empty();
    }

    /// Get the current key.
    async fn get_key(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let session = self.verify(req)?;

        let key = self.db.get_key(&session.user)?;

        return json_ok(&KeyInfo { key });

        #[derive(Serialize)]
        struct KeyInfo {
            key: Option<String>,
        }
    }

    /// Handle listing players.
    async fn player_list(&self) -> Result<Response<Body>, Error> {
        let players = self.players.read().expect("poisoned");
        let keys = players.keys().map(|s| s.as_str()).collect::<Vec<&str>>();
        json_ok(&keys)
    }

    /// Get information for a single player.
    async fn player_get(&self, id: &str) -> Result<Response<Body>, Error> {
        let players = self.players.read().expect("poisoned");
        let player = players.get(id);
        json_ok(&player)
    }

    /// Verify the specified request.
    fn verify<B>(&self, req: &Request<B>) -> Result<session::SessionData, Error> {
        let session = self
            .session
            .verify(req)?
            .ok_or_else(|| Error::Unauthorized)?;
        Ok(session)
    }

    /// Handle auth redirect coming back.
    async fn handle_auth_redirect(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let uri = req.uri();
        let mut r = json_empty()?;
        *r.status_mut() = StatusCode::TEMPORARY_REDIRECT;

        let query = match uri.query() {
            Some(query) => query,
            None => return Err(Error::bad_request("missing query parameters")),
        };

        let query = serde_urlencoded::from_str::<oauth2::TokenQuery>(query)
            .map_err(|_| Error::bad_request("bad query parameters"))?;

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
                ))
            }
        };

        let token = flow.handle_received_token(exchange_token, query).await?;

        let return_to = match action {
            Action::RegisterOrLogin(action) => {
                let result = self.auth_twitch_token(token.access_token.secret()).await?;

                self.db.add_connection(
                    &result.login,
                    &db::Connection {
                        id: "login".to_string(),
                        token,
                    },
                )?;

                self.session.set_cookie(
                    r.headers_mut(),
                    "session",
                    session::SessionData { user: result.login },
                )?;

                action.return_to
            }
            Action::Connect(action) => {
                let session = self.verify(req)?;

                // NB: wrong user received the redirect.
                if action.user != session.user {
                    return Err(Error::Unauthorized);
                }

                self.db.add_connection(
                    &session.user,
                    &db::Connection {
                        id: action.id,
                        token,
                    },
                )?;

                action.return_to
            }
        };

        let return_to = match return_to {
            Some(url) => url.to_string(),
            None => self.config.base_url.to_string(),
        };

        r.headers_mut().insert(
            header::LOCATION,
            return_to.parse().map_err(failure::Error::from)?,
        );

        Ok(r)
    }

    /// Handle login or registration.
    async fn handle_login(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        let exchange_token = self.login_flow.exchange_token();

        let r = json_ok(&Login {
            auth_url: &exchange_token.auth_url,
        })?;

        self.pending_tokens.lock().insert(
            exchange_token.csrf_token.secret().to_string(),
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
    async fn handle_logout(&self, _: &Request<Body>) -> Result<Response<Body>, Error> {
        let mut r = json_empty()?;
        self.session.delete_cookie(r.headers_mut(), "session")?;
        Ok(r)
    }

    /// Show the current session.
    async fn handle_current(&self, req: &Request<Body>) -> Result<Response<Body>, Error> {
        json_ok(&self.session.verify(req)?)
    }

    /// Handle a playlist update.
    async fn player_update(&self, req: Request<Body>) -> Result<Response<Body>, Error> {
        let session = self.session.verify(&req)?;
        let twitch_token = extract_twitch_token(&req);
        let update = receive_json::<PlayerUpdate>(req, Self::MAX_BYTES);

        // NB: need to support special case for backwards compatibility.
        let (user, update) = match session {
            Some(session) => (session.user, update.await?),
            None => {
                let token = twitch_token.ok_or_else(|| Error::Unauthorized)?;
                let (auth, update) =
                    future::try_join(self.auth_twitch_token(&token), update).await?;
                (auth.login, update)
            }
        };

        {
            let mut players = self.players.write().expect("poisoned");
            let player = players.entry(user).or_insert_with(Default::default);
            player.current = update.current.map(Item::into_player_item);
            player.items = update
                .items
                .into_iter()
                .map(Item::into_player_item)
                .collect();
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

            let mut it = string.splitn(2, " ");

            match (it.next(), it.next()) {
                (Some("OAuth"), Some(token)) => Some(token.to_string()),
                _ => None,
            }
        }
    }

    /// Test for authentication, if enabled.
    async fn auth_twitch_token(&self, token: &str) -> Result<twitch::ValidateToken, Error> {
        if self.no_auth {
            return Ok(twitch::ValidateToken {
                client_id: String::from("client_id"),
                login: token.to_string(),
                scopes: vec![],
                user_id: String::from("user_id"),
            });
        }

        Ok(self
            .id_twitch_client
            .validate_token(token)
            .await
            .map_err(Error::Error)?)
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

#[derive(Debug, Default, Deserialize, Serialize)]
struct Player {
    current: Option<PlayerItem>,
    items: Vec<PlayerItem>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PlayerItem {
    /// Name of the song.
    name: String,
    /// Artists of the song.
    #[serde(default)]
    artists: Option<String>,
    /// The URL of a track.
    track_url: String,
    /// User who requested the song.
    #[serde(default)]
    user: Option<String>,
    /// Length of the song.
    duration: String,
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
    pub fn into_player_item(self) -> PlayerItem {
        let track_id = self.track_id;

        PlayerItem {
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
    .map_err(failure::Error::from)?;

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
async fn receive_json<T>(req: Request<Body>, max_bytes: usize) -> Result<T, Error>
where
    T: de::DeserializeOwned,
{
    let mut body = req.into_body();

    let mut bytes = Vec::new();
    let mut received = 0;

    while let Some(chunk) = body
        .next()
        .await
        .transpose()
        .map_err(failure::Error::from)?
    {
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
            .map_err(failure::Error::from)?,
    );

    Ok(r)
}

/// Construct a JSON OK response.
pub fn json_ok(body: impl Serialize) -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(&body).map_err(failure::Error::from)?;

    let mut r = Response::new(Body::from(body));

    r.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/json".parse().map_err(failure::Error::from)?,
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
