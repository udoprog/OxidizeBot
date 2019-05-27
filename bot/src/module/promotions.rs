use crate::{command, config, db, irc, module, prelude::*, timer, utils};
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct Handler<'a> {
    enabled: Arc<RwLock<bool>>,
    pub promotions: &'a db::Promotions,
}

impl<'a> command::Handler for Handler<'a> {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

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

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    frequency: Option<utils::Duration>,
}

pub struct Module {
    default_frequency: Option<utils::Duration>,
}

impl Module {
    pub fn load(config: &config::Config) -> Self {
        let mut default_frequency = None;

        for m in &config.modules {
            match *m {
                module::Config::Promotions(ref config) => {
                    log::warn!("`[[modules]] type = \"countdown\"` configuration is deprecated");
                    default_frequency = config.frequency.clone();
                }
                _ => (),
            }
        }

        Module { default_frequency }
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
            settings,
            idle,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        let default_frequency = self
            .default_frequency
            .clone()
            .unwrap_or_else(|| utils::Duration::seconds(5 * 60));

        let enabled = settings.sync_var(futures, "promotions/enabled", false)?;

        let (mut setting, frequency) =
            settings.init_and_stream("promotions/frequency", default_frequency)?;

        handlers.insert(
            "promo",
            Handler {
                enabled: enabled.clone(),
                promotions,
            },
        );

        let promotions = promotions.clone();
        let sender = sender.clone();
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
                    _ = interval.select_next_some() => {
                        if !*enabled.read() {
                            continue;
                        }

                        if idle.is_idle() {
                            log::trace!("channel is too idle to send a promotion");
                        } else {
                            let promotions = promotions.clone();
                            let sender = sender.clone();

                            tokio::spawn(future01::lazy(move || {
                                if let Err(e) = promote(promotions, sender) {
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
fn promote(promotions: db::Promotions, sender: irc::Sender) -> Result<(), failure::Error> {
    let channel = sender.channel();

    if let Some(p) = pick(promotions.list(channel)) {
        let text = p.render(&PromoData { channel })?;
        promotions.bump_promoted_at(&*p)?;
        sender.privmsg(text);
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
