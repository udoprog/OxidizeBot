use crate::{api, prelude::*};
use hashbrown::HashSet;
use parking_lot::RwLock;
use std::{sync::Arc, time};
use tokio::timer;

#[derive(Debug, Default)]
pub struct StreamInfo {
    pub stream: Option<api::twitch::Stream>,
    pub user: Option<api::twitch::User>,
    pub title: Option<String>,
    pub game: Option<String>,
    pub subs: Vec<api::twitch::Subscription>,
    pub subs_set: HashSet<String>,
}

impl StreamInfo {
    /// Check if a name is a subscriber.
    pub fn is_subscriber(&self, name: &str) -> bool {
        self.subs_set.contains(name)
    }
}

/// Set up a reward loop.
pub fn setup(
    streamer: String,
    interval: time::Duration,
    twitch: api::Twitch,
) -> (
    Arc<RwLock<StreamInfo>>,
    impl Future<Output = Result<(), failure::Error>>,
) {
    let info = Arc::new(RwLock::new(StreamInfo::default()));

    let mut interval = timer::Interval::new(time::Instant::now(), interval).compat();

    let future_info = info.clone();

    let future = async move {
        while let Some(i) = interval.next().await {
            let _ = i?;

            let stream = twitch.stream_by_login(streamer.as_str());
            let channel = twitch.channel_by_login(streamer.as_str());
            let streamer = twitch.user_by_login(streamer.as_str());

            let (stream, channel, streamer) = future::try_join3(stream, channel, streamer).await?;

            let subscriptions = match streamer.as_ref() {
                Some(streamer) => {
                    twitch
                        .stream_subscriptions(&streamer.id, vec![])
                        .try_concat()
                        .await?
                }
                None => Default::default(),
            };

            let mut info = future_info.write();
            info.user = streamer;
            info.stream = stream;
            info.title = Some(channel.status);
            info.game = channel.game;
            info.subs = subscriptions;
            info.subs_set = info
                .subs
                .iter()
                .map(|s| s.user_name.to_lowercase())
                .collect();
        }

        Ok(())
    };

    (info, future)
}
