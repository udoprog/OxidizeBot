use crate::{command, db, module, template};

/// Handler for the !alias command.
pub struct Handler {
    pub aliases: db::Aliases,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        ctx.check_moderator()?;

        let next = command_base!(ctx, self.aliases, "!alias", "alias");

        match next {
            Some("edit") => {
                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                let template = match template::Template::compile(ctx.rest()) {
                    Ok(template) => template,
                    Err(e) => {
                        ctx.respond(format!("Bad alias template: {}", e));
                        return Ok(());
                    }
                };

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
        "alias"
    }

    fn hook(
        &self,
        module::HookContext {
            handlers, aliases, ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        handlers.insert(
            "alias",
            Handler {
                aliases: aliases.clone(),
            },
        );
        Ok(())
    }
}
