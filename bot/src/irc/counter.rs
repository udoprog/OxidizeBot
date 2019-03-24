use crate::{command, db};

/// Handler for the !badword command.
pub struct Counter {
    pub counters: db::Counters<db::Database>,
}

impl command::Handler for Counter {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        match ctx.next() {
            Some("list") => {
                let mut names = self
                    .counters
                    .list(ctx.user.target)
                    .into_iter()
                    .map(|c| format!("!{}", c.key.name))
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    ctx.respond("No custom counters.");
                } else {
                    names.sort();
                    ctx.respond(format!("Custom counters: {}", names.join(", ")));
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

                self.counters.edit(ctx.user.target, name, ctx.rest())?;
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

                if self.counters.delete(ctx.user.target, name)? {
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
