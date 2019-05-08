use crate::{command, module, settings};

/// Handler for the !admin command.
pub struct Handler {
    settings: settings::Settings,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        ctx.check_moderator()?;

        match ctx.next() {
            Some("refresh-mods") => {
                // The response from the /mods command will be received by the Handler.
                ctx.privmsg("/mods");
            }
            Some("version") => {
                ctx.respond(format!("Bot Version {}", env!("CARGO_PKG_VERSION")));
            }
            Some("shutdown") => {
                if ctx.shutdown.shutdown() {
                    ctx.respond("Shutting down...");
                } else {
                    ctx.respond("Already called shutdown...");
                }
            }
            // Get or set a setting.
            Some("setting") => {
                let key = match ctx.next() {
                    Some(key) => key,
                    None => {
                        ctx.respond(format!(
                            "Expected: {p} <key>",
                            p = ctx.alias.unwrap_or("!admin get")
                        ));
                        return Ok(());
                    }
                };

                if key.starts_with("secrets/") {
                    ctx.respond("Cannot set the value of a secret!");
                    return Ok(());
                }

                match ctx.rest().trim() {
                    "" => {
                        let value = match self.settings.get::<Option<serde_json::Value>>(key)? {
                            Some(value) => value,
                            None => {
                                ctx.respond("No such setting");
                                return Ok(());
                            }
                        };

                        ctx.respond(serde_json::to_string(&value)?);
                    }
                    value => {
                        let value = match serde_json::from_str(value) {
                            Ok(value) => value,
                            Err(_) => {
                                ctx.respond("Value must be valid JSON");
                                return Ok(());
                            }
                        };

                        self.settings.set_json(key, value)?;
                        ctx.respond(format!("Updated the {} setting", key));
                    }
                }
            }
            _ => {
                ctx.respond(format!(
                    "Expected one of: \
                     {p} refresh-mods, \
                     {p} version, \
                     {p} shutdown, \
                     {p} setting.",
                    p = ctx.alias.unwrap_or("!admin"),
                ));
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
        "admin"
    }

    fn hook(
        &self,
        module::HookContext {
            handlers, settings, ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        handlers.insert(
            "admin",
            Handler {
                settings: settings.clone(),
            },
        );

        Ok(())
    }
}
