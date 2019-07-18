use crate::{prelude::*, timer};
use chrono::{DateTime, Utc};
use failure::Error;
use futures::{
    compat::{Compat01As03, Compat01As03Sink},
    ready,
};
use hashbrown::HashSet;
use std::{collections::VecDeque, fmt, time::Duration};
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

pub struct PubSub {
    /// New client being constructed.
    client_new: Option<Compat01As03<ClientNew<TlsStream<TcpStream>>>>,
    /// Connected client sink.
    client_sink: Option<Compat01As03Sink<futures01::stream::SplitSink<PubSubClient>, OwnedMessage>>,
    /// Connect client stream.
    client_stream: Option<Compat01As03<futures01::stream::SplitStream<PubSubClient>>>,
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
}

impl Stream for PubSub {
    type Item = Result<Message, Error>;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut s = self.as_mut();

        loop {
            if let Poll::Ready(..) = Pin::new(&mut s.ping_interval).poll_next(ctx) {
                // NB: are we connected? if so, send ping.
                if let Some(client_sink) = s.client_sink.take() {
                    // TODO: send ping.
                    log::trace!("Sending ping");
                    s.client_sink = Some(client_sink);
                }
            }

            if let Some(ping_timeout) = s.ping_timeout.as_mut() {
                if let Poll::Ready(..) = Pin::new(ping_timeout).poll(ctx) {
                    let client_new = s.builder.clone().async_connect_secure(None);
                    s.client_new = Some(client_new.compat());
                    s.ping_timeout = None;
                }
            }

            if let Some(client_new) = s.client_new.as_mut() {
                let (client, _) = ready!(Pin::new(client_new).poll(ctx))?;
                let (sink, stream) = client.split();
                s.client_sink = Some(sink.sink_compat());
                s.client_stream = Some(stream.compat());
                s.client_new = None;
            }

            if let Some(stream) = s.client_stream.as_mut() {
                let message = ready!(Pin::new(stream).poll_next(ctx));

                let message = match message {
                    Some(message) => match message {
                        Ok(message) => message,
                        Err(e) => {
                            log::warn!("error in connection: {}", e);
                            s.client_sink = None;
                            s.client_stream = None;
                            let client_new = s.builder.clone().async_connect_secure(None);
                            s.client_new = Some(client_new.compat());
                            continue;
                        }
                    },
                    None => return Poll::Ready(None),
                };

                let message = match &message {
                    OwnedMessage::Text(ref text) => text.as_bytes(),
                    OwnedMessage::Binary(ref binary) => binary,
                    OwnedMessage::Ping(..) => continue,
                    OwnedMessage::Pong(..) => continue,
                    OwnedMessage::Close(..) => continue,
                };

                let message = serde_json::from_slice::<ProtocolMessage>(message)?;

                match message {
                    ProtocolMessage::Pong => {
                        s.ping_timeout = None;
                    }
                    ProtocolMessage::Reconnect => {
                        let client_new = s.builder.clone().async_connect_secure(None);
                        s.client_new = Some(client_new.compat());
                        continue;
                    }
                    ProtocolMessage::Message { topic, message } => {
                        log::trace!("{}: {}", topic, message);
                    }
                    m => log::warn!("unexpected message: {:?}", m),
                }
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Listen {
    topics: Vec<String>,
    auth_token: String,
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
