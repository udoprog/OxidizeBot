use crate::{injector, prelude::*, settings, timer};
use failure::{format_err, Error};
use std::time;
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

#[derive(Clone)]
pub struct Obs {
    tx: mpsc::UnboundedSender<Request>,
}

impl Obs {
    pub fn send(&self, request: Request) -> Result<(), failure::Error> {
        self.tx.unbounded_send(request)?;
        Ok(())
    }
}

/// Setup an OBS connection.
fn construct(
    injector: &injector::Injector,
    url: &str,
) -> Option<future::BoxFuture<'static, Result<(), Error>>> {
    let url = match str::parse(url) {
        Ok(url) => url,
        Err(e) => {
            injector.clear::<Obs>();
            log::warn!("bad url: {}: {}", url, e);
            return None;
        }
    };

    let (tx, rx) = mpsc::unbounded::<Request>();

    let connect = ClientBuilder::from_url(&url)
        .async_connect_insecure()
        .compat();

    let future = async move {
        let (client, _) = connect.await?;
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

    injector.update(Obs { tx });
    Some(future.boxed())
}

/// Setup a websocket interface to OBS.
pub fn setup<'a>(
    settings: &settings::Settings,
    injector: &'a injector::Injector,
) -> Result<impl Future<Output = Result<(), Error>> + 'a, Error> {
    let (mut url_stream, mut url) = settings.stream_opt::<String>("obs/url")?;

    let mut obs_stream = url.as_ref().and_then(|u| construct(injector, u));

    let future = async move {
        loop {
            futures::select! {
                update = url_stream.select_next_some() => {
                    url = update;
                    obs_stream = url.as_ref().and_then(|u| construct(injector, u));
                }
                result = obs_stream.current() => {
                    match result {
                        Ok(()) => log::warn!("obs stream ended"),
                        Err(e) => log::trace!("obs stream errored: {}", e),
                    }

                    timer::Delay::new(time::Instant::now() + time::Duration::from_secs(5)).await?;
                    obs_stream = url.as_ref().and_then(|u| construct(injector, u));
                }
            }
        }
    };

    Ok(future)
}
