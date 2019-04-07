use crate::{command, db, module};

/// Handler for the !alias command.
pub struct Handler {
    pub aliases: db::Aliases,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        match ctx.next() {
            Some("list") => {
                let mut names = self
                    .aliases
                    .list(ctx.user.target)
                    .into_iter()
                    .map(|c| c.key.name.to_string())
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    ctx.respond("No custom aliases.");
                } else {
                    names.sort();
                    ctx.respond(format!("Custom aliases: {}", names.join(", ")));
                }
            }
            Some("edit") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                self.aliases.edit(ctx.user.target, name, ctx.rest())?;
                ctx.respond("Edited alias.");
            }
            Some("delete") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                if self.aliases.delete(ctx.user.target, name)? {
                    ctx.respond(format!("Deleted alias `{}`.", name));
                } else {
                    ctx.respond("No such alias.");
                }
            }
            Some("rename") => {
                ctx.check_moderator()?;

                let (from, to) = match (ctx.next(), ctx.next()) {
                    (Some(from), Some(to)) => (from, to),
                    _ => {
                        ctx.respond("Expected: !alias rename <from> <to>");
                        return Ok(());
                    }
                };

                match self.aliases.rename(ctx.user.target, from, to) {
                    Ok(()) => ctx.respond(format!("Renamed alias {} -> {}", from, to)),
                    Err(db::RenameError::Conflict) => {
                        ctx.respond(format!("Already an alias named {}", to))
                    }
                    Err(db::RenameError::Missing) => {
                        ctx.respond(format!("No such alias: {}", from))
                    }
                }
            }
            None | Some(..) => {
                ctx.respond("Expected: list, edit, or delete.");
            }
        }

        Ok(())
    }
}

pub struct Module {}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {}

impl Module {
    pub fn load(_: &Config) -> Result<Self, failure::Error> {
        Ok(Module {})
    }
}

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "alias"
    }

    fn hook(
        &self,
        module::HookContext {
            handlers, aliases, ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        handlers.insert(
            "alias",
            Handler {
                aliases: aliases.clone(),
            },
        );
        Ok(())
    }
}
