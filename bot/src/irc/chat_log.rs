use crate::{
    api::{twitch::Channel, Twitch},
    emotes, injector, irc, message_log, settings,
    storage::Cache,
};
use failure::Error;

pub struct Builder {
    twitch: Twitch,
    pub(crate) message_log: message_log::MessageLog,
    pub(crate) cache_stream: injector::Stream<Cache>,
    pub(crate) cache: Option<Cache>,
    pub(crate) enabled_stream: settings::Stream<bool>,
    pub(crate) enabled: bool,
    pub(crate) emotes_enabled_stream: settings::Stream<bool>,
    pub(crate) emotes_enabled: bool,
}

impl Builder {
    pub fn new(
        twitch: Twitch,
        injector: &injector::Injector,
        message_log: message_log::MessageLog,
        settings: settings::Settings,
    ) -> Result<Self, Error> {
        let (cache_stream, cache) = injector.stream::<Cache>();

        let (enabled_stream, enabled) = settings.stream("enabled").or_default()?;

        let (emotes_enabled_stream, emotes_enabled) =
            settings.stream("emotes-enabled").or_default()?;

        message_log.enabled(enabled);

        Ok(Self {
            twitch,
            message_log,
            cache_stream,
            cache,
            enabled_stream,
            enabled,
            emotes_enabled_stream,
            emotes_enabled,
        })
    }

    /// Construct a new chat log with the specified configuration.
    pub fn build(&self) -> Result<Option<ChatLog>, Error> {
        if !self.enabled {
            return Ok(None);
        }

        let emotes = match (self.emotes_enabled, self.cache.as_ref()) {
            (true, Some(cache)) => Some(emotes::Emotes::new(cache.clone(), self.twitch.clone())?),
            _ => None,
        };

        Ok(Some(ChatLog {
            message_log: self.message_log.clone(),
            emotes,
        }))
    }
}

#[derive(Clone)]
pub struct ChatLog {
    /// Log to add messages to.
    pub message_log: message_log::MessageLog,
    /// Handler of emotes.
    emotes: Option<emotes::Emotes>,
}

impl ChatLog {
    pub async fn observe(&self, tags: &irc::Tags, channel: &Channel, name: &str, message: &str) {
        let rendered = match self.emotes.as_ref() {
            Some(emotes) => match emotes.render(&tags, channel, name, message).await {
                Ok(rendered) => Some(rendered),
                Err(e) => {
                    log::warn!("failed to render emotes: {}", e);
                    None
                }
            },
            None => None,
        };

        self.message_log.push_back(&tags, &name, message, rendered);
    }
}
