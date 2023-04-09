use std::collections::HashSet;
use std::pin::pin;
use std::sync::Arc;
use std::time;

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use tracing::Instrument;

use crate::api;
use crate::api::twitch;
use crate::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct Data {
    pub(crate) stream: Option<twitch::new::Stream>,
    pub(crate) title: Option<String>,
    pub(crate) game: Option<String>,
    pub(crate) subs: Vec<twitch::new::Subscription>,
    pub(crate) subs_set: HashSet<String>,
}

/// Notify on changes in stream state.
pub(crate) enum StreamState {
    Started,
    Stopped,
}

#[derive(Debug, Clone)]
pub(crate) struct StreamInfo {
    pub(crate) data: Arc<RwLock<Data>>,
}

impl StreamInfo {
    /// Check if a name is a subscriber.
    pub(crate) fn is_subscriber(&self, name: &str) -> bool {
        self.data.read().subs_set.contains(name)
    }

    /// Refresh the known list of subscribers.
    pub(crate) async fn refresh_subs(&self, streamer: &api::TwitchAndUser) -> Result<()> {
        let subs = {
            let mut out = Vec::new();

            let mut stream = pin!(streamer.client.subscriptions(&streamer.user.id, vec![]));

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
    #[tracing::instrument(skip_all, fields(id = ?streamer.user.id))]
    pub(crate) async fn refresh_channel<'a>(
        &'a self,
        streamer: &'a api::TwitchAndUser,
    ) -> Result<()> {
        let channel = match streamer.client.channels(&streamer.user.id).await {
            Ok(Some(channel)) => channel,
            Ok(None) => {
                tracing::warn!("No channel matching the given id`");
                return Ok(());
            }
            Err(e) => {
                log_warn!(e, "Failed to refresh channel");
                return Ok(());
            }
        };

        let mut info = self.data.write();
        info.title = channel.title;
        info.game = channel.game_name;
        Ok(())
    }

    /// Refresh the stream info.
    #[tracing::instrument(skip_all, fields(id = ?streamer.user.id))]
    pub(crate) async fn refresh_stream<'a>(
        &'a self,
        streamer: &'a api::TwitchAndUser,
        stream_state_tx: &'a mpsc::Sender<StreamState>,
    ) -> Result<()> {
        let mut streams = pin!(streamer.client.streams(&streamer.user.id).await);

        let stream = streams.next().await.transpose()?;

        let update = match (self.data.read().stream.is_some(), stream.is_some()) {
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

        if self.data.read().stream == stream {
            return Ok(());
        }

        self.data.write().stream = stream;
        Ok(())
    }
}

/// Set up a stream information loop.
pub(crate) fn setup(
    streamer: api::TwitchAndUser,
    stream_state_tx: mpsc::Sender<StreamState>,
) -> (StreamInfo, impl Future<Output = Result<()>>) {
    let stream_info = StreamInfo {
        data: Default::default(),
    };

    let mut stream_interval = tokio::time::interval(time::Duration::from_secs(30));
    let mut subs_interval = tokio::time::interval(time::Duration::from_secs(60 * 10));

    let stream_info2 = stream_info.clone();

    let future = async move {
        loop {
            streamer.client.token.wait_until_ready().await?;

            tokio::select! {
                _ = subs_interval.tick() => {
                    if let Err(error) = stream_info.refresh_subs(&streamer).await {
                        log_error!(error, "Failed to refresh subscriptions");
                    }
                }
                _ = stream_interval.tick() => {
                    let stream = stream_info
                        .refresh_stream(&streamer, &stream_state_tx);
                    let channel = stream_info
                        .refresh_channel(&streamer);

                    let (stream, channel) = tokio::join!(stream, channel);

                    if let Err(error) = stream {
                        log_error!(error, "Failed to referesh stream");
                    }

                    if let Err(error) = channel {
                        log_error!(error, "Failed to referesh channel");
                    }
                }
            }
        }
    };

    (stream_info2, future.in_current_span())
}
