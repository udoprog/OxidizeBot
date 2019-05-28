use crate::{auth, command, db, module, prelude::*, utils};
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the `!afterstream` command.
pub struct AfterStream<'a> {
    pub enabled: Arc<RwLock<bool>>,
    pub cooldown: Arc<RwLock<utils::Cooldown>>,
    pub after_streams: &'a db::AfterStreams,
}

impl command::Handler for AfterStream<'_> {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::AfterStream)
    }

    fn handle<'m>(&mut self, ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

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

        self.after_streams
            .push(ctx.user.target, ctx.user.name, ctx.rest())?;
        ctx.respond("Reminder added.");
        Ok(())
    }
}

pub struct Module;

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "afterstream"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            futures,
            after_streams,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        let settings = settings.scoped(&["afterstream"]);
        let mut vars = settings.vars();

        handlers.insert(
            "afterstream",
            AfterStream {
                enabled: vars.var("enabled", true)?,
                cooldown: vars.var(
                    "cooldown",
                    utils::Cooldown::from_duration(utils::Duration::seconds(30)),
                )?,
                after_streams,
            },
        );

        futures.push(vars.run().boxed());
        Ok(())
    }
}
