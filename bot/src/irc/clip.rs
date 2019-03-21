use crate::{twitch, utils, utils::BoxFuture};
use failure::format_err;
use futures::{future, Future as _};
use std::sync::{Arc, RwLock};

/// Handler for the `!clip` command.
pub struct Clip {
    pub stream_info: Arc<RwLock<Option<super::StreamInfo>>>,
    pub clip_cooldown: utils::Cooldown,
    pub twitch: twitch::Twitch,
}

impl super::CommandHandler for Clip {
    fn handle<'m>(
        &mut self,
        ctx: super::CommandContext<'_>,
        user: super::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        if !self.clip_cooldown.is_open() {
            user.respond("A clip was already created recently");
            return Ok(());
        }

        let stream_info = self.stream_info.read().expect("poisoned");

        let user_id = match stream_info.as_ref().and_then(|s| s.user.as_ref()) {
            Some(user) => user.id.as_str(),
            None => {
                log::error!("No information available on the current stream");
                user.respond("Cannot clip right now, stream is not live.");
                return Ok(());
            }
        };

        let title = match it.rest().trim() {
            "" => None,
            other => Some(other.to_string()),
        };

        let user = user.as_owned_user();

        let future = self.twitch.create_clip(user_id);

        let future = future.then::<_, BoxFuture<(), failure::Error>>({
            let _twitch = self.twitch.clone();

            move |result| {
                let result = match result {
                    Ok(Some(clip)) => {
                        user.respond(format!("Created clip at {}/{}", twitch::CLIPS_URL, clip.id));

                        if let Some(_title) = title {
                            log::warn!("can't update title right now :(")
                        }

                        Ok(())
                    }
                    Ok(None) => {
                        user.respond("Failed to create clip, sorry :(");
                        Err(format_err!("created clip, but API returned nothing"))
                    }
                    Err(e) => {
                        user.respond("Failed to create clip, sorry :(");
                        Err(format_err!("failed to create clip: {}", e))
                    }
                };

                Box::new(future::result(result))
            }
        });

        ctx.spawn(future.map_err(|e| {
            utils::log_err("error when posting clip", e);
        }));

        Ok(())
    }
}
