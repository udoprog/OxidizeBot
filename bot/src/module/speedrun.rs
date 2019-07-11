use crate::{
    api::{
        self,
        speedrun::{
            Category, CategoryType, Embed, Embeds, Game, GameRecord, Level, Page, Players,
            RelatedPlayer, Run, RunInfo, User, Variable, Variables,
        },
    },
    auth, command, module,
    prelude::*,
    storage::Cache,
    utils::{self, Duration},
};
use failure::{format_err, Error};
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the !speedrun command.
pub struct Speedrun {
    speedrun: CachedSpeedrun,
    enabled: Arc<RwLock<bool>>,
    top: Arc<RwLock<u32>>,
}

impl Speedrun {
    /// Query a user.
    fn query_personal_bests(&self, mut ctx: command::Context<'_>) -> Result<(), Error> {
        let mut query_user = None;
        let mut category_filter = CategoryFilter::default();
        let mut games = Vec::new();
        let mut match_level = None;
        let mut abbrev = false;

        category_filter.ty = Some(CategoryType::PerGame);

        while let Some(arg) = ctx.next().as_ref().map(String::as_str) {
            match arg {
                "--game" => match ctx.next() {
                    Some(g) => games.push(g.to_lowercase()),
                    None => {
                        ctx.respond("Expected argument to `--game`");
                        return Ok(());
                    }
                },
                "--per-level" => category_filter.ty = Some(CategoryType::PerLevel),
                "--level" => match ctx.next() {
                    Some(level) => {
                        match_level = Some(level.to_lowercase());
                        category_filter.ty = Some(CategoryType::PerLevel)
                    }
                    None => {
                        ctx.respond("Expected argument to `--level`");
                        return Ok(());
                    }
                },
                "--category" => match ctx.next() {
                    Some(u) => {
                        category_filter.category_name = Some(u.to_lowercase());
                        // since we are matching by name we need to show all.
                        category_filter.misc = true;
                    }
                    None => {
                        ctx.respond("Expected argument to `--category`");
                        return Ok(());
                    }
                },
                "--sub-category" => match ctx.next() {
                    Some(u) => {
                        category_filter.sub_category_name = Some(u.to_lowercase());
                        // since we are matching by sub category we need all.
                        category_filter.misc = true;
                    }
                    None => {
                        ctx.respond("Expected argument to `--sub-category`");
                        return Ok(());
                    }
                },
                "--misc" => category_filter.misc = true,
                "--misc-only" => {
                    category_filter.main = false;
                    category_filter.misc = true;
                }
                "--abbrev" => abbrev = true,
                other if other.starts_with("--") => {
                    ctx.respond(format!("`{}` is not a valid parameter", other));
                    return Ok(());
                }
                other if query_user.is_none() => {
                    query_user = Some(other.to_lowercase());
                }
                _ => {
                    ctx.respond("did not expect more arguments");
                    return Ok(());
                }
            }
        }

        let query_user = query_user.unwrap_or_else(|| ctx.user.name().to_lowercase());

        let speedrun = self.speedrun.clone();
        let user = ctx.user.clone();
        let async_user = user.clone();

        let future = async move {
            let user = async_user;
            let match_level = match_level.as_ref().map(|s| s.as_str());

            let u = match speedrun.user_by_id(&query_user).await? {
                Some(u) => u,
                None => {
                    user.respond(format!("No user on speedrun.com named `{}`", query_user));
                    return Ok(());
                }
            };

            let mut embeds = Embeds::default();
            embeds.push(Embed::Game);
            embeds.push(Embed::Category);
            let personal_bests = speedrun.user_personal_bests(&u.id, &embeds).await?;

            let personal_bests = match personal_bests {
                Some(personal_bests) => personal_bests,
                None => {
                    user.respond("No personal bests found");
                    return Ok(());
                }
            };

            let mut by_game = HashMap::<String, Group>::new();

            for mut run in personal_bests {
                let game = match run.game.take() {
                    Some(game) => game.data,
                    None => continue,
                };

                if !games.is_empty() {
                    if !games.iter().any(|g| game.matches(g)) {
                        continue;
                    }
                }

                let category = match run.category.take() {
                    Some(category) => category.data,
                    None => continue,
                };

                if !category_filter.match_category(&category) {
                    continue;
                }

                let levels = speedrun.game_levels(&game.id);
                let variables = speedrun.category_variables(&run.run.category);

                let (levels, variables) = future::try_join(levels, variables).await?;

                let variables = match variables {
                    Some(variables) => variables,
                    None => continue,
                };

                let sub_categories = SubCategory::from_variables(&variables);

                let mut name = category.name.clone();

                if let Some(c) = SubCategory::match_run(&run.run, &sub_categories) {
                    if !category_filter.match_sub_category(&c) {
                        continue;
                    }

                    if abbrev {
                        name = format!("{} {}", name, abbreviate_text(&c.label));
                    } else {
                        name = format!("{} / {}", name, c.label);
                    }
                } else {
                    // skip if we are filtering over sub categories.
                    if category_filter.sub_category_name.is_some() {
                        continue;
                    }
                }

                if let Some(levels) = levels {
                    if let Some(level) = match_levels(run.run.level.as_ref(), &levels) {
                        if let Some(match_level) = match_level {
                            if !level.matches(match_level) {
                                continue;
                            }
                        }

                        if abbrev {
                            name = format!("{} {}", level.name, abbreviate_text(&name));
                        } else {
                            name = format!("{} ({})", level.name, name);
                        }
                    }
                }

                by_game
                    .entry(game.id.clone())
                    .or_insert_with(|| Group::new(game))
                    .runs
                    .push(GroupRun { name, run });
            }

            let mut results = Vec::new();

            for (_, group) in by_game {
                let mut runs = Vec::new();

                for GroupRun { name, run } in group.runs {
                    let duration = utils::compact_duration(run.run.times.primary.as_std());
                    runs.push(format!("{}: {} (#{})", name, duration, run.place));
                }

                results.push(format!(
                    "{} ({}) -> {}",
                    group.game.names.name(),
                    group.game.abbreviation,
                    runs.join(" - ")
                ));
            }

            user.respond_lines(results, "*no runs*");
            Ok::<(), Error>(())
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

        return Ok(());

        /// Runs per game.
        struct Group {
            game: Game,
            runs: Vec<GroupRun>,
        }

        impl Group {
            pub fn new(game: Game) -> Self {
                Self {
                    game,
                    runs: Vec::new(),
                }
            }
        }

        struct GroupRun {
            name: String,
            run: Run,
        }
    }

    /// Query a game.
    fn query_game(&self, mut ctx: command::Context<'_>) -> Result<(), Error> {
        let top = *self.top.read();

        let game_query = match ctx.next_str("<game> [options]") {
            Some(game_query) => String::from(game_query),
            None => return Ok(()),
        };

        let mut match_user = None;
        let mut category_filter = CategoryFilter::default();
        let mut abbrev = false;

        category_filter.ty = Some(CategoryType::PerGame);

        while let Some(arg) = ctx.next().as_ref().map(String::as_str) {
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
                        category_filter.category_name = Some(u.to_lowercase());
                        // since we are matching by name we need to show all.
                        category_filter.misc = true;
                    }
                    None => {
                        ctx.respond("Expected argument to `--category`");
                        return Ok(());
                    }
                },
                "--sub-category" => match ctx.next() {
                    Some(u) => {
                        category_filter.sub_category_name = Some(u.to_lowercase());
                        // since we are matching by sub category we need all.
                        category_filter.misc = true;
                    }
                    None => {
                        ctx.respond("Expected argument to `--sub-category`");
                        return Ok(());
                    }
                },
                "--misc" => category_filter.misc = true,
                "--misc-only" => {
                    category_filter.misc = true;
                    category_filter.main = false;
                }
                "--abbrev" => abbrev = true,
                other => {
                    ctx.respond(format!("`{}` is not a valid parameter", other));
                    return Ok(());
                }
            }
        }

        let speedrun = self.speedrun.clone();
        let user = ctx.user.clone();
        let async_user = user.clone();

        let future = async move {
            let user = async_user;
            let match_user = match_user.as_ref().map(|s| s.as_str());

            let game = speedrun.game_by_id(&game_query).await?;

            let game = match game {
                Some(game) => game,
                None => {
                    user.respond(format!("No game matching `{}`", game_query));
                    return Ok::<(), Error>(());
                }
            };

            let mut embeds = Embeds::default();
            embeds.push(Embed::Variables);

            let categories = speedrun.game_categories_by_id(&game.id, &embeds).await?;

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
                if !category_filter.match_category(category) {
                    continue;
                }

                let variables = match &category.variables {
                    Some(variables) => &variables.data,
                    None => continue,
                };

                let sub_categories = SubCategory::from_variables(variables);

                if sub_categories.is_empty() {
                    if category_filter.sub_category_name.is_some() {
                        continue;
                    }

                    categories_to_use.push((category.name.clone(), Variables::default(), category));
                    continue;
                }

                for c in sub_categories {
                    if !category_filter.match_sub_category(&c) {
                        continue;
                    }

                    let mut name = category.name.clone();
                    let mut variables = Variables::default();

                    variables.insert(c.key, c.value);

                    if abbrev {
                        name = format!("{} {}", name, abbreviate_text(&c.label));
                    } else {
                        name = format!("{} ({})", name, c.label);
                    }

                    categories_to_use.push((name, variables, category));
                }
            }

            let num_categories = categories_to_use.len();
            let mut embeds = Embeds::default();
            embeds.push(Embed::Players);

            for (name, variables, category) in categories_to_use {
                let records = speedrun
                    .leaderboard(&game.id, &category.id, top, &variables, &embeds)
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
                            let _ = embedded_players.insert(p.id.clone(), p);
                        }
                    }
                }

                let mut runs = Vec::new();

                for run in records.runs.into_iter() {
                    if runs.len() >= 3 || num_categories > 1 && runs.len() >= 1 {
                        break;
                    }

                    let mut names = Vec::new();

                    for player in run.run.players {
                        let name =
                            Self::player_name(&speedrun, &player, match_user, &embedded_players)
                                .await?;
                        names.extend(name);
                    }

                    let duration = utils::compact_duration(run.run.times.primary.as_std());

                    let names = utils::human_list(&names).unwrap_or_else(|| String::from("*none*"));

                    runs.push(format!(
                        "{names}: {duration} (#{place})",
                        names = names,
                        duration = duration,
                        place = run.place
                    ));
                }

                let runs = match runs.as_slice() {
                    [] => continue,
                    runs => runs.join(" - "),
                };

                results.push(format!("{} -> {}", name, runs));
            }

            user.respond_lines(results, "*no runs*");
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

        Ok(())
    }

    /// Extract the player name from a related player.
    async fn player_name<'a>(
        speedrun: &'a CachedSpeedrun,
        player: &'a RelatedPlayer,
        match_user: Option<&'a str>,
        embedded_players: &'a HashMap<String, User>,
    ) -> Result<Option<String>, Error> {
        match *player {
            RelatedPlayer::Player(ref player) => {
                let user = match embedded_players.get(&player.id) {
                    Some(user) => user.clone(),
                    None => match speedrun.user_by_id(&player.id).await? {
                        Some(user) => user,
                        None => return Ok(None),
                    },
                };

                if let Some(match_user) = match_user {
                    if !user.matches(match_user) {
                        return Ok(None);
                    }
                }

                Ok(Some(user.names.name().to_string()))
            }
            RelatedPlayer::Guest(ref guest) => {
                if let Some(match_user) = match_user {
                    if !guest.name.contains(match_user) {
                        return Ok(None);
                    }
                }

                Ok(Some(guest.name.clone()))
            }
        }
    }
}

impl command::Handler for Speedrun {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Speedrun)
    }

    fn handle<'slf: 'a, 'ctx: 'a, 'a>(
        &'slf mut self,
        mut ctx: command::Context<'ctx>,
    ) -> future::BoxFuture<'a, Result<(), failure::Error>> {
        Box::pin(async move {
            if !*self.enabled.read() {
                return Ok(());
            }

            match ctx.next().as_ref().map(String::as_str) {
                Some("personal-bests") => {
                    self.query_personal_bests(ctx)?;
                }
                Some("record") | Some("game") => {
                    self.query_game(ctx)?;
                }
                _ => {
                    ctx.respond("Expected argument: record, personal-bests.");
                }
            }

            Ok(())
        })
    }
}

#[derive(serde::Serialize)]
#[serde(tag = "method")]
pub enum Key<'a> {
    UserById {
        user: &'a str,
    },
    UserPersonalBests {
        user_id: &'a str,
        embeds: &'a Embeds,
    },
    CategoryVariables {
        category_id: &'a str,
    },
    CategoryRecordsById {
        category_id: &'a str,
        top: u32,
    },
    GameById {
        game_id: &'a str,
    },
    GameCategoriesById {
        game_id: &'a str,
        embeds: &'a Embeds,
    },
    GameLevels {
        game_id: &'a str,
    },
    Leaderboard {
        game_id: &'a str,
        category_id: &'a str,
        top: u32,
        variables: &'a Variables,
        embeds: &'a Embeds,
    },
}

#[derive(Clone)]
struct CachedSpeedrun {
    cache: Cache,
    speedrun: api::Speedrun,
}

impl CachedSpeedrun {
    /// Get cached user information by ID.
    pub async fn user_by_id(&self, user: &str) -> Result<Option<User>, Error> {
        self.cache
            .wrap(
                Key::UserById { user },
                Duration::hours(24 * 7),
                self.speedrun.user_by_id(user),
            )
            .await
    }

    /// Get personal bests by user.
    pub async fn user_personal_bests(
        &self,
        user_id: &str,
        embeds: &Embeds,
    ) -> Result<Option<Vec<Run>>, Error> {
        self.cache
            .wrap(
                Key::UserPersonalBests {
                    user_id,
                    embeds: &embeds,
                },
                Duration::hours(2),
                self.speedrun.user_personal_bests(user_id, &embeds),
            )
            .await
    }

    /// Get the variables of a category.
    pub async fn category_variables(
        &self,
        category_id: &str,
    ) -> Result<Option<Vec<Variable>>, Error> {
        self.cache
            .wrap(
                Key::CategoryVariables { category_id },
                Duration::hours(2),
                self.speedrun.category_variables(category_id),
            )
            .await
    }

    /// Get cached user information by ID.
    #[allow(unused)]
    pub async fn category_records_by_id(
        &self,
        category_id: &str,
        top: u32,
    ) -> Result<Option<Page<GameRecord>>, Error> {
        self.cache
            .wrap(
                Key::CategoryRecordsById { category_id, top },
                Duration::hours(24),
                self.speedrun.category_records_by_id(category_id, top),
            )
            .await
    }

    /// Get cached game record by ID.
    pub async fn game_by_id(&self, game_id: &str) -> Result<Option<Game>, Error> {
        self.cache
            .wrap(
                Key::GameById { game_id },
                Duration::hours(24 * 7),
                self.speedrun.game_by_id(game_id),
            )
            .await
    }

    /// Get cached game categories by ID.
    pub async fn game_categories_by_id(
        &self,
        game_id: &str,
        embeds: &Embeds,
    ) -> Result<Option<Vec<Category>>, Error> {
        self.cache
            .wrap(
                Key::GameCategoriesById { game_id, embeds },
                Duration::hours(24),
                self.speedrun.game_categories_by_id(game_id, embeds),
            )
            .await
    }

    /// Get cached game levels by ID.
    pub async fn game_levels(&self, game_id: &str) -> Result<Option<Vec<Level>>, Error> {
        self.cache
            .wrap(
                Key::GameLevels { game_id },
                Duration::hours(72),
                self.speedrun.game_levels(game_id),
            )
            .await
    }

    /// Get the specified leaderboard.
    pub async fn leaderboard(
        &self,
        game_id: &str,
        category_id: &str,
        top: u32,
        variables: &Variables,
        embeds: &Embeds,
    ) -> Result<Option<GameRecord>, Error> {
        self.cache
            .wrap(
                Key::Leaderboard {
                    game_id,
                    category_id,
                    top,
                    variables,
                    embeds,
                },
                Duration::hours(6),
                self.speedrun
                    .leaderboard(game_id, category_id, top, variables, embeds),
            )
            .await
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

        let cache: Cache = injector.get().ok_or_else(|| format_err!("missing cache"))?;
        let speedrun = injector
            .get()
            .ok_or_else(|| format_err!("missing speedrun api"))?;

        let speedrun = CachedSpeedrun {
            cache: cache.namespaced("speedrun"),
            speedrun,
        };

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

struct SubCategory {
    key: String,
    value: String,
    label: String,
    misc: bool,
}

impl SubCategory {
    /// Convert variables into a collection of sub categories.
    fn from_variables(variables: &[Variable]) -> Vec<SubCategory> {
        let mut results = Vec::new();

        let variable = match variables.iter().filter(|v| v.is_subcategory).next() {
            Some(variable) => variable,
            None => return results,
        };

        for (key, value) in &variable.values.values {
            let misc = value.flags.miscellaneous.unwrap_or_default();

            results.push(SubCategory {
                key: variable.id.clone(),
                value: key.to_string(),
                label: value.label.to_string(),
                misc,
            });
        }

        results
    }

    /// Match a run against a collection of sub categories.
    fn match_run<'a>(run: &RunInfo, sub_categories: &'a [SubCategory]) -> Option<&'a SubCategory> {
        for c in sub_categories {
            let value = match run.values.get(&c.key) {
                Some(value) => value,
                None => continue,
            };

            if *value == c.value {
                return Some(c);
            }
        }

        None
    }
}

/// Function to abbreviate texts.
///
/// Numeric components are left as is and require a space after it.
/// For example: `100%` stays `100%`.
///
/// Non-numeric ascii text components are abbreviated with their first letter in uppercase.
/// For example: `No Mission Skips` becomes `NMS`.
fn abbreviate_text(mut text: &str) -> String {
    if text
        .chars()
        .all(|c| !c.is_whitespace() && (c.is_uppercase() || c.is_numeric()))
    {
        return text.to_string();
    }

    let mut out = String::new();

    // If the last added element requires spacing.
    let mut last_spacing = false;

    while text.len() > 0 {
        let c = match text.chars().next() {
            Some(c) => c,
            None => break,
        };

        match c {
            // numeric argument
            '1'..='9' => {
                if last_spacing {
                    out.push(' ');
                }

                let e = text.find(' ').unwrap_or(text.len());
                out.push_str(&text[..e]);
                text = &text[e..];

                last_spacing = true;
            }
            'a'..='z' | 'A'..='Z' => {
                if last_spacing {
                    out.push(' ');
                    last_spacing = false;
                }

                let e = text.find(' ').unwrap_or(text.len());

                for c in c.to_uppercase() {
                    out.push(c);
                }

                text = &text[e..];
            }
            _ => {
                text = &text[1..];
                continue;
            }
        }
    }

    out
}

/// A filter over categories.
pub struct CategoryFilter {
    /// The category type to filter for.
    ty: Option<CategoryType>,
    /// Match main categories.
    main: bool,
    /// Match misc categories.
    misc: bool,
    /// Match by category name.
    category_name: Option<String>,
    /// Match by sub-category name.
    sub_category_name: Option<String>,
}

impl Default for CategoryFilter {
    fn default() -> Self {
        Self {
            ty: None,
            main: true,
            misc: false,
            category_name: None,
            sub_category_name: None,
        }
    }
}

impl CategoryFilter {
    /// Match against a category.
    fn match_category(&self, category: &Category) -> bool {
        if let Some(ty) = self.ty {
            if category.ty != ty {
                return false;
            }
        }

        if category.miscellaneous {
            if !self.misc {
                return false;
            }
        } else {
            if !self.main {
                return false;
            }
        }

        if let Some(category_name) = self.category_name.as_ref() {
            if category.name.to_lowercase() != *category_name {
                return false;
            }
        }

        true
    }

    /// Match against a sub-category.
    fn match_sub_category(&self, sub_category: &SubCategory) -> bool {
        if sub_category.misc {
            if !self.misc {
                return false;
            }
        } else {
            if !self.main {
                return false;
            }
        }

        if let Some(name) = self.sub_category_name.as_ref() {
            if sub_category.label.to_lowercase() != *name {
                return false;
            }
        }

        true
    }
}

/// Match all known levels against the specified run.
fn match_levels<'a>(level: Option<&String>, levels: &'a [Level]) -> Option<&'a Level> {
    let level = match level {
        Some(level) => level,
        None => return None,
    };

    levels.iter().find(|l| l.id == *level)
}

#[cfg(test)]
mod tests {
    use super::abbreviate_text;

    #[test]
    fn test_abbreviate_text() {
        assert_eq!("100% NMS", abbreviate_text("100% No Mission Skips"));
    }
}
