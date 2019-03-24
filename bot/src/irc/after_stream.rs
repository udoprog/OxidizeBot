use crate::{command, db, utils};

/// Handler for the `!afterstream` command.
pub struct AfterStream {
    pub cooldown: utils::Cooldown,
    pub db: db::Database,
}

impl command::Handler for AfterStream {
    fn handle<'m>(&mut self, ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        if !self.cooldown.is_open() {
            ctx.respond("An afterstream was already created recently.");
            return Ok(());
        }

        self.db.insert_afterstream(ctx.user.name, ctx.rest())?;
        ctx.respond("Reminder added.");
        Ok(())
    }
}
