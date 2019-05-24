use crate::{api, command, config, currency, db, module, prelude::*, utils};
use failure::format_err;
use hashbrown::HashSet;

pub struct Handler {
    reward: i64,
    db: db::Database,
    currency: currency::Currency,
    twitch: api::Twitch,
    cooldown: utils::Cooldown,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !self.cooldown.is_open() {
            ctx.respond("A !swearjar command was recently issued, please wait a bit longer!");
            return Ok(());
        }

        let db = self.db.clone();
        let twitch = self.twitch.clone();
        let currency = self.currency.clone();
        let sender = ctx.sender.clone();
        let streamer = ctx.streamer.to_string();
        let channel = ctx.user.target.to_string();
        let reward = self.reward;

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
                channel.as_str(),
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

pub struct Module {
    reward: i64,
    cooldown: utils::Cooldown,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// Amount payed out for each swear.
    reward: i64,
    #[serde(default = "default_cooldown")]
    cooldown: utils::Cooldown,
}

fn default_cooldown() -> utils::Cooldown {
    utils::Cooldown::from_duration(utils::Duration::seconds(60 * 10))
}

impl Module {
    pub fn load(_config: &config::Config, module: &Config) -> Result<Self, failure::Error> {
        Ok(Module {
            reward: module.reward,
            cooldown: module.cooldown.clone(),
        })
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
            currency,
            twitch,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        let currency = currency
            .ok_or_else(|| format_err!("currency required for !swearjar module"))?
            .clone();

        handlers.insert(
            "swearjar",
            Handler {
                reward: self.reward,
                db: db.clone(),
                currency,
                twitch: twitch.clone(),
                cooldown: self.cooldown.clone(),
            },
        );

        Ok(())
    }
}
