use crate::twitch;
use failure::format_err;
use futures::prelude::*;
use hashbrown::HashMap;
use hyper::{body::Body, error, header, server, Method, Request, Response, StatusCode};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};
use tower_service::Service;

static SPOTIFY_TRACK_URL: &'static str = "https://open.spotify.com/track";
static GITHUB_URL: &'static str = "https://github.com/udoprog/OxidizeBot";

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
) -> Result<impl Future<Output = Result<(), error::Error>>, failure::Error> {
    let mut reg = handlebars::Handlebars::new();
    reg.register_partial("layout", include_str!("web/layout.html.hbs"))?;
    reg.register_template_string("index", include_str!("web/index.html.hbs"))?;
    reg.register_template_string("privacy", include_str!("web/privacy.html.hbs"))?;
    reg.register_template_string("player", include_str!("web/player.html.hbs"))?;

    let server = Server {
        handler: Arc::new(Handler::new(Arc::new(reg), no_auth)?),
    };

    let addr: SocketAddr = str::parse(&format!("0.0.0.0:8080"))?;

    // TODO: add graceful shutdown.
    let server_future = server::Server::bind(&addr).serve(MakeSvc(server));

    Ok(server_future)
}

pub enum Error {
    /// Client performed a bad request.
    BadRequest(failure::Error),
    /// The resource could not be found.
    NotFound,
    /// Generic error.
    Error(failure::Error),
}

impl<E> From<E> for Error
where
    E: 'static + std::error::Error + Send + Sync,
{
    fn from(value: E) -> Error {
        Error::Error(value.into())
    }
}

pub struct Handler {
    players: Arc<RwLock<HashMap<String, Player>>>,
    id_twitch_client: twitch::IdTwitchClient,
    reg: Arc<handlebars::Handlebars>,
    no_auth: bool,
}

impl Handler {
    /// Construct a new server.
    pub fn new(reg: Arc<handlebars::Handlebars>, no_auth: bool) -> Result<Self, failure::Error> {
        Ok(Self {
            players: Arc::new(RwLock::new(Default::default())),
            id_twitch_client: twitch::IdTwitchClient::new()?,
            reg,
            no_auth,
        })
    }

    async fn handle_call(&self, req: Request<Body>) -> Response<Body> {
        let uri = req.uri();

        log::info!("{} {}", req.method(), uri.path());

        let mut it = uri.path().split("/");
        it.next();

        let route = (req.method(), (it.next(), it.next()));

        let result = match route {
            (&Method::GET, (Some(""), None)) => self.handle_index().await,
            (&Method::GET, (Some("privacy"), None)) => self.handle_privacy().await,
            (&Method::GET, (Some("player"), Some(login))) => self.handle_player_show(login).await,
            (&Method::GET, (Some("api"), Some("players"))) => self.player_list().await,
            (&Method::POST, (Some("api"), Some("player"))) => self.player_update(req).await,
            _ => future::err(Error::NotFound).await,
        };

        match result {
            Ok(response) => return response,
            Err(e) => {
                let result = match e {
                    Error::BadRequest(e) => {
                        log::error!("BAD REQUEST: {}", e);
                        bad_request()
                    }
                    Error::NotFound => {
                        let mut r = Response::new(Body::from("No such page :("));
                        *r.status_mut() = StatusCode::NOT_FOUND;
                        return r;
                    }
                    Error::Error(e) => {
                        log::error!("error: {}", e);
                        let mut r = Response::new(Body::from("server error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return r;
                    }
                };

                match result {
                    Ok(result) => result,
                    Err(Error::Error(e)) => {
                        log::error!("error: {}", e);
                        let mut r = Response::new(Body::from("server error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return r;
                    }
                    Err(_) => {
                        log::error!("unknown error :(");
                        let mut r = Response::new(Body::from("server error"));
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

impl Handler {
    const MAX_BYTES: usize = 10_000;

    /// Handles the index page.
    pub async fn handle_index(&self) -> Result<Response<Body>, Error> {
        let players = self.players.read().expect("poisoned");

        let players = {
            let mut out = Vec::new();

            for name in players.keys() {
                out.push(Player {
                    name: name.as_str(),
                })
            }

            out
        };

        let data = Data {
            github_url: GITHUB_URL,
            players: players,
        };

        let body = self.reg.render("index", &data)?;
        return html(body);

        #[derive(serde::Serialize)]
        struct Data<'a> {
            github_url: &'static str,
            players: Vec<Player<'a>>,
        }

        #[derive(serde::Serialize)]
        struct Player<'a> {
            name: &'a str,
        }
    }

    /// Handles the index page.
    pub async fn handle_privacy(&self) -> Result<Response<Body>, Error> {
        let data = Data {};
        let body = self.reg.render("privacy", &data)?;
        return html(body);

        #[derive(serde::Serialize)]
        struct Data {}
    }

    /// Handle listing players.
    async fn player_list(&self) -> Result<Response<Body>, Error> {
        let players = self.players.read().expect("poisoned");
        let keys = players.keys().map(|s| s.as_str()).collect::<Vec<&str>>();
        json_ok(&keys)
    }

    /// Handle a playlist update.
    async fn handle_player_show(&self, login: &str) -> Result<Response<Body>, Error> {
        let players = self.players.read().expect("poisoned");

        let player = players.get(login).ok_or(Error::NotFound)?;

        let data = Data {
            login: login,
            player: &player,
        };

        let body = self.reg.render("player", &data)?;
        return html(body);

        #[derive(serde::Serialize)]
        struct Data<'a> {
            login: &'a str,
            player: &'a Player,
        }
    }

    fn extract_token<B>(&self, req: &Request<B>) -> Option<String> {
        let header = match req.headers().get(header::AUTHORIZATION) {
            Some(auth) => auth,
            None => return None,
        };

        let string = match header.to_str() {
            Ok(string) => string,
            Err(e) => {
                log::error!("Bad Authorization header: {}", e);
                return None;
            }
        };

        let mut it = string.splitn(2, " ");

        match (it.next(), it.next()) {
            (Some("OAuth"), Some(token)) => Some(token.to_string()),
            _ => None,
        }
    }

    /// Handle a playlist update.
    async fn player_update(&self, req: Request<Body>) -> Result<Response<Body>, Error> {
        let token = match self.extract_token(&req) {
            Some(token) => token,
            None => {
                return Err(Error::BadRequest(format_err!(
                    "Missing token from Authorization header"
                )));
            }
        };

        let req = receive_json::<PlayerUpdate>(req, Self::MAX_BYTES).boxed();

        let (update, auth) = future::try_join(req, self.auth(&token)).await?;

        {
            let mut players = self.players.write().expect("poisoned");
            let player = players.entry(auth.login).or_insert_with(Default::default);
            player.current = update.current.map(Item::into_player_item);
            player.items = update
                .items
                .into_iter()
                .map(Item::into_player_item)
                .collect();
        }

        return json_ok(&ResponseBody {});

        #[derive(Debug, serde::Deserialize)]
        struct PlayerUpdate {
            /// Current song.
            #[serde(default)]
            current: Option<Item>,
            /// Songs.
            #[serde(default)]
            items: Vec<Item>,
        }
    }

    /// Test for authentication, if enabled.
    async fn auth(&self, token: &str) -> Result<twitch::ValidateToken, Error> {
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
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
struct Player {
    current: Option<PlayerItem>,
    items: Vec<PlayerItem>,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Default, serde::Serialize)]
struct ResponseBody {}

/// Construct a JSON OK response.
pub fn bad_request() -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(&ResponseBody::default())?;

    let mut r = Response::new(Body::from(body));

    *r.status_mut() = StatusCode::BAD_REQUEST;

    r.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/json".parse().expect("valid header value"),
    );

    Ok(r)
}

/// Concats the body and makes sure the request is not too large.
async fn receive_json<T>(req: Request<Body>, max_bytes: usize) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let mut body = req.into_body();

    let mut bytes = Vec::new();
    let mut received = 0;

    while let Some(chunk) = body.next().await.transpose()? {
        received += chunk.len();

        if received > max_bytes {
            return Err(Error::BadRequest(format_err!("request too large")));
        }

        bytes.extend(chunk);
    }

    serde_json::from_slice::<T>(&bytes).map_err(|e| Error::BadRequest(e.into()))
}

/// Construct a HTML response.
pub fn html(body: String) -> Result<Response<Body>, Error> {
    let mut r = Response::new(Body::from(body));

    r.headers_mut()
        .insert(header::CONTENT_TYPE, "text/html; charset=utf-8".parse()?);

    Ok(r)
}

/// Construct a JSON OK response.
pub fn json_ok(body: &impl serde::Serialize) -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(body)?;

    let mut r = Response::new(Body::from(body));

    r.headers_mut()
        .insert(header::CONTENT_TYPE, "application/json".parse()?);

    Ok(r)
}

#[derive(Debug)]
pub struct ReceivedToken {
    pub code: String,
    pub state: String,
}
