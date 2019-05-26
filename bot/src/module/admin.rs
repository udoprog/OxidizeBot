use crate::{command, db, module, settings};

/// Handler for the !admin command.
pub struct Handler<'a> {
    settings: &'a settings::Settings,
    aliases: &'a db::Aliases,
    commands: &'a db::Commands,
    promotions: &'a db::Promotions,
    themes: &'a db::Themes,
}

impl<'a> command::Handler for Handler<'a> {
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

                let value = match self.value_in_set(&mut ctx, key) {
                    Some(ty) => ty,
                    None => return Ok(()),
                };

                let mut values = self
                    .settings
                    .get::<Vec<serde_json::Value>>(key)?
                    .unwrap_or_default();

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

                let value = match self.value_in_set(&mut ctx, key) {
                    Some(ty) => ty,
                    None => return Ok(()),
                };

                let mut values = self
                    .settings
                    .get::<Vec<serde_json::Value>>(key)?
                    .unwrap_or_default();

                values.retain(|v| v != &value);
                self.settings.set(key, values)?;
                ctx.respond(format!("Updated the {} setting", key));
            }
            Some("enable-group") => {
                let group = match ctx.next() {
                    Some(group) => group,
                    None => {
                        ctx.respond(format!(
                            "Expected: {p} <group>",
                            p = ctx.alias.unwrap_or("!alias enable-group")
                        ));
                        return Ok(());
                    }
                };

                self.aliases.enable_group(ctx.user.target, group)?;
                self.commands.enable_group(ctx.user.target, group)?;
                self.promotions.enable_group(ctx.user.target, group)?;
                self.themes.enable_group(ctx.user.target, group)?;

                ctx.respond(format!("Enabled group {}", group));
            }
            Some("disable-group") => {
                let group = match ctx.next() {
                    Some(group) => group,
                    None => {
                        ctx.respond(format!(
                            "Expected: {p} <group>",
                            p = ctx.alias.unwrap_or("!alias disable-group")
                        ));
                        return Ok(());
                    }
                };

                self.aliases.disable_group(ctx.user.target, group)?;
                self.commands.disable_group(ctx.user.target, group)?;
                self.promotions.disable_group(ctx.user.target, group)?;
                self.themes.disable_group(ctx.user.target, group)?;

                ctx.respond(format!("Disabled group {}", group));
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
                        let schema = match self.settings.schema.lookup(key) {
                            Some(schema) => schema,
                            None => {
                                ctx.respond("No such setting");
                                return Ok(());
                            }
                        };

                        let value = match schema.ty.parse_as_json(value) {
                            Ok(value) => value,
                            Err(e) => {
                                ctx.respond(format!(
                                    "Value is not a valid {} type: {}",
                                    schema.ty, e
                                ));
                                return Ok(());
                            }
                        };

                        if let Some(scope) = schema.scope.clone() {
                            log::warn!("scope required: {}", scope);

                            if !ctx.has_scope(scope) {
                                ctx.respond(
                                    "You are not permitted to modify that setting, sorry :(",
                                );
                                return Ok(());
                            }
                        }

                        let value_string = serde_json::to_string(&value)?;
                        self.settings.set_json(key, value)?;
                        ctx.respond(format!("Updated setting {} = {}", key, value_string));
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

impl<'a> Handler<'a> {
    /// Get a value that corresponds with the given set.
    fn value_in_set(
        &mut self,
        ctx: &mut command::Context<'_, '_>,
        key: &str,
    ) -> Option<serde_json::Value> {
        let schema = match self.settings.schema.lookup(key) {
            Some(schema) => schema,
            None => {
                ctx.respond("No such setting");
                return None;
            }
        };

        let ty = match schema.ty {
            settings::Type {
                kind: settings::Kind::Set { value },
                ..
            } => value,
            other => {
                ctx.respond(format!("Configuration is a {}, but expected a set", other));
                return None;
            }
        };

        let value = match ty.parse_as_json(ctx.rest()) {
            Ok(value) => value,
            Err(e) => {
                ctx.respond(format!("Value is not a valid {} type: {}", ty, e));
                return None;
            }
        };

        Some(value)
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
            handlers,
            settings,
            aliases,
            commands,
            promotions,
            themes,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        handlers.insert(
            "admin",
            Handler {
                settings,
                aliases,
                commands,
                promotions,
                themes,
            },
        );

        Ok(())
    }
}
