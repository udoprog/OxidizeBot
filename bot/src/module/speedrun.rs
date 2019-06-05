use crate::{
    api, auth, command,
    db::Cache,
    module,
    prelude::*,
    utils::{self, Duration},
};
use failure::format_err;
use failure::Error;
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the !speedrun command.
pub struct Speedrun {
    speedrun: CachedSpeedrun,
    enabled: Arc<RwLock<bool>>,
    top: Arc<RwLock<u32>>,
}

impl command::Handler for Speedrun {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Speedrun)
    }

    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        match ctx.next() {
            Some("record") => {
                let top = *self.top.read();

                let game = match ctx.next_str("<game> [user]", "!speedrun record") {
                    Some(game) => String::from(game),
                    None => return Ok(()),
                };

                let mut match_user = None;
                let mut match_category = None;
                let mut include_misc = false;

                while let Some(arg) = ctx.next() {
                    match arg {
                        "--user" => match ctx.next() {
                            Some(u) => match_user = Some(u.to_string()),
                            None => {
                                ctx.respond("Expected argument to `--user`");
                                return Ok(());
                            }
                        },
                        "--category" => match ctx.next() {
                            Some(u) => {
                                match_category = Some(u.to_lowercase());
                                // since we are matching by name we need to show all.
                                include_misc = true;
                            }
                            None => {
                                ctx.respond("Expected argument to `--category`");
                                return Ok(());
                            }
                        },
                        "--misc" => include_misc = true,
                        other => {
                            ctx.respond(format!("`{}` is not a valid parameter", other));
                            return Ok(());
                        }
                    }
                }

                let speedrun = self.speedrun.clone();
                let user = ctx.user.as_owned_user();

                ctx.spawn_result("speedrun/record", async move {
                    let game = speedrun.game_by_id(game).await?;

                    let game = match game {
                        Some(game) => game,
                        None => {
                            user.respond("No such game on speedrun.com");
                            return Ok(());
                        }
                    };

                    let categories = speedrun.game_categories_by_id(game.id.clone()).await?;

                    let categories = match categories {
                        Some(categories) => categories,
                        None => {
                            user.respond("No categories for that game");
                            return Ok(());
                        }
                    };

                    let mut results = Vec::new();

                    for category in categories {
                        if category.ty != api::speedrun::CategoryType::PerGame {
                            continue;
                        }

                        if category.miscellaneous && !include_misc {
                            continue;
                        }

                        if let Some(match_category) = match_category.as_ref() {
                            if !category.name.to_lowercase().contains(match_category) {
                                continue;
                            }
                        }

                        let records = speedrun
                            .category_records_by_id(category.id.clone(), top)
                            .await?;

                        let records = match records.and_then(|r| r.data.into_iter().next()) {
                            Some(records) => records,
                            None => continue,
                        };

                        let mut runs = Vec::new();

                        for run in records.runs.into_iter() {
                            if runs.len() >= 3 {
                                break;
                            }

                            let player = match run.run.players.iter().next() {
                                Some(player) => player,
                                None => continue,
                            };

                            let user = match speedrun.user_by_id(player.id.clone()).await? {
                                Some(user) => user,
                                None => continue,
                            };

                            if let Some(match_user) = match_user.as_ref() {
                                if !user.names.matches(match_user) {
                                    continue;
                                }
                            }

                            let duration = utils::digital_duration(run.run.times.primary.as_std());
                            runs.push(format!(
                                "#{}: {} ({})",
                                run.place,
                                user.names.name(),
                                duration
                            ));
                        }

                        let runs = match runs.as_slice() {
                            [] => continue,
                            runs => format!("{}", runs.join(", ")),
                        };

                        results.push(format!("{} -> {}", category.name, runs));
                    }

                    match results.as_slice() {
                        [] => {
                            user.respond("*no runs*");
                        }
                        results => {
                            let results = format!("{}.", results.join("; "));
                            user.respond(format!("{}", results));
                        }
                    };

                    Ok(())
                });
            }
            _ => {
                ctx.respond(format!(
                    "Expected: {c} record",
                    c = ctx.alias.unwrap_or("!speedrun record")
                ));
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
struct CachedSpeedrun {
    cache: Cache,
    speedrun: api::Speedrun,
}

impl CachedSpeedrun {
    /// Get cached user information by ID.
    pub async fn user_by_id(&self, user: String) -> Result<Option<api::speedrun::User>, Error> {
        let key = format!("speedrun/users/{}", user);
        let future = self.speedrun.user_by_id(user);
        self.cache.wrap(key, Duration::hours(72), future).await
    }

    /// Get cached user information by ID.
    pub async fn category_records_by_id(
        &self,
        category: String,
        top: u32,
    ) -> Result<Option<api::speedrun::Page<api::speedrun::GameRecord>>, Error> {
        let key = format!("speedrun/categories/{}/records/top:{}", category, top);
        let future = self.speedrun.category_records_by_id(category, top);
        self.cache.wrap(key, Duration::hours(72), future).await
    }

    /// Get cached game record by ID.
    pub async fn game_by_id(&self, game: String) -> Result<Option<api::speedrun::Game>, Error> {
        let key = format!("speedrun/games/{}", game);
        let future = self.speedrun.game_by_id(game);
        self.cache.wrap(key, Duration::hours(24), future).await
    }

    /// Get cached game record by ID.
    pub async fn game_categories_by_id(
        &self,
        game: String,
    ) -> Result<Option<Vec<api::speedrun::Category>>, Error> {
        let key = format!("speedrun/games/{}/categories", game);
        let future = self.speedrun.game_categories_by_id(game);
        self.cache.wrap(key, Duration::hours(24), future).await
    }
}

pub struct Module;

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "8ball"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            futures,
            injector,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        let mut vars = settings.vars();

        let cache = injector.get().ok_or_else(|| format_err!("missing cache"))?;
        let speedrun = injector
            .get()
            .ok_or_else(|| format_err!("missing speedrun api"))?;

        let speedrun = CachedSpeedrun { cache, speedrun };

        handlers.insert(
            "speedrun",
            Speedrun {
                speedrun,
                enabled: vars.var("speedrun/enabled", false)?,
                top: vars.var("speedrun/top", 20)?,
            },
        );

        futures.push(vars.run().boxed());
        Ok(())
    }
}
