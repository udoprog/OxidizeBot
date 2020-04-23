use crate::{
    api, auth, command, module,
    prelude::*,
    stream_info,
    utils::{Cooldown, Duration},
};
use anyhow::Error;
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the `!clip` command.
pub struct Clip {
    pub enabled: Arc<RwLock<bool>>,
    pub stream_info: stream_info::StreamInfo,
    pub clip_cooldown: Arc<RwLock<Cooldown>>,
    pub twitch: api::Twitch,
}

#[async_trait]
impl command::Handler for Clip {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Clip)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<(), Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        if !self.clip_cooldown.write().is_open() {
            ctx.respond("A clip was already created recently");
            return Ok(());
        }

        let stream_user = self.stream_info.user.clone();

        let title = match ctx.rest().trim() {
            "" => None,
            other => Some(other.to_string()),
        };

        let twitch = self.twitch.clone();
        let user = ctx.user.clone();

        match twitch.create_clip(&stream_user.id).await {
            Ok(Some(clip)) => {
                user.respond(format!(
                    "Created clip at {}/{}",
                    api::twitch::CLIPS_URL,
                    clip.id
                ));

                if let Some(_title) = title {
                    log::warn!("Title was requested, but it can't be set (right now)")
                }
            }
            Ok(None) => {
                user.respond("Failed to create clip, sorry :(");
                log::error!("created clip, but API returned nothing");
            }
            Err(e) => {
                user.respond("Failed to create clip, sorry :(");
                log_error!(e, "error when posting clip");
            }
        }

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "clip"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            stream_info,
            twitch,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), Error> {
        let settings = settings.scoped("clip");

        handlers.insert(
            "clip",
            Clip {
                enabled: settings.var("enabled", true)?,
                stream_info: stream_info.clone(),
                clip_cooldown: settings
                    .var("cooldown", Cooldown::from_duration(Duration::seconds(30)))?,
                twitch: twitch.clone(),
            },
        );

        Ok(())
    }
}
