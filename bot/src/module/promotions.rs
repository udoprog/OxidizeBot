use crate::{auth, command, db, irc, module, prelude::*, utils};
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct Handler {
    enabled: Arc<RwLock<bool>>,
    promotions: Arc<RwLock<Option<db::Promotions>>>,
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&mut self, mut ctx: command::Context<'_>) -> Result<(), anyhow::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let promotions = match self.promotions.read().clone() {
            Some(promotions) => promotions,
            None => return Ok(()),
        };

        let next = command_base!(ctx, promotions, "promotion", PromoEdit);

        match next.as_deref() {
            Some("edit") => {
                ctx.check_scope(auth::Scope::PromoEdit)?;

                let name = ctx_try!(ctx.next_str("<name> <frequency> <template..>"));
                let frequency = ctx_try!(ctx.next_parse("<name> <frequency> <template..>"));
                let template = ctx_try!(ctx.rest_parse("<name> <frequency> <template..>"));

                promotions.edit(ctx.channel(), &name, frequency, template)?;
                ctx.respond("Edited promo.");
            }
            None | Some(..) => {
                ctx.respond("Expected: show, list, edit, delete, enable, disable, or group.");
            }
        }

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "promotions"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector,
            handlers,
            futures,
            sender,
            settings,
            idle,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), anyhow::Error> {
        let settings = settings.scoped("promotions");
        let enabled = settings.var("enabled", false)?;

        let (mut setting, frequency) = settings
            .stream("frequency")
            .or_with_else(|| utils::Duration::seconds(5 * 60))?;

        handlers.insert(
            "promo",
            Handler {
                enabled: enabled.clone(),
                promotions: injector.var()?,
            },
        );

        let (mut promotions_stream, mut promotions) = injector.stream::<db::Promotions>();
        let sender = sender.clone();
        let mut interval = tokio::time::interval(frequency.as_std()).fuse();
        let idle = idle.clone();

        let future = async move {
            loop {
                // TODO: check that this actually works.
                futures::select! {
                    update = promotions_stream.select_next_some() => {
                        promotions = update;
                    }
                    duration = setting.next() => {
                        if let Some(duration) = duration {
                            interval = tokio::time::interval(duration.as_std()).fuse();
                        }
                    }
                    _ = interval.select_next_some() => {
                        if !*enabled.read() {
                            continue;
                        }

                        let promotions = match promotions.as_ref() {
                            Some(promotions) => promotions,
                            None => continue,
                        };

                        if idle.is_idle() {
                            log::trace!("channel is too idle to send a promotion");
                        } else {
                            let promotions = promotions.clone();
                            let sender = sender.clone();

                            tokio::spawn(async move {
                                if let Err(e) = promote(promotions, sender) {
                                    log::error!("failed to send promotion: {}", e);
                                }
                            });
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
fn promote(promotions: db::Promotions, sender: irc::Sender) -> Result<(), anyhow::Error> {
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
