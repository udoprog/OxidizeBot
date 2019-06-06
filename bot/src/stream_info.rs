use crate::{api, prelude::*, timer};
use hashbrown::HashSet;
use parking_lot::RwLock;
use std::{sync::Arc, time};

#[derive(Debug, Default)]
pub struct Data {
    pub stream: Option<api::twitch::Stream>,
    pub user: Option<api::twitch::User>,
    pub title: Option<String>,
    pub game: Option<String>,
    pub subs: Vec<api::twitch::Subscription>,
    pub subs_set: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub data: Arc<RwLock<Data>>,
}

impl StreamInfo {
    /// Check if a name is a subscriber.
    pub fn is_subscriber(&self, name: &str) -> bool {
        self.data.read().subs_set.contains(name)
    }

    /// Refresh the stream info.
    pub async fn refresh<'a>(&'a self, twitch: &'a api::Twitch, streamer: &'a str) {
        let stream = twitch.stream_by_login(streamer);

        let streamer = async {
            let streamer = twitch.user_by_login(streamer).await?;

            let streamer = match streamer {
                Some(streamer) => streamer,
                None => return Ok((None, None, None)),
            };

            let channel = twitch.channel_by_login(&streamer.id);

            let subs = twitch
                .stream_subscriptions(&streamer.id, vec![])
                .try_concat();

            let (subs, channel) = match future::try_join(subs, channel).await {
                Ok((subs, channel)) => (Some(subs), Some(channel)),
                Err(e) => {
                    log_err!(e, "failed to fetch subscriptions or channel");
                    (None, None)
                }
            };

            Ok((Some(streamer), subs, channel))
        };

        let result = future::try_join(stream, streamer).await;

        let (stream, (streamer, subs, channel)) = match result {
            Ok(result) => result,
            Err(e) => {
                log_err!(e, "failed to refresh stream info");
                return;
            }
        };

        let mut info = self.data.write();
        info.user = streamer;
        info.stream = stream;

        if let Some(channel) = channel {
            info.title = Some(channel.status);
            info.game = channel.game;
        }

        if let Some(subs) = subs {
            info.subs = subs;
            info.subs_set = info
                .subs
                .iter()
                .map(|s| s.user_name.to_lowercase())
                .collect();
        }
    }
}

/// Set up a reward loop.
pub fn setup<'a>(
    streamer: &'a str,
    interval: time::Duration,
    twitch: api::Twitch,
) -> (
    StreamInfo,
    impl Future<Output = Result<(), failure::Error>> + 'a,
) {
    let stream_info = StreamInfo {
        data: Default::default(),
    };

    let mut interval = timer::Interval::new(time::Instant::now(), interval);

    let future_info = stream_info.clone();

    let future = async move {
        twitch.token.wait_until_ready().await?;

        while let Some(_) = interval.next().await.transpose()? {
            future_info.refresh(&twitch, streamer).await;
        }

        Ok(())
    };

    (stream_info, future)
}
