use crate::{auth, command, db, irc, module, prelude::*, timer, utils};
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct Handler<'a> {
    enabled: Arc<RwLock<bool>>,
    pub promotions: &'a db::Promotions,
}

impl command::Handler for Handler<'_> {
    fn handle<'slf: 'a, 'ctx: 'a, 'a>(
        &'slf mut self,
        mut ctx: command::Context<'ctx>,
    ) -> future::BoxFuture<'a, Result<(), failure::Error>> {
        Box::pin(async move {
            if !*self.enabled.read() {
                return Ok(());
            }

            let next = command_base!(ctx, self.promotions, "promotion", PromoEdit);

            match next.as_ref().map(String::as_str) {
                Some("edit") => {
                    ctx.check_scope(auth::Scope::PromoEdit)?;

                    let name = ctx_try!(ctx.next_str("<name> <frequency> <template..>"));
                    let frequency = ctx_try!(ctx.next_parse("<name> <frequency> <template..>"));
                    let template = ctx_try!(ctx.rest_parse("<name> <frequency> <template..>"));

                    self.promotions
                        .edit(ctx.user.target(), &name, frequency, template)?;
                    ctx.respond("Edited promo.");
                }
                None | Some(..) => {
                    ctx.respond("Expected: show, list, edit, delete, enable, disable, or group.");
                }
            }

            Ok(())
        })
    }
}

pub struct Module;

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
        let mut vars = settings.vars();
        let enabled = vars.var("promotions/enabled", false)?;
        futures.push(vars.run().boxed());

        let (mut setting, frequency) = settings
            .stream("promotions/frequency")
            .or_with_else(|| utils::Duration::seconds(5 * 60))?;

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
