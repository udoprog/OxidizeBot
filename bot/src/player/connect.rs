use crate::{api, player, track_id::SpotifyId, utils::BoxFuture};
use futures::{sync::mpsc, Async, Future, Poll, Stream};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::timer;

/// Setup a player.
pub fn setup(spotify: Arc<api::Spotify>) -> Result<(ConnectPlayer, ConnectDevice), failure::Error> {
    let device = Arc::new(RwLock::new(None));

    let (config_tx, config_rx) = mpsc::unbounded();

    let player = ConnectPlayer {
        spotify: spotify.clone(),
        device: device.clone(),
        play: None,
        pause: None,
        stop: None,
        volume: None,
        timeout: None,
        config_rx,
    };

    // Configuration interface.
    let interface = ConnectDevice {
        spotify,
        config_tx,
        device,
    };

    Ok((player, interface))
}

pub struct ConnectPlayer {
    spotify: Arc<api::Spotify>,
    /// Currently configured device.
    device: Arc<RwLock<Option<api::spotify::Device>>>,
    /// Last play command.
    play: Option<(super::Source, BoxFuture<bool, failure::Error>)>,
    /// Last pause command.
    pause: Option<(super::Source, BoxFuture<bool, failure::Error>)>,
    /// Last stop command.
    stop: Option<(BoxFuture<bool, failure::Error>)>,
    /// Last volume command.
    volume: Option<(super::Source, BoxFuture<bool, failure::Error>, u32)>,
    /// Timeout for end of song.
    timeout: Option<timer::Delay>,
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
}

impl ConnectPlayer {
    /// Synchronize the state of the player with the given song.
    pub fn play_sync(&mut self, song: Option<&super::Song>) {
        self.timeout = match song {
            Some(song) if song.state().is_playing() => Some(timer::Delay::new(song.deadline())),
            _ => None,
        };
    }

    /// Detach the player, cancelling any timed events or effects.
    pub fn detach(&mut self) {
        self.timeout = None;
    }

    /// Play the specified song.
    pub fn play(&mut self, kind: super::Source, song: &super::Song, spotify_id: &SpotifyId) {
        let track_uri = format!("spotify:track:{}", spotify_id.to_base62());

        let device = self.device.read();
        let device_id = device.as_ref().map(|d| d.id.as_str());

        self.play = Some((
            kind,
            Box::new(self.spotify.me_player_play(
                device_id,
                Some(track_uri.as_str()),
                Some(song.elapsed().as_millis() as u64),
            )),
        ));

        self.timeout = Some(timer::Delay::new(song.deadline()));
    }

    pub fn pause(&mut self, kind: super::Source) {
        let device = self.device.read();
        let device_id = device.as_ref().map(|d| d.id.as_str());

        self.pause = Some((kind, Box::new(self.spotify.me_player_pause(device_id))));
        self.timeout = None;
    }

    pub fn stop(&mut self) {
        let device = self.device.read();
        let device_id = device.as_ref().map(|d| d.id.as_str());

        self.stop = Some(Box::new(self.spotify.me_player_pause(device_id)));
        self.timeout = None;
    }

    pub fn volume(&mut self, kind: super::Source, volume: u32) {
        let device = self.device.read();
        let device_id = device.as_ref().map(|d| d.id.as_str());

        let future = Box::new(
            self.spotify
                .me_player_volume(device_id, (volume as f32) / 100f32),
        );

        self.volume = Some((kind, future, volume));
    }
}

impl Stream for ConnectPlayer {
    type Item = player::IntegrationEvent;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, failure::Error> {
        use self::player::IntegrationEvent::*;

        loop {
            if let Some(timeout) = self.timeout.as_mut() {
                match timeout.poll()? {
                    Async::NotReady => (),
                    Async::Ready(()) => {
                        self.timeout = None;
                        return Ok(Async::Ready(Some(EndOfTrack)));
                    }
                }
            }

            if let Some(kind) = handle_future(&mut self.play, "play command") {
                return Ok(Async::Ready(Some(Playing(kind))));
            }

            if let Some(kind) = handle_future(&mut self.pause, "pause command") {
                return Ok(Async::Ready(Some(Pausing(kind))));
            }

            if let Some(mut stop) = self.stop.take() {
                match stop.poll() {
                    Err(e) => log_err!(e, "failed to issue stop command"),
                    Ok(Async::NotReady) => self.stop = Some(stop),
                    Ok(Async::Ready(found)) => {
                        if !found {
                            log::warn!("no playing device found");
                        }

                        return Ok(Async::Ready(Some(Stopping)));
                    }
                }
            }

            if let Some(mut volume) = self.volume.take() {
                match volume.1.poll() {
                    Err(e) => log_err!(e, "failed to issue volume command"),
                    Ok(Async::NotReady) => self.volume = Some(volume),
                    Ok(Async::Ready(found)) => {
                        if !found {
                            log::warn!("no playing device found");
                        }

                        let kind = volume.0;
                        let volume = volume.2;
                        return Ok(Async::Ready(Some(Volume(kind, volume))));
                    }
                }
            }

            if let Some(e) = try_infinite_empty!(self.config_rx.poll()) {
                match e {
                    ConfigurationEvent::DeviceChanged => {
                        return Ok(Async::Ready(Some(DeviceChanged)))
                    }
                }
            }

            return Ok(Async::NotReady);
        }
    }
}

fn handle_future(
    future: &mut Option<(
        super::Source,
        impl Future<Item = bool, Error = failure::Error>,
    )>,
    what: &'static str,
) -> Option<super::Source> {
    let (kind, pollable) = match future.as_mut() {
        Some(future) => future,
        None => return None,
    };

    let result = match pollable.poll() {
        Ok(Async::Ready(found)) => {
            if !found {
                log::warn!("no playing device found");
            }

            Some(*kind)
        }
        Ok(Async::NotReady) => return None,
        Err(e) => {
            log_err!(e, "failed to issue {what}", what = what);
            None
        }
    };

    *future = None;
    result
}

pub enum ConfigurationEvent {
    /// Indicate that the current device has been changed.
    DeviceChanged,
}

#[derive(Clone)]
pub struct ConnectDevice {
    spotify: Arc<api::Spotify>,
    config_tx: mpsc::UnboundedSender<ConfigurationEvent>,
    pub device: Arc<RwLock<Option<api::spotify::Device>>>,
}

impl ConnectDevice {
    /// Get the current device.
    pub fn current_device(&self) -> Option<api::spotify::Device> {
        self.device.read().clone()
    }

    /// List all available devices.
    pub fn list_devices(
        &self,
    ) -> impl Future<Item = Vec<api::spotify::Device>, Error = failure::Error> {
        self.spotify.my_player_devices()
    }

    /// Set which device to perform playback from.
    pub fn set_device(&self, device: Option<api::spotify::Device>) -> Option<api::spotify::Device> {
        let old = std::mem::replace(&mut *self.device.write(), device);

        if let Err(_) = self
            .config_tx
            .unbounded_send(ConfigurationEvent::DeviceChanged)
        {
            log::error!("failed to configure device");
        }

        return old;
    }
}
