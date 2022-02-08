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
    pub stream: Option<twitch::new::Stream>,
    pub title: Option<String>,
    pub game: Option<String>,
    pub subs: Vec<twitch::new::Subscription>,
    pub subs_set: HashSet<String>,
}

/// Notify on changes in stream state.
pub enum StreamState {
    Started,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub user: Arc<api::User>,
    pub data: Arc<RwLock<Data>>,
}

impl StreamInfo {
    /// Check if a name is a subscriber.
    pub fn is_subscriber(&self, name: &str) -> bool {
        self.data.read().subs_set.contains(name)
    }

    /// Refresh the known list of subscribers.
    pub async fn refresh_subs(&self, twitch: &api::Twitch, streamer: &api::User) -> Result<()> {
        let subs = {
            let mut out = Vec::new();

            let stream = twitch.new_stream_subscriptions(&streamer.id, vec![]);
            tokio::pin!(stream);

            while let Some(sub) = stream.next().await.transpose()? {
                out.push(sub);
            }

            out
        };

        let mut info = self.data.write();
        info.subs = subs;
        info.subs_set = info
            .subs
            .iter()
            .map(|s| s.user_name.to_lowercase())
            .collect();

        Ok(())
    }

    /// Refresh channel.
    pub async fn refresh_channel<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer: &'a api::User,
    ) -> Result<()> {
        let channel = match twitch.new_channel_by_id(&streamer.id).await {
            Ok(Some(channel)) => channel,
            Ok(None) => {
                log::error!("no channel matching the given id `{id}`", id = streamer.id);
                return Ok(());
            }
            Err(e) => {
                log_error!(e, "failed to refresh channel");
                return Ok(());
            }
        };

        let mut info = self.data.write();
        info.title = channel.title;
        info.game = channel.game_name;
        Ok(())
    }

    /// Refresh the stream info.
    pub async fn refresh_stream<'a>(
        &'a self,
        twitch: &'a api::Twitch,
        streamer: &'a api::User,
        stream_state_tx: &'a mut mpsc::Sender<StreamState>,
    ) -> Result<()> {
        let stream = match twitch.new_stream_by_id(&streamer.id).await {
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
    streamer: Arc<api::User>,
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

    let mut stream_interval = tokio::time::interval(time::Duration::from_secs(30));
    let mut subs_interval = tokio::time::interval(time::Duration::from_secs(60 * 10));

    let future_info = stream_info.clone();

    let future = async move {
        twitch.token.wait_until_ready().await?;

        loop {
            tokio::select! {
                _ = subs_interval.tick() => {
                    if let Err(e) = future_info.refresh_subs(&twitch, &*streamer).await {
                        log_error!(e, "failed to refresh subscriptions");
                    }
                }
                _ = stream_interval.tick() => {
                    let stream = future_info
                        .refresh_stream(&twitch, &*streamer, &mut stream_state_tx);

                    let channel = future_info
                        .refresh_channel(&twitch, &*streamer);

                    tokio::try_join!(stream, channel)?;
                }
            }
        }
    };

    (stream_info, stream_state_rx, future)
}
