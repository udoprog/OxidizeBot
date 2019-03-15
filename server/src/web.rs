use crate::utils;
use futures::{future, sync::oneshot, Future};
use hashbrown::HashMap;
use hyper::{body::Body, error, header, server, service, Request, Response, StatusCode, Uri};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

pub const URL: &'static str = "http://localhost:12345";
pub const REDIRECT_URI: &'static str = "/redirect";

pub fn setup() -> Result<(Server, impl Future<Item = (), Error = error::Error>), failure::Error> {
    let mut reg = handlebars::Handlebars::new();
    reg.register_template_string("index", include_str!("web/index.html.hbs"))?;

    let server = Server::new(Arc::new(reg));

    let addr: SocketAddr = str::parse(&format!("0.0.0.0:12345"))?;

    // TODO: add graceful shutdown.
    let server_future = server::Server::bind(&addr).serve({
        let server = server.clone();
        move || future::ok::<Server, error::Error>(server.clone())
    });

    Ok((server, server_future))
}

struct ExpectedToken {
    url: url::Url,
    title: String,
    channel: oneshot::Sender<ReceivedToken>,
}

/// Interface to the server.
#[derive(Clone)]
pub struct Server {
    reg: Arc<handlebars::Handlebars>,
    /// Callbacks for when we have received a token.
    token_callbacks: Arc<RwLock<HashMap<String, ExpectedToken>>>,
}

impl Server {
    /// Construct a new server.
    pub fn new(reg: Arc<handlebars::Handlebars>) -> Self {
        Self {
            reg,
            token_callbacks: Arc::new(RwLock::new(HashMap::default())),
        }
    }

    /// Receive an Oauth 2.0 token.
    ///
    /// # Arguments
    ///
    /// * `url` the url to visit to authenticate.
    /// * `title` the title of the authentication.
    /// * `state` the CSRF state to match against.
    pub fn receive_token(
        &self,
        url: url::Url,
        title: String,
        state: String,
    ) -> oneshot::Receiver<ReceivedToken> {
        let (tx, rx) = oneshot::channel::<ReceivedToken>();
        let mut inner = self.token_callbacks.write().expect("lock poisoned");

        inner.insert(
            state,
            ExpectedToken {
                url,
                title,
                channel: tx,
            },
        );

        rx
    }
}

impl service::Service for Server {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = error::Error;
    type Future = future::FutureResult<Response<Self::ResBody>, Self::Error>;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        let uri = req.uri();

        let result = match uri.path() {
            REDIRECT_URI => Ok(self.handle_oauth2_redirect(uri)),
            "/" => self.handle_index(),
            _ => {
                let mut r = Response::new(Body::from("No such page :("));
                *r.status_mut() = StatusCode::NOT_FOUND;
                return future::ok(r);
            }
        };

        let response = match result {
            Ok(response) => response,
            Err(_) => {
                let mut r = Response::new(Body::from("server error"));
                *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                return future::ok(r);
            }
        };

        future::ok(response)
    }
}

impl Server {
    /// Handles Oauth 2.0 authentication redirect.
    pub fn handle_index(&mut self) -> Result<Response<Body>, failure::Error> {
        let inner = self.token_callbacks.read().expect("lock poisoned");

        let mut auth = Vec::new();

        for expected in inner.values() {
            auth.push(Auth {
                url: expected.url.to_string(),
                title: expected.title.to_string(),
            });
        }

        auth.sort_by(|a, b| a.title.cmp(&b.title));

        let data = Data { auth };

        let body = self.reg.render("index", &data)?;

        let mut r = Response::new(Body::from(body));
        r.headers_mut().insert(
            header::CONTENT_TYPE,
            "text/html; charset=utf-8"
                .parse()
                .expect("valid header value"),
        );
        return Ok(r);

        #[derive(serde::Serialize)]
        struct Data {
            auth: Vec<Auth>,
        }

        #[derive(serde::Serialize)]
        struct Auth {
            title: String,
            url: String,
        }
    }

    /// Handles Oauth 2.0 authentication redirect.
    pub fn handle_oauth2_redirect(&mut self, uri: &Uri) -> Response<Body> {
        let query = match uri.query() {
            Some(query) => query,
            None => {
                let mut r = Response::new(Body::from("Missing query in URL"));
                *r.status_mut() = StatusCode::BAD_REQUEST;
                return r;
            }
        };

        let mut state = None;
        let mut code = None;

        for (key, value) in utils::query_pairs(query) {
            let key = match key.decode_utf8().ok() {
                Some(key) => key,
                None => continue,
            };

            let value = match value.and_then(|v| v.decode_utf8().ok()) {
                Some(value) => value,
                None => continue,
            };

            match (key.as_ref(), value) {
                ("state", value) => {
                    state = Some(value);
                }
                ("code", value) => {
                    code = Some(value);
                }
                ("scope", _) => {
                    // ignore
                }
                (key, _) => {
                    log::warn!("unhandled query parameter: {}", key);
                }
            }
        }

        let state = match state {
            Some(state) => String::from(state),
            None => {
                let mut r = Response::new(Body::from("Missing `state` query parameter"));
                *r.status_mut() = StatusCode::BAD_REQUEST;
                return r;
            }
        };

        let code = match code {
            Some(code) => String::from(code),
            None => {
                let mut r = Response::new(Body::from("Missing `code` query parameter"));
                *r.status_mut() = StatusCode::BAD_REQUEST;
                return r;
            }
        };

        let mut inner = self.token_callbacks.write().expect("lock poisoned");

        if let Some(callback) = inner.remove(&state) {
            let _ = callback.channel.send(ReceivedToken { state, code });
            // TODO: return a page that redirects you to the index page.
            let mut r = Response::new(Body::from("Token received, feel free to close the window."));
            *r.status_mut() = StatusCode::TEMPORARY_REDIRECT;
            r.headers_mut()
                .insert(header::LOCATION, URL.parse().expect("valid header value"));
            return r;
        }

        let mut r = Response::new(Body::from("Sorry, I did not expect that :("));
        *r.status_mut() = StatusCode::BAD_REQUEST;
        return r;
    }
}

#[derive(Debug)]
pub struct ReceivedToken {
    pub code: String,
    pub state: String,
}
