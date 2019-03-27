use crate::{
    spotify,
    utils::{self, BoxFuture},
};
use failure::format_err;
use futures::{Async, Future, Poll, Stream};
use std::sync::Arc;
use tokio::timer;
use tokio_core::reactor::Core;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// Device to use with connect player.
    #[serde(default)]
    pub device: Option<String>,
}

/// Setup a player.
pub fn setup(
    core: &mut Core,
    config: &Config,
    spotify: Arc<spotify::Spotify>,
) -> Result<Box<dyn super::PlayerInterface>, failure::Error> {
    let devices = core.run(spotify.my_player_devices())?;

    for (i, device) in devices.iter().enumerate() {
        log::info!("device #{}: {}", i, device.name)
    }

    let device = match config.device.as_ref() {
        Some(device) => devices.into_iter().find(|d| d.name == *device),
        None => devices.into_iter().next(),
    };

    let device = device.ok_or_else(|| format_err!("No connected devices found"))?;

    let player = ConnectPlayer {
        spotify,
        device,
        play: None,
        pause: None,
        volume: None,
        timeout: None,
    };

    Ok(Box::new(player))
}

struct ConnectPlayer {
    spotify: Arc<spotify::Spotify>,
    device: spotify::Device,
    /// Last play command.
    play: Option<BoxFuture<(), failure::Error>>,
    /// Last pause command.
    pause: Option<BoxFuture<(), failure::Error>>,
    /// Last volume command.
    volume: Option<(BoxFuture<(), failure::Error>, u32)>,
    /// Timeout for end of song.
    timeout: Option<timer::Delay>,
}

impl super::PlayerInterface for ConnectPlayer {
    fn stop(&mut self) {
        self.pause = Some(Box::new(self.spotify.me_player_pause(&self.device.id)));
        self.timeout = None;
    }

    fn play(&mut self, song: &super::Song) {
        let track_uri = format!("spotify:track:{}", song.item.track_id.to_base62());

        self.play = Some(Box::new(self.spotify.me_player_play(
            &self.device.id,
            Some(track_uri.as_str()),
            Some(song.elapsed().as_millis() as u64),
        )));

        self.timeout = Some(timer::Delay::new(song.deadline()));
    }

    fn pause(&mut self) {
        self.pause = Some(Box::new(self.spotify.me_player_pause(&self.device.id)));
        self.timeout = None;
    }

    fn volume(&mut self, volume: u32) {
        let future = Box::new(
            self.spotify
                .me_player_volume(&self.device.id, (volume as f32) / 100f32),
        );
        self.volume = Some((future, volume));
    }
}

impl Stream for ConnectPlayer {
    type Item = super::PlayerEvent;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<super::PlayerEvent>, failure::Error> {
        loop {
            let mut not_ready = true;

            if let Some(timeout) = self.timeout.as_mut() {
                match timeout.poll()? {
                    Async::Ready(()) => {
                        self.timeout = None;
                        return Ok(Async::Ready(Some(super::PlayerEvent::EndOfTrack)));
                    }
                    Async::NotReady => (),
                }
            }

            if handle_future(&mut self.play, &mut not_ready, "play command") {
                return Ok(Async::Ready(Some(super::PlayerEvent::Playing)));
            }

            if handle_future(&mut self.pause, &mut not_ready, "pause command") {
                return Ok(Async::Ready(Some(super::PlayerEvent::Pausing)));
            }

            if let Some((future, volume)) = self.volume.as_mut() {
                match future.poll() {
                    Ok(Async::Ready(())) => {
                        let volume = *volume;
                        self.volume = None;
                        return Ok(Async::Ready(Some(super::PlayerEvent::Volume(volume))));
                    }
                    Ok(Async::NotReady) => (),
                    Err(e) => {
                        utils::log_err("failed to issue volume command", e);
                    }
                }
            }

            if not_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}

fn handle_future(
    future: &mut Option<BoxFuture<(), failure::Error>>,
    not_ready: &mut bool,
    what: &'static str,
) -> bool {
    let pollable = match future.as_mut() {
        Some(future) => future,
        None => return false,
    };

    let result = match pollable.poll() {
        Ok(Async::Ready(())) => true,
        Ok(Async::NotReady) => return false,
        Err(e) => {
            utils::log_err(format!("failed to issue {what}", what = what), e);
            false
        }
    };

    *future = None;
    *not_ready = false;
    result
}
