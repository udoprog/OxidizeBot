use crate::{command, db};
use failure::format_err;

/// Handler for the !badword command.
pub struct BadWord<'a> {
    pub bad_words: &'a db::Words,
}

impl command::Handler for BadWord<'_> {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        match ctx.next() {
            Some("edit") => {
                ctx.check_moderator()?;

                let word = ctx.next().ok_or_else(|| format_err!("expected word"))?;
                let why = match ctx.rest() {
                    "" => None,
                    other => Some(other),
                };

                self.bad_words.edit(word, why)?;
                ctx.respond("Bad word edited");
            }
            Some("delete") => {
                ctx.check_moderator()?;

                let word = ctx.next().ok_or_else(|| format_err!("expected word"))?;

                if self.bad_words.delete(word)? {
                    ctx.respond("Bad word removed.");
                } else {
                    ctx.respond("Bad word did not exist.");
                }
            }
            None => {
                ctx.respond("!badword is a word filter, removing unwanted messages.");
            }
            Some(_) => {
                ctx.respond("Expected: edit, or delete.");
            }
        }

        Ok(())
    }
}
