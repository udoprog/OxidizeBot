use crate::{db, utils};

/// Handler for the !command command.
pub struct Command {
    pub commands: db::Commands<db::Database>,
}

impl super::CommandHandler for Command {
    fn handle<'m>(
        &mut self,
        mut ctx: super::CommandContext<'_>,
        user: super::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        match it.next() {
            Some("list") => {
                let mut names = self
                    .commands
                    .list(user.target)
                    .into_iter()
                    .map(|c| format!("!{}", c.key.name))
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    user.respond("No custom commands.");
                } else {
                    names.sort();
                    user.respond(format!("Custom commands: {}", names.join(", ")));
                }
            }
            Some("edit") => {
                ctx.check_moderator(&user)?;

                let name = match it.next() {
                    Some(name) => name,
                    None => {
                        user.respond("Expected name.");
                        failure::bail!("bad command");
                    }
                };

                self.commands.edit(user.target, name, it.rest())?;
                user.respond("Edited command.");
            }
            Some("delete") => {
                ctx.check_moderator(&user)?;

                let name = match it.next() {
                    Some(name) => name,
                    None => {
                        user.respond("Expected name.");
                        failure::bail!("bad command");
                    }
                };

                if self.commands.delete(user.target, name)? {
                    user.respond(format!("Deleted command `{}`.", name));
                } else {
                    user.respond("No such command.");
                }
            }
            None | Some(..) => {
                user.respond("Expected: list, edit, or delete.");
            }
        }

        Ok(())
    }
}
