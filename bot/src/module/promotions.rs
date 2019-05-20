use crate::{command, db, idle, irc, module, settings, template, utils};
use chrono::Utc;
use futures::{future, Async, Future, Poll, Stream as _};
use std::sync::Arc;
use tokio_timer::Interval;

pub struct Handler {
    pub promotions: db::Promotions,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        let next = command_base!(ctx, self.promotions, "!promo", "promotion");

        match next {
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

                let template = match template::Template::compile(ctx.rest()) {
                    Ok(template) => template,
                    Err(e) => {
                        ctx.respond(format!("Bad promotion template: {}", e));
                        return Ok(());
                    }
                };

                self.promotions
                    .edit(ctx.user.target, name, frequency, template)?;
                ctx.respond("Edited promo.");
            }
            None | Some(..) => {
                ctx.respond("Expected: show, list, edit, delete, enable, disable, or group.");
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
    fn ty(&self) -> &'static str {
        "promotions"
    }

    fn hook(
        &self,
        module::HookContext {
            handlers,
            promotions,
            futures,
            sender,
            config,
            settings,
            idle,
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
        let channel = config.irc.channel.clone();

        let interval = Interval::new_interval(frequency.as_std());

        futures.push(Box::new(PromotionFuture {
            interval,
            setting,
            promotions: promotions.clone(),
            sender: sender.clone(),
            channel,
            idle: idle.clone(),
        }));

        Ok(())
    }
}

struct PromotionFuture {
    interval: tokio_timer::Interval,
    // channel for configuration updates.
    setting: settings::Stream<utils::Duration>,
    promotions: db::Promotions,
    sender: irc::Sender,
    channel: Arc<String>,
    idle: idle::Idle,
}

impl Future for PromotionFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let mut not_ready = true;

            if let Some(interval) = try_infinite!(self.setting.poll()) {
                self.interval = tokio_timer::Interval::new_interval(interval.as_std());
                not_ready = false;
            }

            if let Some(_) = try_infinite!(self.interval.poll()) {
                if self.idle.is_idle() {
                    log::trace!("channel is too idle to send a promotion");
                } else {
                    let promotions = self.promotions.clone();
                    let sender = self.sender.clone();
                    let channel = self.channel.clone();

                    tokio::spawn(future::lazy(move || {
                        if let Err(e) = promote(promotions, sender, &*channel) {
                            log::error!("failed to send promotion: {}", e);
                        }

                        Ok(())
                    }));
                }

                not_ready = false;
            }

            if not_ready {
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
