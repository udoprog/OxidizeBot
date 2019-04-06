use crate::{command, db, utils};

/// Handler for the `!afterstream` command.
pub struct AfterStream {
    pub cooldown: utils::Cooldown,
    pub after_streams: db::AfterStreams,
}

impl command::Handler for AfterStream {
    fn handle<'m>(&mut self, ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        if !self.cooldown.is_open() {
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
