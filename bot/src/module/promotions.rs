use crate::{command, db, irc, module, prelude::*, timer, utils};
use chrono::Utc;
use std::sync::Arc;

pub struct Handler {
    pub promotions: db::Promotions,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        let next = command_base!(ctx, self.promotions, "!promo", "promotion");

        match next {
            Some("edit") => {
                ctx.check_moderator()?;

                let name = ctx_try!(ctx.next_str("<name> <frequency> <template..>", "!promo edit"));
                let frequency =
                    ctx_try!(ctx.next_parse("<name> <frequency> <template..>", "!promo edit"));
                let template =
                    ctx_try!(ctx.rest_parse("<name> <frequency> <template..>", "!promo edit"));

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

        let (mut setting, frequency) =
            settings.init_and_stream("promotions/frequency", self.frequency.clone())?;

        let promotions = promotions.clone();
        let sender = sender.clone();
        let channel = config.irc.channel.clone();

        let mut interval = timer::Interval::new_interval(frequency.as_std());
        let idle = idle.clone();

        let future = async move {
            loop {
                // TODO: check that this actually works.
                futures::select! {
                    duration = setting.next() => {
                        if let Some(duration) = duration {
                            interval = timer::Interval::new_interval(duration.as_std());
                        }
                    }
                    _ = interval.next() => {
                        if idle.is_idle() {
                            log::trace!("channel is too idle to send a promotion");
                        } else {
                            let promotions = promotions.clone();
                            let sender = sender.clone();
                            let channel = channel.clone();

                            tokio::spawn(future01::lazy(move || {
                                if let Err(e) = promote(promotions, sender, &*channel) {
                                    log::error!("failed to send promotion: {}", e);
                                }

                                Ok(())
                            }));
                        }
                    }
                }
            }
        };

        futures.push(future.boxed());
        Ok(())
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
