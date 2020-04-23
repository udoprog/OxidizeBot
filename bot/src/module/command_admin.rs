use crate::{auth, command, db, module, prelude::*};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct Handler {
    pub enabled: Arc<RwLock<bool>>,
    pub commands: Arc<RwLock<Option<db::Commands>>>,
}

#[async_trait]
impl command::Handler for Handler {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Command)
    }

    async fn handle(&mut self, mut ctx: command::Context) -> Result<(), anyhow::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let commands = match self.commands.read().clone() {
            Some(commands) => commands,
            None => return Ok(()),
        };

        let next = command_base!(ctx, commands, "command", CommandEdit);

        match next.as_deref() {
            Some("edit") => {
                ctx.check_scope(auth::Scope::CommandEdit).await?;

                let name = ctx_try!(ctx.next_str("<name>"));
                let template = ctx_try!(ctx.rest_parse("<name> <template>"));
                commands.edit(ctx.channel(), &name, template)?;

                ctx.respond("Edited command.");
            }
            Some("pattern") => {
                ctx.check_scope(auth::Scope::CommandEdit).await?;

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

                if !commands.edit_pattern(ctx.channel(), &name, pattern)? {
                    ctx.respond(format!("No such command: `{}`", name));
                    return Ok(());
                }

                ctx.respond("Edited pattern for command.");
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
        "command"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector,
            handlers,
            settings,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), anyhow::Error> {
        let enabled = settings.var("command/enabled", true)?;
        let commands = injector.var()?;
        handlers.insert("command", Handler { enabled, commands });
        Ok(())
    }
}
