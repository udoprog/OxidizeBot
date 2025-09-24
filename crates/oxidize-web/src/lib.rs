#![allow(clippy::too_many_arguments)]

mod cache;
mod chat;
mod settings;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::pin;
use std::sync::Arc;

use anyhow::{bail, Context, Error, Result};
use api::setbac::ConnectionMeta;
use async_injector::{Injector, Key};
use common::models::spotify::senum::DeviceType;
use common::models::TrackId;
use common::sink::SinkExt;
use common::tags;
use common::{Channel, Duration};
use serde::{de, Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock, RwLockReadGuard};
use tracing::Instrument;
use warp::{body, filters, path, Filter};

use self::assets::Asset;
use self::cache::Cache;
use self::chat::Chat;
use self::settings::Settings;

/// URL of public web interface.
pub const URL: &str = "http://localhost:12345";

mod assets {
    #[derive(rust_embed::RustEmbed)]
    #[folder = "$CARGO_MANIFEST_DIR/../../bot-ui/dist"]
    pub(crate) struct Asset;
}

#[derive(Debug)]
struct CustomReject(Error);

impl warp::reject::Reject for CustomReject {}

pub(crate) fn custom_reject(error: impl Into<Error>) -> warp::Rejection {
    warp::reject::custom(CustomReject(error.into()))
}

#[derive(Debug)]
enum WebError {
    BadRequest,
    NotFound,
    Custom(Error),
}

impl From<Error> for WebError {
    fn from(value: Error) -> Self {
        WebError::Custom(value)
    }
}

impl fmt::Display for WebError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WebError::BadRequest => "bad request".fmt(fmt),
            WebError::NotFound => "not found".fmt(fmt),
            WebError::Custom(e) => e.fmt(fmt),
        }
    }
}

impl std::error::Error for WebError {}

#[derive(Default, Serialize)]
#[non_exhaustive]
struct Empty;

const EMPTY: Empty = Empty;

#[derive(Clone, Serialize)]
struct AudioDevice {
    is_current: bool,
    name: String,
    id: String,
    r#type: String,
}

#[derive(Serialize)]
pub(crate) struct Current {
    channel: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DisabledBody {
    disabled: bool,
}

/// Aliases endpoint.
#[derive(Clone)]
struct Aliases(async_injector::Ref<db::Aliases>);

impl Aliases {
    fn route(
        aliases: async_injector::Ref<db::Aliases>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Aliases(aliases);

        let list = warp::get()
            .and(path!("aliases" / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| {
                    let api = api.clone();
                    async move { api.list(channel.as_channel()).await.map_err(custom_reject) }
                }
            });

        let delete = warp::delete()
            .and(path!("aliases" / Fragment / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    let api = api.clone();
                    async move {
                        api.delete(channel.as_channel(), name.as_str())
                            .await
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
                        api.edit(channel.as_channel(), name.as_str(), body.template)
                            .await
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
                        api.edit_disabled(channel.as_channel(), name.as_str(), body.disabled)
                            .await
                            .map_err(custom_reject)
                    }
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(Deserialize)]
        pub(crate) struct PutAlias {
            template: template::Template,
        }
    }

    /// Access underlying aliases abstraction.
    async fn aliases(&self) -> Result<RwLockReadGuard<'_, db::Aliases>> {
        match self.0.read().await {
            Some(out) => Ok(out),
            None => bail!("aliases not configured"),
        }
    }

    /// Get the list of all aliases.
    async fn list(&self, channel: &Channel) -> Result<impl warp::Reply> {
        let aliases = self.aliases().await?.list_all(channel).await?;
        Ok(warp::reply::json(&aliases))
    }

    /// Edit the given alias by key.
    async fn edit(
        &self,
        channel: &Channel,
        name: &str,
        template: template::Template,
    ) -> Result<impl warp::Reply> {
        self.aliases().await?.edit(channel, name, template).await?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given alias's disabled status.
    async fn edit_disabled(
        &self,
        channel: &Channel,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply> {
        let aliases = self.aliases().await?;

        if disabled {
            aliases.disable(channel, name).await?;
        } else {
            aliases.enable(channel, name).await?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given alias by key.
    async fn delete(&self, channel: &Channel, name: &str) -> Result<impl warp::Reply> {
        self.aliases().await?.delete(channel, name).await?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Commands endpoint.
#[derive(Clone)]
struct Commands(async_injector::Ref<db::Commands>);

impl Commands {
    fn route(
        commands: async_injector::Ref<db::Commands>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Commands(commands);

        let list = warp::get()
            .and(path!("commands" / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| {
                    let api = api.clone();
                    async move { api.list(channel.as_channel()).await.map_err(custom_reject) }
                }
            });

        let delete = warp::delete()
            .and(path!("commands" / Fragment / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    let api = api.clone();
                    async move {
                        api.delete(channel.as_channel(), name.as_str())
                            .await
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
                        api.edit_disabled(channel.as_channel(), name.as_str(), body.disabled)
                            .await
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
                        api.edit(channel.as_channel(), name.as_str(), body.template)
                            .await
                            .map_err(custom_reject)
                    }
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(Deserialize)]
        pub(crate) struct PutCommand {
            template: template::Template,
        }
    }

    /// Access underlying commands abstraction.
    async fn commands(&self) -> Result<RwLockReadGuard<'_, db::Commands>> {
        match self.0.read().await {
            Some(out) => Ok(out),
            None => bail!("commands not configured"),
        }
    }

    /// Get the list of all commands.
    async fn list(&self, channel: &Channel) -> Result<impl warp::Reply> {
        let commands = self.commands().await?.list_all(channel).await?;
        Ok(warp::reply::json(&commands))
    }

    /// Edit the given command by key.
    async fn edit(
        &self,
        channel: &Channel,
        name: &str,
        template: template::Template,
    ) -> Result<impl warp::Reply> {
        self.commands().await?.edit(channel, name, template).await?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given command's disabled status.
    async fn edit_disabled(
        &self,
        channel: &Channel,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply> {
        let commands = self.commands().await?;

        if disabled {
            commands.disable(channel, name).await?;
        } else {
            commands.enable(channel, name).await?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given command by key.
    async fn delete(&self, channel: &Channel, name: &str) -> Result<impl warp::Reply> {
        self.commands().await?.delete(channel, name).await?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Promotions endpoint.
#[derive(Clone)]
struct Promotions(async_injector::Ref<db::Promotions>);

impl Promotions {
    fn route(
        promotions: async_injector::Ref<db::Promotions>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Promotions(promotions);

        let list = warp::get()
            .and(path!("promotions" / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| {
                    let api = api.clone();
                    async move { api.list(channel.as_channel()).await.map_err(custom_reject) }
                }
            });

        let delete = warp::delete()
            .and(path!("promotions" / Fragment / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    let api = api.clone();

                    async move {
                        api.delete(channel.as_channel(), name.as_str())
                            .await
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
                            channel.as_channel(),
                            name.as_str(),
                            body.frequency,
                            body.template,
                        )
                        .await
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
                        api.edit_disabled(channel.as_channel(), name.as_str(), body.disabled)
                            .await
                            .map_err(custom_reject)
                    }
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(Deserialize)]
        pub(crate) struct PutPromotion {
            frequency: Duration,
            template: template::Template,
        }
    }

    /// Access underlying promotions abstraction.
    async fn promotions(&self) -> Result<RwLockReadGuard<'_, db::Promotions>> {
        match self.0.read().await {
            Some(out) => Ok(out),
            None => bail!("promotions not configured"),
        }
    }

    /// Get the list of all promotions.
    async fn list(&self, channel: &Channel) -> Result<impl warp::Reply> {
        let promotions = self.promotions().await?.list_all(channel).await?;
        Ok(warp::reply::json(&promotions))
    }

    /// Edit the given promotion by key.
    async fn edit(
        &self,
        channel: &Channel,
        name: &str,
        frequency: Duration,
        template: template::Template,
    ) -> Result<impl warp::Reply> {
        self.promotions()
            .await?
            .edit(channel, name, frequency, template)
            .await?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given promotion's disabled status.
    async fn edit_disabled(
        &self,
        channel: &Channel,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply> {
        let promotions = self.promotions().await?;

        if disabled {
            promotions.disable(channel, name).await?;
        } else {
            promotions.enable(channel, name).await?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given promotion by key.
    async fn delete(&self, channel: &Channel, name: &str) -> Result<impl warp::Reply> {
        self.promotions().await?.delete(channel, name).await?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Themes endpoint.
#[derive(Clone)]
struct Themes(async_injector::Ref<db::Themes>);

impl Themes {
    fn route(themes: async_injector::Ref<db::Themes>) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Themes(themes);

        let list = warp::get()
            .and(path!("themes" / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment| {
                    let api = api.clone();
                    async move { api.list(channel.as_channel()).await.map_err(custom_reject) }
                }
            });

        let delete = warp::delete()
            .and(path!("themes" / Fragment / Fragment).and(path::end()))
            .and_then({
                let api = api.clone();
                move |channel: Fragment, name: Fragment| {
                    let api = api.clone();

                    async move {
                        api.delete(channel.as_channel(), name.as_str())
                            .await
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
                        api.edit(channel.as_channel(), name.as_str(), body.track_id)
                            .await
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
                        api.edit_disabled(channel.as_channel(), name.as_str(), body.disabled)
                            .await
                            .map_err(custom_reject)
                    }
                }
            });

        return list.or(delete).or(edit).or(edit_disabled).boxed();

        #[derive(Deserialize)]
        pub(crate) struct PutTheme {
            track_id: TrackId,
        }
    }

    /// Access underlying themes abstraction.
    async fn themes(&self) -> Result<RwLockReadGuard<'_, db::Themes>> {
        match self.0.read().await {
            Some(out) => Ok(out),
            None => bail!("themes not configured"),
        }
    }

    /// Get the list of all promotions.
    async fn list(&self, channel: &Channel) -> Result<impl warp::Reply> {
        let promotions = self.themes().await?.list_all(channel).await?;
        Ok(warp::reply::json(&promotions))
    }

    /// Edit the given promotion by key.
    async fn edit(
        &self,
        channel: &Channel,
        name: &str,
        track_id: TrackId,
    ) -> Result<impl warp::Reply> {
        self.themes().await?.edit(channel, name, track_id).await?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Set the given promotion's disabled status.
    async fn edit_disabled(
        &self,
        channel: &Channel,
        name: &str,
        disabled: bool,
    ) -> Result<impl warp::Reply> {
        let themes = self.themes().await?;

        if disabled {
            themes.disable(channel, name).await?;
        } else {
            themes.enable(channel, name).await?;
        }

        Ok(warp::reply::json(&EMPTY))
    }

    /// Delete the given promotion by key.
    async fn delete(&self, channel: &Channel, name: &str) -> Result<impl warp::Reply> {
        self.themes().await?.delete(channel, name).await?;
        Ok(warp::reply::json(&EMPTY))
    }
}

/// Auth API endpoints.
#[derive(Clone)]
struct Auth {
    active_connections: Arc<RwLock<HashMap<String, ConnectionMeta>>>,
    auth: auth::Auth,
    settings: async_injector::Ref<::settings::Settings<::auth::Scope>>,
}

#[derive(Deserialize)]
pub(crate) struct AuthKeyQuery {
    #[serde(default)]
    key: Option<Fragment>,
}

impl Auth {
    fn route(
        auth: auth::Auth,
        active_connections: Arc<RwLock<HashMap<String, ConnectionMeta>>>,
        settings: async_injector::Ref<::settings::Settings<::auth::Scope>>,
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
                    async move { api.connections().await.map_err(custom_reject) }
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
                        async move { api.grants().await.map_err(custom_reject) }
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
                                .await
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
                                .await
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
                        async move { api.set_key(query).await.map_err(custom_reject) }
                    }
                }))
            .boxed();

        return route;

        #[derive(Deserialize)]
        pub(crate) struct PutGrant {
            scope: auth::Scope,
            role: auth::Role,
        }
    }

    /// Get a list of things that need authentication.
    async fn connections(&self) -> Result<impl warp::Reply, WebError> {
        let active_connections = self.active_connections.read().await;
        let mut out = Vec::new();

        for c in active_connections.values() {
            out.push(c.clone());
        }

        out.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(warp::reply::json(&out))
    }

    /// Get the list of all scopes.
    fn scopes(&self) -> Result<impl warp::Reply> {
        let scopes = self.auth.scopes();
        Ok(warp::reply::json(&scopes))
    }

    /// Get the list of all roles.
    fn roles(&self) -> Result<impl warp::Reply> {
        let roles = self.auth.roles();
        Ok(warp::reply::json(&roles))
    }

    /// Get the list of all auth in the bot.
    async fn grants(&self) -> Result<impl warp::Reply> {
        let auth = self.auth.list().await;
        Ok(warp::reply::json(&auth))
    }

    /// Delete a single scope assignment.
    async fn delete_grant(&self, scope: &str, role: &str) -> Result<impl warp::Reply> {
        let scope = str::parse(scope)?;
        let role = str::parse(role)?;
        self.auth.delete(scope, role).await?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Insert a single scope assignment.
    async fn insert_grant(&self, scope: auth::Scope, role: auth::Role) -> Result<impl warp::Reply> {
        self.auth.insert(scope, role).await?;
        Ok(warp::reply::json(&EMPTY))
    }

    async fn set_key(&self, key: AuthKeyQuery) -> Result<impl warp::Reply> {
        match self.settings.read().await {
            Some(settings) => {
                if let Some(key) = key.key {
                    tracing::info!("Setting remote secret key");
                    settings.set("remote/secret-key", key.as_str()).await?;
                } else {
                    tracing::warn!("Received fragment doesn't contain key");
                }
            }
            None => {
                tracing::warn!("No settings available");
            }
        }

        let mut parts = URL.parse::<warp::http::Uri>()?.into_parts();
        parts.path_and_query = Some(warp::http::uri::PathAndQuery::from_static(
            "?received-key=true",
        ));
        let uri = warp::http::Uri::from_parts(parts)?;
        Ok(warp::redirect::temporary(uri))
    }
}

/// API to manage device.
#[derive(Clone)]
struct Api {
    version: &'static str,
    player: async_injector::Ref<player::Player>,
    after_streams: async_injector::Ref<db::AfterStreams>,
    currency: async_injector::Ref<currency::Currency>,
    latest: ::settings::Var<Option<api::github::Release>>,
}

impl Api {
    /// Handle request to set device.
    async fn set_device(self, id: String) -> Result<impl warp::Reply, WebError> {
        let player = self.player.read().await;

        let player = match player.as_deref() {
            Some(player) => player,
            None => return Err(WebError::BadRequest),
        };

        let devices = player.list_devices().await?;

        if let Some(device) = devices.iter().find(|d| d.id == id) {
            player.set_device(device.id.clone()).await?;
            return Ok(warp::reply::json(&EMPTY));
        }

        Err(WebError::BadRequest)
    }

    /// Get a list of things that need authentication.
    async fn devices(self) -> Result<impl warp::Reply, WebError> {
        let player = self.player.read().await;

        let player = match player.as_deref() {
            Some(player) => player,
            None => {
                tracing::warn!("No player available");
                let data = Devices::default();
                return Ok(warp::reply::json(&data));
            }
        };

        let c = player.current_device().await;
        let data = match player.list_devices().await {
            Ok(data) => data,
            Err(_) => {
                let data = Devices::default();
                return Ok(warp::reply::json(&data));
            }
        };

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
        fn device_to_string(device: &DeviceType) -> &'static str {
            match *device {
                DeviceType::Computer => "Computer",
                DeviceType::Smartphone => "Smart Phone",
                DeviceType::Speaker => "Speaker",
                DeviceType::CastAudio => "Cast Audio",
                _ => "Unknown",
            }
        }

        #[derive(Default, Serialize)]
        struct Devices {
            devices: Vec<AudioDevice>,
            current: Option<AudioDevice>,
        }
    }

    /// Access underlying after streams abstraction.
    async fn after_streams(&self) -> Result<RwLockReadGuard<'_, db::AfterStreams>> {
        match self.after_streams.read().await {
            Some(out) => Ok(out),
            None => bail!("after streams not configured"),
        }
    }

    /// Get the list of available after streams.
    async fn get_after_streams(&self) -> Result<impl warp::Reply> {
        let after_streams = self.after_streams().await?.list().await?;
        Ok(warp::reply::json(&after_streams))
    }

    /// Get the list of available after streams.
    async fn delete_after_stream(&self, id: i32) -> Result<impl warp::Reply> {
        self.after_streams().await?.delete(id).await?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Import balances.
    async fn import_balances(
        self,
        balances: Vec<db::models::Balance>,
    ) -> Result<impl warp::Reply, WebError> {
        self.currency
            .read()
            .await
            .as_ref()
            .ok_or_else(|| WebError::NotFound)?
            .import_balances(balances)
            .await?;

        Ok(warp::reply::json(&EMPTY))
    }

    /// Export balances.
    async fn export_balances(self) -> Result<impl warp::Reply, WebError> {
        let balances = self
            .currency
            .read()
            .await
            .as_ref()
            .ok_or_else(|| WebError::NotFound)?
            .export_balances()
            .await?;

        Ok(warp::reply::json(&balances))
    }

    /// Get version information.
    async fn version(&self) -> Result<impl warp::Reply, WebError> {
        let info = Version {
            version: self.version,
            latest: self.latest.load().await.map(to_latest),
        };

        return Ok(warp::reply::json(&info));

        #[derive(Serialize)]
        struct Version {
            version: &'static str,
            latest: Option<Latest>,
        }

        #[derive(Serialize)]
        struct Latest {
            version: String,
            asset: Option<Asset>,
        }

        #[derive(Serialize)]
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
#[tracing::instrument(skip_all)]
pub async fn run(
    version: &'static str,
    injector: &Injector,
    message_log: messagelog::MessageLog,
    message_bus: bus::Bus<messagelog::Event>,
    global_bus: bus::Bus<bus::Global>,
    youtube_bus: bus::Bus<bus::YouTube>,
    command_bus: bus::Bus<bus::Command>,
    auth: auth::Auth,
    latest: ::settings::Var<Option<api::github::Release>>,
) -> Result<(Server, impl Future<Output = Result<()>>)> {
    let addr: SocketAddr = str::parse("0.0.0.0:12345")?;

    let channel = injector
        .var_key(Key::<String>::tagged(tags::Globals::Channel)?)
        .await;

    let player = injector.var().await;
    let active_connections: Arc<RwLock<HashMap<String, ConnectionMeta>>> = Default::default();

    let api = Api {
        version,
        player: player.clone(),
        after_streams: injector.var().await,
        currency: injector.var().await,
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
                    async move { api.version().await.map_err(custom_reject) }
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
                    async move { api.delete_after_stream(id).await.map_err(custom_reject) }
                }
            }))
            .boxed();

        let route = route
            .or(warp::get().and(warp::path("after-streams")).and_then({
                let api = api.clone();
                move || {
                    let api = api.clone();
                    async move { api.get_after_streams().await.map_err(custom_reject) }
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
                injector.var().await,
            ))
            .boxed());
        let route = route.or(Aliases::route(injector.var().await));
        let route = route.or(Commands::route(injector.var().await));
        let route = route.or(Promotions::route(injector.var().await));
        let route = route.or(Themes::route(injector.var().await));
        let route = route.or(Settings::route(injector.var().await));
        let route = route.or(Cache::route(injector.var().await));
        let route = route.or(Chat::route(command_bus, message_log));

        // TODO: move endpoint into abstraction thingie.
        let route = route
            .or(
                warp::get().and(path!("current").and(path::end()).and_then(move || {
                    let channel = channel.clone();

                    async move {
                        let current = Current {
                            channel: channel.load().await,
                        };
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
    let fallback = fallback.map(|f| f.data);

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
    let server_future = service.try_bind_ephemeral(addr)?.1;

    let (connections_tx, mut connections_rx) = mpsc::unbounded_channel();

    let server_future = async move {
        let mut server_future = pin!(server_future);

        loop {
            tokio::select! {
                _ = server_future.as_mut() => {
                    bail!("server ended");
                }
                out = connections_rx.recv() => {
                    let (id, connection) = out.context("End of connection updates")?;

                    if let Some(connection) = connection {
                        let _ = active_connections.write().await.insert(id, connection);
                    } else {
                        let _ = active_connections.write().await.remove(&id);
                    }
                }
            }
        }
    };

    let server = Server { connections_tx };
    return Ok((server, server_future.in_current_span()));

    fn serve(
        path: &str,
        fallback: Option<Cow<'static, [u8]>>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let (mime, asset) = match Asset::get(path) {
            Some(asset) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                (mime, asset.data)
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

pub(crate) struct Fragment {
    string: String,
}

impl Fragment {
    /// Borrow as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        self.string.as_str()
    }

    /// Borrow as a string slice.
    pub(crate) fn as_channel(&self) -> &Channel {
        Channel::new(self.string.as_str())
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

impl<'de> Deserialize<'de> for Fragment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;

        let string = percent_encoding::percent_decode(string.as_bytes())
            .decode_utf8()
            .map_err(de::Error::custom)?
            .to_string();

        Ok(Fragment { string })
    }
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
async fn recover(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(e) = err.find::<WebError>() {
        let code = match *e {
            WebError::BadRequest => warp::http::StatusCode::BAD_REQUEST,
            WebError::NotFound => warp::http::StatusCode::NOT_FOUND,
            WebError::Custom(_) => warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        let msg = e.to_string();

        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: msg,
        });

        Ok(warp::reply::with_status(json, code))
    } else if let Some(e) = err.find::<CustomReject>() {
        // TODO: Also log which endpoint caused the error
        tracing::error!("Endpoint error caused by: {}", e.0);

        let json = warp::reply::json(&ErrorMessage {
            code: 500,
            message: e.0.to_string(),
        });

        Ok(warp::reply::with_status(
            json,
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else {
        // Could be a NOT_FOUND, or METHOD_NOT_ALLOWED... here we just
        // let warp use its default rendering.
        Err(err)
    }
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

/// Interface to the server.
#[derive(Clone)]
pub struct Server {
    /// Callbacks for when we have received a token.
    connections_tx: mpsc::UnboundedSender<(String, Option<ConnectionMeta>)>,
}

impl Server {
    pub fn update_connection(&self, id: &str, connection: ConnectionMeta) {
        let _ = self.connections_tx.send((id.into(), Some(connection)));
    }

    pub fn clear_connection(&self, id: &str) {
        let _ = self.connections_tx.send((id.into(), None));
    }
}

/// Connecting a bus to a websocket connection.
fn send_bus<T>(bus: bus::Bus<T>) -> filters::BoxedFilter<(impl warp::Reply,)>
where
    T: bus::Message,
{
    warp::ws()
        .map({
            move |ws: warp::ws::Ws| {
                let bus = bus.clone();

                ws.on_upgrade(move |websocket: warp::filters::ws::WebSocket| async {
                    if let Err(e) = send_bus_forward(bus, websocket).await {
                        common::log_error!(e, "Websocket error");
                    }
                })
            }
        })
        .boxed()
}

/// Forward the bus message to the websocket.
async fn send_bus_forward<T>(bus: bus::Bus<T>, mut ws: warp::filters::ws::WebSocket) -> Result<()>
where
    T: bus::Message,
{
    // add a receiver and forward all new messages.
    let mut rx = bus.subscribe();

    // send all cached messages.
    for m in bus.latest().await {
        let m = filters::ws::Message::text(serde_json::to_string(&m)?);
        ws.send(m).await?;
    }

    loop {
        let m = rx.recv().await?;
        let m = filters::ws::Message::text(serde_json::to_string(&m)?);
        ws.send(m).await?;
    }
}
