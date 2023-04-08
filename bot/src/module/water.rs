use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

use crate::api;
use crate::auth;
use crate::command;
use crate::currency::Currency;
use crate::module;
use crate::prelude::*;
use crate::stream_info;
use crate::utils;

#[derive(Clone)]
pub(crate) struct Reward {
    user: String,
    amount: i64,
}

pub(crate) struct Handler {
    enabled: settings::Var<bool>,
    cooldown: settings::Var<utils::Cooldown>,
    currency: injector::Ref<Currency>,
    waters: Mutex<Vec<(DateTime<Utc>, Option<Reward>)>>,
    stream_info: stream_info::StreamInfo,
    reward_multiplier: settings::Var<u32>,
    streamer: api::TwitchAndUser,
}

impl Handler {
    async fn check_waters(
        &self,
        waters: &mut Vec<(DateTime<Utc>, Option<Reward>)>,
    ) -> Result<(DateTime<Utc>, Option<Reward>)> {
        if let Some((when, user)) = waters.last() {
            return Ok((*when, user.clone()));
        }

        let started_at = self
            .stream_info
            .data
            .read()
            .stream
            .as_ref()
            .map(|s| s.started_at);

        let started_at = started_at.ok_or(respond_err!(
            "Sorry, the !water command is currently not available :("
        ))?;

        waters.push((started_at, None));
        Ok((started_at, None))
    }
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let currency = match self.currency.load().await {
            Some(currency) => currency,
            None => {
                respond!(ctx, "No currency configured for stream, sorry :(");
                return Ok(());
            }
        };

        if !self.cooldown.write().await.is_open() {
            respond!(
                ctx,
                "A !water command was recently issued, please wait a bit longer!"
            );
            return Ok(());
        }

        let a = ctx.next();

        match a.as_deref() {
            Some("undo") => {
                ctx.check_scope(auth::Scope::WaterUndo).await?;
                let mut waters = self.waters.lock().await;
                let (_, reward) = self.check_waters(&mut waters).await?;

                waters.pop();

                let reward = match reward {
                    Some(reward) => reward,
                    None => {
                        respond!(ctx, "No one has been rewarded for !water yet cmonBruh");
                        return Ok(());
                    }
                };

                ctx.privmsg(format!(
                    "{user} issued a bad !water that is now being undone FeelsBadMan",
                    user = reward.user
                ))
                .await;

                if let Err(e) = currency.balance_add(&reward.user, -reward.amount).await {
                    tracing::error!("Failed to undo water from database: {}", e);
                }
            }
            None => {
                let mut waters = self.waters.lock().await;
                let (last, _) = self.check_waters(&mut waters).await?;

                let user = match ctx.user.real() {
                    Some(user) => user,
                    None => {
                        ctx.privmsg("Can only get balance for real users.").await;
                        return Ok(());
                    }
                };

                let now = Utc::now();
                let diff = now - last;
                let amount = i64::max(0i64, diff.num_minutes());
                let amount = (amount * self.reward_multiplier.load().await as i64) / 100i64;

                waters.push((
                    now,
                    Some(Reward {
                        user: user.login().to_string(),
                        amount,
                    }),
                ));

                respond!(
                    ctx,
                    "{streamer}, DRINK SOME WATER! {user} has been rewarded {amount} {currency} for the reminder.",
                    streamer = self.streamer.user.login,
                    user = user.display_name(),
                    amount = amount,
                    currency = currency.name
                );

                if let Err(e) = currency.balance_add(user.login(), amount).await {
                    tracing::error!("Failed to appply water balance: {}", e);
                }
            }
            Some(_) => {
                respond!(ctx, "Expected: !water, or !water undo.");
            }
        }

        Ok(())
    }
}

pub(crate) struct Module;

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
            streamer,
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        let enabled = settings.var("water/enabled", false).await?;
        let cooldown = settings
            .var(
                "water/cooldown",
                utils::Cooldown::from_duration(utils::Duration::seconds(60)),
            )
            .await?;
        let reward_multiplier = settings.var("water/reward%", 100).await?;

        handlers.insert(
            "water",
            Handler {
                enabled,
                cooldown,
                currency: injector.var().await,
                waters: Mutex::new(Vec::new()),
                stream_info: stream_info.clone(),
                reward_multiplier,
                streamer: streamer.clone(),
            },
        );

        Ok(())
    }
}
