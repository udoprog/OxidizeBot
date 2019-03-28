use failure::format_err;
use futures::{Async, Future, Poll, Stream};
use parking_lot::Mutex;
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{
    io::{self, AsyncRead, WriteHalf},
    net::{TcpListener, TcpStream},
};
use tokio_bus::{Bus, BusReader};

/// Notifier system.
pub struct Notifier {
    bus: Mutex<Bus<Notification>>,
    address: SocketAddr,
}

impl Notifier {
    /// Create a new notifier.
    pub fn new() -> Self {
        Notifier {
            bus: Mutex::new(Bus::new(1024)),
            address: SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 4444),
        }
    }

    /// Send a message to the bus.
    pub fn send(&self, n: Notification) -> Result<(), failure::Error> {
        let mut bus = self.bus.lock();

        if let Err(_) = bus.try_broadcast(n) {
            failure::bail!("bus is full");
        }

        Ok(())
    }

    /// Listen for incoming connections and handle bus messages to connected sockets.
    pub fn listen(
        self: Arc<Self>,
    ) -> Result<impl Future<Item = (), Error = failure::Error>, failure::Error> {
        let listener = TcpListener::bind(&self.address)?;

        Ok(listener
            .incoming()
            .map_err(failure::Error::from)
            .and_then(move |s| {
                let (_, writer) = s.split();
                let rx = self.bus.lock().add_rx();

                let handler = BusHandler::new(writer, rx)
                    .map_err(|e| {
                        log::error!("failed to process outgoing message: {}", e);
                    })
                    .for_each(|_| Ok(()));

                tokio::spawn(handler);
                Ok(())
            })
            .for_each(|_| Ok(())))
    }
}

enum BusHandlerState {
    Receiving,
    Serialize(Notification),
    Send(io::WriteAll<WriteHalf<TcpStream>, String>),
}

/// Handles reading messages of a buss and writing them to a TcpStream.
struct BusHandler {
    writer: Option<WriteHalf<TcpStream>>,
    rx: BusReader<Notification>,
    state: BusHandlerState,
}

impl BusHandler {
    pub fn new(writer: WriteHalf<TcpStream>, rx: BusReader<Notification>) -> Self {
        Self {
            writer: Some(writer),
            rx,
            state: BusHandlerState::Receiving,
        }
    }
}

impl Stream for BusHandler {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        use self::BusHandlerState::*;

        loop {
            self.state = match self.state {
                Receiving => match self.rx.poll() {
                    Ok(Async::Ready(Some(m))) => Serialize(m),
                    Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(failure::Error::from(e)),
                },
                Serialize(ref m) => match (serde_json::to_string(m), self.writer.take()) {
                    (Ok(json), Some(writer)) => Send(io::write_all(writer, format!("{}\n", json))),
                    (_, None) => return Err(format_err!("writer not available")),
                    (Err(e), _) => return Err(failure::Error::from(e)),
                },
                Send(ref mut f) => match f.poll() {
                    Ok(Async::Ready((writer, _))) => {
                        self.writer = Some(writer);
                        self.state = Receiving;
                        continue;
                    }
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(failure::Error::from(e)),
                },
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum Notification {
    #[serde(rename = "firework")]
    Firework,
    #[serde(rename = "ping")]
    Ping,
}
