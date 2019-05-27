use crate::{command, db, module};

/// Handler for the !alias command.
pub struct Handler<'a> {
    pub aliases: &'a db::Aliases,
}

impl<'a> command::Handler for Handler<'a> {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        ctx.check_moderator()?;

        let next = command_base!(ctx, self.aliases, "!alias", "alias");

        match next {
            Some("edit") => {
                ctx.check_moderator()?;

                let name = ctx_try!(ctx.next_str("<name>", "!alias edit"));
                let template = ctx_try!(ctx.rest_parse("<name> <template>", "!alias edit"));
                self.aliases.edit(ctx.user.target, name, template)?;

                ctx.respond("Edited alias");
            }
            None | Some(..) => {
                ctx.respond("Expected: show, list, edit, delete, enable, disable, or group.");
            }
        }

        Ok(())
    }
}

pub struct Module;

impl Module {
    pub fn load() -> Self {
        Module
    }
}

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "alias"
    }

    fn hook(
        &self,
        module::HookContext {
            handlers, aliases, ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        handlers.insert("alias", Handler { aliases });
        Ok(())
    }
}
