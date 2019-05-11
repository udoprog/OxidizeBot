use crate::{command, db, module, template};

/// Handler for the !alias command.
pub struct Handler {
    pub aliases: db::Aliases,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        ctx.check_moderator()?;

        match ctx.next() {
            Some("clear-group") => {
                command_clear_group!(ctx, self.aliases, "!alias clear-group", "alias")
            }
            Some("group") => command_group!(ctx, self.aliases, "!alias group", "alias"),
            Some("enable") => command_enable!(ctx, self.aliases, "!alias enable", "alias"),
            Some("disable") => command_disable!(ctx, self.aliases, "!alias disable", "alias"),
            Some("show") => {
                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                let alias = self.aliases.get(ctx.user.target, &name);

                match alias {
                    Some(alias) => {
                        ctx.respond(format!("{} -> {}", alias.key.name, alias.template));
                    }
                    None => {
                        ctx.respond(format!("No alias named `{}`.", name));
                    }
                }
            }
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
                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                let template = match template::Template::compile(ctx.rest()) {
                    Ok(template) => template,
                    Err(e) => {
                        ctx.respond(format!("Bad alias template: {}", e));
                        return Ok(());
                    }
                };

                self.aliases.edit(ctx.user.target, name, template)?;
                ctx.respond("Edited alias");
            }
            Some("delete") => {
                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected: !alias delete <name>");
                        return Ok(());
                    }
                };

                if self.aliases.delete(ctx.user.target, name)? {
                    ctx.respond(format!("Deleted alias `{}`", name));
                } else {
                    ctx.respond("No such alias");
                }
            }
            Some("rename") => {
                let (from, to) = match (ctx.next(), ctx.next()) {
                    (Some(from), Some(to)) => (from, to),
                    _ => {
                        ctx.respond("Expected: !alias rename <from> <to>");
                        return Ok(());
                    }
                };

                match self.aliases.rename(ctx.user.target, from, to) {
                    Ok(()) => ctx.respond(format!("Renamed alias {} -> {}.", from, to)),
                    Err(db::RenameError::Conflict) => {
                        ctx.respond(format!("Already an alias named `{}`.", to))
                    }
                    Err(db::RenameError::Missing) => {
                        ctx.respond(format!("No alias named `{}`.", from))
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
