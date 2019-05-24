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

            let streamer = async {
                let streamer = twitch.user_by_login(streamer.as_str()).await?;

                let streamer = match streamer {
                    Some(streamer) => streamer,
                    None => return Ok((None, None)),
                };

                let subs = twitch
                    .stream_subscriptions(&streamer.id, vec![])
                    .try_concat();

                let subs = match subs.await {
                    Ok(subs) => Some(subs),
                    Err(e) => {
                        log_err!(e, "failed to fetch subscriptions");
                        None
                    }
                };

                Ok((Some(streamer), subs))
            };

            let (stream, channel, (streamer, subs)) =
                future::try_join3(stream, channel, streamer).await?;

            let mut info = future_info.write();
            info.user = streamer;
            info.stream = stream;
            info.title = Some(channel.status);
            info.game = channel.game;

            if let Some(subs) = subs {
                info.subs = subs;
                info.subs_set = info
                    .subs
                    .iter()
                    .map(|s| s.user_name.to_lowercase())
                    .collect();
            }
        }

        Ok(())
    };

    (info, future)
}
