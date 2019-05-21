use crate::prelude::*;
use failure::{format_err, Error};
use std::sync::Arc;
use websocket::{ClientBuilder, OwnedMessage};

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
pub fn setup(_: Arc<Config>) -> Result<(Obs, impl Future<Output = Result<(), Error>>), Error> {
    let (tx, rx) = mpsc::unbounded::<Request>();

    let connect = ClientBuilder::new("ws://localhost:4444")?.async_connect_insecure();

    let future = async move {
        let (client, _) = connect.compat().await?;
        let (client_tx, _) = client.split();

        let forward = rx
            .map(|v| Ok(v))
            .compat()
            .and_then::<_, Result<OwnedMessage, Error>>(|m| {
                Ok(OwnedMessage::Text(serde_json::to_string(&m)?))
            })
            .from_err::<Error>()
            .forward(client_tx.sink_map_err(|_| format_err!("error from sink")))
            .map(|_| ());

        forward.compat().await?;
        Ok(())
    };

    let obs = Obs { tx };
    Ok((obs, future))
}
