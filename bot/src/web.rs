use crate::{
    player, spotify,
    utils::{self, BoxFuture},
};
use futures::{future, sync::oneshot, Future as _};
use hashbrown::HashMap;
use hyper::{
    body::Body, error, header, server, service, Method, Request, Response, StatusCode, Uri,
};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

pub const URL: &'static str = "http://localhost:12345";
pub const REDIRECT_URI: &'static str = "/redirect";

pub fn setup() -> Result<(Server, BoxFuture<(), error::Error>), failure::Error> {
    let mut reg = handlebars::Handlebars::new();
    reg.register_partial("layout", include_str!("web/layout.html.hbs"))?;
    reg.register_template_string("index", include_str!("web/index.html.hbs"))?;

    let server = Server::new(Arc::new(reg));

    let addr: SocketAddr = str::parse(&format!("0.0.0.0:12345"))?;

    // TODO: add graceful shutdown.
    let server_future = server::Server::bind(&addr).serve({
        let server = server.clone();
        move || future::ok::<Server, error::Error>(server.clone())
    });

    Ok((server, Box::new(server_future)))
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
    /// Player interface.
    player: Arc<RwLock<Option<player::PlayerClient>>>,
}

impl Server {
    /// Construct a new server.
    pub fn new(reg: Arc<handlebars::Handlebars>) -> Self {
        Self {
            reg,
            token_callbacks: Arc::new(RwLock::new(HashMap::default())),
            player: Default::default(),
        }
    }

    /// Set the player interface.
    pub fn set_player(&self, player: player::PlayerClient) {
        *self.player.write().expect("poisoned") = Some(player);
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

        let mut it = uri.path().split("/");
        it.next();

        let route = (req.method(), (it.next(), it.next(), it.next()));

        let result = match route {
            (&Method::GET, (Some("redirect"), None, None)) => self.handle_oauth2_redirect(uri),
            (&Method::GET, (Some("api"), Some("set-device"), Some(id))) => {
                self.handle_set_device(id)
            }
            (&Method::GET, (Some(""), None, None)) => self.handle_index(),
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
        let mut current_device = None;
        let mut audio_devices = Vec::new();

        if let Some(player) = self.player.read().expect("poisoned").as_ref() {
            let current = player.current_device();

            for device in player.list_devices().wait()? {
                let current = current
                    .as_ref()
                    .map(|d| d.id == device.id)
                    .unwrap_or_default();

                audio_devices.push(AudioDevice {
                    name: device.name.to_string(),
                    id: device.id.to_string(),
                    current,
                    r#type: device_to_string(&device._type).to_string(),
                })
            }

            current_device = current.map(|d| AudioDevice {
                name: d.name.to_string(),
                id: d.id.to_string(),
                current: true,
                r#type: device_to_string(&d._type).to_string(),
            })
        }

        let token_callbacks = self.token_callbacks.read().expect("lock poisoned");

        let mut auth = Vec::new();

        for expected in token_callbacks.values() {
            auth.push(Auth {
                url: expected.url.to_string(),
                title: expected.title.to_string(),
            });
        }

        auth.sort_by(|a, b| a.title.cmp(&b.title));

        let data = Data {
            auth,
            audio_devices: &audio_devices,
            current_device: current_device.as_ref(),
        };

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
        struct Data<'a> {
            auth: Vec<Auth>,
            audio_devices: &'a [AudioDevice],
            current_device: Option<&'a AudioDevice>,
        }

        #[derive(serde::Serialize)]
        struct Auth {
            title: String,
            url: String,
        }

        #[derive(serde::Serialize)]
        struct AudioDevice {
            current: bool,
            name: String,
            id: String,
            r#type: String,
        }
    }

    /// Handle request to set device.
    fn handle_set_device(&mut self, id: &str) -> Result<Response<Body>, failure::Error> {
        if let Some(player) = self.player.read().expect("poisoned").as_ref() {
            let mut audio_devices = Vec::new();

            if let Some(player) = self.player.read().expect("poisoned").as_ref() {
                audio_devices = player.list_devices().wait()?;
            }

            if let Some(device) = audio_devices.iter().find(|d| d.id == id) {
                player.set_device(device.clone());
            }
        }

        return redirect(URL);
    }

    /// Handles Oauth 2.0 authentication redirect.
    fn handle_oauth2_redirect(&mut self, uri: &Uri) -> Result<Response<Body>, failure::Error> {
        let query = match uri.query() {
            Some(query) => query,
            None => {
                return bad_request("Missing query in URL");
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
                return bad_request("Missing `state` query parameter");
            }
        };

        let code = match code {
            Some(code) => String::from(code),
            None => {
                return bad_request("Missing `code` query parameter");
            }
        };

        let mut inner = self.token_callbacks.write().expect("lock poisoned");

        if let Some(callback) = inner.remove(&state) {
            let _ = callback.channel.send(ReceivedToken { state, code });
            return redirect(URL);
        }

        bad_request("Sorry, I did not expect that :(")
    }
}

/// Convert a spotify device into a string.
fn device_to_string(device: &spotify::DeviceType) -> &'static str {
    match *device {
        spotify::DeviceType::Computer => "Computer",
        spotify::DeviceType::Smartphone => "Smart Phone",
        spotify::DeviceType::Speaker => "Speaker",
        spotify::DeviceType::Unknown => "Unknown",
    }
}

fn redirect(url: &str) -> Result<Response<Body>, failure::Error> {
    // TODO: return a page that redirects you to the index page.
    let mut r = Response::new(Body::from(format!("Being redirected to: {}", url)));
    *r.status_mut() = StatusCode::TEMPORARY_REDIRECT;
    r.headers_mut().insert(header::LOCATION, URL.parse()?);
    Ok(r)
}

fn bad_request(what: &str) -> Result<Response<Body>, failure::Error> {
    let mut r = Response::new(Body::from(what.to_string()));
    *r.status_mut() = StatusCode::BAD_REQUEST;
    Ok(r)
}

#[derive(Debug)]
pub struct ReceivedToken {
    pub code: String,
    pub state: String,
}
