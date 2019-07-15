use crate::twitch;
use failure::format_err;
use futures::{future, Future, Stream};
use hashbrown::HashMap;
use hyper::{body::Body, error, header, server, service, Method, Request, Response, StatusCode};
use std::{
    mem,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

static SPOTIFY_TRACK_URL: &'static str = "https://open.spotify.com/track";
static GITHUB_URL: &'static str = "https://github.com/udoprog/OxidizeBot";

pub fn setup(
    no_auth: bool,
) -> Result<impl Future<Item = (), Error = error::Error>, failure::Error> {
    let mut reg = handlebars::Handlebars::new();
    reg.register_partial("layout", include_str!("web/layout.html.hbs"))?;
    reg.register_template_string("index", include_str!("web/index.html.hbs"))?;
    reg.register_template_string("privacy", include_str!("web/privacy.html.hbs"))?;
    reg.register_template_string("player", include_str!("web/player.html.hbs"))?;

    let server = Server::new(Arc::new(reg), no_auth)?;

    let addr: SocketAddr = str::parse(&format!("0.0.0.0:8080"))?;

    // TODO: add graceful shutdown.
    let server_future = server::Server::bind(&addr).serve({
        let server = server.clone();
        move || future::ok::<Server, error::Error>(server.clone())
    });

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

/// Interface to the server.
#[derive(Clone)]
pub struct Server {
    players: Arc<RwLock<HashMap<String, Player>>>,
    id_twitch_client: twitch::IdTwitchClient,
    reg: Arc<handlebars::Handlebars>,
    no_auth: bool,
}

impl Server {
    /// Construct a new server.
    pub fn new(reg: Arc<handlebars::Handlebars>, no_auth: bool) -> Result<Self, failure::Error> {
        Ok(Self {
            players: Arc::new(RwLock::new(Default::default())),
            id_twitch_client: twitch::IdTwitchClient::new()?,
            reg,
            no_auth,
        })
    }
}

impl service::Service for Server {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = failure::Error;
    type Future = Box<dyn Future<Item = Response<Self::ResBody>, Error = Self::Error> + Send>;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        let uri = req.uri();

        log::info!("{} {}", req.method(), uri.path());

        let mut it = uri.path().split("/");
        it.next();

        let route = (req.method(), (it.next(), it.next()));

        let future: Box<dyn Future<Item = Response<Self::ResBody>, Error = Error> + Send> =
            match route {
                (&Method::GET, (Some(""), None)) => Box::new(self.handle_index()),
                (&Method::GET, (Some("privacy"), None)) => Box::new(self.handle_privacy()),
                (&Method::GET, (Some("player"), Some(login))) => {
                    Box::new(self.handle_player_show(login))
                }
                (&Method::GET, (Some("api"), Some("players"))) => Box::new(self.player_list()),
                (&Method::POST, (Some("api"), Some("player"))) => Box::new(self.player_update(req)),
                _ => Box::new(future::err(Error::NotFound)),
            };

        let future = future.then(|result| match result {
            Ok(response) => Ok(response),
            Err(e) => {
                let result = match e {
                    Error::BadRequest(e) => {
                        log::error!("BAD REQUEST: {}", e);
                        bad_request()
                    }
                    Error::NotFound => {
                        let mut r = Response::new(Body::from("No such page :("));
                        *r.status_mut() = StatusCode::NOT_FOUND;
                        return Ok(r);
                    }
                    Error::Error(e) => {
                        log::error!("error: {}", e);
                        let mut r = Response::new(Body::from("server error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return Ok(r);
                    }
                };

                match result {
                    Ok(result) => Ok(result),
                    Err(Error::Error(e)) => {
                        log::error!("error: {}", e);
                        let mut r = Response::new(Body::from("server error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return Ok(r);
                    }
                    Err(_) => {
                        log::error!("unknown error :(");
                        let mut r = Response::new(Body::from("server error"));
                        *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return Ok(r);
                    }
                }
            }
        });

        Box::new(future)
    }
}

impl Server {
    const MAX_BYTES: usize = 10_000;

    /// Handles the index page.
    pub fn handle_index(&mut self) -> impl Future<Item = Response<Body>, Error = Error> {
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

        let body = match self.reg.render("index", &data) {
            Ok(body) => body,
            Err(e) => return future::err(e.into()),
        };

        return future::result(html(body));

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
    pub fn handle_privacy(&mut self) -> impl Future<Item = Response<Body>, Error = Error> {
        let data = Data {};

        let body = match self.reg.render("privacy", &data) {
            Ok(body) => body,
            Err(e) => return future::err(e.into()),
        };

        return future::result(html(body));

        #[derive(serde::Serialize)]
        struct Data {}
    }

    /// Handle listing players.
    fn player_list(&mut self) -> impl Future<Item = Response<Body>, Error = Error> {
        let players = self.players.read().expect("poisoned");
        let keys = players.keys().map(|s| s.as_str()).collect::<Vec<&str>>();
        future::result(json_ok(&keys))
    }

    /// Handle a playlist update.
    fn handle_player_show(
        &mut self,
        login: &str,
    ) -> impl Future<Item = Response<Body>, Error = Error> {
        let players = self.players.read().expect("poisoned");

        let player = match players.get(login) {
            Some(items) => items,
            None => return future::err(Error::NotFound),
        };

        let data = Data {
            login: login,
            player: &player,
        };

        let body = match self.reg.render("player", &data) {
            Ok(body) => body,
            Err(e) => return future::err(Error::Error(e.into())),
        };

        return future::result(html(body));

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
    fn player_update(
        &mut self,
        mut req: Request<Body>,
    ) -> Box<dyn Future<Item = Response<Body>, Error = Error> + Send> {
        let body = mem::replace(req.body_mut(), Body::default());

        let future = receive_json::<PlayerUpdate>(body, Self::MAX_BYTES)
            .join(self.auth(&req))
            .and_then({
                let players = self.players.clone();

                move |(update, auth)| {
                    let mut players = players.write().expect("poisoned");
                    let player = players.entry(auth.login).or_insert_with(Default::default);
                    player.current = update.current.map(Item::into_player_item);
                    player.items = update
                        .items
                        .into_iter()
                        .map(Item::into_player_item)
                        .collect();
                    json_ok(&ResponseBody {})
                }
            });

        return Box::new(future);

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
    fn auth(
        &self,
        req: &Request<Body>,
    ) -> Box<dyn Future<Item = twitch::ValidateToken, Error = Error> + Send> {
        let token = match self.extract_token(&req) {
            Some(token) => token,
            None => {
                return Box::new(future::err(Error::BadRequest(format_err!(
                    "Missing token from Authorization header"
                ))))
            }
        };

        if self.no_auth {
            return Box::new(future::ok(twitch::ValidateToken {
                client_id: String::from("client_id"),
                login: token,
                scopes: vec![],
                user_id: String::from("user_id"),
            }));
        }

        Box::new(
            self.id_twitch_client
                .validate_token(&token)
                .map_err(Error::Error),
        )
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
fn receive_json<T>(body: Body, max_bytes: usize) -> impl Future<Item = T, Error = Error>
where
    T: serde::de::DeserializeOwned,
{
    let mut received = 0;

    let future = body.map_err(Error::from).and_then(move |chunk| {
        received += chunk.len();

        if received > max_bytes {
            return Err(Error::BadRequest(format_err!("request too large")));
        }

        Ok(chunk)
    });

    future.concat2().and_then({
        move |body| match serde_json::from_slice::<T>(body.as_ref()) {
            Ok(body) => Ok(body),
            Err(e) => Err(Error::BadRequest(e.into())),
        }
    })
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
