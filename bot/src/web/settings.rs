use crate::web::{Fragment, EMPTY};
use anyhow::bail;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::{collections::HashSet, sync::Arc};
use warp::{body, filters, path, Filter as _};

#[derive(serde::Deserialize)]
pub struct PutSetting {
    value: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct SettingsQuery {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    feature: Option<bool>,
}

/// Settings endpoint.
#[derive(Clone)]
pub struct Settings(Arc<RwLock<Option<crate::settings::Settings>>>);

impl Settings {
    pub fn route(
        settings: Arc<RwLock<Option<crate::settings::Settings>>>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Settings(settings);

        let list = warp::get()
            .and(warp::path("settings").and(warp::query::<SettingsQuery>()))
            .and_then({
                let api = api.clone();
                move |query: SettingsQuery| {
                    let api = api.clone();

                    async move { api.get_settings(query).map_err(super::custom_reject) }
                }
            })
            .boxed();

        let get = warp::get()
            .and(warp::path("settings").and(path::tail()).and_then({
                let api = api.clone();
                move |key: path::Tail| {
                    let api = api.clone();

                    async move {
                        let key =
                            str::parse::<Fragment>(key.as_str()).map_err(super::custom_reject)?;
                        api.get_setting(key.as_str()).map_err(super::custom_reject)
                    }
                }
            }))
            .boxed();

        let delete = warp::delete()
            .and(warp::path("settings").and(path::tail()).and_then({
                let api = api.clone();

                move |key: path::Tail| {
                    let api = api.clone();

                    async move {
                        let key =
                            str::parse::<Fragment>(key.as_str()).map_err(super::custom_reject)?;
                        api.delete_setting(key.as_str())
                            .map_err(super::custom_reject)
                    }
                }
            }))
            .boxed();

        let edit = warp::put()
            .and(
                warp::path("settings")
                    .and(path::tail().and(body::json()))
                    .and_then({
                        move |key: path::Tail, body: PutSetting| {
                            let api = api.clone();

                            async move {
                                let key = str::parse::<Fragment>(key.as_str())
                                    .map_err(super::custom_reject)?;
                                api.edit_setting(key.as_str(), body.value)
                                    .map_err(super::custom_reject)
                            }
                        }
                    }),
            )
            .boxed();

        list.or(get).or(delete).or(edit).boxed()
    }

    /// Access underlying settings abstraction.
    fn settings(
        &self,
    ) -> Result<MappedRwLockReadGuard<'_, crate::settings::Settings>, anyhow::Error> {
        match RwLockReadGuard::try_map(self.0.read(), |c| c.as_ref()) {
            Ok(out) => Ok(out),
            Err(_) => bail!("settings not configured"),
        }
    }

    /// Get the list of all settings in the bot.
    fn get_settings(&self, query: SettingsQuery) -> Result<impl warp::Reply, anyhow::Error> {
        let mut settings = match query.prefix {
            Some(prefix) => {
                let mut out = Vec::new();

                for prefix in prefix.split(',') {
                    out.extend(self.settings()?.list_by_prefix(&prefix)?);
                }

                out
            }
            None => self.settings()?.list()?,
        };

        if let Some(key) = query.key {
            let key = key
                .split(',')
                .map(|s| s.to_string())
                .collect::<HashSet<_>>();

            let mut out = Vec::with_capacity(settings.len());

            for s in settings {
                if key.contains(&s.key) {
                    out.push(s);
                }
            }

            settings = out;
        }

        if let Some(feature) = query.feature {
            let mut out = Vec::with_capacity(settings.len());

            for s in settings {
                if s.schema.feature == feature {
                    out.push(s);
                }
            }

            settings = out;
        }

        Ok(warp::reply::json(&settings))
    }

    /// Delete the given setting by key.
    fn delete_setting(&self, key: &str) -> Result<impl warp::Reply, anyhow::Error> {
        self.settings()?.clear(key)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Get the given setting by key.
    fn get_setting(&self, key: &str) -> Result<impl warp::Reply, anyhow::Error> {
        let setting: Option<crate::settings::Setting> = self
            .settings()?
            .setting::<serde_json::Value>(key)?
            .map(|s| s.to_setting());
        Ok(warp::reply::json(&setting))
    }

    /// Delete the given setting by key.
    fn edit_setting(
        &self,
        key: &str,
        value: serde_json::Value,
    ) -> Result<impl warp::Reply, anyhow::Error> {
        self.settings()?.set_json(key, value)?;
        Ok(warp::reply::json(&EMPTY))
    }
}
