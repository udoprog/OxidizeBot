use crate::{
    api,
    auth::Scope,
    command,
    currency::Currency,
    module,
    prelude::*,
    task,
    utils::{Cooldown, Duration},
};
use anyhow::{bail, Error};
use parking_lot::RwLock;
use std::{collections::HashSet, sync::Arc};

pub struct Handler<'a> {
    enabled: Arc<RwLock<bool>>,
    reward: Arc<RwLock<i64>>,
    cooldown: Arc<RwLock<Cooldown>>,
    currency: Arc<RwLock<Option<Currency>>>,
    twitch: &'a api::Twitch,
}

#[async_trait]
impl<'a> command::Handler for Handler<'a> {
    fn scope(&self) -> Option<Scope> {
        Some(Scope::SwearJar)
    }

    async fn handle(&mut self, ctx: command::Context) -> Result<(), Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let currency = self.currency.read();
        let currency = match currency.as_ref() {
            Some(currency) => currency.clone(),
            None => {
                ctx.respond("No currency configured for stream, sorry :(");
                return Ok(());
            }
        };

        if !self.cooldown.write().is_open() {
            ctx.respond("A !swearjar command was recently issued, please wait a bit longer!");
            return Ok(());
        }

        let twitch = self.twitch.clone();
        let user = ctx.user.clone();
        let reward = *self.reward.read();

        let future = async move {
            let chatters = twitch.chatters(user.channel()).await?;

            let mut u = HashSet::new();
            u.extend(chatters.viewers);
            u.extend(chatters.moderators);

            if u.is_empty() {
                bail!("no chatters to reward");
            }

            let total_reward = reward * u.len() as i64;

            currency
                .balance_add(user.channel(), &user.streamer().name, -total_reward)
                .await?;

            currency
                .balances_increment(user.channel(), u, reward, 0)
                .await?;

            user.sender().privmsg(format!(
                "/me has taken {} {currency} from {streamer} and given it to the viewers for listening to their bad mouth!",
                total_reward, currency = currency.name, streamer = user.streamer().display_name,
            ));

            Ok(())
        };

        task::spawn(future.map(|result| match result {
            Ok(()) => (),
            Err(e) => {
                log_error!(e, "Failed to reward users for !swearjar");
            }
        }));

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "swearjar"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            twitch,
            injector,
            futures,
            settings,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        let enabled = settings.var("swearjar/enabled", false)?;
        let reward = settings.var("swearjar/reward", 10)?;

        let (mut cooldown_stream, cooldown) = settings
            .stream("swearjar/cooldown")
            .or_with(Duration::seconds(60 * 10))?;

        let cooldown = Arc::new(RwLock::new(Cooldown::from_duration(cooldown)));

        let currency = injector.var()?;

        handlers.insert(
            "swearjar",
            Handler {
                enabled,
                reward,
                cooldown: cooldown.clone(),
                currency,
                twitch,
            },
        );

        let future = async move {
            loop {
                futures::select! {
                    update = cooldown_stream.select_next_some() => {
                        cooldown.write().cooldown = update;
                    }
                }
            }
        };

        futures.push(future.boxed());
        Ok(())
    }
}
