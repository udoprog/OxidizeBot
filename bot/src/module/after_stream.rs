use anyhow::Result;
use async_trait::async_trait;
use common::Cooldown;
use common::Duration;

use chat::command;
use chat::module;

/// Handler for the `!afterstream` command.
pub(crate) struct AfterStream {
    pub(crate) enabled: settings::Var<bool>,
    pub(crate) cooldown: settings::Var<Cooldown>,
    pub(crate) after_streams: async_injector::Ref<db::AfterStreams>,
}

#[async_trait]
impl command::Handler for AfterStream {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::AfterStream)
    }

    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let user = match ctx.user.real() {
            Some(user) => user,
            None => {
                chat::respond!(ctx, "Only real users can add after stream messages");
                return Ok(());
            }
        };

        let after_streams = match self.after_streams.load().await {
            Some(after_streams) => after_streams,
            None => return Ok(()),
        };

        if !self.cooldown.write().await.is_open() {
            chat::respond!(ctx, "An afterstream was already created recently.");
            return Ok(());
        }

        if ctx.rest().trim().is_empty() {
            chat::respond!(
                ctx,
                "You add a reminder by calling !afterstream <reminder>, \
                 like \"!afterstream remember that you are awesome <3\"",
            );
            return Ok(());
        }

        after_streams
            .push(ctx.channel(), user.login(), ctx.rest())
            .await?;
        chat::respond!(ctx, "Reminder added.");
        Ok(())
    }
}

pub(crate) struct Module;

#[async_trait]
impl chat::Module for Module {
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
    ) -> Result<()> {
        let settings = settings.scoped("afterstream");

        handlers.insert(
            "afterstream",
            AfterStream {
                enabled: settings.var("enabled", true).await?,
                cooldown: settings
                    .var("cooldown", Cooldown::from_duration(Duration::seconds(30)))
                    .await?,
                after_streams: injector.var().await,
            },
        );

        Ok(())
    }
}
