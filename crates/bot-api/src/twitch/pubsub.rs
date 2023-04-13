use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::{bail, Result};
use async_fuse::Fuse;
use async_injector::{Injector, Key};
use backoff::backoff::Backoff;
use chrono::{DateTime, Utc};
use common::sink::SinkExt;
use common::stream::Stream;
use common::{tags, BoxStream};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::time::{self, Interval, Sleep};
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::Uri;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::Instrument;

use crate::twitch::Data;

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

impl<T> Stream for TwitchStream<T> {
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
        let text = serde_json::to_string(&frame)?;
        tracing::trace!(">> {:?}", frame);
        let message = tungstenite::Message::Text(text);
        self.stream.send(message).await?;
        Ok(())
    }

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<String>>> {
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
                    tracing::warn!("Unhandled websocket message: {:?}", message);
                    continue;
                }
            };

            return Poll::Ready(Some(Ok(text)));
        }
    }
}

/// Connect to the pub/sub websocket once available.
#[tracing::instrument(skip_all)]
pub fn connect<S>(
    settings: &settings::Settings<S>,
    injector: &Injector,
) -> impl Future<Output = Result<()>>
where
    S: settings::Scope,
{
    task(settings.clone(), injector.clone()).in_current_span()
}

struct State {
    enabled: bool,
    ws: TwitchPubSub,
    client: Fuse<Client>,
    streamer: Option<crate::TwitchAndUser>,
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
                common::log_error!(e, "Error when closing stream");
            }

            tracing::info!("Disconnected from Twitch Pub/Sub!");
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
            tracing::info!("Attempting to reconnect");
            let backoff = self.reconnect_backoff.next_backoff().unwrap_or_default();
            tracing::warn!("Reconnecting in {:?}", backoff);
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
                common::log_error!(e, "Failed to build pub/sub client");
                self.recover().await;
            }
        };

        async fn try_build_client(streamer: &crate::TwitchAndUser) -> Result<Client> {
            tracing::trace!("Connecting to Twitch Pub/Sub");

            let auth_token = streamer.client.token.read().map(|(t, _)| t);

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
                    auth_token,
                },
                String::from("initialize"),
            );

            client.send(self::transport::Frame::Listen(data)).await?;
            Ok(client)
        }
    }

    fn deserialize_frame(text: &str) -> Result<self::transport::Frame> {
        let value = serde_json::from_str::<serde_json::Value>(text)?;
        let frame = self::transport::Frame::deserialize(&value)?;
        Ok(frame)
    }

    /// Handle an incoming message as a frame.
    async fn handle_frame(&mut self, text: &str) -> Result<()> {
        let frame = match Self::deserialize_frame(text) {
            Ok(frame) => frame,
            Err(e) => {
                tracing::trace!("<< raw: {}", text);
                return Err(e);
            }
        };

        tracing::trace!("<< {:?}", frame);

        match frame {
            self::transport::Frame::Response(response) => {
                if let Some(error) = response.error {
                    tracing::warn!("Got error `{}`, disconnecting", error);
                    self.recover().await;
                } else if response.nonce.as_deref() == Some("initialize") {
                    tracing::info!("Connected to Twitch Pub/Sub!");
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

async fn task<S>(settings: settings::Settings<S>, injector: Injector) -> Result<()>
where
    S: settings::Scope,
{
    let settings = settings.scoped("pubsub");

    let (mut enabled_stream, enabled) = settings.stream::<bool>("enabled").or_default().await?;

    let inner = Arc::new(Inner {
        redemptions: tokio::sync::broadcast::channel(1024).0,
    });

    let ws = TwitchPubSub {
        inner: inner.clone(),
    };

    injector.update(ws.clone()).await;

    let streamer_key = Key::<crate::TwitchAndUser>::tagged(tags::Twitch::Streamer)?;
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
                            common::log_error!(e, "Error in websocket");
                            state.recover().await;
                            continue;
                        }
                    }
                    None => {
                        tracing::error!("End of websocket stream");
                        state.recover().await;
                        continue;
                    },
                };

                if let Err(e) = state.handle_frame(&message).await {
                    common::log_error!(e, "Failed to handle message");
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
                tracing::warn!("Did not receive pong in time!");
                state.recover().await;
            }
        }
    }
}

struct Inner {
    redemptions: broadcast::Sender<Redemption>,
}

pub mod transport {
    use serde::{Deserialize, Serialize};

    use crate::token::TokenPayload;

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum Frame {
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

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Response {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "empty_string"
        )]
        pub nonce: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "empty_string"
        )]
        pub error: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Data<T> {
        #[serde(default)]
        pub nonce: Option<String>,
        pub data: T,
    }

    impl<T> Data<T> {
        /// Construct a data with nonce.
        pub fn with_nonce(data: T, nonce: String) -> Self {
            Self {
                nonce: Some(nonce),
                data,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Listen {
        pub topics: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub auth_token: Option<TokenPayload>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Message {
        pub topic: String,
        pub message: String,
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
}

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
