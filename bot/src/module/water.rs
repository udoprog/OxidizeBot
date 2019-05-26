use crate::{command, config, currency, db, module, stream_info, utils};
use chrono::{DateTime, Utc};
use failure::format_err;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct Reward {
    user: String,
    amount: i64,
}

pub struct Handler {
    db: db::Database,
    currency: currency::Currency,
    cooldown: utils::Cooldown,
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
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !self.cooldown.is_open() {
            ctx.respond("A !water command was recently issued, please wait a bit longer!");
            return Ok(());
        }

        match ctx.next() {
            Some("undo") => {
                ctx.check_moderator()?;
                let (_, reward) = match self.check_waters(&mut ctx) {
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

                let db = self.db.clone();
                let target = ctx.user.target.to_string();

                ctx.spawn(async move {
                    let op = db.balance_add(target, reward.user, -reward.amount);

                    match op.await {
                        Ok(()) => (),
                        Err(e) => {
                            log::error!("failed to undo water from database: {}", e);
                        }
                    }
                });
            }
            None => {
                let (last, _) = match self.check_waters(&mut ctx) {
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
                    "{streamer}, DRINK SOME WATER! {user} has been rewarded {amount} {currency} for the reminder.", streamer = ctx.streamer,
                    user = ctx.user.name,
                    amount = amount,
                    currency = self.currency.name
                ));

                let db = self.db.clone();
                let target = ctx.user.target.to_string();

                ctx.spawn(async move {
                    let op = db.balance_add(target, user, amount);

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

pub struct Module {
    cooldown: utils::Cooldown,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_cooldown")]
    cooldown: utils::Cooldown,
}

fn default_cooldown() -> utils::Cooldown {
    utils::Cooldown::from_duration(utils::Duration::seconds(60))
}

impl Module {
    pub fn load(_config: &config::Config, module: &Config) -> Result<Self, failure::Error> {
        Ok(Module {
            cooldown: module.cooldown.clone(),
        })
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
            db,
            handlers,
            currency,
            stream_info,
            settings,
            futures,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        let reward_multiplier = settings.sync_var(futures, "water/reward%", 100)?;

        let currency = currency
            .ok_or_else(|| format_err!("currency required for !swearjar module"))?
            .clone();

        handlers.insert(
            "water",
            Handler {
                db: db.clone(),
                currency,
                cooldown: self.cooldown.clone(),
                waters: Vec::new(),
                stream_info: stream_info.clone(),
                reward_multiplier,
            },
        );

        Ok(())
    }
}
