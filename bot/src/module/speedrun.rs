use crate::{
    api::{
        self,
        speedrun::{
            Category, CategoryType, Embed, Embeds, Game, GameRecord, Page, Players, RelatedPlayer,
            User, Variables,
        },
    },
    auth, command,
    db::Cache,
    module,
    prelude::*,
    utils::{self, Duration},
};
use failure::format_err;
use failure::Error;
use hashbrown::HashMap;
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

    fn handle(&mut self, ctx: &mut command::Context<'_, '_>) -> Result<(), Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        match ctx.next() {
            Some("record") => {
                let top = *self.top.read();

                let game_query = match ctx.next_str("<game> [user]", "!speedrun record") {
                    Some(game_query) => String::from(game_query),
                    None => return Ok(()),
                };

                let mut match_user = None;
                let mut match_category = None;
                let mut match_sub_category = None;
                let mut include_main = true;
                let mut include_misc = false;

                while let Some(arg) = ctx.next() {
                    match arg {
                        "--user" => match ctx.next() {
                            Some(u) => match_user = Some(u.to_lowercase()),
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
                        "--sub-category" => match ctx.next() {
                            Some(u) => {
                                match_sub_category = Some(u.to_lowercase());
                                // since we are matching by sub category we need all.
                                include_misc = true;
                            }
                            None => {
                                ctx.respond("Expected argument to `--sub-category`");
                                return Ok(());
                            }
                        },
                        "--misc" => include_misc = true,
                        "--misc-only" => {
                            include_misc = true;
                            include_main = false;
                        }
                        other => {
                            ctx.respond(format!("`{}` is not a valid parameter", other));
                            return Ok(());
                        }
                    }
                }

                let speedrun = self.speedrun.clone();
                let user = ctx.user.as_owned_user();
                let async_user = user.clone();

                let future = async move {
                    let user = async_user;

                    let game = speedrun.game_by_id(game_query.clone()).await?;

                    let game = match game {
                        Some(game) => game,
                        None => {
                            user.respond(format!("No game matching `{}`", game_query));
                            return Ok::<(), Error>(());
                        }
                    };

                    let mut embeds = Embeds::default();
                    embeds.push(Embed::Variables);
                    let categories = speedrun
                        .game_categories_by_id(game.id.clone(), embeds)
                        .await?;

                    let categories = match categories {
                        Some(categories) => categories,
                        None => {
                            user.respond("No categories for that game");
                            return Ok(());
                        }
                    };

                    let mut results = Vec::new();

                    let mut categories_to_use = Vec::new();

                    for category in &categories {
                        if category.ty != CategoryType::PerGame {
                            continue;
                        }

                        if category.miscellaneous {
                            if !include_misc {
                                continue;
                            }
                        } else {
                            if !include_main {
                                continue;
                            }
                        }

                        let variables = match &category.variables {
                            Some(variables) => &variables.data,
                            None => continue,
                        };

                        let mut variations = Vec::new();

                        match variables.iter().filter(|v| v.is_subcategory).next() {
                            Some(variable) => {
                                for (key, value) in &variable.values.values {
                                    let misc = value.flags.miscellaneous.unwrap_or_default();

                                    if misc {
                                        if !include_misc {
                                            continue;
                                        }
                                    } else {
                                        if !include_main {
                                            continue;
                                        }
                                    }

                                    variations.push(Some(Variation {
                                        key: variable.id.clone(),
                                        value: key.to_string(),
                                        label: value.label.to_string(),
                                    }));
                                }
                            }
                            None => {
                                variations.push(None);
                            }
                        };

                        for m in variations {
                            if let Some(match_category) = match_category.as_ref() {
                                if !category.name.to_lowercase().contains(match_category) {
                                    continue;
                                }
                            }

                            // match a sub-category.
                            match (match_sub_category.as_ref(), m.as_ref()) {
                                (Some(_), None) => continue,
                                (Some(sub_category), Some(Variation { ref label, .. })) => {
                                    if !label.to_lowercase().contains(sub_category) {
                                        continue;
                                    }
                                }
                                _ => (),
                            }

                            let mut name = category.name.clone();
                            let mut variables = Variables::default();

                            match m {
                                Some(Variation { key, value, label }) => {
                                    variables.variables.insert(key, value);
                                    name = format!("{} ({})", name, label);
                                }
                                None => (),
                            }

                            categories_to_use.push((name, variables, category));
                        }
                    }

                    let num_categories = categories_to_use.len();
                    let mut embeds = Embeds::default();
                    embeds.push(Embed::Players);

                    for (name, variables, category) in categories_to_use {
                        let records = speedrun
                            .leaderboard(
                                game.id.clone(),
                                category.id.clone(),
                                top,
                                variables,
                                embeds.clone(),
                            )
                            .await?;

                        let records = match records {
                            Some(records) => records,
                            None => continue,
                        };

                        // Embedded players.
                        let mut embedded_players = HashMap::new();

                        if let Some(players) = records.players {
                            for p in players.data {
                                if let Players::User(p) = p {
                                    embedded_players.insert(p.id.clone(), p);
                                }
                            }
                        }

                        let mut runs = Vec::new();

                        for run in records.runs.into_iter() {
                            if runs.len() >= 3 || num_categories > 1 && runs.len() >= 1 {
                                break;
                            }

                            let player = match run.run.players.iter().next() {
                                Some(player) => player,
                                None => continue,
                            };

                            let name = match player {
                                RelatedPlayer::Player(player) => {
                                    let user = match embedded_players.get(&player.id) {
                                        Some(user) => user.clone(),
                                        None => match speedrun.user_by_id(player.id.clone()).await?
                                        {
                                            Some(user) => user,
                                            None => continue,
                                        },
                                    };

                                    if let Some(match_user) = match_user.as_ref() {
                                        if !user.matches(match_user) {
                                            continue;
                                        }
                                    }

                                    user.names.name().to_string()
                                }
                                RelatedPlayer::Guest(guest) => {
                                    if let Some(match_user) = match_user.as_ref() {
                                        if !guest.name.contains(match_user) {
                                            continue;
                                        }
                                    }

                                    guest.name.clone()
                                }
                            };

                            let duration = utils::compact_duration(run.run.times.primary.as_std());

                            runs.push(format!("#{}: {} - {}", run.place, name, duration));
                        }

                        let runs = match runs.as_slice() {
                            [] => continue,
                            runs => format!("{}", runs.join(", ")),
                        };

                        results.push(format!("{} -> {}", name, runs));
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
                };

                ctx.spawn(async move {
                    match future.await {
                        Ok(()) => (),
                        Err(e) => {
                            user.respond("Failed to fetch records :(");
                            log_err!(e, "Failed to fetch records");
                        }
                    }
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

struct Variation {
    key: String,
    value: String,
    label: String,
}

#[derive(Clone)]
struct CachedSpeedrun {
    cache: Cache,
    speedrun: api::Speedrun,
}

impl CachedSpeedrun {
    /// Get cached user information by ID.
    pub async fn user_by_id(&self, user: String) -> Result<Option<User>, Error> {
        let key = format!("speedrun:users/{}", user);
        let future = self.speedrun.user_by_id(user);
        self.cache.wrap(key, Duration::hours(24 * 7), future).await
    }

    /// Get cached user information by ID.
    #[allow(unused)]
    pub async fn category_records_by_id(
        &self,
        category: String,
        top: u32,
    ) -> Result<Option<Page<GameRecord>>, Error> {
        let key = format!("speedrun:categories/{}/records/top:{}", category, top);
        let future = self.speedrun.category_records_by_id(category, top);
        self.cache.wrap(key, Duration::hours(24), future).await
    }

    /// Get cached game record by ID.
    pub async fn game_by_id(&self, game: String) -> Result<Option<Game>, Error> {
        let key = format!("speedrun:games/{}", game);
        let future = self.speedrun.game_by_id(game);
        self.cache.wrap(key, Duration::hours(24 * 7), future).await
    }

    /// Get cached game record by ID.
    pub async fn game_categories_by_id(
        &self,
        game: String,
        embeds: Embeds,
    ) -> Result<Option<Vec<Category>>, Error> {
        let embeds_key = embeds.to_query().unwrap_or_default();
        let key = format!("speedrun:games/{}/categories/embed:{}", game, embeds_key);
        let future = self.speedrun.game_categories_by_id(game, embeds);
        self.cache.wrap(key, Duration::hours(24), future).await
    }

    /// Get the specified leaderboard.
    pub async fn leaderboard(
        &self,
        game: String,
        category: String,
        top: u32,
        variables: Variables,
        embeds: Embeds,
    ) -> Result<Option<GameRecord>, Error> {
        let variables_key = variables.cache_key();
        let embeds_key = embeds.to_query().unwrap_or_default();
        let key = format!(
            "speedrun:leaderboards/{}/category/{}/top:{}/variables:{}/embed:{}",
            game, category, top, variables_key, embeds_key,
        );
        let future = self
            .speedrun
            .leaderboard(game, category, top, variables, embeds);
        self.cache.wrap(key, Duration::hours(6), future).await
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
