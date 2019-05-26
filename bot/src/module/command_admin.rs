use crate::{command, db, module};

pub struct Handler<'a> {
    pub commands: &'a db::Commands,
}

impl<'a> command::Handler for Handler<'a> {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        let next = command_base!(ctx, self.commands, "!command", "command");

        match next {
            Some("edit") => {
                ctx.check_moderator()?;

                let name = ctx_try!(ctx.next_str("<name>", "!command edit"));
                let template = ctx_try!(ctx.rest_parse("<name> <template>", "!command edit"));
                self.commands.edit(ctx.user.target, name, template)?;

                ctx.respond("Edited command.");
            }
            None | Some(..) => {
                ctx.respond("Expected: show, list, edit, delete, enable, disable, or group.");
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
        "command"
    }

    fn hook(
        &self,
        module::HookContext {
            handlers, commands, ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        handlers.insert("command", Handler { commands });
        Ok(())
    }
}
