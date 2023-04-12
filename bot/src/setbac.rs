use std::future::Future;

use anyhow::Result;
use api::setbac::PlayerUpdate;
use api::Setbac;
use async_injector::{Injector, Key};
use common::tags;
use tracing::Instrument;
use url::Url;

const DEFAULT_API_URL: &str = "https://setbac.tv";

/// Run update loop shipping information to the remote server.
#[tracing::instrument(skip_all)]
pub(crate) async fn run<S>(
    settings: &settings::Settings<S>,
    injector: &Injector,
    global_bus: bus::Bus<bus::Global>,
) -> Result<impl Future<Output = Result<()>>>
where
    S: settings::Scope,
{
    let settings = settings.scoped("remote");

    let (mut api_url_stream, api_url) = settings
        .stream("api-url")
        .or(Some(String::from(DEFAULT_API_URL)))
        .optional()
        .await?;

    let (mut secret_key_stream, secret_key) = settings.stream("secret-key").optional().await?;
    let (mut enabled_stream, enabled) = settings.stream("enabled").or_with(false).await?;
    let (mut player_stream, player) = injector.stream::<player::Player>().await;
    let (mut streamer_token_stream, streamer_token) = injector
        .stream_key(Key::<api::Token>::tagged(tags::Token::Twitch(
            tags::Twitch::Streamer,
        ))?)
        .await;

    let mut builder = Builder {
        injector: injector.clone(),
        enabled,
        streamer_token,
        global_bus,
        player,
        secret_key,
        api_url: None,
        rx: None,
        setbac: None,
    };

    builder.api_url = api_url.and_then(|s| parse_url(&s));
    builder.update().await;

    let future = async move {
        loop {
            tokio::select! {
                update = streamer_token_stream.recv() => {
                    tracing::info!("Received new streamer token");
                    builder.streamer_token = update;
                    builder.update().await;
                }
                secret_key = secret_key_stream.recv() => {
                    tracing::info!("Received new secret key");
                    builder.secret_key = secret_key;
                    builder.update().await;
                }
                update = player_stream.recv() => {
                    builder.player = update;
                    builder.update().await;
                }
                api_url = api_url_stream.recv() => {
                    builder.api_url = api_url.and_then(|s| parse_url(&s));
                    builder.update().await;
                }
                enabled = enabled_stream.recv() => {
                    builder.enabled = enabled;
                    builder.update().await;
                }
                // TODO: Change to use async-fuse.
                event = async { builder.rx.as_mut().unwrap().recv().await }, if builder.rx.is_some() => {
                    let event = event?;

                    // Only update on switches to current song and all necessary
                    // components are available.
                    let (bus::Global::SongModified, Some(setbac), Some(player)) = (event, &builder.setbac, &builder.player) else {
                        continue;
                    };

                    tracing::trace!("Pushing remote player update");

                    let mut update = PlayerUpdate::default();
                    update.current = player.current().await.map(|c| c.item().as_ref().into());

                    for i in player.list().await {
                        update.items.push(i.as_ref().into());
                    }

                    if let Err(error) = setbac.player_update(update).await {
                        common::log_error!(error, "Failed to perform remote player update");
                    }
                }
            }
        }
    };

    Ok(future.in_current_span())
}

struct Builder {
    injector: Injector,
    enabled: bool,
    streamer_token: Option<api::Token>,
    global_bus: bus::Bus<bus::Global>,
    player: Option<player::Player>,
    api_url: Option<Url>,
    secret_key: Option<String>,
    rx: Option<bus::Reader<bus::Global>>,
    setbac: Option<Setbac>,
}

impl Builder {
    async fn update(&mut self) {
        if self.enabled {
            self.rx = Some(self.global_bus.subscribe());
        } else {
            self.rx = None;
        }

        self.setbac = match self.api_url.as_ref() {
            Some(api_url) => {
                let setbac = Setbac::new(
                    crate::USER_AGENT,
                    self.streamer_token.clone(),
                    self.secret_key.clone(),
                    api_url.clone(),
                );

                self.injector.update(setbac.clone()).await;
                Some(setbac)
            }
            None => {
                self.injector.clear::<Setbac>().await;
                None
            }
        };
    }
}

fn parse_url(url: &str) -> Option<Url> {
    match str::parse(url) {
        Ok(api_url) => Some(api_url),
        Err(e) => {
            common::log_warn!(e, "Bad api url: {}", url);
            None
        }
    }
}
