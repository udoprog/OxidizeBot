use crate::{auth, command, db, module, prelude::*, utils};
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the `!afterstream` command.
pub struct AfterStream {
    pub enabled: Arc<RwLock<bool>>,
    pub cooldown: Arc<RwLock<utils::Cooldown>>,
    pub after_streams: Arc<RwLock<Option<db::AfterStreams>>>,
}

#[async_trait]
impl command::Handler for AfterStream {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::AfterStream)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<(), anyhow::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let user = match ctx.user.real() {
            Some(user) => user,
            None => {
                ctx.respond("Only real users can add after stream messages");
                return Ok(());
            }
        };

        let after_streams = match self.after_streams.read().clone() {
            Some(after_streams) => after_streams,
            None => return Ok(()),
        };

        if !self.cooldown.write().is_open() {
            ctx.respond("An afterstream was already created recently.");
            return Ok(());
        }

        if ctx.rest().trim().is_empty() {
            ctx.respond(
                "You add a reminder by calling !afterstream <reminder>, \
                 like \"!afterstream remember that you are awesome <3\"",
            );
            return Ok(());
        }

        after_streams.push(ctx.channel(), user.name(), ctx.rest())?;
        ctx.respond("Reminder added.");
        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "afterstream"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            injector,
            handlers,
            settings,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), anyhow::Error> {
        let settings = settings.scoped("afterstream");

        handlers.insert(
            "afterstream",
            AfterStream {
                enabled: settings.var("enabled", true)?,
                cooldown: settings.var(
                    "cooldown",
                    utils::Cooldown::from_duration(utils::Duration::seconds(30)),
                )?,
                after_streams: injector.var()?,
            },
        );

        Ok(())
    }
}
