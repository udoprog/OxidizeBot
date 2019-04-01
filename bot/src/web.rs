use crate::{bus, player, spotify, utils::BoxFuture};
use futures::{stream, sync::oneshot, Future as _, Sink as _, Stream as _};
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::{fmt, net::SocketAddr, path::Path, sync::Arc};
use warp::{http::Uri, Filter as _};

pub const URL: &'static str = "http://localhost:12345";
pub const REDIRECT_URI: &'static str = "/redirect";

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

struct WithTemplate<T>
where
    T: serde::Serialize,
{
    name: &'static str,
    value: T,
}

fn render<T>(
    template: WithTemplate<T>,
    hbs: &Arc<handlebars::Handlebars>,
) -> Result<impl warp::Reply, Error>
where
    T: serde::Serialize,
{
    let body = hbs
        .render(template.name, &template.value)
        .map_err(failure::Error::from)?;
    Ok(warp::reply::html(body))
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

#[derive(serde::Serialize)]
struct IndexData {
    version: String,
    auth: Vec<Auth>,
    audio_devices: Vec<AudioDevice>,
    current_device: Option<AudioDevice>,
}

/// Index web handler.
#[derive(Clone)]
struct Index {
    player: Arc<RwLock<Option<player::PlayerClient>>>,
    token_callbacks: Arc<RwLock<HashMap<String, ExpectedToken>>>,
}

impl Index {
    fn handle(&self) -> Result<WithTemplate<IndexData>, Error> {
        let mut current_device = None;
        let mut audio_devices = Vec::new();

        if let Some(player) = self.player.read().as_ref() {
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

        let token_callbacks = self.token_callbacks.read();

        let mut auth = Vec::new();

        for expected in token_callbacks.values() {
            auth.push(Auth {
                url: expected.url.to_string(),
                title: expected.title.to_string(),
            });
        }

        auth.sort_by(|a, b| a.title.cmp(&b.title));

        let data = IndexData {
            version: crate::VERSION.to_string(),
            auth,
            audio_devices: audio_devices,
            current_device: current_device,
        };

        return Ok(WithTemplate {
            name: "index",
            value: data,
        });

        /// Convert a spotify device into a string.
        fn device_to_string(device: &spotify::DeviceType) -> &'static str {
            match *device {
                spotify::DeviceType::Computer => "Computer",
                spotify::DeviceType::Smartphone => "Smart Phone",
                spotify::DeviceType::Speaker => "Speaker",
                spotify::DeviceType::Unknown => "Unknown",
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct RedirectQuery {
    state: String,
    code: String,
    #[serde(rename = "scope")]
    _scope: String,
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

/// API to manage device.
#[derive(Clone)]
struct DeviceApi {
    player: Arc<RwLock<Option<player::PlayerClient>>>,
}

impl DeviceApi {
    /// Handle request to set device.
    fn set_device(&self, id: String) -> Result<impl warp::Reply, Error> {
        if let Some(player) = self.player.read().as_ref() {
            let mut audio_devices = Vec::new();

            if let Some(player) = self.player.read().as_ref() {
                audio_devices = player.list_devices().wait()?;
            }

            if let Some(device) = audio_devices.iter().find(|d| d.id == id) {
                player.set_device(device.clone());
            }
        }

        Ok(warp::redirect(Uri::from_static(URL)))
    }
}

/// Set up the web endpoint.
pub fn setup(
    web_root: &Path,
    bus: Arc<bus::Bus>,
) -> Result<(Server, BoxFuture<(), failure::Error>), failure::Error> {
    let mut hb = handlebars::Handlebars::new();
    hb.register_partial("layout", include_str!("web/layout.html.hbs"))?;
    hb.register_template_string("index", include_str!("web/index.html.hbs"))?;

    if !web_root.is_dir() {
        failure::bail!("missing directory: {}", web_root.display());
    }

    let hb = Arc::new(hb);

    let addr: SocketAddr = str::parse(&format!("0.0.0.0:12345"))?;

    let player = Arc::new(RwLock::new(None));
    let token_callbacks = Arc::new(RwLock::new(HashMap::<String, ExpectedToken>::new()));

    let index = Index {
        player: player.clone(),
        token_callbacks: token_callbacks.clone(),
    };

    let index = warp::path::end()
        .and_then(move || index.handle().map_err(warp::reject::custom))
        .and_then({
            let hb = hb.clone();
            move |w| render(w, &hb).map_err(warp::reject::custom)
        })
        .boxed();

    let oauth2_redirect = Oauth2Redirect {
        token_callbacks: token_callbacks.clone(),
    };

    let oauth2_redirect = path!("redirect")
        .and(warp::query::<RedirectQuery>())
        .and_then(move |query| oauth2_redirect.handle(query).map_err(warp::reject::custom))
        .boxed();

    let device_api = DeviceApi {
        player: player.clone(),
    };

    let device_api = path!("api" / "set-device" / String)
        .and_then(move |id| device_api.set_device(id).map_err(warp::reject::custom))
        .boxed();

    let ws = warp::path("ws")
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
                    .forward(tx.sink_map_err(|e| failure::format_err!("error from sink: {}", e)))
                    .map(|_| ())
                    .map_err(|e| {
                        log::error!("websocket error: {}", e);
                    })
            })
        })
        .boxed();

    let static_dir = path!("static")
        .and(warp::fs::dir(web_root.to_owned()))
        .boxed();

    let overlay_html = path!("overlay")
        .and(warp::fs::file(web_root.join("overlay.html")))
        .boxed();

    let page_routes = index.or(overlay_html).boxed();

    let routes = warp::get2()
        .and(
            page_routes
                .or(oauth2_redirect)
                .or(device_api)
                .or(static_dir)
                .or(ws),
        )
        .recover(customize_error);

    let service = warp::serve(routes);

    let server = Server {
        player: player.clone(),
        token_callbacks: token_callbacks.clone(),
    };

    let server_future = service.bind(addr).map_err(|_| {
        // TODO: do we know _why_?
        failure::format_err!("web service errored")
    });

    Ok((server, Box::new(server_future)))
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
fn customize_error(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
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
