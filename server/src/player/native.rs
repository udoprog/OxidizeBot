use crate::secrets;
use failure::format_err;
use futures::{sync::oneshot, Stream};
use librespot::{
    core::{authentication::Credentials, config::SessionConfig, session::Session},
    playback::{audio_backend, config::PlayerConfig, player},
};
use tokio_core::reactor::Core;

/// We don't have any player event that are relevant.
fn translate_event(_: player::PlayerEvent) -> super::PlayerEvent {
    super::PlayerEvent::Filtered
}

/// Setup a player.
pub fn setup(
    core: &mut Core,
    config: &super::Config,
    secrets: &secrets::Secrets,
) -> Result<
    (
        Box<dyn super::PlayerInterface + 'static>,
        super::PlayerEventStream,
    ),
    failure::Error,
> {
    let secret_config = secrets.load::<SecretConfig>("spotify::native")?;

    let session_config = SessionConfig::default();
    let mut player_config = PlayerConfig::default();
    player_config.volume = config.volume.map(|v| (u32::min(100u32, v) as f32) / 100f32);

    let credentials = Credentials::with_password(
        secret_config.username.clone(),
        secret_config.password.clone(),
    );

    let backend = audio_backend::find(None).ok_or_else(|| format_err!("no audio backend"))?;

    let session = core.run(Session::connect(
        session_config,
        credentials,
        None,
        core.handle(),
    ))?;

    let speaker = config.speaker.clone();

    let (player, events) = player::Player::new(player_config, session.clone(), None, move || {
        (backend)(speaker)
    });

    Ok((Box::new(player), Box::new(events.map(translate_event))))
}

impl super::PlayerInterface for player::Player {
    fn stop(&mut self) {
        player::Player::stop(self);
    }

    fn play(&mut self) {
        player::Player::play(self);
    }

    fn pause(&mut self) {
        player::Player::pause(self);
    }

    fn load(&mut self, item: &super::Item, offset_ms: u32) -> oneshot::Receiver<()> {
        player::Player::load(self, item.track_id.0, false, offset_ms)
    }

    fn volume(&mut self, volume: Option<f32>) {
        player::Player::volume(self, volume);
    }
}

#[derive(serde::Deserialize)]
pub struct SecretConfig {
    username: String,
    password: String,
}
