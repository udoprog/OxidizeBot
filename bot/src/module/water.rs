use crate::{auth, command, currency::Currency, module, prelude::*, stream_info, utils};
use anyhow::Error;
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
        ctx: &mut command::Context,
    ) -> Option<(DateTime<Utc>, Option<Reward>)> {
        if let Some((when, user)) = self.waters.last() {
            return Some((*when, user.clone()));
        }

        let started_at = self
            .stream_info
            .data
            .read()
            .stream
            .as_ref()
            .map(|s| s.started_at);

        let started_at = match started_at {
            Some(started_at) => started_at,
            None => {
                ctx.respond("Sorry, the !water command is currently not available :(");
                return None;
            }
        };

        self.waters.push((started_at, None));
        Some((started_at, None))
    }
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&mut self, mut ctx: command::Context) -> Result<(), Error> {
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

        let a = ctx.next();

        match a.as_deref() {
            Some("undo") => {
                ctx.check_scope(auth::Scope::WaterUndo).await?;

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

                if let Err(e) = currency
                    .balance_add(ctx.channel(), &reward.user, -reward.amount)
                    .await
                {
                    log::error!("failed to undo water from database: {}", e);
                }
            }
            None => {
                let (last, _) = match self.check_waters(&mut ctx) {
                    Some(water) => water,
                    None => return Ok(()),
                };

                let user = match ctx.user.real() {
                    Some(user) => user,
                    None => {
                        ctx.privmsg("Can only get balance for real users.");
                        return Ok(());
                    }
                };

                let now = Utc::now();
                let diff = now - last;
                let amount = i64::max(0i64, diff.num_minutes());
                let amount = (amount * *self.reward_multiplier.read() as i64) / 100i64;

                self.waters.push((
                    now,
                    Some(Reward {
                        user: user.name().to_string(),
                        amount,
                    }),
                ));

                ctx.respond(format!(
                    "{streamer}, DRINK SOME WATER! {user} has been rewarded {amount} {currency} for the reminder.", streamer = ctx.user.streamer().display_name,
                    user = user.display_name(),
                    amount = amount,
                    currency = currency.name
                ));

                if let Err(e) = currency
                    .balance_add(ctx.channel(), user.name(), amount)
                    .await
                {
                    log::error!("failed to appply water balance: {}", e);
                }
            }
            Some(_) => {
                ctx.respond("Expected: !water, or !water undo.");
            }
        }

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "water"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            stream_info,
            settings,
            injector,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        let enabled = settings.var("water/enabled", false)?;
        let cooldown = settings.var(
            "water/cooldown",
            utils::Cooldown::from_duration(utils::Duration::seconds(60)),
        )?;
        let reward_multiplier = settings.var("water/reward%", 100)?;

        handlers.insert(
            "water",
            Handler {
                enabled,
                cooldown,
                currency: injector.var()?,
                waters: Vec::new(),
                stream_info: stream_info.clone(),
                reward_multiplier,
            },
        );

        Ok(())
    }
}
