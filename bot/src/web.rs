use crate::{
    api, auth, bus, currency::Currency, db, player, prelude::*, settings, template,
    track_id::TrackId, utils,
};
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
    NotFound,
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
            Error::NotFound => "not found".fmt(fmt),
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

#[derive(serde::Serialize)]
pub struct Current {
    channel: String,
}

#[derive(serde::Deserialize)]
pub struct PutSetting {
    value: serde_json::Value,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DisabledBody {
    disabled: bool,
}

/// Aliases endpoint.
#[derive(Clone)]
struct Aliases(db::Aliases);

impl Aliases {
    fn route(aliases: db::Aliases) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Aliases(aliases);

        let list = warp::get2()
            .and(path!("aliases" / Fragment).and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| api.list(channel.as_str()).map_err(warp::reject::custom)
            });

        let delete = warp::delete2()
            .and(path!("aliases" / Fragment / Fragment).and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    api.delete(channel.as_str(), name.as_str())
                        .map_err(warp::reject::custom)
                }
            });

        let edit = warp::put2()
            .and(path!("aliases" / Fragment / Fragment).and(warp::filters::path::end()))
            .and(warp::body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: PutAlias| {
                    api.edit(channel.as_str(), name.as_str(), body.template)
                        .map_err(warp::reject::custom)
                }
            });

        let edit_disabled = warp::post2()
            .and(
                path!("aliases" / Fragment / Fragment / "disabled").and(warp::filters::path::end()),
            )
            .and(warp::body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: DisabledBody| {
                    api.edit_disabled(channel.as_str(), name.as_str(), body.disabled)
                        .map_err(warp::reject::custom)
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(serde::Deserialize)]
        pub struct PutAlias {
            template: template::Template,
        }
    }

    /// Get the list of all aliases.
    fn list(&self, channel: &str) -> Result<impl warp::Reply, failure::Error> {
        let aliases = self.0.list_all(channel)?;
        Ok(warp::reply::json(&aliases))
    }

    /// Edit the given alias by key.
    fn edit(
        &self,
        channel: &str,
        name: &str,
        template: template::Template,
    ) -> Result<impl warp::Reply, failure::Error> {
        self.0.edit(channel, name, template)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given alias's disabled status.
    fn edit_disabled(
        &self,
        channel: &str,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply, failure::Error> {
        if disabled {
            self.0.disable(channel, name)?;
        } else {
            self.0.enable(channel, name)?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given alias by key.
    fn delete(&self, channel: &str, name: &str) -> Result<impl warp::Reply, failure::Error> {
        self.0.delete(channel, name)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Commands endpoint.
#[derive(Clone)]
struct Commands(db::Commands);

impl Commands {
    fn route(commands: db::Commands) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Commands(commands);

        let list = warp::get2()
            .and(path!("commands" / Fragment).and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| api.list(channel.as_str()).map_err(warp::reject::custom)
            });

        let delete = warp::delete2()
            .and(path!("commands" / Fragment / Fragment).and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    api.delete(channel.as_str(), name.as_str())
                        .map_err(warp::reject::custom)
                }
            });

        let edit_disabled = warp::post2()
            .and(
                path!("commands" / Fragment / Fragment / "disabled")
                    .and(warp::filters::path::end()),
            )
            .and(warp::body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: DisabledBody| {
                    api.edit_disabled(channel.as_str(), name.as_str(), body.disabled)
                        .map_err(warp::reject::custom)
                }
            });

        let edit = warp::put2()
            .and(path!("commands" / Fragment / Fragment).and(warp::filters::path::end()))
            .and(warp::body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: PutCommand| {
                    api.edit(channel.as_str(), name.as_str(), body.template)
                        .map_err(warp::reject::custom)
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(serde::Deserialize)]
        pub struct PutCommand {
            template: template::Template,
        }
    }

    /// Get the list of all commands.
    fn list(&self, channel: &str) -> Result<impl warp::Reply, failure::Error> {
        let commands = self.0.list_all(channel)?;
        Ok(warp::reply::json(&commands))
    }

    /// Edit the given command by key.
    fn edit(
        &self,
        channel: &str,
        name: &str,
        template: template::Template,
    ) -> Result<impl warp::Reply, failure::Error> {
        self.0.edit(channel, name, template)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given command's disabled status.
    fn edit_disabled(
        &self,
        channel: &str,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply, failure::Error> {
        if disabled {
            self.0.disable(channel, name)?;
        } else {
            self.0.enable(channel, name)?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given command by key.
    fn delete(&self, channel: &str, name: &str) -> Result<impl warp::Reply, failure::Error> {
        self.0.delete(channel, name)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Promotions endpoint.
#[derive(Clone)]
struct Promotions(db::Promotions);

impl Promotions {
    fn route(promotions: db::Promotions) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Promotions(promotions);

        let list = warp::get2()
            .and(path!("promotions" / Fragment).and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| api.list(channel.as_str()).map_err(warp::reject::custom)
            });

        let delete = warp::delete2()
            .and(path!("promotions" / Fragment / Fragment).and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    api.delete(channel.as_str(), name.as_str())
                        .map_err(warp::reject::custom)
                }
            });

        let edit = warp::put2()
            .and(path!("promotions" / Fragment / Fragment).and(warp::filters::path::end()))
            .and(warp::body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: PutPromotion| {
                    api.edit(
                        channel.as_str(),
                        name.as_str(),
                        body.frequency,
                        body.template,
                    )
                    .map_err(warp::reject::custom)
                }
            });

        let edit_disabled = warp::post2()
            .and(
                path!("promotions" / Fragment / Fragment / "disabled")
                    .and(warp::filters::path::end()),
            )
            .and(warp::body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: DisabledBody| {
                    api.edit_disabled(channel.as_str(), name.as_str(), body.disabled)
                        .map_err(warp::reject::custom)
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(serde::Deserialize)]
        pub struct PutPromotion {
            frequency: utils::Duration,
            template: template::Template,
        }
    }

    /// Get the list of all promotions.
    fn list(&self, channel: &str) -> Result<impl warp::Reply, failure::Error> {
        let promotions = self.0.list_all(channel)?;
        Ok(warp::reply::json(&promotions))
    }

    /// Edit the given promotion by key.
    fn edit(
        &self,
        channel: &str,
        name: &str,
        frequency: utils::Duration,
        template: template::Template,
    ) -> Result<impl warp::Reply, failure::Error> {
        self.0.edit(channel, name, frequency, template)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given promotion's disabled status.
    fn edit_disabled(
        &self,
        channel: &str,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply, failure::Error> {
        if disabled {
            self.0.disable(channel, name)?;
        } else {
            self.0.enable(channel, name)?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given promotion by key.
    fn delete(&self, channel: &str, name: &str) -> Result<impl warp::Reply, failure::Error> {
        self.0.delete(channel, name)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Themes endpoint.
#[derive(Clone)]
struct Themes(db::Themes);

impl Themes {
    fn route(themes: db::Themes) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Themes(themes);

        let list = warp::get2()
            .and(path!("themes" / Fragment).and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| api.list(channel.as_str()).map_err(warp::reject::custom)
            });

        let delete = warp::delete2()
            .and(path!("themes" / Fragment / Fragment).and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    api.delete(channel.as_str(), name.as_str())
                        .map_err(warp::reject::custom)
                }
            });

        let edit = warp::put2()
            .and(path!("themes" / Fragment / Fragment).and(warp::filters::path::end()))
            .and(warp::body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: PutTheme| {
                    api.edit(channel.as_str(), name.as_str(), body.track_id)
                        .map_err(warp::reject::custom)
                }
            });

        let edit_disabled = warp::post2()
            .and(path!("themes" / Fragment / Fragment / "disabled").and(warp::filters::path::end()))
            .and(warp::body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: DisabledBody| {
                    api.edit_disabled(channel.as_str(), name.as_str(), body.disabled)
                        .map_err(warp::reject::custom)
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(serde::Deserialize)]
        pub struct PutTheme {
            track_id: TrackId,
        }
    }

    /// Get the list of all promotions.
    fn list(&self, channel: &str) -> Result<impl warp::Reply, failure::Error> {
        let promotions = self.0.list_all(channel)?;
        Ok(warp::reply::json(&promotions))
    }

    /// Edit the given promotion by key.
    fn edit(
        &self,
        channel: &str,
        name: &str,
        track_id: TrackId,
    ) -> Result<impl warp::Reply, failure::Error> {
        self.0.edit(channel, name, track_id)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given promotion's disabled status.
    fn edit_disabled(
        &self,
        channel: &str,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply, failure::Error> {
        if disabled {
            self.0.disable(channel, name)?;
        } else {
            self.0.enable(channel, name)?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given promotion by key.
    fn delete(&self, channel: &str, name: &str) -> Result<impl warp::Reply, failure::Error> {
        self.0.delete(channel, name)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Auth API endpoints.
#[derive(Clone)]
struct Auth {
    token_callbacks: Arc<RwLock<HashMap<String, ExpectedToken>>>,
    auth: auth::Auth,
}

impl Auth {
    fn route(
        auth: auth::Auth,
        token_callbacks: Arc<RwLock<HashMap<String, ExpectedToken>>>,
    ) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Auth {
            auth,
            token_callbacks,
        };

        let route = warp::get2()
            .and(warp::path!("pending").and(warp::filters::path::end()))
            .and_then({
                let api = api.clone();
                move || api.pending().map_err(warp::reject::custom)
            })
            .boxed();

        let route = route
            .or(warp::get2()
                .and(warp::path!("roles").and(warp::filters::path::end()))
                .and_then({
                    let api = api.clone();
                    move || api.roles().map_err(warp::reject::custom)
                }))
            .boxed();

        let route = route
            .or(warp::get2()
                .and(warp::path!("scopes").and(warp::filters::path::end()))
                .and_then({
                    let api = api.clone();
                    move || api.scopes().map_err(warp::reject::custom)
                }))
            .boxed();

        let route = route
            .or(warp::get2()
                .and(warp::path!("grants").and(warp::filters::path::end()))
                .and_then({
                    let api = api.clone();
                    move || api.grants().map_err(warp::reject::custom)
                }))
            .boxed();

        let route = route
            .or(warp::put2()
                .and(warp::path!("grants").and(warp::filters::path::end()))
                .and(warp::body::json())
                .and_then({
                    let api = api.clone();
                    move |body: PutGrant| {
                        api.insert_grant(body.scope, body.role)
                            .map_err(warp::reject::custom)
                    }
                }))
            .boxed();

        let route = route
            .or(warp::delete2()
                .and(warp::path!("grants" / Fragment / Fragment).and(warp::filters::path::end()))
                .and_then({
                    let api = api.clone();
                    move |scope: Fragment, role: Fragment| {
                        api.delete_grant(scope.as_str(), role.as_str())
                            .map_err(warp::reject::custom)
                    }
                }))
            .boxed();

        return route;

        #[derive(serde::Deserialize)]
        pub struct PutGrant {
            scope: auth::Scope,
            role: auth::Role,
        }
    }

    /// Get a list of things that need authentication.
    fn pending(&self) -> Result<impl warp::Reply, Error> {
        let mut auth = Vec::new();

        for expected in self.token_callbacks.read().values() {
            auth.push(AuthPending {
                url: expected.url.to_string(),
                title: expected.title.to_string(),
            });
        }

        auth.sort_by(|a, b| a.title.cmp(&b.title));
        return Ok(warp::reply::json(&auth));

        #[derive(serde::Serialize)]
        struct AuthPending {
            title: String,
            url: String,
        }
    }

    /// Get the list of all scopes.
    fn scopes(&self) -> Result<impl warp::Reply, failure::Error> {
        let scopes = self.auth.scopes();
        Ok(warp::reply::json(&scopes))
    }

    /// Get the list of all roles.
    fn roles(&self) -> Result<impl warp::Reply, failure::Error> {
        let roles = self.auth.roles();
        Ok(warp::reply::json(&roles))
    }

    /// Get the list of all auth in the bot.
    fn grants(&self) -> Result<impl warp::Reply, failure::Error> {
        let auth = self.auth.list();
        Ok(warp::reply::json(&auth))
    }

    /// Delete a single scope assignment.
    fn delete_grant(&self, scope: &str, role: &str) -> Result<impl warp::Reply, failure::Error> {
        let scope = str::parse(scope)?;
        let role = str::parse(role)?;
        self.auth.delete(scope, role)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Insert a single scope assignment.
    fn insert_grant(
        &self,
        scope: auth::Scope,
        role: auth::Role,
    ) -> Result<impl warp::Reply, failure::Error> {
        self.auth.insert(scope, role)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// API to manage device.
#[derive(Clone)]
struct Api {
    player: Arc<RwLock<Option<player::PlayerClient>>>,
    after_streams: db::AfterStreams,
    db: db::Database,
    settings: settings::Settings,
    currency: Arc<RwLock<Option<Currency>>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Balance {
    name: String,
    balance: i64,
}

impl Api {
    /// Handle request to set device.
    async fn set_device(self, id: String) -> Result<impl warp::Reply, Error> {
        let player = match self.player.read().clone() {
            Some(player) => player,
            None => return Err(Error::BadRequest),
        };

        let devices = player.list_devices().await?;

        if let Some(device) = devices.iter().find(|d| d.id == id) {
            player.set_device(device.clone());
            return Ok(warp::reply::json(&EMPTY));
        }

        Err(Error::BadRequest)
    }

    /// Get a list of things that need authentication.
    async fn devices(self) -> Result<impl warp::Reply, Error> {
        let player = match self.player.read().clone() {
            Some(player) => player,
            None => {
                let data = Devices::default();
                return Ok(warp::reply::json(&data));
            }
        };

        let c = player.current_device();
        let data = player.list_devices().await?;

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
        return Ok(warp::reply::json(&data));

        /// Convert a spotify device into a string.
        fn device_to_string(device: &api::spotify::DeviceType) -> &'static str {
            match *device {
                api::spotify::DeviceType::Computer => "Computer",
                api::spotify::DeviceType::Smartphone => "Smart Phone",
                api::spotify::DeviceType::Speaker => "Speaker",
                api::spotify::DeviceType::Unknown => "Unknown",
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
    fn delete_setting(&self, key: &str) -> Result<impl warp::Reply, failure::Error> {
        self.settings.clear(key)?;
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

    /// Import balances.
    async fn import_balances(
        self,
        balances: Vec<db::models::Balance>,
    ) -> Result<impl warp::Reply, Error> {
        let currency = self.currency.read().as_ref().cloned();

        match currency {
            Some(currency) => currency.import_balances(balances).await?,
            None => return Err(Error::NotFound),
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Export balances.
    async fn export_balances(self) -> Result<impl warp::Reply, Error> {
        let currency = self.currency.read().as_ref().cloned();

        let balances = match currency {
            Some(currency) => currency.export_balances().await?,
            None => return Err(Error::NotFound),
        };

        Ok(warp::reply::json(&balances))
    }
}

/// Set up the web endpoint.
pub fn setup(
    web_root: Option<&Path>,
    global_bus: Arc<bus::Bus<bus::Global>>,
    youtube_bus: Arc<bus::Bus<bus::YouTube>>,
    after_streams: db::AfterStreams,
    db: db::Database,
    settings: settings::Settings,
    auth: auth::Auth,
    aliases: db::Aliases,
    commands: db::Commands,
    promotions: db::Promotions,
    themes: db::Themes,
    channel: Arc<RwLock<Option<String>>>,
    currency: Arc<RwLock<Option<Currency>>>,
) -> Result<
    (
        Server,
        future::BoxFuture<'static, Result<(), failure::Error>>,
    ),
    failure::Error,
> {
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
        after_streams,
        db,
        settings,
        currency,
    };

    let api = {
        let route = warp::post2()
            .and(path!("device" / String))
            .and_then({
                let api = api.clone();
                move |id| {
                    api.clone()
                        .set_device(id)
                        .map_err(warp::reject::custom)
                        .boxed()
                        .compat()
                }
            })
            .boxed();

        let route = route
            .or(warp::get2().and(warp::path("devices")).and_then({
                let api = api.clone();
                move || {
                    api.clone()
                        .devices()
                        .map_err(warp::reject::custom)
                        .boxed()
                        .compat()
                }
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
            .or(warp::delete2()
                .and(warp::path("setting").and(warp::filters::path::tail()))
                .and_then({
                    let api = api.clone();
                    move |key: warp::filters::path::Tail| {
                        let key =
                            str::parse::<Fragment>(key.as_str()).map_err(warp::reject::custom)?;
                        api.delete_setting(key.as_str())
                            .map_err(warp::reject::custom)
                    }
                }))
            .boxed();

        let route = route
            .or(warp::put2()
                .and(warp::path("setting"))
                .and(warp::filters::path::tail().and(warp::body::json()))
                .and_then({
                    let api = api.clone();
                    move |key: warp::filters::path::Tail, body: PutSetting| {
                        let key =
                            str::parse::<Fragment>(key.as_str()).map_err(warp::reject::custom)?;
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

        let route = route
            .or(warp::put2()
                .and(warp::path("balances"))
                .and(warp::body::json())
                .and_then({
                    let api = api.clone();
                    move |balances: Vec<db::models::Balance>| {
                        api.clone()
                            .import_balances(balances)
                            .map_err(warp::reject::custom)
                            .boxed()
                            .compat()
                    }
                }))
            .boxed();

        let route = route
            .or(warp::get2().and(warp::path("balances")).and_then({
                let api = api.clone();
                move || {
                    api.clone()
                        .export_balances()
                        .map_err(warp::reject::custom)
                        .boxed()
                        .compat()
                }
            }))
            .boxed();

        let route = route.or(warp::path("auth")
            .and(Auth::route(auth, token_callbacks.clone()))
            .boxed());
        let route = route.or(Aliases::route(aliases));
        let route = route.or(Commands::route(commands));
        let route = route.or(Promotions::route(promotions));
        let route = route.or(Themes::route(themes));

        let route = route
            .or(warp::get2()
                .and(path!("current").and(warp::filters::path::end()))
                .and_then(move || {
                    let channel = channel.read();

                    let channel = match channel.as_ref() {
                        Some(channel) => channel,
                        None => return Err(warp::reject::not_found()),
                    };

                    let current = Current {
                        channel: channel.to_string(),
                    };

                    Ok::<_, warp::Rejection>(warp::reply::json(&current))
                }))
            .boxed();

        warp::path("api").and(route)
    };

    let ws_overlay = warp::get2()
        .and(warp::path!("ws" / "overlay"))
        .and(send_bus(global_bus).recover(recover));

    let ws_youtube = warp::get2()
        .and(warp::path!("ws" / "youtube"))
        .and(send_bus(youtube_bus).recover(recover));

    let routes = oauth2_redirect.recover(recover);
    let routes = routes.or(api.recover(recover));
    let routes = routes.or(ws_youtube.recover(recover));
    let routes = routes.or(ws_overlay.recover(recover));

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

        server_future.compat().boxed()
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

        server_future.compat().boxed()
    };

    let server = Server {
        player: player.clone(),
        token_callbacks: token_callbacks.clone(),
    };

    Ok((server, server_future))
}

struct Fragment {
    string: String,
}

impl Fragment {
    /// Borrow as a string slice.
    pub fn as_str(&self) -> &str {
        self.string.as_str()
    }
}

impl std::str::FromStr for Fragment {
    type Err = std::str::Utf8Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = percent_encoding::percent_decode(s.as_bytes()).decode_utf8()?;
        Ok(Fragment {
            string: s.to_string(),
        })
    }
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
fn recover(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(e) = err.find_cause::<Error>() {
        let code = match *e {
            Error::BadRequest => warp::http::StatusCode::BAD_REQUEST,
            Error::NotFound => warp::http::StatusCode::NOT_FOUND,
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

/// Connecting a bus to a websocket connection.
fn send_bus<T>(bus: Arc<bus::Bus<T>>) -> warp::filters::BoxedFilter<(impl warp::Reply,)>
where
    T: bus::Message,
{
    warp::ws2()
        .map({
            let bus = bus.clone();

            move |ws: warp::ws::Ws2| {
                let bus = bus.clone();

                ws.on_upgrade(move |websocket| {
                    let (tx, _) = websocket.split();

                    let rx = stream01::iter_ok(bus.latest()).chain(bus.add_rx());

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
            }
        })
        .boxed()
}
