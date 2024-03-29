use anyhow::Result;
use async_trait::async_trait;

use chat::command;
use chat::module;

/// Handler for the !admin command.
pub(crate) struct Handler {
    settings: settings::Settings<::auth::Scope>,
    aliases: async_injector::Ref<db::Aliases>,
    commands: async_injector::Ref<db::Commands>,
    promotions: async_injector::Ref<db::Promotions>,
    themes: async_injector::Ref<db::Themes>,
}

impl Handler {
    /// List settings by prefix.
    async fn list_settings_by_prefix(
        &self,
        ctx: &mut command::Context<'_>,
        key: &str,
    ) -> Result<()> {
        let mut results = Vec::new();

        let settings = self.settings.list_by_prefix(key).await?;

        for setting in settings.iter().take(10) {
            // NB: security issue if this was present.
            if key.starts_with("secrets/") || setting.schema().is_secret() {
                continue;
            }

            let value = serde_json::to_string(setting.value())?;

            let value = if value.len() > 20 {
                "*too long*"
            } else {
                &value
            };

            results.push(format!(
                "..{} = {}",
                setting.key().trim_start_matches(key),
                value,
            ));
        }

        if results.is_empty() {
            chat::respond!(ctx, "No settings starting with `{}`", key);
        } else {
            let mut response = results.join(", ");

            if results.len() < settings.len() {
                let more = settings.len() - results.len();
                response = format!("{} .. and {} more", response, more);
            }

            ctx.respond(response).await;
        }

        Ok(())
    }
}

#[async_trait]
impl command::Handler for Handler {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Admin)
    }

    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        match ctx.next().as_deref() {
            Some("refresh-mods") => {
                ctx.notify().refresh_mods.notify_one();
                chat::respond!(ctx, "Refreshing information on mods");
            }
            Some("refresh-vips") => {
                ctx.notify().refresh_vips.notify_one();
                chat::respond!(ctx, "Refreshing information on vips");
            }
            Some("refresh") => {
                ctx.notify().refresh_mods.notify_one();
                ctx.notify().refresh_vips.notify_one();
                chat::respond!(ctx, "Refreshing information on mods and vips");
            }
            Some("version") => {
                chat::respond!(ctx, "OxidizeBot Version {}", crate::VERSION);
            }
            Some("shutdown") | Some("restart") => {
                ctx.notify().restart.notify_one();
                chat::respond!(ctx, "Restarting...");
            }
            // Insert a value into a setting.
            Some("push") => {
                let key = key(ctx)?;
                let value = self.edit_value_in_set(ctx, &key).await?;

                let mut values = self
                    .settings
                    .get::<Vec<serde_json::Value>>(&key)
                    .await?
                    .unwrap_or_default();

                values.push(value);
                self.settings.set(&key, values).await?;
                chat::respond!(ctx, "Updated the {} setting", key);
            }
            // Delete a value from a setting.
            Some("delete") => {
                let key = key(ctx)?;
                let value = self.edit_value_in_set(ctx, &key).await?;

                let mut values = self
                    .settings
                    .get::<Vec<serde_json::Value>>(&key)
                    .await?
                    .unwrap_or_default();

                values.retain(|v| v != &value);
                self.settings.set(&key, values).await?;
                chat::respond!(ctx, "Updated the {} setting", key);
            }
            Some("toggle") => {
                self.toggle(ctx).await?;
            }
            Some("enable-group") => {
                let group = ctx
                    .next()
                    .ok_or(chat::respond_err!("Expected <group> to enable"))?;

                if let Some(aliases) = self.aliases.read().await.as_deref() {
                    aliases.enable_group(ctx.channel(), &group).await?;
                }

                if let Some(commands) = self.commands.read().await.as_deref() {
                    commands.enable_group(ctx.channel(), &group).await?;
                }

                if let Some(promotions) = self.promotions.read().await.as_deref() {
                    promotions.enable_group(ctx.channel(), &group).await?;
                }

                if let Some(themes) = self.themes.read().await.as_deref() {
                    themes.enable_group(ctx.channel(), &group).await?;
                }

                chat::respond!(ctx, "Enabled group {}", group);
            }
            Some("disable-group") => {
                let group = ctx
                    .next()
                    .ok_or(chat::respond_err!("Expected <group> to disable"))?;

                if let Some(aliases) = self.aliases.read().await.as_deref() {
                    aliases.disable_group(ctx.channel(), &group).await?;
                }

                if let Some(commands) = self.commands.read().await.as_deref() {
                    commands.disable_group(ctx.channel(), &group).await?;
                }

                if let Some(promotions) = self.promotions.read().await.as_deref() {
                    promotions.disable_group(ctx.channel(), &group).await?;
                }

                if let Some(themes) = self.themes.read().await.as_deref() {
                    themes.disable_group(ctx.channel(), &group).await?;
                }

                chat::respond!(ctx, "Disabled group {}", group);
            }
            // Get or set settings.
            Some("settings") => {
                let key = key(ctx)?;

                match ctx.rest().trim() {
                    "" => {
                        let setting = match self
                            .settings
                            .setting::<Option<serde_json::Value>>(&key)
                            .await?
                        {
                            Some(value) => value,
                            None => return self.list_settings_by_prefix(ctx, &key).await,
                        };

                        if setting.schema().is_secret() {
                            chat::respond_bail!("Cannot show secret setting `{}`", key);
                        }

                        chat::respond!(
                            ctx,
                            "{} = {}",
                            key,
                            serde_json::to_string(&setting.value())?
                        );
                    }
                    value => {
                        let schema = self
                            .settings
                            .lookup(&key)
                            .ok_or(chat::respond_err!("No such setting"))?;

                        let value = schema.ty().parse_as_json(value).map_err(|e| {
                            chat::respond_err!("Value is not a valid {} type: {}", schema.ty(), e)
                        })?;

                        if let Some(scope) = schema.scope() {
                            if !ctx.user.has_scope(scope).await {
                                chat::respond_bail!(
                                    "You are not permitted to modify that setting, sorry :(",
                                );
                            }
                        }

                        let value_string = serde_json::to_string(&value)?;
                        self.settings.set_json(&key, value).await?;
                        chat::respond!(ctx, "Updated setting {} = {}", key, value_string);
                    }
                }
            }
            _ => {
                chat::respond!(
                    ctx,
                    "Expected one of: \
                     refresh-mods, \
                     refresh-vips, \
                     version, \
                     shutdown, \
                     settings.",
                );
            }
        }

        Ok(())
    }
}

impl Handler {
    /// Handler for the toggle command.
    async fn toggle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        let key = key(ctx)?;

        let setting = self
            .settings
            .setting::<serde_json::Value>(&key)
            .await?
            .ok_or(chat::respond_err!("No setting matching key: {}", key))?;

        if let Some(scope) = setting.schema().scope() {
            if !ctx.user.has_scope(scope).await {
                chat::respond!(
                    ctx,
                    "You are not permitted to modify that setting, sorry :("
                );
                return Ok(());
            }
        }

        // Check type of the setting.
        let toggled = match setting.schema().ty() {
            settings::Type {
                kind: settings::Kind::Bool,
                ..
            } => match setting.value() {
                Some(serde_json::Value::Bool(value)) => serde_json::Value::Bool(!value),
                // non-booleans are interpreted as `false`.
                _ => serde_json::Value::Bool(false),
            },
            other => {
                chat::respond!(
                    ctx,
                    "Can only toggle bool settings, but {} is a {}",
                    key,
                    other
                );
                return Ok(());
            }
        };

        let value_string = serde_json::to_string(&toggled)?;
        self.settings.set_json(&key, toggled).await?;
        chat::respond!(ctx, "Updated setting {} = {}", key, value_string);
        Ok(())
    }

    /// Parse the rest of the context as a value corresponding to the given set.
    ///
    /// Also tests that we have the permission to modify the specified setting.
    async fn edit_value_in_set(
        &self,
        ctx: &mut command::Context<'_>,
        key: &str,
    ) -> Result<serde_json::Value> {
        let schema = self
            .settings
            .lookup(key)
            .ok_or(chat::respond_err!("No such setting"))?;

        // Test schema permissions.
        if let Some(scope) = schema.scope() {
            if !ctx.user.has_scope(scope).await {
                return Err(chat::respond_err!(
                    "You are not permitted to modify that setting, sorry :("
                )
                .into());
            }
        }

        let ty = match schema.ty() {
            settings::Type {
                kind: settings::Kind::Set { value },
                ..
            } => value,
            other => {
                return Err(
                    chat::respond_err!("Configuration is a {}, but expected a set", other).into(),
                );
            }
        };

        let value = ty
            .parse_as_json(ctx.rest())
            .map_err(|e| chat::respond_err!("Value is not a valid {} type: {}", ty, e))?;

        Ok(value)
    }
}

/// Extract a settings key from the context.
fn key(ctx: &mut command::Context<'_>) -> Result<String> {
    let key = ctx.next().ok_or(chat::respond_err!("Expected <key>"))?;

    if key.starts_with("secrets/") {
        chat::respond_bail!("Cannot access secrets through chat!");
    }

    Ok(key)
}

pub(crate) struct Module;

#[async_trait]
impl chat::Module for Module {
    fn ty(&self) -> &'static str {
        "admin"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector,
            handlers,
            settings,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<()> {
        handlers.insert(
            "admin",
            Handler {
                settings: settings.clone(),
                aliases: injector.var().await,
                commands: injector.var().await,
                promotions: injector.var().await,
                themes: injector.var().await,
            },
        );

        Ok(())
    }
}
