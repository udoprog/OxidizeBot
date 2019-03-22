use crate::{command, db, irc, utils};

/// Handler for the `!afterstream` command.
pub struct AfterStream {
    pub cooldown: utils::Cooldown,
    pub db: db::Database,
}

impl command::Handler for AfterStream {
    fn handle<'m>(
        &mut self,
        _: command::Context<'_>,
        user: irc::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        if !self.cooldown.is_open() {
            user.respond("An afterstream was already created recently.");
            return Ok(());
        }

        self.db.insert_afterstream(&user.name, it.rest())?;
        user.respond("Reminder added.");
        Ok(())
    }
}
