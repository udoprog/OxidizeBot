use crate::command;

/// Handler for the !admin command.
pub struct Admin {}

impl command::Handler for Admin {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        ctx.check_moderator()?;

        match ctx.next() {
            Some("refresh-mods") => {
                // The response from the /mods command will be received by the Handler.
                ctx.privmsg("/mods");
            }
            Some("shutdown") => {
                if ctx.shutdown.shutdown() {
                    ctx.respond("Shutting down...");
                } else {
                    ctx.respond("Already called shutdown...");
                }
            }
            None | Some(..) => {
                ctx.respond("Expected: refresh-mods.");
            }
        }

        Ok(())
    }
}
