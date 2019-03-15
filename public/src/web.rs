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

pub fn setup() -> Result<impl Future<Item = (), Error = error::Error>, failure::Error> {
    let mut reg = handlebars::Handlebars::new();
    reg.register_template_string("index", include_str!("web/index.html.hbs"))?;

    let server = Server::new(Arc::new(reg))?;

    let addr: SocketAddr = str::parse(&format!("0.0.0.0:8080"))?;

    // TODO: add graceful shutdown.
    let server_future = server::Server::bind(&addr).serve({
        let server = server.clone();
        move || future::ok::<Server, error::Error>(server.clone())
    });

    Ok(server_future)
}

pub enum Error {
    BadRequest(failure::Error),
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
    queues: Arc<RwLock<HashMap<String, Vec<Item>>>>,
    id_twitch_client: twitch::IdTwitchClient,
    reg: Arc<handlebars::Handlebars>,
}

impl Server {
    /// Construct a new server.
    pub fn new(reg: Arc<handlebars::Handlebars>) -> Result<Self, failure::Error> {
        Ok(Self {
            queues: Arc::new(RwLock::new(Default::default())),
            id_twitch_client: twitch::IdTwitchClient::new()?,
            reg,
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

        let mut it = uri.path().split("/");
        it.next();

        let future: Box<dyn Future<Item = Response<Self::ResBody>, Error = Error> + Send> =
            match (req.method(), (it.next(), it.next())) {
                (&Method::GET, (None, None)) => Box::new(self.handle_index()),
                (&Method::GET, (Some("queue"), None)) => Box::new(self.handle_queue_list()),
                (&Method::GET, (Some("queue"), Some(login))) => {
                    Box::new(self.handle_queue_request(login))
                }
                (&Method::POST, (Some("api"), Some("queue"))) => {
                    Box::new(self.handle_queue_update(req))
                }
                _ => {
                    let mut r = Response::new(Body::from("No such page :("));
                    *r.status_mut() = StatusCode::NOT_FOUND;
                    return Box::new(future::ok(r));
                }
            };

        let future = future.then(|result| match result {
            Ok(response) => Ok(response),
            Err(e) => {
                let result = match e {
                    Error::BadRequest(e) => {
                        log::error!("BAD REQUEST: {}", e);
                        bad_request()
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

    /// Handles Oauth 2.0 authentication redirect.
    pub fn handle_index(&mut self) -> impl Future<Item = Response<Body>, Error = Error> {
        let data = Data {};

        let body = match self.reg.render("index", &data) {
            Ok(body) => body,
            Err(e) => return future::err(e.into()),
        };

        let mut r = Response::new(Body::from(body));
        r.headers_mut().insert(
            header::CONTENT_TYPE,
            "text/html; charset=utf-8"
                .parse()
                .expect("valid header value"),
        );

        return future::ok(r);

        #[derive(serde::Serialize)]
        struct Data {}
    }

    /// Handle listing queues.
    pub fn handle_queue_list(&mut self) -> impl Future<Item = Response<Body>, Error = Error> {
        let queues = self.queues.read().expect("poisoned");
        let keys = queues.keys().map(|s| s.as_str()).collect::<Vec<&str>>();
        future::result(json_ok(&keys))
    }

    /// Handle a playlist update.
    pub fn handle_queue_request(
        &mut self,
        login: &str,
    ) -> impl Future<Item = Response<Body>, Error = Error> {
        let queues = self.queues.read().expect("poisoned");

        let queue = match queues.get(login) {
            Some(queue) => queue,
            None => {
                return future::err(Error::BadRequest(format_err!(
                    "no queue for login: {}",
                    login
                )))
            }
        };

        future::result(json_ok(&queue))
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
    pub fn handle_queue_update(
        &mut self,
        mut req: Request<Body>,
    ) -> Box<dyn Future<Item = Response<Body>, Error = Error> + Send> {
        let body = mem::replace(req.body_mut(), Body::default());

        let token = match self.extract_token(&req) {
            Some(token) => token,
            None => {
                return Box::new(future::err(Error::BadRequest(format_err!(
                    "Missing token from Authorization header"
                ))))
            }
        };

        let future = receive_json::<QueueUpdate>(body, Self::MAX_BYTES)
            .join(
                self.id_twitch_client
                    .validate_token(&token)
                    .map_err(Error::Error),
            )
            .and_then({
                let queues = self.queues.clone();

                move |(update, result)| {
                    let mut queues = queues.write().expect("poisoned");
                    let queue = queues.entry(result.login).or_insert_with(Default::default);
                    *queue = update.items;
                    json_ok(&ResponseBody {})
                }
            });

        return Box::new(future);

        #[derive(Debug, serde::Deserialize)]
        struct QueueUpdate {
            items: Vec<Item>,
            #[serde(default)]
            login: Option<String>,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Item {
    name: String,
    artists: Vec<String>,
    spotify_id: String,
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

/// Construct a JSON OK response.
pub fn json_ok(body: &impl serde::Serialize) -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(body)?;

    let mut r = Response::new(Body::from(body));

    r.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/json".parse().expect("valid header value"),
    );

    Ok(r)
}

#[derive(Debug)]
pub struct ReceivedToken {
    pub code: String,
    pub state: String,
}
