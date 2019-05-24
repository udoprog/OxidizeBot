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
    twitch: api::Twitch,
    streamer: Arc<String>,
    pub data: Arc<RwLock<Data>>,
}

impl StreamInfo {
    /// Check if a name is a subscriber.
    pub fn is_subscriber(&self, name: &str) -> bool {
        self.data.read().subs_set.contains(name)
    }

    /// Refresh the stream info.
    pub async fn refresh(&self) {
        let stream = self.twitch.stream_by_login(self.streamer.as_str());
        let channel = self.twitch.channel_by_login(self.streamer.as_str());

        let streamer = async {
            let streamer = self.twitch.user_by_login(self.streamer.as_str()).await?;

            let streamer = match streamer {
                Some(streamer) => streamer,
                None => return Ok((None, None)),
            };

            let subs = self
                .twitch
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

        let result = future::try_join3(stream, channel, streamer).await;

        let (stream, channel, (streamer, subs)) = match result {
            Ok(result) => result,
            Err(e) => {
                log_err!(e, "failed to refresh stream info");
                return;
            }
        };

        let mut info = self.data.write();
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
}

/// Set up a reward loop.
pub fn setup(
    streamer: String,
    interval: time::Duration,
    twitch: api::Twitch,
) -> (StreamInfo, impl Future<Output = Result<(), failure::Error>>) {
    let stream_info = StreamInfo {
        twitch,
        streamer: Arc::new(streamer),
        data: Default::default(),
    };

    let stream_info = stream_info;

    let mut interval = timer::Interval::new(time::Instant::now(), interval);

    let future_info = stream_info.clone();

    let future = async move {
        while let Some(_) = interval.next().await.transpose()? {
            future_info.refresh().await;
        }

        Ok(())
    };

    (stream_info, future)
}
