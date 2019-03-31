use crate::{command, db};

/// Handler for the !command command.
pub struct Handler {
    pub commands: db::Commands<db::Database>,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        match ctx.next() {
            Some("list") => {
                let mut names = self
                    .commands
                    .list(ctx.user.target)
                    .into_iter()
                    .map(|c| c.key.name.to_string())
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    ctx.respond("No custom commands.");
                } else {
                    names.sort();
                    ctx.respond(format!("Custom commands: {}", names.join(", ")));
                }
            }
            Some("edit") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        failure::bail!("bad command");
                    }
                };

                self.commands.edit(ctx.user.target, name, ctx.rest())?;
                ctx.respond("Edited command.");
            }
            Some("delete") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        failure::bail!("bad command");
                    }
                };

                if self.commands.delete(ctx.user.target, name)? {
                    ctx.respond(format!("Deleted command `{}`.", name));
                } else {
                    ctx.respond("No such command.");
                }
            }
            None | Some(..) => {
                ctx.respond("Expected: list, edit, or delete.");
            }
        }

        Ok(())
    }
}
