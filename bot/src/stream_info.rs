use crate::{api, utils::BoxFuture};
use failure::format_err;
use futures::{future, try_ready, Async, Future, Poll, Stream as _};
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

struct Fetch {
    user: Option<api::twitch::User>,
    subs: Vec<api::twitch::Subscription>,
    stream: Option<api::twitch::Stream>,
    channel: api::twitch::Channel,
}

enum State {
    Interval,
    FetchInfo(BoxFuture<Fetch, failure::Error>),
}

/// Future associated with reloading stream information.
pub struct StreamInfoFuture {
    streamer: String,
    twitch: api::Twitch,
    interval: timer::Interval,
    state: State,
    info: Arc<RwLock<StreamInfo>>,
}

type SubscriptionsFuture = BoxFuture<Vec<api::twitch::Subscription>, failure::Error>;

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
                        let user = self.twitch.user_by_login(self.streamer.as_str()).and_then({
                            let twitch = self.twitch.clone();

                            move |user| {
                                let subs: SubscriptionsFuture = match user.as_ref() {
                                    Some(user) => Box::new(
                                        twitch
                                            .stream_subscriptions(&user.id, vec![])
                                            .concat2()
                                            .or_else(|e| {
                                                log_err!(e, "failed to get subscriptions");
                                                Ok(vec![])
                                            }),
                                    ),
                                    None => Box::new(future::ok(vec![])),
                                };

                                future::ok(user).join(subs)
                            }
                        });

                        let stream = self.twitch.stream_by_login(self.streamer.as_str());
                        let channel = self.twitch.channel_by_login(self.streamer.as_str());

                        let future =
                            user.join3(stream, channel)
                                .map(|((user, subs), stream, channel)| Fetch {
                                    user,
                                    subs,
                                    stream,
                                    channel,
                                });

                        self.state = State::FetchInfo(Box::new(future));
                    }
                },
                State::FetchInfo(ref mut future) => {
                    let Fetch {
                        user,
                        subs,
                        stream,
                        channel,
                    } = match future.poll() {
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
                    info.subs = subs;
                    info.subs_set = info
                        .subs
                        .iter()
                        .map(|s| s.user_name.to_lowercase())
                        .collect();

                    self.state = State::Interval;
                }
            }
        }
    }
}
