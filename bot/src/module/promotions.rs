use crate::auth;
use crate::command;
use crate::db;
use crate::irc;
use crate::module;
use crate::prelude::*;
use crate::utils;
use chrono::Utc;

pub struct Handler {
    enabled: settings::Var<bool>,
    promotions: injector::Ref<db::Promotions>,
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context) -> Result<(), anyhow::Error> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let promotions = match self.promotions.load().await {
            Some(promotions) => promotions,
            None => return Ok(()),
        };

        let next = command_base!(ctx, promotions, "promotion", PromoEdit);

        match next.as_deref() {
            Some("edit") => {
                ctx.check_scope(auth::Scope::PromoEdit).await?;

                let name = ctx.next_str("<name> <frequency> <template..>")?;
                let frequency = ctx.next_parse("<name> <frequency> <template..>")?;
                let template = ctx.rest_parse("<name> <frequency> <template..>")?;

                promotions
                    .edit(ctx.channel(), &name, frequency, template)
                    .await?;
                respond!(ctx, "Edited promo.");
            }
            None | Some(..) => {
                respond!(
                    ctx,
                    "Expected: show, list, edit, delete, enable, disable, or group."
                );
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
        }: module::HookContext<'_>,
    ) -> Result<(), anyhow::Error> {
        let settings = settings.scoped("promotions");
        let enabled = settings.var("enabled", false).await?;

        let (mut setting, frequency) = settings
            .stream("frequency")
            .or_with_else(|| utils::Duration::seconds(5 * 60))
            .await?;

        handlers.insert(
            "promo",
            Handler {
                enabled: enabled.clone(),
                promotions: injector.var().await,
            },
        );

        let (mut promotions_stream, mut promotions) = injector.stream::<db::Promotions>().await;
        let sender = sender.clone();
        let mut interval = tokio::time::interval(frequency.as_std());
        let idle = idle.clone();

        let future = async move {
            loop {
                // TODO: check that this actually works.
                tokio::select! {
                    update = promotions_stream.recv() => {
                        promotions = update;
                    }
                    duration = setting.recv() => {
                        interval = tokio::time::interval(duration.as_std());
                    }
                    _ = interval.tick() => {
                        if !enabled.load().await {
                            continue;
                        }

                        let promotions = match promotions.as_ref() {
                            Some(promotions) => promotions,
                            None => continue,
                        };

                        if idle.is_idle().await {
                            log::trace!("channel is too idle to send a promotion");
                        } else {
                            let promotions = promotions.clone();
                            let sender = sender.clone();

                            if let Err(e) = promote(promotions, sender).await {
                                log::error!("failed to send promotion: {}", e);
                            }
                        }
                    }
                }
            }
        };

        futures.push(Box::pin(future));
        Ok(())
    }
}

/// Run the next promotion.
async fn promote(promotions: db::Promotions, sender: irc::Sender) -> Result<(), anyhow::Error> {
    let channel = sender.channel();

    if let Some(p) = pick(promotions.list(channel).await) {
        let text = p.render(&PromoData { channel })?;
        promotions.bump_promoted_at(&*p).await?;
        sender.privmsg(text).await;
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
