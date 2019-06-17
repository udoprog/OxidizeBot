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

    /// Refresh the known list of subscribers.
    pub async fn refresh_subs<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer: &'a api::twitch::User,
    ) {
        let subs = twitch
            .stream_subscriptions(&streamer.id, vec![])
            .try_concat();

        let subs = match subs.await {
            Ok(subs) => subs,
            Err(e) => {
                log_err!(e, "failed to fetch subscriptions");
                return;
            }
        };

        let mut info = self.data.write();
        info.subs = subs;
        info.subs_set = info
            .subs
            .iter()
            .map(|s| s.user_name.to_lowercase())
            .collect();
    }

    /// Get streamer information.
    pub async fn fetch_streamer<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer_id: &'a str,
    ) -> Option<api::twitch::User> {
        let user = match twitch.user_by_login(streamer_id).await {
            Ok(user) => user,
            Err(e) => {
                log_err!(e, "failed to fetch streamer");
                return None;
            }
        };

        let mut info = self.data.write();
        info.user = user.clone();
        user
    }

    /// Refresh channel.
    pub async fn refresh_channel<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer_id: &'a str,
    ) -> Result<(), Error> {
        let channel = match twitch.channel_by_login(streamer_id).await {
            Ok(channel) => channel,
            Err(e) => {
                log_err!(e, "failed to refresh channel");
                return Ok(());
            }
        };

        let mut info = self.data.write();
        info.title = Some(channel.status);
        info.game = channel.game;
        Ok(())
    }

    /// Refresh the stream info.
    pub async fn refresh_stream<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer_id: &'a str,
        stream_state_tx: &'a mut mpsc::Sender<StreamState>,
    ) -> Result<(), Error> {
        let stream = match twitch.stream_by_login(streamer_id).await {
            Ok(stream) => stream,
            Err(e) => {
                log_err!(e, "failed to refresh stream");
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
        info.stream = stream;
        Ok(())
    }
}

/// Set up a reward loop.
pub fn setup<'a>(
    streamer_id: &'a str,
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

    let now = time::Instant::now();
    let mut stream_interval = timer::Interval::new(now.clone(), time::Duration::from_secs(30));
    let mut subs_interval = timer::Interval::new(now.clone(), time::Duration::from_secs(60 * 10));
    let mut streamer_interval = timer::Interval::new(
        now.clone() + time::Duration::from_secs(60),
        time::Duration::from_secs(60),
    );

    let future_info = stream_info.clone();

    let future = async move {
        twitch.token.wait_until_ready().await?;

        let mut streamer = future_info.fetch_streamer(&twitch, streamer_id).await;

        loop {
            futures::select! {
                update = subs_interval.select_next_some() => {
                    update?;

                    if let Some(streamer) = streamer.as_ref() {
                        future_info.refresh_subs(&twitch, streamer).await;
                    }
                }
                update = streamer_interval.select_next_some() => {
                    update?;

                    let update = future_info.fetch_streamer(&twitch, streamer_id).await;

                    if let Some(update) = update {
                        streamer = Some(update);
                    }
                }
                update = stream_interval.select_next_some() => {
                    update?;

                    let stream = future_info
                        .refresh_stream(&twitch, streamer_id, &mut stream_state_tx);

                    let channel = future_info
                        .refresh_channel(&twitch, streamer_id);

                    future::try_join(stream, channel).await?;
                }
            }
        }
    };

    (stream_info, stream_state_rx, future)
}
