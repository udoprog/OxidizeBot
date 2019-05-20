use crate::utils::BoxFuture;
use failure::{format_err, Error};
use futures::{sync::mpsc, try_ready, Async, Future, Poll, Sink, Stream as _};
use std::sync::Arc;
use websocket::{client::r#async as c, r#async as a, ClientBuilder, OwnedMessage};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "request-type")]
pub enum Kind {
    #[serde(rename = "StopStreaming")]
    StopStreaming,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Request {
    #[serde(rename = "message-id")]
    message_id: uuid::Uuid,
    #[serde(flatten)]
    kind: Kind,
}

impl Request {
    /// Create a new request.
    pub fn new(kind: Kind) -> Request {
        Request {
            message_id: uuid::Uuid::new_v4(),
            kind,
        }
    }
}

pub struct Obs {
    tx: mpsc::UnboundedSender<Request>,
}

impl Obs {
    pub fn send(&self, request: Request) -> Result<(), failure::Error> {
        self.tx.unbounded_send(request)?;
        Ok(())
    }
}

/// Setup a websocket interface to OBS.
pub fn setup(_: Arc<Config>) -> Result<(Obs, ObsFuture), Error> {
    let (tx, rx) = mpsc::unbounded();

    let connect = ClientBuilder::new("ws://localhost:4444")?.async_connect_insecure();

    let future = ObsFuture {
        connect: Some(connect),
        connected: None,
        rx: Some(rx),
    };

    let obs = Obs { tx };

    Ok((obs, future))
}

pub struct ObsFuture {
    connect: Option<c::ClientNew<a::TcpStream>>,
    connected: Option<BoxFuture<(), failure::Error>>,
    rx: Option<mpsc::UnboundedReceiver<Request>>,
}

impl Future for ObsFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if let Some(connect) = self.connect.as_mut() {
                let (client, _) = try_ready!(connect.poll());
                let (client_tx, _) = client.split();
                self.connect = None;

                let rx = match self.rx.take() {
                    Some(rx) => rx,
                    None => failure::bail!("rx not available"),
                };

                let forward = rx
                    .map_err(|()| format_err!("error from receiver"))
                    .and_then::<_, Result<OwnedMessage, Error>>(|m| {
                        Ok(OwnedMessage::Text(serde_json::to_string(&m)?))
                    })
                    .from_err::<Error>()
                    .forward(client_tx.sink_map_err(|_| format_err!("error from sink")))
                    .map(|_| ());

                let forward: BoxFuture<(), failure::Error> = Box::new(forward);
                self.connected = Some(forward);
            }

            while let Some(connected) = self.connected.as_mut() {
                try_ready!(connected.poll());
            }

            return Ok(Async::NotReady);
        }
    }
}
