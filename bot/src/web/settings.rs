use crate::web::{Fragment, EMPTY};
use hashbrown::HashSet;
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
pub struct Settings(crate::settings::Settings);

impl Settings {
    pub fn route(settings: crate::settings::Settings) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Settings(settings);

        let list = warp::get2()
            .and(warp::path("settings").and(warp::query::<SettingsQuery>()))
            .and_then({
                let api = api.clone();
                move |query: SettingsQuery| api.settings(query).map_err(warp::reject::custom)
            })
            .boxed();

        let get = warp::get2()
            .and(warp::path("settings").and(path::tail()).and_then({
                let api = api.clone();
                move |key: path::Tail| {
                    let key = str::parse::<Fragment>(key.as_str()).map_err(warp::reject::custom)?;
                    api.get_setting(key.as_str()).map_err(warp::reject::custom)
                }
            }))
            .boxed();

        let delete = warp::delete2()
            .and(warp::path("settings").and(path::tail()).and_then({
                let api = api.clone();
                move |key: path::Tail| {
                    let key = str::parse::<Fragment>(key.as_str()).map_err(warp::reject::custom)?;
                    api.delete_setting(key.as_str())
                        .map_err(warp::reject::custom)
                }
            }))
            .boxed();

        let edit = warp::put2()
            .and(
                warp::path("settings")
                    .and(path::tail().and(body::json()))
                    .and_then({
                        let api = api.clone();
                        move |key: path::Tail, body: PutSetting| {
                            let key = str::parse::<Fragment>(key.as_str())
                                .map_err(warp::reject::custom)?;
                            api.edit_setting(key.as_str(), body.value)
                                .map_err(warp::reject::custom)
                        }
                    }),
            )
            .boxed();

        return list.or(get).or(delete).or(edit).boxed();
    }

    /// Get the list of all settings in the bot.
    fn settings(&self, query: SettingsQuery) -> Result<impl warp::Reply, failure::Error> {
        let mut settings = match query.prefix {
            Some(prefix) => {
                let mut out = Vec::new();

                for prefix in prefix.split(",") {
                    out.extend(self.0.list_by_prefix(&prefix)?);
                }

                out
            }
            None => self.0.list()?,
        };

        if let Some(key) = query.key {
            let key = key
                .split(",")
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
    fn delete_setting(&self, key: &str) -> Result<impl warp::Reply, failure::Error> {
        self.0.clear(key)?;
        Ok(warp::reply::json(&EMPTY))
    }

    /// Get the given setting by key.
    fn get_setting(&self, key: &str) -> Result<impl warp::Reply, failure::Error> {
        let setting: Option<crate::settings::Setting> = self
            .0
            .setting::<serde_json::Value>(key)?
            .map(|s| s.to_setting());
        Ok(warp::reply::json(&setting))
    }

    /// Delete the given setting by key.
    fn edit_setting(
        &self,
        key: &str,
        value: serde_json::Value,
    ) -> Result<impl warp::Reply, failure::Error> {
        self.0.set_json(key, value)?;
        Ok(warp::reply::json(&EMPTY))
    }
}
