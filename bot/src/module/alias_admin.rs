use anyhow::Result;
use async_trait::async_trait;

use chat::command;
use chat::module;

/// Handler for the !alias command.
pub(crate) struct Handler {
    pub(crate) aliases: async_injector::Ref<db::Aliases>,
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        let aliases = match self.aliases.load().await {
            Some(aliases) => aliases,
            None => return Ok(()),
        };

        let next = command_base!(ctx, aliases, "alias", AliasEdit);

        match next.as_deref() {
            Some("edit") => {
                ctx.check_scope(auth::Scope::AliasEdit).await?;

                let name = ctx.next_str("<name>")?;
                let template = ctx.rest_parse("<name> <template>")?;
                aliases.edit(ctx.channel(), &name, template).await?;

                chat::respond!(ctx, "Edited alias");
            }
            Some("pattern") => {
                ctx.check_scope(auth::Scope::AliasEdit).await?;

                let name = ctx.next_str("<name> [pattern]")?;

                let pattern = match ctx.rest() {
                    pattern if pattern.trim().is_empty() => None,
                    pattern => match regex::Regex::new(pattern) {
                        Ok(pattern) => Some(pattern),
                        Err(e) => {
                            ctx.user.respond(format!("Bad pattern provided: {e}")).await;
                            return Ok(());
                        }
                    },
                };

                if !aliases.edit_pattern(ctx.channel(), &name, pattern).await? {
                    chat::respond!(ctx, format!("No such alias: `{}`", name));
                    return Ok(());
                }

                chat::respond!(ctx, "Edited pattern for alias.");
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
        "alias"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector, handlers, ..
        }: module::HookContext<'_, '_>,
    ) -> Result<()> {
        handlers.insert(
            "alias",
            Handler {
                aliases: injector.var().await,
            },
        );
        Ok(())
    }
}
