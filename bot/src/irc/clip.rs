use crate::{api, command, stream_info, utils};

/// Handler for the `!clip` command.
pub struct Clip {
    pub stream_info: stream_info::StreamInfo,
    pub clip_cooldown: utils::Cooldown,
    pub twitch: api::Twitch,
}

impl command::Handler for Clip {
    fn handle<'m>(&mut self, ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        if !self.clip_cooldown.is_open() {
            ctx.respond("A clip was already created recently");
            return Ok(());
        }

        let stream_info = self.stream_info.data.read();

        let user_id = match stream_info.user.as_ref() {
            Some(user) => user.id.to_string(),
            None => {
                log::error!("No information available on the current stream");
                ctx.respond("Cannot clip right now, stream is not live.");
                return Ok(());
            }
        };

        let title = match ctx.rest().trim() {
            "" => None,
            other => Some(other.to_string()),
        };

        let twitch = self.twitch.clone();
        let user = ctx.user.as_owned_user();

        ctx.spawn(async move {
            match twitch.create_clip(user_id.as_str()).await {
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
                    log_err!(e, "error when posting clip");
                }
            }
        });

        Ok(())
    }
}
