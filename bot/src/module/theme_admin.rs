use crate::{command, db, module};

pub struct Handler {
    pub themes: db::Themes,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        match ctx.next() {
            Some("clear-group") => {
                command_clear_group!(ctx, self.themes, "!theme clear-group", "theme")
            }
            Some("group") => command_group!(ctx, self.themes, "!theme group", "theme"),
            Some("enable") => command_enable!(ctx, self.themes, "!theme enable", "theme"),
            Some("disable") => command_disable!(ctx, self.themes, "!theme disable", "theme"),
            Some("list") => {
                let mut names = self
                    .themes
                    .list(ctx.user.target)
                    .into_iter()
                    .map(|c| c.key.name.to_string())
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    ctx.respond("No custom themes.");
                } else {
                    names.sort();
                    ctx.respond(format!("Custom themes: {}", names.join(", ")));
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

                let track_id = match ctx.next() {
                    Some(track_id) => match str::parse(track_id) {
                        Ok(track_id) => track_id,
                        Err(e) => {
                            ctx.respond(format!("Bad track id: {}", e));
                            return Ok(());
                        }
                    },
                    None => {
                        ctx.respond(format!(
                            "Expected: {p} <track>",
                            p = ctx.alias.unwrap_or("!theme edit")
                        ));
                        return Ok(());
                    }
                };

                self.themes.edit(ctx.user.target, name, track_id)?;
                ctx.respond("Edited theme.");
            }
            Some("edit-duration") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                let start = match ctx.next() {
                    Some(start) => match str::parse(start) {
                        Ok(start) => start,
                        Err(e) => {
                            ctx.respond(format!("Bad start: {}", e));
                            return Ok(());
                        }
                    },
                    None => {
                        ctx.respond(format!(
                            "Expected: {p} <start>",
                            p = ctx.alias.unwrap_or("!theme edit")
                        ));
                        return Ok(());
                    }
                };

                let end = match ctx.next() {
                    Some(start) => match str::parse(start) {
                        Ok(start) => Some(start),
                        Err(e) => {
                            ctx.respond(format!("Bad start: {}", e));
                            return Ok(());
                        }
                    },
                    None => None,
                };

                self.themes
                    .edit_duration(ctx.user.target, name, start, end)?;
                ctx.respond("Edited theme.");
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

                if self.themes.delete(ctx.user.target, name)? {
                    ctx.respond(format!("Deleted theme `{}`.", name));
                } else {
                    ctx.respond("No such theme.");
                }
            }
            Some("rename") => {
                ctx.check_moderator()?;

                let (from, to) = match (ctx.next(), ctx.next()) {
                    (Some(from), Some(to)) => (from, to),
                    _ => {
                        ctx.respond("Expected: !theme rename <from> <to>");
                        return Ok(());
                    }
                };

                match self.themes.rename(ctx.user.target, from, to) {
                    Ok(()) => ctx.respond(format!("Renamed theme {} -> {}", from, to)),
                    Err(db::RenameError::Conflict) => {
                        ctx.respond(format!("Already a theme named {}", to))
                    }
                    Err(db::RenameError::Missing) => {
                        ctx.respond(format!("No such theme: {}", from))
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
        "theme"
    }

    fn hook(
        &self,
        module::HookContext {
            handlers, themes, ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        handlers.insert(
            "theme",
            Handler {
                themes: themes.clone(),
            },
        );
        Ok(())
    }
}
