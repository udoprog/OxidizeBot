use crate::{command, db, module};

pub struct Handler {
    pub themes: db::Themes,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        let next = command_base!(ctx, self.themes, "!theme", "theme");

        match next {
            Some("edit") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond(format!(
                            "Expected: {p} <name>",
                            p = ctx.alias.unwrap_or("!theme edit")
                        ));
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
                        ctx.respond(format!(
                            "Expected: {p} <name>",
                            p = ctx.alias.unwrap_or("!theme edit-duration")
                        ));
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
            None | Some(..) => {
                ctx.respond(
                    "Expected: show, list, edit, edit-duration, delete, enable, disable, or group.",
                );
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
