use crate::{command, db, module};

pub struct Handler<'a> {
    pub themes: &'a db::Themes,
}

impl<'a> command::Handler for Handler<'a> {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        let next = command_base!(ctx, self.themes, "!theme", "theme");

        match next {
            Some("edit") => {
                ctx.check_moderator()?;

                let name = ctx_try!(ctx.next_str("<name> <track-id>", "!theme edit"));
                let track_id = ctx_try!(ctx.next_parse("<name> <track-id>", "!theme edit"));

                self.themes.edit(ctx.user.target, name, track_id)?;
                ctx.respond("Edited theme.");
            }
            Some("edit-duration") => {
                ctx.check_moderator()?;

                let name = ctx_try!(ctx.next_str("<name> <start> <end>", "!theme edit-duration"));
                let start =
                    ctx_try!(ctx.next_parse("<name> <start> <end>", "!theme edit-duration"));
                let end = ctx_try!(ctx.next_parse_optional());

                self.themes
                    .edit_duration(ctx.user.target, name, start, end)?;
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

pub struct Module {}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {}

impl Module {
    pub fn load(_: &Config) -> Result<Self, failure::Error> {
        Ok(Module {})
    }
}

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "theme"
    }

    fn hook(
        &self,
        module::HookContext {
            handlers, themes, ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        handlers.insert("theme", Handler { themes });
        Ok(())
    }
}
