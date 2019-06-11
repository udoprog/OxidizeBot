use crate::{
    auth, command, config, currency::Currency, db, module, prelude::*, stream_info, utils,
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct Reward {
    user: String,
    amount: i64,
}

pub struct Handler {
    enabled: Arc<RwLock<bool>>,
    cooldown: Arc<RwLock<utils::Cooldown>>,
    currency: Arc<RwLock<Option<Currency>>>,
    waters: Vec<(DateTime<Utc>, Option<Reward>)>,
    stream_info: stream_info::StreamInfo,
    reward_multiplier: Arc<RwLock<u32>>,
}

impl Handler {
    fn check_waters(
        &mut self,
        ctx: &mut command::Context<'_, '_>,
    ) -> Option<(DateTime<Utc>, Option<Reward>)> {
        if let Some((when, user)) = self.waters.last() {
            return Some((when.clone(), user.clone()));
        }

        let started_at = self
            .stream_info
            .data
            .read()
            .stream
            .as_ref()
            .map(|s| s.started_at.clone());

        let started_at = match started_at {
            Some(started_at) => started_at,
            None => {
                ctx.respond("Sorry, the !water command is currently not available :(");
                return None;
            }
        };

        self.waters.push((started_at.clone(), None));
        Some((started_at, None))
    }
}

impl command::Handler for Handler {
    fn handle(&mut self, ctx: &mut command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let currency = self.currency.read().as_ref().cloned();
        let currency = match currency {
            Some(currency) => currency,
            None => {
                ctx.respond("No currency configured for stream, sorry :(");
                return Ok(());
            }
        };

        if !self.cooldown.write().is_open() {
            ctx.respond("A !water command was recently issued, please wait a bit longer!");
            return Ok(());
        }

        match ctx.next() {
            Some("undo") => {
                ctx.check_scope(auth::Scope::WaterUndo)?;

                let (_, reward) = match self.check_waters(ctx) {
                    Some(water) => water,
                    None => return Ok(()),
                };

                self.waters.pop();

                let reward = match reward {
                    Some(reward) => reward,
                    None => {
                        ctx.respond("No one has been rewarded for !water yet cmonBruh");
                        return Ok(());
                    }
                };

                ctx.privmsg(format!(
                    "{user} issued a bad !water that is now being undone FeelsBadMan",
                    user = reward.user
                ));

                let target = ctx.user.target.to_string();

                ctx.spawn(async move {
                    let op = currency.balance_add(target, reward.user, -reward.amount);

                    match op.await {
                        Ok(()) => (),
                        Err(e) => {
                            log::error!("failed to undo water from database: {}", e);
                        }
                    }
                });
            }
            None => {
                let (last, _) = match self.check_waters(ctx) {
                    Some(water) => water,
                    None => return Ok(()),
                };

                let now = Utc::now();
                let diff = now.clone() - last;
                let amount = i64::max(0i64, diff.num_minutes());
                let amount = (amount * *self.reward_multiplier.read() as i64) / 100i64;
                let user = db::user_id(ctx.user.name);

                self.waters.push((
                    now,
                    Some(Reward {
                        user: user.clone(),
                        amount,
                    }),
                ));

                ctx.respond(format!(
                    "{streamer}, DRINK SOME WATER! {user} has been rewarded {amount} {currency} for the reminder.", streamer = ctx.user.streamer,
                    user = ctx.user.name,
                    amount = amount,
                    currency = currency.name
                ));

                let target = ctx.user.target.to_string();

                ctx.spawn(async move {
                    let op = currency.balance_add(target, user, amount);

                    match op.await {
                        Ok(()) => (),
                        Err(e) => {
                            log::error!("failed to undo water from database: {}", e);
                        }
                    }
                });
            }
            Some(_) => {
                ctx.respond("Expected: !water, or !water undo.");
            }
        }

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    cooldown: Option<utils::Cooldown>,
}

pub struct Module {
    default_cooldown: Option<utils::Cooldown>,
}

impl Module {
    pub fn load(config: &config::Config) -> Self {
        let mut default_cooldown = None;

        for m in &config.modules {
            match *m {
                module::Config::Water(ref config) => {
                    log::warn!("`[[modules]] type = \"water\"` configuration is deprecated");
                    default_cooldown = config.cooldown.clone();
                }
                _ => (),
            }
        }

        Module { default_cooldown }
    }
}

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "water"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            stream_info,
            settings,
            futures,
            injector,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        let default_cooldown = self
            .default_cooldown
            .clone()
            .unwrap_or_else(|| utils::Cooldown::from_duration(utils::Duration::seconds(60)));

        let mut vars = settings.vars();
        let enabled = vars.var("water/enabled", false)?;
        let cooldown = vars.var("water/cooldown", default_cooldown)?;
        let reward_multiplier = vars.var("water/reward%", 100)?;
        futures.push(vars.run().boxed());

        let currency = injector.var(futures);

        handlers.insert(
            "water",
            Handler {
                enabled,
                cooldown: cooldown.clone(),
                currency: currency.clone(),
                waters: Vec::new(),
                stream_info: stream_info.clone(),
                reward_multiplier,
            },
        );

        Ok(())
    }
}
