use crate::{command, db, irc, module, utils};
use chrono::Utc;
use futures::{future, Async, Future, Poll, Stream as _};
use std::sync::Arc;
use tokio_timer::Interval;

pub struct Handler {
    pub promotions: db::Promotions,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        match ctx.next() {
            Some("list") => {
                let mut names = self
                    .promotions
                    .list(ctx.user.target)
                    .into_iter()
                    .map(|c| c.key.name.to_string())
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    ctx.respond("No custom promotions.");
                } else {
                    names.sort();
                    ctx.respond(format!("Custom promotions: {}", names.join(", ")));
                }
            }
            Some("edit") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                let frequency = match ctx.next() {
                    Some(frequency) => match str::parse::<utils::Duration>(frequency) {
                        Ok(frequency) => frequency,
                        Err(_) => {
                            ctx.respond(format!("Bad <frequency>: {}", frequency));
                            return Ok(());
                        }
                    },
                    None => {
                        ctx.respond("Expected frequency.");
                        return Ok(());
                    }
                };

                self.promotions
                    .edit(ctx.user.target, name, frequency, ctx.rest())?;
                ctx.respond("Edited promo.");
            }
            Some("delete") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                if self.promotions.delete(ctx.user.target, name)? {
                    ctx.respond(format!("Deleted promo `{}`.", name));
                } else {
                    ctx.respond("No such promo.");
                }
            }
            Some("rename") => {
                ctx.check_moderator()?;

                let (from, to) = match (ctx.next(), ctx.next()) {
                    (Some(from), Some(to)) => (from, to),
                    _ => {
                        ctx.respond("Expected: !promo rename <from> <to>");
                        return Ok(());
                    }
                };

                match self.promotions.rename(ctx.user.target, from, to) {
                    Ok(()) => ctx.respond(format!("Renamed promo {} -> {}", from, to)),
                    Err(db::RenameError::Conflict) => {
                        ctx.respond(format!("Already a promo named {}", to))
                    }
                    Err(db::RenameError::Missing) => {
                        ctx.respond(format!("No such promo: {}", from))
                    }
                }
            }
            None | Some(..) => {
                ctx.respond("Expected: list, edit, or delete.");
            }
        }

        Ok(())
    }
}

pub struct Module {
    frequency: utils::Duration,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_duration")]
    frequency: utils::Duration,
}

fn default_duration() -> utils::Duration {
    utils::Duration::seconds(5 * 60)
}

impl Module {
    pub fn load(config: &Config) -> Result<Self, failure::Error> {
        Ok(Module {
            frequency: config.frequency.clone(),
        })
    }
}

impl super::Module for Module {
    fn hook(
        &self,
        module::HookContext {
            handlers,
            promotions,
            futures,
            sender,
            irc_config,
            settings,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        handlers.insert(
            "promo",
            Handler {
                promotions: promotions.clone(),
            },
        );

        let (setting, frequency) =
            settings.init_and_stream("promotions/frequency", self.frequency.clone())?;

        let promotions = promotions.clone();
        let sender = sender.clone();
        let channel = irc_config.channel.to_string();

        let interval = Interval::new_interval(frequency.as_std());

        futures.push(Box::new(PromotionFuture {
            interval,
            setting,
            promotions: promotions.clone(),
            sender: sender.clone(),
            channel: channel.clone(),
        }));

        Ok(())
    }
}

struct PromotionFuture {
    interval: tokio_timer::Interval,
    // channel for configuration updates.
    setting: db::settings::Stream<utils::Duration>,
    promotions: db::Promotions,
    sender: irc::Sender,
    channel: String,
}

impl Future for PromotionFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let mut setting_not_ready = false;
            let mut interval_not_ready = false;

            let rx = match self.setting.poll() {
                Ok(rx) => rx,
                Err(_) => failure::bail!("rx queue errored"),
            };

            match rx {
                Async::NotReady => setting_not_ready = true,
                Async::Ready(None) => failure::bail!("rx queue ended"),
                Async::Ready(Some(interval)) => {
                    self.interval = tokio_timer::Interval::new_interval(interval.as_std());
                }
            }

            let interval = match self.interval.poll() {
                Ok(interval) => interval,
                Err(_) => failure::bail!("interval queue errored"),
            };

            match interval {
                Async::NotReady => interval_not_ready = true,
                Async::Ready(None) => failure::bail!("interval queue ended"),
                Async::Ready(Some(_)) => {
                    let promotions = self.promotions.clone();
                    let sender = self.sender.clone();
                    let channel = self.channel.clone();

                    tokio::spawn(future::lazy(move || {
                        if let Err(e) = promote(promotions, sender, &channel) {
                            log::error!("failed to send promotion: {}", e);
                        }

                        Ok(())
                    }));
                }
            }

            if setting_not_ready && interval_not_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}

/// Run the next promotion.
fn promote(
    promotions: db::Promotions,
    sender: irc::Sender,
    channel: &str,
) -> Result<(), failure::Error> {
    if let Some(p) = pick(promotions.list(channel)) {
        let text = p.render(&PromoData { channel })?;
        promotions.bump_promoted_at(&*p)?;
        sender.privmsg(channel, text);
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct PromoData<'a> {
    channel: &'a str,
}

/// Pick the best promo.
fn pick(mut promotions: Vec<Arc<db::Promotion>>) -> Option<Arc<db::Promotion>> {
    promotions.sort_by(|a, b| a.promoted_at.cmp(&b.promoted_at));

    let now = Utc::now();

    for p in promotions {
        let promoted_at = match p.promoted_at.as_ref() {
            None => return Some(p),
            Some(promoted_at) => promoted_at,
        };

        if now.clone().signed_duration_since(promoted_at.clone()) < p.frequency.as_chrono() {
            continue;
        }

        return Some(p);
    }

    None
}
