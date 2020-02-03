use self::assets::Asset;
use crate::{
    api, api::setbac::ConnectionMeta, auth, bus, currency::Currency, db, injector, message_log,
    player, prelude::*, template, track_id::TrackId, utils,
};
use anyhow::bail;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::{borrow::Cow, collections::HashMap, fmt, net::SocketAddr, sync::Arc};
use warp::{body, filters, path, Filter as _};

mod cache;
mod chat;
mod settings;

use self::{cache::Cache, chat::Chat, settings::Settings};

pub const URL: &str = "http://localhost:12345";

mod assets {
    #[derive(rust_embed::RustEmbed)]
    #[folder = "$CARGO_MANIFEST_DIR/../bot-ui/dist"]
    pub struct Asset;
}

#[derive(Debug)]
struct CustomReject(anyhow::Error);

impl warp::reject::Reject for CustomReject {}

pub(crate) fn custom_reject(error: impl Into<anyhow::Error>) -> warp::Rejection {
    warp::reject::custom(CustomReject(error.into()))
}

#[derive(Debug)]
enum Error {
    BadRequest,
    NotFound,
    Custom(anyhow::Error),
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
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

#[derive(serde::Serialize)]
pub struct Current {
    channel: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DisabledBody {
    disabled: bool,
}

/// Aliases endpoint.
#[derive(Clone)]
struct Aliases(Arc<RwLock<Option<db::Aliases>>>);

impl Aliases {
    fn route(
        aliases: Arc<RwLock<Option<db::Aliases>>>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Aliases(aliases);

        let list = warp::get()
            .and(path!("aliases" / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| {
                    let api = api.clone();
                    async move { api.list(channel.as_str()).map_err(custom_reject) }
                }
            });

        let delete = warp::delete()
            .and(path!("aliases" / Fragment / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    let api = api.clone();
                    async move {
                        api.delete(channel.as_str(), name.as_str())
                            .map_err(custom_reject)
                    }
                }
            });

        let edit = warp::put()
            .and(path!("aliases" / Fragment / Fragment).and(path::end()))
            .and(body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: PutAlias| {
                    let api = api.clone();
                    async move {
                        api.edit(channel.as_str(), name.as_str(), body.template)
                            .map_err(custom_reject)
                    }
                }
            });

        let edit_disabled = warp::post()
            .and(path!("aliases" / Fragment / Fragment / "disabled").and(path::end()))
            .and(body::json())
            .and_then({
                move |channel: Fragment, name: Fragment, body: DisabledBody| {
                    let api = api.clone();
                    async move {
                        api.edit_disabled(channel.as_str(), name.as_str(), body.disabled)
                            .map_err(custom_reject)
                    }
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(serde::Deserialize)]
        pub struct PutAlias {
            template: template::Template,
        }
    }

    /// Access underlying aliases abstraction.
    fn aliases(&self) -> Result<MappedRwLockReadGuard<'_, db::Aliases>, anyhow::Error> {
        match RwLockReadGuard::try_map(self.0.read(), |c| c.as_ref()) {
            Ok(out) => Ok(out),
            Err(_) => bail!("aliases not configured"),
        }
    }

    /// Get the list of all aliases.
    fn list(&self, channel: &str) -> Result<impl warp::Reply, anyhow::Error> {
        let aliases = self.aliases()?.list_all(channel)?;
        Ok(warp::reply::json(&aliases))
    }

    /// Edit the given alias by key.
    fn edit(
        &self,
        channel: &str,
        name: &str,
        template: template::Template,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        self.aliases()?.edit(channel, name, template)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given alias's disabled status.
    fn edit_disabled(
        &self,
        channel: &str,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        if disabled {
            self.aliases()?.disable(channel, name)?;
        } else {
            self.aliases()?.enable(channel, name)?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given alias by key.
    fn delete(&self, channel: &str, name: &str) -> Result<impl warp::Reply, anyhow::Error> {
        self.aliases()?.delete(channel, name)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Commands endpoint.
#[derive(Clone)]
struct Commands(Arc<RwLock<Option<db::Commands>>>);

impl Commands {
    fn route(
        commands: Arc<RwLock<Option<db::Commands>>>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Commands(commands);

        let list = warp::get()
            .and(path!("commands" / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| {
                    let api = api.clone();
                    async move { api.list(channel.as_str()).map_err(custom_reject) }
                }
            });

        let delete = warp::delete()
            .and(path!("commands" / Fragment / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    let api = api.clone();
                    async move {
                        api.delete(channel.as_str(), name.as_str())
                            .map_err(custom_reject)
                    }
                }
            });

        let edit_disabled = warp::post()
            .and(path!("commands" / Fragment / Fragment / "disabled").and(path::end()))
            .and(body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: DisabledBody| {
                    let api = api.clone();

                    async move {
                        api.edit_disabled(channel.as_str(), name.as_str(), body.disabled)
                            .map_err(custom_reject)
                    }
                }
            });

        let edit = warp::put()
            .and(path!("commands" / Fragment / Fragment).and(path::end()))
            .and(body::json())
            .and_then({
                move |channel: Fragment, name: Fragment, body: PutCommand| {
                    let api = api.clone();
                    async move {
                        api.edit(channel.as_str(), name.as_str(), body.template)
                            .map_err(custom_reject)
                    }
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(serde::Deserialize)]
        pub struct PutCommand {
            template: template::Template,
        }
    }

    /// Access underlying commands abstraction.
    fn commands(&self) -> Result<MappedRwLockReadGuard<'_, db::Commands>, anyhow::Error> {
        match RwLockReadGuard::try_map(self.0.read(), |c| c.as_ref()) {
            Ok(out) => Ok(out),
            Err(_) => bail!("commands not configured"),
        }
    }

    /// Get the list of all commands.
    fn list(&self, channel: &str) -> Result<impl warp::Reply, anyhow::Error> {
        let commands = self.commands()?.list_all(channel)?;
        Ok(warp::reply::json(&commands))
    }

    /// Edit the given command by key.
    fn edit(
        &self,
        channel: &str,
        name: &str,
        template: template::Template,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        self.commands()?.edit(channel, name, template)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given command's disabled status.
    fn edit_disabled(
        &self,
        channel: &str,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        if disabled {
            self.commands()?.disable(channel, name)?;
        } else {
            self.commands()?.enable(channel, name)?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given command by key.
    fn delete(&self, channel: &str, name: &str) -> Result<impl warp::Reply, anyhow::Error> {
        self.commands()?.delete(channel, name)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Promotions endpoint.
#[derive(Clone)]
struct Promotions(Arc<RwLock<Option<db::Promotions>>>);

impl Promotions {
    fn route(
        promotions: Arc<RwLock<Option<db::Promotions>>>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Promotions(promotions);

        let list = warp::get()
            .and(path!("promotions" / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| {
                    let api = api.clone();
                    async move { api.list(channel.as_str()).map_err(custom_reject) }
                }
            });

        let delete = warp::delete()
            .and(path!("promotions" / Fragment / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    let api = api.clone();

                    async move {
                        api.delete(channel.as_str(), name.as_str())
                            .map_err(custom_reject)
                    }
                }
            });

        let edit = warp::put()
            .and(path!("promotions" / Fragment / Fragment).and(path::end()))
            .and(body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: PutPromotion| {
                    let api = api.clone();

                    async move {
                        api.edit(
                            channel.as_str(),
                            name.as_str(),
                            body.frequency,
                            body.template,
                        )
                        .map_err(custom_reject)
                    }
                }
            });

        let edit_disabled = warp::post()
            .and(path!("promotions" / Fragment / Fragment / "disabled").and(path::end()))
            .and(body::json())
            .and_then({
                move |channel: Fragment, name: Fragment, body: DisabledBody| {
                    let api = api.clone();

                    async move {
                        api.edit_disabled(channel.as_str(), name.as_str(), body.disabled)
                            .map_err(custom_reject)
                    }
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(serde::Deserialize)]
        pub struct PutPromotion {
            frequency: utils::Duration,
            template: template::Template,
        }
    }

    /// Access underlying promotions abstraction.
    fn promotions(&self) -> Result<MappedRwLockReadGuard<'_, db::Promotions>, anyhow::Error> {
        match RwLockReadGuard::try_map(self.0.read(), |c| c.as_ref()) {
            Ok(out) => Ok(out),
            Err(_) => bail!("promotions not configured"),
        }
    }

    /// Get the list of all promotions.
    fn list(&self, channel: &str) -> Result<impl warp::Reply, anyhow::Error> {
        let promotions = self.promotions()?.list_all(channel)?;
        Ok(warp::reply::json(&promotions))
    }

    /// Edit the given promotion by key.
    fn edit(
        &self,
        channel: &str,
        name: &str,
        frequency: utils::Duration,
        template: template::Template,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        self.promotions()?
            .edit(channel, name, frequency, template)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given promotion's disabled status.
    fn edit_disabled(
        &self,
        channel: &str,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        if disabled {
            self.promotions()?.disable(channel, name)?;
        } else {
            self.promotions()?.enable(channel, name)?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given promotion by key.
    fn delete(&self, channel: &str, name: &str) -> Result<impl warp::Reply, anyhow::Error> {
        self.promotions()?.delete(channel, name)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Themes endpoint.
#[derive(Clone)]
struct Themes(Arc<RwLock<Option<db::Themes>>>);

impl Themes {
    fn route(themes: Arc<RwLock<Option<db::Themes>>>) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Themes(themes);

        let list = warp::get()
            .and(path!("themes" / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| {
                    let api = api.clone();
                    async move { api.list(channel.as_str()).map_err(custom_reject) }
                }
            });

        let delete = warp::delete()
            .and(path!("themes" / Fragment / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    let api = api.clone();

                    async move {
                        api.delete(channel.as_str(), name.as_str())
                            .map_err(custom_reject)
                    }
                }
            });

        let edit = warp::put()
            .and(path!("themes" / Fragment / Fragment).and(path::end()))
            .and(body::json())
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment, body: PutTheme| {
                    let api = api.clone();

                    async move {
                        api.edit(channel.as_str(), name.as_str(), body.track_id)
                            .map_err(custom_reject)
                    }
                }
            });

        let edit_disabled = warp::post()
            .and(path!("themes" / Fragment / Fragment / "disabled").and(path::end()))
            .and(body::json())
            .and_then({
                move |channel: Fragment, name: Fragment, body: DisabledBody| {
                    let api = api.clone();

                    async move {
                        api.edit_disabled(channel.as_str(), name.as_str(), body.disabled)
                            .map_err(custom_reject)
                    }
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(serde::Deserialize)]
        pub struct PutTheme {
            track_id: TrackId,
        }
    }

    /// Access underlying themes abstraction.
    fn themes(&self) -> Result<MappedRwLockReadGuard<'_, db::Themes>, anyhow::Error> {
        match RwLockReadGuard::try_map(self.0.read(), |c| c.as_ref()) {
            Ok(out) => Ok(out),
            Err(_) => bail!("themes not configured"),
        }
    }

    /// Get the list of all promotions.
    fn list(&self, channel: &str) -> Result<impl warp::Reply, anyhow::Error> {
        let promotions = self.themes()?.list_all(channel)?;
        Ok(warp::reply::json(&promotions))
    }

    /// Edit the given promotion by key.
    fn edit(
        &self,
        channel: &str,
        name: &str,
        track_id: TrackId,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        self.themes()?.edit(channel, name, track_id)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given promotion's disabled status.
    fn edit_disabled(
        &self,
        channel: &str,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        if disabled {
            self.themes()?.disable(channel, name)?;
        } else {
            self.themes()?.enable(channel, name)?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given promotion by key.
    fn delete(&self, channel: &str, name: &str) -> Result<impl warp::Reply, anyhow::Error> {
        self.themes()?.delete(channel, name)?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Auth API endpoints.
#[derive(Clone)]
struct Auth {
    active_connections: Arc<RwLock<HashMap<String, ConnectionMeta>>>,
    auth: auth::Auth,
    settings: Arc<RwLock<Option<crate::settings::Settings>>>,
}

#[derive(serde::Deserialize)]
pub struct AuthKeyQuery {
    #[serde(default)]
    key: Option<Fragment>,
}

impl Auth {
    fn route(
        auth: auth::Auth,
        active_connections: Arc<RwLock<HashMap<String, ConnectionMeta>>>,
        settings: Arc<RwLock<Option<crate::settings::Settings>>>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Auth {
            auth,
            active_connections,
            settings,
        };

        let route = warp::get()
            .and(warp::path!("connections").and(path::end()))
            .and_then({
                let api = api.clone();
                move || {
                    let api = api.clone();
                    async move { api.connections().map_err(custom_reject) }
                }
            })
            .boxed();

        let route = route
            .or(warp::get()
                .and(warp::path!("roles").and(path::end()))
                .and_then({
                    let api = api.clone();
                    move || {
                        let api = api.clone();
                        async move { api.roles().map_err(custom_reject) }
                    }
                }))
            .boxed();

        let route = route
            .or(warp::get()
                .and(warp::path!("scopes").and(path::end()))
                .and_then({
                    let api = api.clone();
                    move || {
                        let api = api.clone();
                        async move { api.scopes().map_err(custom_reject) }
                    }
                }))
            .boxed();

        let route = route
            .or(warp::get()
                .and(warp::path!("grants").and(path::end()))
                .and_then({
                    let api = api.clone();
                    move || {
                        let api = api.clone();
                        async move { api.grants().map_err(custom_reject) }
                    }
                }))
            .boxed();

        let route = route
            .or(warp::put()
                .and(warp::path!("grants").and(path::end()))
                .and(body::json())
                .and_then({
                    let api = api.clone();
                    move |body: PutGrant| {
                        let api = api.clone();
                        async move {
                            api.insert_grant(body.scope, body.role)
                                .map_err(custom_reject)
                        }
                    }
                }))
            .boxed();

        let route = route
            .or(warp::delete()
                .and(warp::path!("grants" / Fragment / Fragment).and(path::end()))
                .and_then({
                    let api = api.clone();
                    move |scope: Fragment, role: Fragment| {
                        let api = api.clone();
                        async move {
                            api.delete_grant(scope.as_str(), role.as_str())
                                .map_err(custom_reject)
                        }
                    }
                }))
            .boxed();

        let route = route
            .or(warp::get()
                .and(
                    warp::path!("key")
                        .and(warp::query::<AuthKeyQuery>())
                        .and(path::end()),
                )
                .and_then({
                    move |query: AuthKeyQuery| {
                        let api = api.clone();
                        async move { api.set_key(query).map_err(custom_reject) }
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
    fn connections(&self) -> Result<impl warp::Reply, Error> {
        let active_connections = self.active_connections.read();
        let mut out = Vec::new();

        for c in active_connections.values() {
            out.push(c.clone());
        }

        out.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(warp::reply::json(&out))
    }

    /// Get the list of all scopes.
    fn scopes(&self) -> Result<impl warp::Reply, anyhow::Error> {
        let scopes = self.auth.scopes();
        Ok(warp::reply::json(&scopes))
    }

    /// Get the list of all roles.
    fn roles(&self) -> Result<impl warp::Reply, anyhow::Error> {
        let roles = self.auth.roles();
        Ok(warp::reply::json(&roles))
    }

    /// Get the list of all auth in the bot.
    fn grants(&self) -> Result<impl warp::Reply, anyhow::Error> {
        let auth = self.auth.list();
        Ok(warp::reply::json(&auth))
    }

    /// Delete a single scope assignment.
    fn delete_grant(&self, scope: &str, role: &str) -> Result<impl warp::Reply, anyhow::Error> {
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
    ) -> Result<impl warp::Reply, anyhow::Error> {
        self.auth.insert(scope, role)?;
        Ok(warp::reply::json(&EMPTY))
    }

    fn set_key(&self, key: AuthKeyQuery) -> Result<impl warp::Reply, anyhow::Error> {
        let settings = self.settings.read();

        if let (Some(settings), Some(key)) = (settings.as_ref(), key.key) {
            settings.set("remote/secret-key", key.as_str())?;
        }

        let mut parts = URL.parse::<warp::http::Uri>()?.into_parts();
        parts.path_and_query = Some(warp::http::uri::PathAndQuery::from_static(
            "?received-key=true",
        ));
        let uri = warp::http::Uri::from_parts(parts)?;

        Ok(warp::redirect::redirect(uri))
    }
}

/// API to manage device.
#[derive(Clone)]
struct Api {
    player: Arc<RwLock<Option<player::Player>>>,
    after_streams: Arc<RwLock<Option<db::AfterStreams>>>,
    db: db::Database,
    currency: Arc<RwLock<Option<Currency>>>,
    latest: Arc<RwLock<Option<api::github::Release>>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Balance {
    name: String,
    #[serde(default)]
    balance: i64,
    #[serde(default)]
    watch_time: i64,
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
            player.set_device(device.id.clone())?;
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
            let is_current = c.as_ref().map(|d| *d == device.id).unwrap_or_default();

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
                api::spotify::DeviceType::CastAudio => "Cast Audio",
                _ => "Unknown",
            }
        }

        #[derive(Default, serde::Serialize)]
        struct Devices {
            devices: Vec<AudioDevice>,
            current: Option<AudioDevice>,
        }
    }

    /// Access underlying after streams abstraction.
    fn after_streams(&self) -> Result<MappedRwLockReadGuard<'_, db::AfterStreams>, anyhow::Error> {
        match RwLockReadGuard::try_map(self.after_streams.read(), |c| c.as_ref()) {
            Ok(out) => Ok(out),
            Err(_) => bail!("after streams not configured"),
        }
    }

    /// Get the list of available after streams.
    fn get_after_streams(&self) -> Result<impl warp::Reply, anyhow::Error> {
        let after_streams = self.after_streams()?.list()?;
        Ok(warp::reply::json(&after_streams))
    }

    /// Get the list of available after streams.
    fn delete_after_stream(&self, id: i32) -> Result<impl warp::Reply, anyhow::Error> {
        self.after_streams()?.delete(id)?;
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

    /// Get version information.
    fn version(&self) -> Result<impl warp::Reply, Error> {
        let info = Version {
            version: crate::VERSION,
            latest: self.latest.read().clone().map(to_latest),
        };

        return Ok(warp::reply::json(&info));

        #[derive(serde::Serialize)]
        struct Version {
            version: &'static str,
            latest: Option<Latest>,
        }

        #[derive(serde::Serialize)]
        struct Latest {
            version: String,
            asset: Option<Asset>,
        }

        #[derive(serde::Serialize)]
        struct Asset {
            name: String,
            download_url: String,
        }

        /// Convert a relase into information on latest release.
        fn to_latest(release: api::github::Release) -> Latest {
            let version = release.tag_name;

            let asset = release
                .assets
                .into_iter()
                .filter(|a| a.name.ends_with(".msi"))
                .map(|a| Asset {
                    name: a.name,
                    download_url: a.browser_download_url,
                })
                .next();

            Latest { version, asset }
        }
    }
}

/// Set up the web endpoint.
pub async fn setup(
    injector: &injector::Injector,
    message_log: message_log::MessageLog,
    message_bus: Arc<bus::Bus<message_log::Event>>,
    global_bus: Arc<bus::Bus<bus::Global>>,
    youtube_bus: Arc<bus::Bus<bus::YouTube>>,
    command_bus: Arc<bus::Bus<bus::Command>>,
    db: db::Database,
    auth: auth::Auth,
    channel: Arc<RwLock<Option<String>>>,
    latest: Arc<RwLock<Option<api::github::Release>>>,
) -> Result<(Server, impl Future<Output = ()>), anyhow::Error> {
    let addr: SocketAddr = str::parse("0.0.0.0:12345")?;

    let player = Arc::new(RwLock::new(None));
    let active_connections: Arc<RwLock<HashMap<String, ConnectionMeta>>> = Default::default();

    let api = Api {
        player: player.clone(),
        after_streams: injector.var()?,
        db,
        currency: injector.var()?,
        latest,
    };

    let api = {
        let route = warp::post()
            .and(path!("device" / String))
            .and_then({
                let api = api.clone();
                move |id| {
                    let api = api.clone();
                    async move { api.clone().set_device(id).await.map_err(custom_reject) }
                }
            })
            .boxed();

        let route = route
            .or(warp::get().and(warp::path("version")).and_then({
                let api = api.clone();
                move || {
                    let api = api.clone();
                    async move { api.version().map_err(custom_reject) }
                }
            }))
            .boxed();

        let route = route
            .or(warp::get().and(warp::path("devices")).and_then({
                let api = api.clone();
                move || {
                    let api = api.clone();
                    async move { api.clone().devices().await.map_err(custom_reject) }
                }
            }))
            .boxed();

        let route = route
            .or(warp::delete().and(path!("after-stream" / i32)).and_then({
                let api = api.clone();
                move |id| {
                    let api = api.clone();
                    async move { api.delete_after_stream(id).map_err(custom_reject) }
                }
            }))
            .boxed();

        let route = route
            .or(warp::get().and(warp::path("after-streams")).and_then({
                let api = api.clone();
                move || {
                    let api = api.clone();
                    async move { api.get_after_streams().map_err(custom_reject) }
                }
            }))
            .boxed();

        let route = route
            .or(warp::put()
                .and(warp::path("balances"))
                .and(body::json())
                .and_then({
                    let api = api.clone();
                    move |balances: Vec<db::models::Balance>| {
                        let api = api.clone();

                        async move {
                            api.clone()
                                .import_balances(balances)
                                .await
                                .map_err(custom_reject)
                        }
                    }
                }))
            .boxed();

        let route = route
            .or(warp::get().and(warp::path("balances")).and_then({
                move || {
                    let api = api.clone();

                    async move { api.clone().export_balances().await.map_err(custom_reject) }
                }
            }))
            .boxed();

        let route = route.or(warp::path("auth")
            .and(Auth::route(
                auth,
                active_connections.clone(),
                injector.var()?,
            ))
            .boxed());
        let route = route.or(Aliases::route(injector.var()?));
        let route = route.or(Commands::route(injector.var()?));
        let route = route.or(Promotions::route(injector.var()?));
        let route = route.or(Themes::route(injector.var()?));
        let route = route.or(Settings::route(injector.var()?));
        let route = route.or(Cache::route(injector.var()?));
        let route = route.or(Chat::route(command_bus, message_log));

        // TODO: move endpoint into abstraction thingie.
        let route = route
            .or(
                warp::get().and(path!("current").and(path::end()).and_then(move || {
                    let channel = channel.clone();

                    async move {
                        let channel = channel.read();

                        let channel = match channel.as_ref() {
                            Some(channel) => Some(channel.to_string()),
                            None => None,
                        };

                        let current = Current { channel };

                        Ok::<_, warp::Rejection>(warp::reply::json(&current))
                    }
                })),
            )
            .boxed();

        warp::path("api").and(route)
    };

    let ws_messages = warp::get()
        .and(warp::path!("ws" / "messages"))
        .and(send_bus(message_bus).recover(recover));

    let ws_overlay = warp::get()
        .and(warp::path!("ws" / "overlay"))
        .and(send_bus(global_bus).recover(recover));

    let ws_youtube = warp::get()
        .and(warp::path!("ws" / "youtube"))
        .and(send_bus(youtube_bus).recover(recover));

    let routes = api.recover(recover);
    let routes = routes.or(ws_messages.recover(recover));
    let routes = routes.or(ws_overlay.recover(recover));
    let routes = routes.or(ws_youtube.recover(recover));

    let fallback = Asset::get("index.html");

    let routes =
        routes.or(warp::get()
            .and(warp::path::tail())
            .and_then(move |tail: path::Tail| {
                let fallback = fallback.clone();
                async move { serve(tail.as_str(), fallback) }
            }));

    let routes = routes.recover(recover);
    let service = warp::serve(routes);

    // TODO: fix when this review is fixed: https://github.com/seanmonstar/warp/pull/265#pullrequestreview-294644379
    let server_future = service.try_bind_ephemeral(addr)?.1.boxed();

    let server = Server {
        player,
        active_connections,
    };

    return Ok((server, server_future));

    fn serve(
        path: &str,
        fallback: Option<Cow<'static, [u8]>>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let (mime, asset) = match Asset::get(path) {
            Some(asset) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                (mime, asset)
            }
            None => {
                let fallback = fallback.ok_or_else(warp::reject::not_found)?;
                (mime::TEXT_HTML_UTF_8, fallback)
            }
        };

        let res = warp::http::Response::builder()
            .header("content-type", mime.to_string())
            .body(asset);

        Ok(res)
    }
}

pub struct Fragment {
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

impl<'de> serde::Deserialize<'de> for Fragment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;

        let string = percent_encoding::percent_decode(string.as_bytes())
            .decode_utf8()
            .map_err(serde::de::Error::custom)?
            .to_string();

        Ok(Fragment { string })
    }
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
async fn recover(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(e) = err.find::<Error>() {
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
    player: Arc<RwLock<Option<player::Player>>>,
    /// Callbacks for when we have received a token.
    active_connections: Arc<RwLock<HashMap<String, ConnectionMeta>>>,
}

impl Server {
    /// Set the player interface.
    pub fn set_player(&self, player: player::Player) {
        *self.player.write() = Some(player);
    }

    pub fn update_connection(&self, id: &str, connection: ConnectionMeta) {
        self.active_connections
            .write()
            .insert(id.to_string(), connection);
    }

    pub fn clear_connection(&self, id: &str) {
        let _ = self.active_connections.write().remove(id);
    }
}

#[derive(Debug)]
pub struct ReceivedToken {
    pub code: String,
    pub state: String,
}

/// Connecting a bus to a websocket connection.
fn send_bus<T>(bus: Arc<bus::Bus<T>>) -> filters::BoxedFilter<(impl warp::Reply,)>
where
    T: bus::Message,
{
    warp::ws()
        .map({
            move |ws: warp::ws::Ws| {
                let bus = bus.clone();

                ws.on_upgrade(move |websocket: warp::filters::ws::WebSocket| async {
                    if let Err(e) = send_bus_forward(bus, websocket).await {
                        log::error!("websocket error: {}", e);
                    }
                })
            }
        })
        .boxed()
}

/// Forward the bus message to the websocket.
async fn send_bus_forward<T>(
    bus: Arc<bus::Bus<T>>,
    websocket: warp::filters::ws::WebSocket,
) -> Result<(), anyhow::Error>
where
    T: bus::Message,
{
    let (mut tx, _) = websocket.split();

    // add a receiver and forward all new messages.
    let mut rx = bus.add_rx();

    // send all cached messages.
    for m in bus.latest() {
        let m = filters::ws::Message::text(serde_json::to_string(&m)?);
        tx.send(m).await?;
    }

    while let Some(m) = rx.next().await {
        let m = filters::ws::Message::text(serde_json::to_string(&m)?);
        tx.send(m).await?;
    }

    Ok(())
}
