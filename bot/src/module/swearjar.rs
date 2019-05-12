use crate::{command, config, currency, db, module, twitch, utils};
use failure::format_err;
use futures::Future as _;
use hashbrown::HashSet;

pub struct Handler {
    reward: i64,
    db: db::Database,
    currency: currency::Currency,
    twitch: twitch::Twitch,
    cooldown: utils::Cooldown,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !self.cooldown.is_open() {
            ctx.respond("A !swearjar command was recently issued, please wait a bit longer!");
            return Ok(());
        }

        ctx.spawn(
            self.twitch
                .chatters(ctx.user.target)
                .and_then(|chatters| {
                    let mut u = HashSet::new();
                    u.extend(chatters.viewers);
                    u.extend(chatters.moderators);

                    if u.is_empty() {
                        Err(format_err!("no chatters to reward"))
                    } else {
                        Ok(u)
                    }
                })
                // update database.
                .and_then({
                    let channel = ctx.user.target.to_string();
                    let db = self.db.clone();
                    let reward = self.reward;
                    let streamer = ctx.streamer.to_string();

                    move |u| {
                        let total_reward = reward * u.len() as i64;

                        db.balance_add(channel.as_str(), streamer.as_str(), -total_reward)
                            .and_then(move |_| {
                                db.balances_increment(channel.as_str(), u, reward)
                            })
                            .map(move |_| total_reward)
                    }
                })
                .map({
                    let channel = ctx.user.target.to_string();
                    let currency = self.currency.clone();
                    let sender = ctx.sender.clone();
                    let streamer = ctx.streamer.to_string();

                    move |total_reward| {
                        sender.privmsg(
                            channel.as_str(),
                            format!(
                                "/me has taken {} {currency} from {streamer} and given it to the viewers for listening to their bad mouth!",
                                total_reward, currency = currency.name, streamer = streamer,
                            ),
                        );
                    }
                })
                // handle any errors.
                .or_else(|e| {
                    log_err!(e, "failed to reward users for !swearjar");
                    Ok(())
                }),
        );

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
