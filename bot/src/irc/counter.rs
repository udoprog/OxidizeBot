use crate::{counters, db, utils};

/// Handler for the !badword command.
pub struct Counter {
    pub counters: counters::Counters<db::Database>,
}

impl super::CommandHandler for Counter {
    fn handle<'m>(
        &mut self,
        mut ctx: super::CommandContext<'_>,
        user: super::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        match it.next() {
            Some("list") => {
                let mut names = self
                    .counters
                    .list(user.target)
                    .into_iter()
                    .map(|c| format!("!{}", c.key.name))
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    user.respond("No custom counters.");
                } else {
                    names.sort();
                    user.respond(format!("Custom counters: {}", names.join(", ")));
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

                self.counters.edit(user.target, name, it.rest())?;
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

                if self.counters.delete(user.target, name)? {
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
