use crate::api;
use crate::api::twitch;
use crate::prelude::*;
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;
use std::time;

#[derive(Debug, Default)]
pub struct Data {
    pub stream: Option<twitch::Stream>,
    pub title: Option<String>,
    pub game: Option<String>,
    pub subs: Vec<twitch::Subscription>,
    pub subs_set: HashSet<String>,
}

/// Notify on changes in stream state.
pub enum StreamState {
    Started,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub user: Arc<twitch::User>,
    pub data: Arc<RwLock<Data>>,
}

impl StreamInfo {
    /// Check if a name is a subscriber.
    pub fn is_subscriber(&self, name: &str) -> bool {
        self.data.read().subs_set.contains(name)
    }

    /// Refresh the known list of subscribers.
    pub async fn refresh_subs<'a>(&'a self, twitch: &'a api::Twitch, streamer: &'a twitch::User) {
        let subs = twitch
            .stream_subscriptions(&streamer.id, vec![])
            .try_concat();

        let subs = match subs.await {
            Ok(subs) => subs,
            Err(e) => {
                log_error!(e, "failed to fetch subscriptions");
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

    /// Refresh channel.
    pub async fn refresh_channel<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer: &'a twitch::User,
    ) -> Result<()> {
        let channel = match twitch.channel_by_id(&streamer.id).await {
            Ok(channel) => channel,
            Err(e) => {
                log_error!(e, "failed to refresh channel");
                return Ok(());
            }
        };

        let mut info = self.data.write();
        info.title = channel.status;
        info.game = channel.game;
        Ok(())
    }

    /// Refresh the stream info.
    pub async fn refresh_stream<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer: &'a twitch::User,
        stream_state_tx: &'a mut mpsc::Sender<StreamState>,
    ) -> Result<()> {
        let stream = match twitch.stream_by_id(&streamer.id).await {
            Ok(stream) => stream,
            Err(e) => {
                log_error!(e, "failed to refresh stream");
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
                .map_err(|_| anyhow!("failed to send stream state update"))?;
        }

        let mut info = self.data.write();
        info.stream = stream;
        Ok(())
    }
}

/// Set up a stream information loop.
pub fn setup(
    streamer: Arc<twitch::User>,
    twitch: api::Twitch,
) -> (
    StreamInfo,
    mpsc::Receiver<StreamState>,
    impl Future<Output = Result<()>>,
) {
    let (mut stream_state_tx, stream_state_rx) = mpsc::channel(64);

    let stream_info = StreamInfo {
        user: streamer.clone(),
        data: Default::default(),
    };

    let mut stream_interval = tokio::time::interval(time::Duration::from_secs(30)).fuse();
    let mut subs_interval = tokio::time::interval(time::Duration::from_secs(60 * 10)).fuse();

    let future_info = stream_info.clone();

    let future = async move {
        twitch.token.wait_until_ready().await?;

        loop {
            futures::select! {
                _ = subs_interval.select_next_some() => {
                    future_info.refresh_subs(&twitch, &*streamer).await;
                }
                _ = stream_interval.select_next_some() => {
                    let stream = future_info
                        .refresh_stream(&twitch, &*streamer, &mut stream_state_tx);

                    let channel = future_info
                        .refresh_channel(&twitch, &*streamer);

                    future::try_join(stream, channel).await?;
                }
            }
        }
    };

    (stream_info, stream_state_rx, future)
}
