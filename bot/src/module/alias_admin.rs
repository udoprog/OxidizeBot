use crate::{auth, command, db, module, prelude::*};

/// Handler for the !alias command.
pub struct Handler<'a> {
    pub aliases: &'a db::Aliases,
}

impl command::Handler for Handler<'_> {
    fn handle<'slf: 'a, 'ctx: 'a, 'a>(
        &'slf mut self,
        mut ctx: command::Context<'ctx>,
    ) -> future::BoxFuture<'a, Result<(), failure::Error>> {
        Box::pin(async move {
            let next = command_base!(ctx, self.aliases, "alias", AliasEdit);

            match next.as_ref().map(String::as_str) {
                Some("edit") => {
                    ctx.check_scope(auth::Scope::AliasEdit)?;

                    let name = ctx_try!(ctx.next_str("<name>"));
                    let template = ctx_try!(ctx.rest_parse("<name> <template>"));
                    self.aliases.edit(ctx.user.target(), &name, template)?;

                    ctx.respond("Edited alias");
                }
                None | Some(..) => {
                    ctx.respond("Expected: show, list, edit, delete, enable, disable, or group.");
                }
            }

            Ok(())
        })
    }
}

pub struct Module;

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
