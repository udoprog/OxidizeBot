use crate::{
    api,
    auth::Scope,
    command, config,
    currency::Currency,
    db, module,
    prelude::*,
    utils::{Cooldown, Duration},
};
use hashbrown::HashSet;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct Handler<'a> {
    enabled: Arc<RwLock<bool>>,
    reward: Arc<RwLock<i64>>,
    cooldown: Arc<RwLock<Cooldown>>,
    db: db::Database,
    currency: Arc<RwLock<Option<Currency>>>,
    twitch: &'a api::Twitch,
}

impl<'a> command::Handler for Handler<'a> {
    fn scope(&self) -> Option<Scope> {
        Some(Scope::SwearJar)
    }

    fn handle<'m>(&mut self, ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let currency = match self.currency.read().clone() {
            Some(currency) => currency,
            None => {
                ctx.respond("No currency configured for stream, sorry :(");
                return Ok(());
            }
        };

        if !self.cooldown.write().is_open() {
            ctx.respond("A !swearjar command was recently issued, please wait a bit longer!");
            return Ok(());
        }

        let db = self.db.clone();
        let twitch = self.twitch.clone();
        let sender = ctx.sender.clone();
        let streamer = ctx.streamer.to_string();
        let channel = ctx.user.target.to_string();
        let reward = *self.reward.read();

        let future = async move {
            let chatters = twitch.chatters(channel.clone()).await?;

            let mut u = HashSet::new();
            u.extend(chatters.viewers);
            u.extend(chatters.moderators);

            if u.is_empty() {
                failure::bail!("no chatters to reward");
            }

            let total_reward = reward * u.len() as i64;

            db.balance_add(channel.clone(), streamer.clone(), -total_reward)
                .await?;

            db.balances_increment(channel.clone(), u, reward).await?;

            sender.privmsg(
                format!(
                    "/me has taken {} {currency} from {streamer} and given it to the viewers for listening to their bad mouth!",
                    total_reward, currency = currency.name, streamer = streamer,
                ),
            );

            Ok(())
        };

        ctx.spawn(future.map(|result| match result {
            Ok(()) => (),
            Err(e) => {
                log_err!(e, "failed to reward users for !swearjar");
            }
        }));

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// Amount payed out for each swear.
    #[serde(default)]
    reward: Option<i64>,
    #[serde(default)]
    cooldown: Option<Duration>,
}

pub struct Module {
    default_reward: Option<i64>,
    default_cooldown: Option<Duration>,
}

impl Module {
    pub fn load(config: &config::Config) -> Self {
        let mut default_reward = None;
        let mut default_cooldown = None;

        for m in &config.modules {
            match *m {
                module::Config::SwearJar(ref config) => {
                    log::warn!("`[[modules]] type = \"swearjar\"` configuration is deprecated");
                    default_reward = config.reward.clone();
                    default_cooldown = config.cooldown.clone();
                }
                _ => (),
            }
        }

        Module {
            default_reward,
            default_cooldown,
        }
    }
}

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "swearjar"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            db,
            handlers,
            twitch,
            injector,
            futures,
            settings,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        let default_reward = self.default_reward.unwrap_or(10);
        let mut vars = settings.vars();
        let enabled = vars.var("swearjar/enabled", false)?;
        let reward = vars.var("swearjar/reward", default_reward)?;
        futures.push(vars.run().boxed());

        let default_cooldown = self
            .default_cooldown
            .clone()
            .unwrap_or(Duration::seconds(60 * 10));
        let (mut cooldown_stream, cooldown) =
            settings.stream("swearjar/cooldown", default_cooldown)?;

        let cooldown = Arc::new(RwLock::new(Cooldown::from_duration(cooldown)));

        let currency = injector.var(futures);

        handlers.insert(
            "swearjar",
            Handler {
                enabled,
                reward,
                cooldown: cooldown.clone(),
                db: db.clone(),
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
