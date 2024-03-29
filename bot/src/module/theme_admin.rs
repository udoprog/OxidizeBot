use anyhow::Result;
use async_trait::async_trait;
use chat::command;
use chat::module;

pub(crate) struct Handler {
    pub(crate) themes: async_injector::Ref<db::Themes>,
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        let themes = match self.themes.load().await {
            Some(themes) => themes,
            None => return Ok(()),
        };

        let next = command_base!(ctx, themes, "theme", ThemeEdit);

        match next.as_deref() {
            Some("edit") => {
                ctx.check_scope(auth::Scope::ThemeEdit).await?;

                let name = ctx.next_str("<name> <track-id>")?;
                let track_id = ctx.next_parse("<name> <track-id>")?;

                themes.edit(ctx.channel(), &name, track_id).await?;
                chat::respond!(ctx, "Edited theme.");
            }
            Some("edit-duration") => {
                ctx.check_scope(auth::Scope::ThemeEdit).await?;

                let name = ctx.next_str("<name> <start> <end>")?;
                let start = ctx.next_parse("<name> <start> <end>")?;
                let end = ctx.next_parse_optional()?;

                themes
                    .edit_duration(ctx.channel(), &name, start, end)
                    .await?;
                chat::respond!(ctx, "Edited theme.");
            }
            None | Some(..) => {
                chat::respond!(
                    ctx,
                    "Expected: show, list, edit, edit-duration, delete, enable, disable, or group.",
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
        "theme"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector, handlers, ..
        }: module::HookContext<'_, '_>,
    ) -> Result<()> {
        handlers.insert(
            "theme",
            Handler {
                themes: injector.var().await,
            },
        );
        Ok(())
    }
}
