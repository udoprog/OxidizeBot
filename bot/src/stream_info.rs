use crate::{api, prelude::*, timer};
use failure::{format_err, Error};
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

/// Notify on changes in stream state.
pub enum StreamState {
    Started,
    Stopped,
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
    pub async fn refresh<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer: &'a str,
        stream_state_tx: &'a mut mpsc::Sender<StreamState>,
    ) -> Result<(), Error> {
        let stream = twitch.stream_by_login(streamer);
        let channel = twitch.channel_by_login(streamer);

        let streamer = async {
            let streamer = twitch.user_by_login(streamer).await?;

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

        let result = future::try_join3(stream, channel, streamer).await;

        let (stream, channel, (streamer, subs)) = match result {
            Ok(result) => result,
            Err(e) => {
                log_err!(e, "failed to refresh stream info");
                return Ok(());
            }
        };

        let stream_is_some = self.data.read().stream.is_some();

        let update = match (stream_is_some, stream.is_some()) {
            (true, false) => Some(StreamState::Stopped),
            (false, true) => Some(StreamState::Started),
            _ => None,
        };

        if let Some(update) = update {
            stream_state_tx
                .send(update)
                .await
                .map_err(|_| format_err!("failed to send stream state update"))?;
        }

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

        Ok(())
    }
}

/// Set up a reward loop.
pub fn setup<'a>(
    streamer: &'a str,
    interval: time::Duration,
    twitch: api::Twitch,
) -> (
    StreamInfo,
    mpsc::Receiver<StreamState>,
    impl Future<Output = Result<(), Error>> + 'a,
) {
    let (mut stream_state_tx, stream_state_rx) = mpsc::channel(64);

    let stream_info = StreamInfo {
        data: Default::default(),
    };

    let mut interval = timer::Interval::new(time::Instant::now(), interval);

    let future_info = stream_info.clone();

    let future = async move {
        twitch.token.wait_until_ready().await?;

        while let Some(_) = interval.next().await.transpose()? {
            future_info
                .refresh(&twitch, streamer, &mut stream_state_tx)
                .await?;
        }

        Err(format_err!("update interval ended"))
    };

    (stream_info, stream_state_rx, future)
}
