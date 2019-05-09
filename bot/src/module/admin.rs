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
                ctx.respond(format!("Bot Version {}", crate::VERSION));
            }
            Some("shutdown") => {
                if ctx.shutdown.shutdown() {
                    ctx.respond("Shutting down...");
                } else {
                    ctx.respond("Already called shutdown...");
                }
            }
            // Insert a value into a setting.
            Some("push") => {
                let key = match key(&mut ctx, "!admin insert") {
                    Some(key) => key,
                    None => return Ok(()),
                };

                let mut values = self
                    .settings
                    .get::<Vec<serde_json::Value>>(key)?
                    .unwrap_or_default();

                let value = match serde_json::from_str(ctx.rest()) {
                    Ok(value) => value,
                    Err(_) => {
                        ctx.respond("Value must be valid JSON");
                        return Ok(());
                    }
                };

                values.push(value);
                self.settings.set(key, values)?;
                ctx.respond(format!("Updated the {} setting", key));
            }
            // Delete a value from a setting.
            Some("delete") => {
                let key = match key(&mut ctx, "!admin delete") {
                    Some(key) => key,
                    None => return Ok(()),
                };

                let mut values = self
                    .settings
                    .get::<Vec<serde_json::Value>>(key)?
                    .unwrap_or_default();

                let value = match serde_json::from_str::<serde_json::Value>(ctx.rest()) {
                    Ok(value) => value,
                    Err(_) => {
                        ctx.respond("Value must be valid JSON");
                        return Ok(());
                    }
                };

                values.retain(|v| v != &value);
                self.settings.set(key, values)?;
                ctx.respond(format!("Updated the {} setting", key));
            }
            // Get or set settings.
            Some("settings") => {
                let key = match key(&mut ctx, "!admin settings") {
                    Some(key) => key,
                    None => return Ok(()),
                };

                match ctx.rest().trim() {
                    "" => {
                        let value = match self.settings.get::<Option<serde_json::Value>>(key)? {
                            Some(value) => value,
                            None => {
                                let mut results = Vec::new();

                                for (key, value) in self.settings.get_by_prefix(key)? {
                                    // NB: security issue if this was present.
                                    if key.starts_with("secrets/") {
                                        continue;
                                    }

                                    results.push(format!(
                                        "{} = {}",
                                        key,
                                        serde_json::to_string(&value)?
                                    ));
                                }

                                if results.is_empty() {
                                    ctx.respond("No such setting");
                                } else {
                                    ctx.respond(results.join(", "));
                                }

                                return Ok(());
                            }
                        };

                        ctx.respond(format!("{} = {}", key, serde_json::to_string(&value)?));
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

/// Extract a settings key from the context.
fn key<'a>(ctx: &mut command::Context<'a, '_>, prefix: &str) -> Option<&'a str> {
    let key = match ctx.next() {
        Some(key) => key,
        None => {
            ctx.respond(format!(
                "Expected: {p} <key>",
                p = ctx.alias.unwrap_or(prefix)
            ));

            return None;
        }
    };

    if key.starts_with("secrets/") {
        ctx.respond("Cannot access secrets through chat!");
        return None;
    }

    Some(key)
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
