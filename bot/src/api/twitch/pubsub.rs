use crate::api;
use crate::injector::{Injector, Key};
use crate::prelude::BoxStream;
use crate::tags;
use anyhow::{bail, Result};
use async_fuse::Fuse;
use backoff::backoff::Backoff as _;
use serde::Deserialize as _;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::{self, Interval, Sleep};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub use self::model::*;

const URL: &str = "wss://pubsub-edge.twitch.tv";

/// Websocket pub/sub integration for twitch.
#[derive(Clone)]
pub struct TwitchPubSub {
    inner: Arc<Inner>,
}

impl TwitchPubSub {
    /// Subscribe for redemptions.
    pub fn redemptions(&self) -> TwitchStream<Redemption> {
        use tokio::sync::broadcast::error::RecvError;

        let mut s = self.inner.redemptions.subscribe();

        TwitchStream {
            stream: Box::pin(async_stream::stream! {
                loop {
                    match s.recv().await {
                        Ok(item) => yield item,
                        Err(RecvError::Closed) => break,
                        Err(RecvError::Lagged(..)) => (),
                    }
                }
            }),
        }
    }
}

pub struct TwitchStream<T> {
    stream: BoxStream<'static, T>,
}

impl<T> crate::stream::Stream for TwitchStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}

struct Client {
    stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

impl Client {
    /// Send a message.
    async fn send(&mut self, frame: self::transport::Frame) -> Result<()> {
        use futures_util::SinkExt as _;

        let text = serde_json::to_string(&frame)?;
        log::trace!(">> {:?}", frame);
        let message = tungstenite::Message::Text(text);
        self.stream.send(message).await?;
        Ok(())
    }

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<String>>> {
        use crate::stream::Stream as _;

        loop {
            let message = match Pin::new(&mut self.as_mut().stream).poll_next(cx)? {
                Poll::Ready(message) => message,
                Poll::Pending => return Poll::Pending,
            };

            let message = match message {
                Some(message) => message,
                None => return Poll::Ready(None),
            };

            let text = match message {
                tungstenite::Message::Text(text) => text,
                tungstenite::Message::Close(..) => return Poll::Ready(None),
                message => {
                    log::warn!("Unhandled websocket message: {:?}", message);
                    continue;
                }
            };

            return Poll::Ready(Some(Ok(text)));
        }
    }
}

/// Connect to the pub/sub websocket once available.
pub fn connect(
    settings: &crate::Settings,
    injector: &Injector,
) -> impl Future<Output = Result<()>> {
    task(settings.clone(), injector.clone())
}

struct State {
    enabled: bool,
    ws: TwitchPubSub,
    client: Fuse<Client>,
    streamer: Option<api::TwitchAndUser>,
    ping_interval: Fuse<Interval>,
    pong_deadline: Fuse<Pin<Box<Sleep>>>,
    reconnect: Fuse<Pin<Box<Sleep>>>,
    reconnect_backoff: backoff::ExponentialBackoff,
}

impl State {
    /// Disconnect and clear client (if connected).
    async fn disconnect(&mut self) {
        if let Some(client) = self.client.as_inner_mut() {
            if let Err(e) = client.stream.close(None).await {
                log_error!(e, "error when closing stream");
            }

            log::info!("Disconnected from Twitch Pub/Sub!");
        }

        self.client.clear();
    }

    /// Clear state.
    async fn clear(&mut self) {
        self.disconnect().await;
        self.ping_interval.clear();
        self.pong_deadline.clear();
        self.reconnect.clear();
    }

    /// An error happened, try to automatically recover the connection.
    async fn recover(&mut self) {
        self.clear().await;

        // NB: if still enabled, set a reconnect.
        if self.enabled {
            log::info!("Attempting to reconnect");
            let backoff = self.reconnect_backoff.next_backoff().unwrap_or_default();
            log::warn!("reconnecting in {:?}", backoff);
            self.reconnect.set(Box::pin(time::sleep(backoff)));
        }
    }

    fn set_pong_deadline(&mut self) {
        self.pong_deadline
            .set(Box::pin(time::sleep(Duration::from_secs(10))));
    }

    // Rebuild state from the current configuration.
    async fn build(&mut self) {
        let streamer = match (self.enabled, self.streamer.as_ref()) {
            (true, Some(streamer)) => streamer,
            _ => {
                self.clear().await;
                return;
            }
        };

        return match try_build_client(streamer).await {
            Ok(client) => {
                self.ping_interval
                    .set(time::interval(Duration::from_secs(60)));
                self.reconnect_backoff.reset();
                self.client.set(client);
            }
            Err(e) => {
                log_error!(e, "failed to build pub/sub client");
                self.recover().await;
            }
        };

        async fn try_build_client(streamer: &api::TwitchAndUser) -> Result<Client> {
            use tungstenite::handshake::client::Request;
            use tungstenite::http::Uri;

            log::trace!("Connecting to Twitch Pub/Sub");

            let auth_token = streamer.client.token.read().await?.access_token.clone();

            let uri = str::parse::<Uri>(URL)?;
            let req = Request::get(uri).body(())?;
            let (stream, _) = tokio_tungstenite::connect_async(req).await?;

            let mut client = Client { stream };

            let data = self::transport::Data::with_nonce(
                self::transport::Listen {
                    topics: vec![
                        format!("channel-points-channel-v1.{}", streamer.user.id),
                        format!("channel-bits-events-v2.{}", streamer.user.id),
                        format!("channel-subscribe-events-v1.{}", streamer.user.id),
                    ],
                    auth_token: self::transport::SecretString(auth_token),
                },
                String::from("initialize"),
            );

            client.send(self::transport::Frame::Listen(data)).await?;
            Ok(client)
        }
    }

    fn deserialize_frame(text: &str) -> Result<self::transport::Frame> {
        let value = serde_json::from_str::<serde_json::Value>(&text)?;
        let frame = self::transport::Frame::deserialize(&value)?;
        Ok(frame)
    }

    /// Handle an incoming message as a frame.
    async fn handle_frame(&mut self, text: &str) -> Result<()> {
        let frame = match Self::deserialize_frame(text) {
            Ok(frame) => frame,
            Err(e) => {
                log::trace!("<< raw: {}", text);
                return Err(e);
            }
        };

        log::trace!("<< {:?}", frame);

        match frame {
            self::transport::Frame::Response(response) => {
                if let Some(error) = response.error {
                    log::warn!("got error `{}`, disconnecting", error);
                    self.recover().await;
                } else {
                    if response.nonce.as_deref() == Some("initialize") {
                        log::info!("Connected to Twitch Pub/Sub!");
                    }
                }
            }
            self::transport::Frame::Pong => {
                self.pong_deadline.clear();
            }
            self::transport::Frame::Message(message) => {
                let m: Message = serde_json::from_str(&message.data.message)?;
                self.handle_message(m).await?;
            }
            self::transport::Frame::Unknonwn => {
                bail!("Unsupported payload: {:?}", text);
            }
            other => {
                bail!("Unsupported frame: {:?}", other);
            }
        }

        Ok(())
    }

    /// Handle an incoming message.
    async fn handle_message(&mut self, m: Message) -> Result<()> {
        match m {
            Message::RewardRedeemed(redeemed) => {
                let _ = self.ws.inner.redemptions.send(redeemed.data.redemption);
            }
        }

        Ok(())
    }
}

async fn task(settings: crate::Settings, injector: Injector) -> Result<()> {
    let settings = settings.scoped("pubsub");

    let (mut enabled_stream, enabled) = settings.stream::<bool>("enabled").or_default().await?;

    let inner = Arc::new(Inner {
        redemptions: tokio::sync::broadcast::channel(1024).0,
    });

    let ws = TwitchPubSub {
        inner: inner.clone(),
    };

    injector.update(ws.clone()).await;

    let streamer_key = Key::<api::TwitchAndUser>::tagged(tags::Twitch::Streamer)?;
    let (mut streamer_stream, streamer) = injector.stream_key(&streamer_key).await;

    let mut state = State {
        enabled,
        ws,
        client: Fuse::empty(),
        streamer,
        ping_interval: Fuse::empty(),
        pong_deadline: Fuse::empty(),
        reconnect: Fuse::empty(),
        reconnect_backoff: {
            let mut backoff = backoff::ExponentialBackoff::default();
            backoff.current_interval = Duration::from_secs(5);
            backoff.initial_interval = Duration::from_secs(5);
            backoff.max_elapsed_time = None;
            backoff
        },
    };

    state.build().await;

    loop {
        tokio::select! {
            message = state.client.as_pin_mut().poll_stream(Client::poll_next) => {
                let message = match message {
                    Some(message) => match message {
                        Ok(message) => message,
                        Err(e) => {
                            log_error!(e, "error in websocket");
                            state.recover().await;
                            continue;
                        }
                    }
                    None => {
                        log::error!("end of websocket stream");
                        state.recover().await;
                        continue;
                    },
                };

                if let Err(e) = state.handle_frame(&message).await {
                    log_error!(e, "failed to handle message");
                }
            }
            enabled = enabled_stream.recv() => {
                state.enabled = enabled;
                state.build().await;
            }
            streamer = streamer_stream.recv() => {
                state.streamer = streamer;
                state.build().await;
            }
            _ = state.ping_interval.as_pin_mut().poll_inner(|mut i, cx| i.poll_tick(cx)) => {
                if let Some(client) = state.client.as_inner_mut() {
                    client.send(self::transport::Frame::Ping).await?;
                    state.set_pong_deadline();
                } else {
                    state.clear().await;
                }
            }
            _ = &mut state.reconnect => {
                state.build().await;
            }
            _ = &mut state.pong_deadline => {
                log::warn!("Did not receive pong in time!");
                state.recover().await;
            }
        }
    }
}

struct Inner {
    redemptions: broadcast::Sender<Redemption>,
}

pub(crate) mod transport {
    use serde::{Deserialize, Serialize};
    use std::fmt;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub(crate) enum Frame {
        #[serde(rename = "PING")]
        Ping,
        #[serde(rename = "PONG")]
        Pong,
        #[serde(rename = "LISTEN")]
        Listen(Data<Listen>),
        #[serde(rename = "RESPONSE")]
        Response(Response),
        #[serde(rename = "MESSAGE")]
        Message(Data<Message>),
        #[serde(other)]
        Unknonwn,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) struct Response {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "empty_string"
        )]
        pub(crate) nonce: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "empty_string"
        )]
        pub(crate) error: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) struct Data<T> {
        #[serde(default)]
        pub(crate) nonce: Option<String>,
        pub(crate) data: T,
    }

    impl<T> Data<T> {
        /// Construct a data with nonce.
        pub(crate) fn with_nonce(data: T, nonce: String) -> Self {
            Self {
                nonce: Some(nonce),
                data,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) struct Listen {
        pub(crate) topics: Vec<String>,
        pub(crate) auth_token: SecretString,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) struct Message {
        pub(crate) topic: String,
        pub(crate) message: String,
    }

    /// Deserializes an empty string as `None`.
    fn empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        Ok(match <Option<String>>::deserialize(deserializer)? {
            Some(string) if !string.is_empty() => Some(string),
            _ => None,
        })
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub(crate) struct SecretString(pub(crate) String);

    impl fmt::Debug for SecretString {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "*redacted*")
        }
    }
}

mod model {
    use crate::api::twitch::Data;
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum Message {
        #[serde(rename = "reward-redeemed")]
        RewardRedeemed(Data<RewardRedeemed>),
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct User {
        pub id: String,
        pub login: String,
        pub display_name: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Image {
        pub url_1x: String,
        pub url_2x: String,
        pub url_4x: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Reward {
        pub id: String,
        pub channel_id: String,
        pub title: String,
        pub prompt: String,
        pub cost: i64,
        pub is_user_input_required: bool,
        pub is_sub_only: bool,
        #[serde(default)]
        pub image: Option<Image>,
        pub default_image: Image,
        pub background_color: String,
        pub is_enabled: bool,
        pub is_paused: bool,
        pub is_in_stock: bool,
        pub max_per_stream: MaxPerStream,
        pub should_redemptions_skip_request_queue: bool,
        #[serde(default)]
        pub template_id: Option<serde_json::Value>,
        pub updated_for_indicator_at: DateTime<Utc>,
        pub max_per_user_per_stream: MaxPerUserPerStream,
        pub global_cooldown: GlobalCooldown,
        #[serde(default)]
        pub redemptions_redeemed_current_stream: Option<serde_json::Value>,
        pub cooldown_expires_at: Option<serde_json::Value>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Redemption {
        pub id: String,
        pub user: User,
        pub channel_id: String,
        pub redeemed_at: DateTime<Utc>,
        pub reward: Reward,
        #[serde(default)]
        pub user_input: Option<String>,
        pub status: Status,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub enum Status {
        #[serde(rename = "FULFILLED")]
        Fulfilled,
        #[serde(rename = "UNFULFILLED")]
        Unfulfilled,
        #[serde(rename = "CANCELED")]
        Canceled,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MaxPerStream {
        pub is_enabled: bool,
        pub max_per_stream: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MaxPerUserPerStream {
        pub is_enabled: bool,
        pub max_per_user_per_stream: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GlobalCooldown {
        pub is_enabled: bool,
        pub global_cooldown_seconds: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RewardRedeemed {
        pub timestamp: String,
        pub redemption: Redemption,
    }
}
