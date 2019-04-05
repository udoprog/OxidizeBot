use crate::{bus, db, player, settings, spotify, utils::BoxFuture};
use futures::{future, stream, sync::oneshot, Future, Sink as _, Stream as _};
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::{fmt, net::SocketAddr, path::Path, sync::Arc};
use warp::{http::Uri, Filter as _};

pub const URL: &'static str = "http://localhost:12345";
pub const REDIRECT_URI: &'static str = "/redirect";

const INDEX_HTML: &'static [u8] = include_bytes!("../ui/dist/index.html");
const MAIN_JS: &'static [u8] = include_bytes!("../ui/dist/main.js");

#[derive(Debug)]
enum Error {
    BadRequest,
    Custom(failure::Error),
}

impl From<failure::Error> for Error {
    fn from(value: failure::Error) -> Self {
        Error::Custom(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::BadRequest => "bad request".fmt(fmt),
            Error::Custom(ref err) => err.fmt(fmt),
        }
    }
}

impl std::error::Error for Error {}

/// A token that is expected to be received.
struct ExpectedToken {
    url: url::Url,
    title: String,
    channel: oneshot::Sender<ReceivedToken>,
}

#[derive(Default, serde::Serialize)]
struct Empty {}

const EMPTY: Empty = Empty {};

#[derive(serde::Serialize)]
struct Auth {
    title: String,
    url: String,
}

#[derive(Clone, serde::Serialize)]
struct AudioDevice {
    is_current: bool,
    name: String,
    id: String,
    r#type: String,
}

#[derive(serde::Deserialize)]
struct RedirectQuery {
    state: String,
    code: String,
}

/// Oauth 2.0 redirect handler
#[derive(Clone)]
struct Oauth2Redirect {
    token_callbacks: Arc<RwLock<HashMap<String, ExpectedToken>>>,
}

impl Oauth2Redirect {
    /// Handles Oauth 2.0 authentication redirect.
    fn handle(&self, query: RedirectQuery) -> Result<impl warp::Reply, Error> {
        let mut inner = self.token_callbacks.write();

        if let Some(callback) = inner.remove(&query.state) {
            let _ = callback.channel.send(ReceivedToken {
                state: query.state,
                code: query.code,
            });

            return Ok(warp::redirect(Uri::from_static(URL)));
        }

        Err(Error::BadRequest)
    }
}

#[derive(serde::Deserialize)]
pub struct PutSetting {
    value: serde_json::Value,
}

/// API to manage device.
#[derive(Clone)]
struct Api {
    player: Arc<RwLock<Option<player::PlayerClient>>>,
    token_callbacks: Arc<RwLock<HashMap<String, ExpectedToken>>>,
    after_streams: db::AfterStreams,
    settings: settings::Settings,
}

impl Api {
    /// Handle request to set device.
    fn set_device(&self, id: String) -> BoxFuture<impl warp::Reply, Error> {
        let player = match self.player.read().clone() {
            Some(player) => player,
            None => return Box::new(future::err(Error::BadRequest)),
        };

        let future = player.list_devices().from_err();

        let future = future.and_then({
            move |devices| {
                if let Some(device) = devices.iter().find(|d| d.id == id) {
                    player.set_device(device.clone());
                    return Ok(warp::reply::json(&EMPTY));
                }

                Err(Error::BadRequest)
            }
        });

        Box::new(future)
    }

    /// Get a list of things that need authentication.
    fn auth(&self) -> Result<impl warp::Reply, Error> {
        let mut auth = Vec::new();

        for expected in self.token_callbacks.read().values() {
            auth.push(Auth {
                url: expected.url.to_string(),
                title: expected.title.to_string(),
            });
        }

        auth.sort_by(|a, b| a.title.cmp(&b.title));
        return Ok(warp::reply::json(&auth));
    }

    /// Get a list of things that need authentication.
    fn devices(&self) -> BoxFuture<impl warp::Reply, Error> {
        let player = match self.player.read().clone() {
            Some(player) => player,
            None => {
                let data = Devices::default();
                return Box::new(future::ok(warp::reply::json(&data)));
            }
        };

        let c = player.current_device();
        let future = player.list_devices().from_err();

        let future = future.map(move |data| {
            let mut devices = Vec::new();
            let mut current = None;

            for device in data {
                let is_current = c.as_ref().map(|d| d.id == device.id).unwrap_or_default();

                let device = AudioDevice {
                    name: device.name.to_string(),
                    id: device.id.to_string(),
                    is_current,
                    r#type: device_to_string(&device._type).to_string(),
                };

                if is_current {
                    current = Some(device.clone());
                }

                devices.push(device);
            }

            let data = Devices { devices, current };
            warp::reply::json(&data)
        });

        return Box::new(future);

        /// Convert a spotify device into a string.
        fn device_to_string(device: &spotify::DeviceType) -> &'static str {
            match *device {
                spotify::DeviceType::Computer => "Computer",
                spotify::DeviceType::Smartphone => "Smart Phone",
                spotify::DeviceType::Speaker => "Speaker",
                spotify::DeviceType::Unknown => "Unknown",
            }
        }

        #[derive(Default, serde::Serialize)]
        struct Devices {
            devices: Vec<AudioDevice>,
            current: Option<AudioDevice>,
        }
    }

    /// Get the list of available after streams.
    fn delete_after_stream(&self, id: i32) -> Result<impl warp::Reply, failure::Error> {
        self.after_streams.delete(id)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Get the list of available after streams.
    fn after_streams(&self) -> Result<impl warp::Reply, failure::Error> {
        let after_streams = self.after_streams.list()?;
        Ok(warp::reply::json(&after_streams))
    }

    /// Get the list of all settings in the bot.
    fn settings(&self) -> Result<impl warp::Reply, failure::Error> {
        let settings = self.settings.list()?;
        Ok(warp::reply::json(&settings))
    }

    /// Delete the given setting by key.
    fn delete_setting(&self, key: String) -> Result<impl warp::Reply, failure::Error> {
        self.settings.clear(&key)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given setting by key.
    fn edit_setting(
        &self,
        key: &str,
        value: serde_json::Value,
    ) -> Result<impl warp::Reply, failure::Error> {
        self.settings.set_json(key, value)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Set up the web endpoint.
pub fn setup(
    web_root: Option<&Path>,
    bus: Arc<bus::Bus>,
    after_streams: db::AfterStreams,
    settings: settings::Settings,
) -> Result<(Server, BoxFuture<(), failure::Error>), failure::Error> {
    let addr: SocketAddr = str::parse(&format!("0.0.0.0:12345"))?;

    let player = Arc::new(RwLock::new(None));
    let token_callbacks = Arc::new(RwLock::new(HashMap::<String, ExpectedToken>::new()));

    let oauth2_redirect = Oauth2Redirect {
        token_callbacks: token_callbacks.clone(),
    };

    let oauth2_redirect = warp::get2()
        .and(path!("redirect"))
        .and(warp::query::<RedirectQuery>())
        .and_then(move |query| oauth2_redirect.handle(query).map_err(warp::reject::custom))
        .boxed();

    let api = Api {
        player: player.clone(),
        token_callbacks: token_callbacks.clone(),
        after_streams,
        settings,
    };

    let api = {
        let route = warp::post2()
            .and(path!("device" / String))
            .and_then({
                let api = api.clone();
                move |id| api.set_device(id).map_err(warp::reject::custom)
            })
            .boxed();

        let route = route
            .or(warp::get2().and(warp::path("auth")).and_then({
                let api = api.clone();
                move || api.auth().map_err(warp::reject::custom)
            }))
            .boxed();

        let route = route
            .or(warp::get2().and(warp::path("devices")).and_then({
                let api = api.clone();
                move || api.devices().map_err(warp::reject::custom)
            }))
            .boxed();

        let route = route
            .or(warp::delete2().and(path!("after-stream" / i32)).and_then({
                let api = api.clone();
                move |id| api.delete_after_stream(id).map_err(warp::reject::custom)
            }))
            .boxed();

        let route = route
            .or(warp::get2().and(warp::path("after-streams")).and_then({
                let api = api.clone();
                move || api.after_streams().map_err(warp::reject::custom)
            }))
            .boxed();

        let route = route
            .or(warp::delete2().and(path!("setting" / String)).and_then({
                let api = api.clone();
                move |key| api.delete_setting(key).map_err(warp::reject::custom)
            }))
            .boxed();

        let route = route
            .or(warp::put2()
                .and(warp::path("setting"))
                .and(warp::filters::path::tail().and(warp::body::json()))
                .and_then({
                    let api = api.clone();
                    move |key: warp::filters::path::Tail, body: PutSetting| {
                        api.edit_setting(key.as_str(), body.value)
                            .map_err(warp::reject::custom)
                    }
                }))
            .boxed();

        let route = route
            .or(warp::get2().and(warp::path("settings")).and_then({
                let api = api.clone();
                move || api.settings().map_err(warp::reject::custom)
            }))
            .boxed();

        warp::path("api").and(route)
    };

    let ws = {
        let route = warp::path!("overlay")
            .and(warp::ws2())
            .map(move |ws: warp::ws::Ws2| {
                let bus = bus.clone();

                ws.on_upgrade(move |websocket| {
                    let (tx, _) = websocket.split();

                    let rx = stream::iter_ok(bus.latest()).chain(bus.add_rx());

                    rx.map_err(|_| failure::format_err!("failed to receive notification"))
                        .and_then(|n| {
                            serde_json::to_string(&n)
                                .map(warp::filters::ws::Message::text)
                                .map_err(failure::Error::from)
                        })
                        .forward(
                            tx.sink_map_err(|e| failure::format_err!("error from sink: {}", e)),
                        )
                        .map(|_| ())
                        .map_err(|e| {
                            log::error!("websocket error: {}", e);
                        })
                })
            })
            .boxed();

        warp::get2()
            .and(warp::path("ws"))
            .and(route)
            .recover(recover)
    };

    let routes = oauth2_redirect.recover(recover);
    let routes = routes.or(api.recover(recover));
    let routes = routes.or(ws.recover(recover));

    let server_future = if let Some(web_root) = web_root {
        let app = warp::get2()
            .and(warp::path("main.js"))
            .and(warp::filters::fs::file(web_root.join("main.js")));
        let app = app.or(warp::get2().and(warp::filters::fs::file(web_root.join("index.html"))));
        let routes = routes.or(app.recover(recover));

        let service = warp::serve(routes);

        let server_future = service.bind(addr).map_err(|_| {
            // TODO: do we know _why_?
            failure::format_err!("web service errored")
        });

        Box::new(server_future) as BoxFuture<(), failure::Error>
    } else {
        let app = warp::get2().and(warp::path("main.js")).map(|| {
            use warp::http::Response;
            Response::builder().body(MAIN_JS)
        });
        let app = app.or(warp::get2().map(|| warp::reply::html(INDEX_HTML)));
        let routes = routes.or(app.recover(recover));
        let service = warp::serve(routes);

        let server_future = service.bind(addr).map_err(|_| {
            // TODO: do we know _why_?
            failure::format_err!("web service errored")
        });

        Box::new(server_future) as BoxFuture<(), failure::Error>
    };

    let server = Server {
        player: player.clone(),
        token_callbacks: token_callbacks.clone(),
    };

    Ok((server, server_future))
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
fn recover(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(e) = err.find_cause::<Error>() {
        let code = match *e {
            Error::BadRequest => warp::http::StatusCode::BAD_REQUEST,
            Error::Custom(_) => warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        let msg = e.to_string();

        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: msg,
        });

        Ok(warp::reply::with_status(json, code))
    } else {
        // Could be a NOT_FOUND, or METHOD_NOT_ALLOWED... here we just
        // let warp use its default rendering.
        Err(err)
    }
}

#[derive(serde::Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

/// Interface to the server.
#[derive(Clone)]
pub struct Server {
    player: Arc<RwLock<Option<player::PlayerClient>>>,
    /// Callbacks for when we have received a token.
    token_callbacks: Arc<RwLock<HashMap<String, ExpectedToken>>>,
}

impl Server {
    /// Set the player interface.
    pub fn set_player(&self, player: player::PlayerClient) {
        *self.player.write() = Some(player);
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
        let mut inner = self.token_callbacks.write();

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

#[derive(Debug)]
pub struct ReceivedToken {
    pub code: String,
    pub state: String,
}
