use crate::{api, utils::BoxFuture};
use failure::format_err;
use futures::{try_ready, Async, Future, Poll, Stream as _};
use parking_lot::RwLock;
use std::{sync::Arc, time};
use tokio::timer;

#[derive(Debug, Default)]
pub struct StreamInfo {
    pub stream: Option<api::twitch::Stream>,
    pub user: Option<api::twitch::User>,
    pub title: Option<String>,
    pub game: Option<String>,
}

/// Set up a reward loop.
pub fn setup(
    streamer: &str,
    interval: time::Duration,
    twitch: api::Twitch,
) -> (Arc<RwLock<StreamInfo>>, StreamInfoFuture) {
    let info = Arc::new(RwLock::new(StreamInfo::default()));

    let future = StreamInfoFuture {
        streamer: streamer.to_string(),
        twitch,
        interval: timer::Interval::new(time::Instant::now(), interval),
        state: State::Interval,
        info: info.clone(),
    };

    (info, future)
}

enum State {
    Interval,
    FetchInfo(
        BoxFuture<
            (
                Option<api::twitch::User>,
                Option<api::twitch::Stream>,
                api::twitch::Channel,
            ),
            failure::Error,
        >,
    ),
}

/// Future associated with reloading stream information.
pub struct StreamInfoFuture {
    streamer: String,
    twitch: api::Twitch,
    interval: timer::Interval,
    state: State,
    info: Arc<RwLock<StreamInfo>>,
}

impl Future for StreamInfoFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<(), failure::Error> {
        loop {
            match self.state {
                State::Interval => match try_ready!(self
                    .interval
                    .poll()
                    .map_err(|_| format_err!("failed to poll interval")))
                {
                    None => failure::bail!("interval stream ended"),
                    Some(_) => {
                        let user = self.twitch.user_by_login(self.streamer.as_str());
                        let stream = self.twitch.stream_by_login(self.streamer.as_str());
                        let channel = self.twitch.channel_by_login(self.streamer.as_str());
                        self.state = State::FetchInfo(Box::new(user.join3(stream, channel)));
                    }
                },
                State::FetchInfo(ref mut future) => {
                    let (user, stream, channel) = match future.poll() {
                        Ok(Async::Ready(v)) => v,
                        Ok(Async::NotReady) => return Ok(Async::NotReady),
                        Err(e) => {
                            log::error!("failed to refresh stream info: {}", e);
                            self.state = State::Interval;
                            continue;
                        }
                    };

                    let mut info = self.info.write();
                    info.user = user;
                    info.stream = stream;
                    info.title = Some(channel.status);
                    info.game = channel.game;

                    self.state = State::Interval;
                }
            }
        }
    }
}
