use crate::{prelude::*, timer};
use chrono::{DateTime, Utc};
use failure::Error;
use futures::compat::{Compat01As03, Compat01As03Sink};
use hashbrown::HashSet;
use std::{
    collections::VecDeque,
    fmt,
    time::{Duration, Instant},
};
use websocket::{
    client::r#async::TlsStream,
    message::OwnedMessage,
    r#async::{
        client::{Client, ClientNew},
        TcpStream,
    },
    ClientBuilder,
};

const URL: &'static str = "wss://pubsub-edge.twitch.tv";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Topic {
    ChannelBitsEventsV2 { channel_id: String },
}

impl Topic {
    /// Construct a new topic abstraction.
    pub fn channel_bits_events_v2(channel_id: String) -> Self {
        Topic::ChannelBitsEventsV2 { channel_id }
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Topic::ChannelBitsEventsV2 { channel_id } => {
                write!(fmt, "channel-bits-events-v2.{}", channel_id)
            }
        }
    }
}

type PubSubClient = Client<TlsStream<TcpStream>>;
type PubSubClientNew = Compat01As03<ClientNew<TlsStream<TcpStream>>>;
type PubSubClientSink = Compat01As03Sink<futures01::stream::SplitSink<PubSubClient>, OwnedMessage>;
type PubSubClientStream = Compat01As03<futures01::stream::SplitStream<PubSubClient>>;

#[derive(Debug, Clone, Copy)]
enum SendState {
    Idle,
    Sending,
}

pub struct PubSub {
    /// New client being constructed.
    client_new: Option<PubSubClientNew>,
    /// Connected client sink.
    client_sink: Option<PubSubClientSink>,
    /// Last message sent.
    send_state: SendState,
    /// Connect client stream.
    client_stream: Option<PubSubClientStream>,
    /// Builder to use.
    builder: ClientBuilder<'static>,
    /// Topics that we should be connected to.
    topics: HashSet<Topic>,
    /// Topics to connect to.
    connect: VecDeque<Topic>,
    /// Timeout until a ping is considered lost.
    ping_timeout: Option<timer::Delay>,
    /// Interval at which to send pings.
    ping_interval: timer::Interval,
}

impl Unpin for PubSub {}

impl PubSub {
    pub fn new() -> Result<Self, Error> {
        let builder = ClientBuilder::new(URL)?;
        let client_new = builder.clone().async_connect_secure(None);

        Ok(PubSub {
            client_new: Some(client_new.compat()),
            client_sink: None,
            client_stream: None,
            send_state: SendState::Idle,
            builder,
            topics: HashSet::new(),
            connect: VecDeque::new(),
            ping_timeout: None,
            ping_interval: timer::Interval::new_interval(Duration::from_secs(15)),
        })
    }

    /// Connect to the given topic.
    pub fn connect(&mut self, topic: Topic) {
        // Already connected to topic.
        if self.topics.contains(&topic) {
            return;
        }

        self.connect.push_back(topic);
    }

    /// Reconnect the websocket.
    fn reconnect(&mut self) {
        self.client_sink = None;
        self.client_stream = None;
        let client_new = self.builder.clone().async_connect_secure(None);
        self.client_new = Some(client_new.compat());
        self.send_state = SendState::Idle;
        self.ping_timeout = None;
    }

    /// Handle new connections.
    fn handle_connect<T>(&mut self, ctx: &mut Context) -> Result<Option<Poll<T>>, Error> {
        let client_new = match self.client_new.as_mut() {
            Some(client_new) => client_new,
            // Nothing connecting.
            None => return Ok(None),
        };

        let (client, _) = match Pin::new(client_new).poll(ctx) {
            Poll::Ready(result) => result?,
            Poll::Pending => return Ok(Some(Poll::Pending)),
        };

        let (sink, stream) = client.split();
        self.client_sink = Some(sink.sink_compat());
        self.client_stream = Some(stream.compat());
        self.client_new = None;
        Ok(None)
    }

    /// Handle sending things.
    fn handle_send(&mut self, ctx: &mut Context) -> Result<(), Error> {
        loop {
            match self.send_state {
                SendState::Idle => {
                    // NB: are we connected? if so, send ping.
                    if let Some(client_sink) = self.client_sink.as_mut() {
                        if !self.connect.is_empty() {
                            let mut topics = Vec::new();

                            while let Some(topic) = self.connect.pop_front() {
                                topics.push(topic.to_string());
                            }

                            let data = serde_json::to_vec(&ProtocolMessage::Listen {
                                nonce: None,
                                data: Listen {
                                    topics,
                                    auth_token: None,
                                },
                            })?;

                            Pin::new(client_sink).start_send(OwnedMessage::Binary(data))?;
                            self.send_state = SendState::Sending;
                            continue;
                        }

                        if let Poll::Ready(..) = Pin::new(&mut self.ping_interval).poll_next(ctx) {
                            // TODO: send ping.
                            let data = serde_json::to_vec(&ProtocolMessage::Ping)?;
                            Pin::new(client_sink).start_send(OwnedMessage::Binary(data))?;
                            self.send_state = SendState::Sending;
                            // NB: need to continue to call flush at least once.
                            continue;
                        }
                    }
                }
                SendState::Sending => {
                    if let Some(client_sink) = self.client_sink.as_mut() {
                        if let Poll::Ready(result) = Pin::new(client_sink).poll_flush(ctx) {
                            if let Err(e) = result {
                                log::warn!("error in connection: {}", e);
                                self.reconnect();
                                return Ok(());
                            }

                            self.ping_timeout =
                                Some(timer::Delay::new(Instant::now() + Duration::from_secs(15)));
                            self.send_state = SendState::Idle;
                        }
                    }
                }
            }

            return Ok(());
        }
    }

    fn handle_receive(&mut self, ctx: &mut Context) -> Result<Option<Message>, Error> {
        let message = match self.client_stream.as_mut() {
            Some(client_stream) => match Pin::new(client_stream).poll_next(ctx) {
                Poll::Ready(Some(message)) => message,
                _ => return Ok(None),
            },
            None => return Ok(None),
        };

        let message = match message {
            Ok(message) => message,
            Err(e) => {
                log::warn!("error in connection: {}", e);
                self.reconnect();
                return Ok(None);
            }
        };

        let message = match &message {
            OwnedMessage::Text(ref text) => text.as_bytes(),
            OwnedMessage::Binary(ref binary) => binary,
            _ => return Ok(None),
        };

        let message = serde_json::from_slice::<ProtocolMessage>(message)?;

        match message {
            ProtocolMessage::Pong => {
                self.ping_timeout = None;
            }
            // Remote end informs us that we need to reconnect.
            ProtocolMessage::Reconnect => {
                self.reconnect();
                return Ok(None);
            }
            ProtocolMessage::Message { topic, message } => {
                log::trace!("{}: {}", topic, message);
                // TODO: decode and return message.
            }
            m => log::warn!("unexpected message: {:?}", m),
        }

        Ok(None)
    }

    fn handle_ping_timeout(&mut self, ctx: &mut Context) {
        // Test if we've encountered a ping timeout.
        if let Some(ping_timeout) = self.ping_timeout.as_mut() {
            if let Poll::Ready(..) = Pin::new(ping_timeout).poll(ctx) {
                // Reconnect on ping timeout.
                self.reconnect();
            }
        }
    }
}

impl Stream for PubSub {
    type Item = Result<Message, Error>;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut s = self.as_mut();

        loop {
            if let Some(poll) = s.handle_connect(ctx)? {
                return poll;
            }

            s.handle_send(ctx)?;
            s.handle_ping_timeout(ctx);

            if let Some(value) = s.handle_receive(ctx)? {
                return Poll::Ready(Some(Ok(value)));
            }

            // A reconnect was triggered and needs to be polled.
            if let Some(poll) = s.handle_connect(ctx)? {
                return poll;
            }

            return Poll::Pending;
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Listen {
    topics: Vec<String>,
    #[serde(default)]
    auth_token: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BitsEventContext {
    #[serde(rename = "cheer")]
    Cheer,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BadgeEntitlement {
    new_version: u64,
    previous_version: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BitsEventData {
    #[serde(default)]
    user_name: Option<String>,
    channel_name: String,
    #[serde(default)]
    user_id: Option<String>,
    channel_id: String,
    time: DateTime<Utc>,
    chat_message: String,
    bits_used: u64,
    total_bits_used: u64,
    context: BitsEventContext,
    #[serde(default)]
    badge_entitlement: Option<BadgeEntitlement>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "message_type")]
pub enum Message {
    #[serde(rename = "bits_event")]
    BitsEvent {
        data: BitsEventData,
        version: String,
        message_id: String,
        is_anonymous: bool,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolMessage {
    #[serde(rename = "PING")]
    Ping,
    #[serde(rename = "PONG")]
    Pong,
    #[serde(rename = "RECONNECT")]
    Reconnect,
    #[serde(rename = "MESSAGE")]
    Message { topic: String, message: String },
    #[serde(rename = "LISTEN")]
    Listen { nonce: Option<String>, data: Listen },
    #[serde(rename = "UNLISTEN")]
    Unlisten { nonce: Option<String> },
}
