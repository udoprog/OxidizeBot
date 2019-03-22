use crate::{command, irc, utils};

/// Handler for the !admin command.
pub struct Admin {}

impl command::Handler for Admin {
    fn handle<'m>(
        &mut self,
        mut ctx: command::Context<'_>,
        user: irc::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        ctx.check_moderator(&user)?;

        match it.next() {
            Some("refresh-mods") => {
                // The response from the /mods command will be received by the Handler.
                ctx.sender.privmsg(user.target, "/mods");
            }
            None | Some(..) => {
                user.respond("Expected: refresh-mods.");
            }
        }

        Ok(())
    }
}
