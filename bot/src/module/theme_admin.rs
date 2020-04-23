use crate::{auth, command, db, module, prelude::*};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct Handler {
    pub themes: Arc<RwLock<Option<db::Themes>>>,
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context) -> Result<(), anyhow::Error> {
        let themes = match self.themes.read().clone() {
            Some(themes) => themes,
            None => return Ok(()),
        };

        let next = command_base!(ctx, themes, "theme", ThemeEdit);

        match next.as_deref() {
            Some("edit") => {
                ctx.check_scope(auth::Scope::ThemeEdit).await?;

                let name = ctx_try!(ctx.next_str("<name> <track-id>"));
                let track_id = ctx_try!(ctx.next_parse("<name> <track-id>"));

                themes.edit(ctx.channel(), &name, track_id)?;
                ctx.respond("Edited theme.");
            }
            Some("edit-duration") => {
                ctx.check_scope(auth::Scope::ThemeEdit).await?;

                let name = ctx_try!(ctx.next_str("<name> <start> <end>"));
                let start = ctx_try!(ctx.next_parse("<name> <start> <end>"));
                let end = ctx_try!(ctx.next_parse_optional());

                themes.edit_duration(ctx.channel(), &name, start, end)?;
                ctx.respond("Edited theme.");
            }
            None | Some(..) => {
                ctx.respond(
                    "Expected: show, list, edit, edit-duration, delete, enable, disable, or group.",
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
        "theme"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector, handlers, ..
        }: module::HookContext<'_>,
    ) -> Result<(), anyhow::Error> {
        handlers.insert(
            "theme",
            Handler {
                themes: injector.var()?,
            },
        );
        Ok(())
    }
}
