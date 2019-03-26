use crate::secrets;
use failure::format_err;
use futures::{
    sync::{mpsc, oneshot},
    Async, Future, Poll, Stream,
};
use librespot::{
    core::{authentication::Credentials, config::SessionConfig, session::Session},
    playback::{audio_backend, config::PlayerConfig, player},
};
use tokio_core::reactor::Core;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// Speaker to use with native player.
    #[serde(default)]
    pub speaker: Option<String>,
}

/// We don't have any player event that are relevant.
fn translate_event(_: player::PlayerEvent) -> super::PlayerEvent {
    super::PlayerEvent::Filtered
}

struct PlayerInterface {
    player: player::Player,
    end_of_track: Option<oneshot::Receiver<()>>,
    events: mpsc::UnboundedReceiver<player::PlayerEvent>,
}

/// Setup a player.
pub fn setup(
    core: &mut Core,
    parent_config: &super::Config,
    config: &Config,
    secrets: &secrets::Secrets,
) -> Result<Box<dyn super::PlayerInterface>, failure::Error> {
    let secret_config = secrets.load::<SecretConfig>("spotify::native")?;

    let session_config = SessionConfig::default();
    let mut player_config = PlayerConfig::default();
    player_config.volume = parent_config
        .volume
        .map(|v| (u32::min(100u32, v) as f32) / 100f32);

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

    Ok(Box::new(PlayerInterface {
        player,
        end_of_track: None,
        events,
    }))
}

impl Stream for PlayerInterface {
    type Item = super::PlayerEvent;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<super::PlayerEvent>, failure::Error> {
        if let Some(end_of_track) = self.end_of_track.as_mut() {
            match end_of_track.poll() {
                Ok(Async::Ready(())) | Err(oneshot::Canceled) => {
                    self.end_of_track = None;
                    return Ok(Async::Ready(Some(super::PlayerEvent::EndOfTrack)));
                }
                Ok(Async::NotReady) => (),
            }
        }

        let poll = match self.events.poll() {
            Ok(poll) => poll,
            Err(()) => failure::bail!("error in events stream"),
        };

        match poll {
            Async::NotReady => (),
            Async::Ready(Some(e)) => return Ok(Async::Ready(Some(translate_event(e)))),
            Async::Ready(None) => failure::bail!("player event stream ended"),
        }

        Ok(Async::NotReady)
    }
}

impl super::PlayerInterface for PlayerInterface {
    fn stop(&mut self) {
        self.player.stop();
    }

    fn play(&mut self, _: &super::Song) {
        self.player.play();
    }

    fn pause(&mut self) {
        self.player.pause();
    }

    fn load(&mut self, song: &super::Song) {
        let future = self.player.load(
            song.item.track_id.0,
            false,
            song.elapsed().as_millis() as u32,
        );
        self.end_of_track = Some(future);
    }

    fn volume(&mut self, volume: Option<f32>) {
        self.player.volume(volume);
    }
}

#[derive(serde::Deserialize)]
pub struct SecretConfig {
    username: String,
    password: String,
}
