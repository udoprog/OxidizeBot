use anyhow::Result;
use async_trait::async_trait;
use chat::command;
use chat::module;

pub(crate) struct Handler {
    pub(crate) enabled: settings::Var<bool>,
    pub(crate) commands: async_injector::Ref<db::Commands>,
}

#[async_trait]
impl command::Handler for Handler {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Command)
    }

    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let commands = match self.commands.load().await {
            Some(commands) => commands,
            None => return Ok(()),
        };

        let next = command_base!(ctx, commands, "command", CommandEdit);

        match next.as_deref() {
            Some("edit") => {
                ctx.check_scope(auth::Scope::CommandEdit).await?;

                let name = ctx.next_str("<name>")?;
                let template = ctx.rest_parse("<name> <template>")?;
                commands.edit(ctx.channel(), &name, template).await?;

                chat::respond!(ctx, "Edited command.");
            }
            Some("pattern") => {
                ctx.check_scope(auth::Scope::CommandEdit).await?;

                let name = ctx.next_str("<name> [pattern]")?;

                let pattern = match ctx.rest() {
                    pattern if pattern.trim().is_empty() => None,
                    pattern => match regex::Regex::new(pattern) {
                        Ok(pattern) => Some(pattern),
                        Err(e) => {
                            ctx.user
                                .respond(format!("Bad pattern provided: {}", e))
                                .await;
                            return Ok(());
                        }
                    },
                };

                if !commands.edit_pattern(ctx.channel(), &name, pattern).await? {
                    chat::respond!(ctx, format!("No such command: `{}`", name));
                    return Ok(());
                }

                chat::respond!(ctx, "Edited pattern for command.");
            }
            None | Some(..) => {
                chat::respond!(
                    ctx,
                    "Expected: show, list, edit, delete, enable, disable, or group."
                );
            }
        }

        Ok(())
    }
}

pub(crate) struct Module;

#[async_trait]
impl chat::Module for Module {
    fn ty(&self) -> &'static str {
        "command"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector,
            handlers,
            settings,
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        let enabled = settings.var("command/enabled", true).await?;
        let commands = injector.var().await;
        handlers.insert("command", Handler { enabled, commands });
        Ok(())
    }
}
