use crate::{api, player, settings::ScopedSettings, track_id::SpotifyId, utils::BoxFuture};
use futures::{sync::mpsc, Async, Future, Poll, Stream};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_core::reactor::Core;

/// Setup a player.
pub fn setup(
    core: &mut Core,
    spotify: Arc<api::Spotify>,
    settings: ScopedSettings,
) -> Result<(ConnectPlayer, ConnectDevice), failure::Error> {
    let device = Arc::new(RwLock::new(None));

    let volume_scale = settings.sync_var(core, "volume-scale", 100)?;

    let (config_tx, config_rx) = mpsc::unbounded();

    let player = ConnectPlayer {
        spotify: spotify.clone(),
        device: device.clone(),
        play: None,
        pause: None,
        stop: None,
        volume: None,
        config_rx,
        volume_scale,
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
    play: Option<BoxFuture<bool, failure::Error>>,
    /// Last pause command.
    pause: Option<BoxFuture<bool, failure::Error>>,
    /// Last stop command.
    stop: Option<(BoxFuture<bool, failure::Error>)>,
    /// Last volume command.
    volume: Option<BoxFuture<bool, failure::Error>>,
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
    /// Scale to use for volume.
    volume_scale: Arc<RwLock<u32>>,
}

impl ConnectPlayer {
    /// Play the specified song.
    pub fn play(&mut self, song: &super::Song, spotify_id: &SpotifyId) {
        let track_uri = format!("spotify:track:{}", spotify_id.to_base62());

        let device = self.device.read();
        let device_id = device.as_ref().map(|d| d.id.as_str());

        self.play = Some(Box::new(self.spotify.me_player_play(
            device_id,
            Some(track_uri.as_str()),
            Some(song.elapsed().as_millis() as u64),
        )));
    }

    pub fn pause(&mut self) {
        let device = self.device.read();
        let device_id = device.as_ref().map(|d| d.id.as_str());
        self.pause = Some(Box::new(self.spotify.me_player_pause(device_id)));
    }

    pub fn stop(&mut self) {
        let device = self.device.read();
        let device_id = device.as_ref().map(|d| d.id.as_str());
        self.stop = Some(Box::new(self.spotify.me_player_pause(device_id)));
    }

    pub fn volume(&mut self, volume: u32) {
        let volume = (volume * *self.volume_scale.read()) / 100u32;

        let device = self.device.read();
        let device_id = device.as_ref().map(|d| d.id.as_str());

        self.volume = Some(Box::new(
            self.spotify
                .me_player_volume(device_id, (volume as f32) / 100f32),
        ));
    }
}

impl Stream for ConnectPlayer {
    type Item = player::IntegrationEvent;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, failure::Error> {
        use self::player::IntegrationEvent::*;

        loop {
            if handle_future(&mut self.stop, "stop command") {
                continue;
            }

            if handle_future(&mut self.play, "play command") {
                continue;
            }

            if handle_future(&mut self.pause, "pause command") {
                continue;
            }

            if handle_future(&mut self.volume, "volume command") {
                continue;
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
    future: &mut Option<impl Future<Item = bool, Error = failure::Error>>,
    what: &'static str,
) -> bool {
    let mut f = match future.take() {
        Some(future) => future,
        None => return false,
    };

    match f.poll() {
        Ok(Async::Ready(found)) => {
            if !found {
                log::warn!("no playing device found");
            }

            true
        }
        Ok(Async::NotReady) => {
            *future = Some(f);
            false
        }
        Err(e) => {
            log_err!(e, "failed to issue {what}", what = what);
            false
        }
    }
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
