use crate::{auth, command, db, module, prelude::*};
use anyhow::Error;
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the !alias command.
pub struct Handler {
    pub aliases: Arc<RwLock<Option<db::Aliases>>>,
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context) -> Result<(), Error> {
        let aliases = match self.aliases.read().clone() {
            Some(aliases) => aliases,
            None => return Ok(()),
        };

        let next = command_base!(ctx, aliases, "alias", AliasEdit);

        match next.as_deref() {
            Some("edit") => {
                ctx.check_scope(auth::Scope::AliasEdit).await?;

                let name = ctx_try!(ctx.next_str("<name>"));
                let template = ctx_try!(ctx.rest_parse("<name> <template>"));
                aliases.edit(ctx.channel(), &name, template)?;

                ctx.respond("Edited alias");
            }
            Some("pattern") => {
                ctx.check_scope(auth::Scope::AliasEdit).await?;

                let name = ctx_try!(ctx.next_str("<name> [pattern]"));

                let pattern = match ctx.rest() {
                    pattern if pattern.trim().is_empty() => None,
                    pattern => match regex::Regex::new(pattern) {
                        Ok(pattern) => Some(pattern),
                        Err(e) => {
                            ctx.user.respond(format!("Bad pattern provided: {}", e));
                            return Ok(());
                        }
                    },
                };

                if !aliases.edit_pattern(ctx.channel(), &name, pattern)? {
                    ctx.respond(format!("No such alias: `{}`", name));
                    return Ok(());
                }

                ctx.respond("Edited pattern for alias.");
            }
            None | Some(..) => {
                ctx.respond("Expected: show, list, edit, delete, enable, disable, or group.");
            }
        }

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "alias"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector, handlers, ..
        }: module::HookContext<'_>,
    ) -> Result<(), Error> {
        handlers.insert(
            "alias",
            Handler {
                aliases: injector.var()?,
            },
        );
        Ok(())
    }
}
