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
    let (mut player_stream, player) = injector.stream::<Player>().await;
    let (mut streamer_token_stream, streamer_token) = injector
        .stream_key(Key::<crate::token::Token>::tagged(tags::Token::Twitch(
            tags::Twitch::Streamer,
        ))?)
        .await;

    let mut remote_builder = RemoteBuilder {
        streamer_token,
        injector: injector.clone(),
        global_bus,
        player,
        enabled,
        api_url: None,
        secret_key,
    };

    remote_builder.api_url = api_url.and_then(|s| parse_url(&s));

    let mut remote = Remote::default();
    remote_builder.init(&mut remote).await;

    let future = async move {
        loop {
            tokio::select! {
                update = streamer_token_stream.recv() => {
                    remote_builder.streamer_token = update;
                    remote_builder.init(&mut remote).await;
                }
                secret_key = secret_key_stream.recv() => {
                    remote_builder.secret_key = secret_key;
                    remote_builder.init(&mut remote).await;
                }
                update = player_stream.recv() => {
                    remote_builder.player = update;
                    remote_builder.init(&mut remote).await;
                }
                api_url = api_url_stream.recv() => {
                    remote_builder.api_url = api_url.and_then(|s| parse_url(&s));
                    remote_builder.init(&mut remote).await;
                }
                enabled = enabled_stream.recv() => {
                    remote_builder.enabled = enabled;
                    remote_builder.init(&mut remote).await;
                }
                event = async { remote.rx.as_mut().unwrap().recv().await }, if remote.rx.is_some() => {
                    let event = event?;

                    // Only update on switches to current song.
                    match event {
                        bus::Global::SongModified => (),
                        _ => continue,
                    };

                    let setbac = match remote.setbac.as_ref() {
                        Some(setbac) => setbac,
                        None => continue,
                    };

                    let player = match remote.player.as_ref() {
                        Some(player) => player,
                        None => continue,
                    };

                    tracing::trace!("Pushing remote player update");

                    let mut update = PlayerUpdate::default();

                    update.current = player.current().await.map(|c| c.item.into());

                    for i in player.list().await {
                        update.items.push(i.into());
                    }

                    if let Err(e) = setbac.player_update(update).await {
                        common::log_error!(e, "Failed to perform remote player update");
                    }
                }
            }
        }
    };

    Ok(future.in_current_span())
}

struct RemoteBuilder {
    streamer_token: Option<crate::token::Token>,
    injector: Injector,
    global_bus: bus::Bus<bus::Global>,
    player: Option<Player>,
    enabled: bool,
    api_url: Option<Url>,
    secret_key: Option<String>,
}

impl RemoteBuilder {
    async fn init(&self, remote: &mut Remote) {
        if self.enabled {
            remote.rx = Some(self.global_bus.subscribe());

            remote.player = self.player.as_ref().cloned();
        } else {
            remote.rx = None;
            remote.player = None;
        }

        remote.setbac = match self.api_url.as_ref() {
            Some(api_url) => {
                let setbac = Setbac::new(
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

#[derive(Default)]
struct Remote {
    rx: Option<bus::Reader<bus::Global>>,
    player: Option<player::Player>,
    setbac: Option<Setbac>,
}
