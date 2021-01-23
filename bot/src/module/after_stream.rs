use crate::auth;
use crate::command;
use crate::db;
use crate::module;
use crate::prelude::*;
use crate::utils;

/// Handler for the `!afterstream` command.
pub struct AfterStream {
    pub enabled: settings::Var<bool>,
    pub cooldown: settings::Var<utils::Cooldown>,
    pub after_streams: injector::Ref<db::AfterStreams>,
}

#[async_trait]
impl command::Handler for AfterStream {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::AfterStream)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<(), anyhow::Error> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let user = match ctx.user.real() {
            Some(user) => user,
            None => {
                respond!(ctx, "Only real users can add after stream messages");
                return Ok(());
            }
        };

        let after_streams = match self.after_streams.load().await {
            Some(after_streams) => after_streams,
            None => return Ok(()),
        };

        if !self.cooldown.write().await.is_open() {
            respond!(ctx, "An afterstream was already created recently.");
            return Ok(());
        }

        if ctx.rest().trim().is_empty() {
            respond!(
                ctx,
                "You add a reminder by calling !afterstream <reminder>, \
                 like \"!afterstream remember that you are awesome <3\"",
            );
            return Ok(());
        }

        after_streams
            .push(ctx.channel(), user.name(), ctx.rest())
            .await?;
        respond!(ctx, "Reminder added.");
        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "afterstream"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            injector,
            handlers,
            settings,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), anyhow::Error> {
        let settings = settings.scoped("afterstream");

        handlers.insert(
            "afterstream",
            AfterStream {
                enabled: settings.var("enabled", true).await?,
                cooldown: settings
                    .var(
                        "cooldown",
                        utils::Cooldown::from_duration(utils::Duration::seconds(30)),
                    )
                    .await?,
                after_streams: injector.var().await,
            },
        );

        Ok(())
    }
}
