use crate::{
    spotify,
    utils::{self, BoxFuture},
};
use failure::format_err;
use futures::{sync::mpsc, Async, Future, Poll, Stream};
use std::sync::{Arc, RwLock};
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
) -> Result<(ConnectPlayer, ConnectInterface), failure::Error> {
    let devices = core.run(spotify.my_player_devices())?;

    for (i, device) in devices.iter().enumerate() {
        log::info!("device #{}: {}", i, device.name)
    }

    let device = match config.device.as_ref() {
        Some(device) => devices.into_iter().find(|d| d.name == *device),
        None => devices.into_iter().next(),
    };

    let device = Arc::new(RwLock::new(device));

    let (config_tx, config_rx) = mpsc::unbounded();

    let player = ConnectPlayer {
        spotify: spotify.clone(),
        device: device.clone(),
        play: None,
        pause: None,
        volume: None,
        timeout: None,
        config_rx,
    };

    // Configuration interface.
    let interface = ConnectInterface {
        spotify,
        config_tx,
        device,
    };

    Ok((player, interface))
}

pub struct ConnectPlayer {
    spotify: Arc<spotify::Spotify>,
    device: Arc<RwLock<Option<spotify::Device>>>,
    /// Last play command.
    play: Option<BoxFuture<(), failure::Error>>,
    /// Last pause command.
    pause: Option<BoxFuture<(), failure::Error>>,
    /// Last volume command.
    volume: Option<(BoxFuture<(), failure::Error>, u32)>,
    /// Timeout for end of song.
    timeout: Option<timer::Delay>,
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
}

impl ConnectPlayer {
    pub fn play(&mut self, song: &super::Song) -> Result<(), super::NotConfigured> {
        let device = self.device.read().expect("poisoned");

        let device = match device.as_ref() {
            Some(device) => device,
            None => return Err(super::NotConfigured),
        };

        let track_uri = format!("spotify:track:{}", song.item.track_id.to_base62());

        self.play = Some(Box::new(self.spotify.me_player_play(
            &device.id,
            Some(track_uri.as_str()),
            Some(song.elapsed().as_millis() as u64),
        )));

        self.timeout = Some(timer::Delay::new(song.deadline()));
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), super::NotConfigured> {
        let device = self.device.read().expect("poisoned");

        let device = match device.as_ref() {
            Some(device) => device,
            None => return Err(super::NotConfigured),
        };

        self.pause = Some(Box::new(self.spotify.me_player_pause(&device.id)));
        self.timeout = None;
        Ok(())
    }

    pub fn volume(&mut self, volume: u32) -> Result<(), super::NotConfigured> {
        let device = self.device.read().expect("poisoned");

        let device = match device.as_ref() {
            Some(device) => device,
            None => return Err(super::NotConfigured),
        };

        let future = Box::new(
            self.spotify
                .me_player_volume(&device.id, (volume as f32) / 100f32),
        );
        self.volume = Some((future, volume));
        Ok(())
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

            match self
                .config_rx
                .poll()
                .map_err(|_| format_err!("failed to receive configuration event"))?
            {
                Async::Ready(None) => failure::bail!("configuration received ended"),
                Async::Ready(Some(ConfigurationEvent::SetDevice(device))) => {
                    *self.device.write().expect("poisoned") = Some(device);
                    return Ok(Async::Ready(Some(super::PlayerEvent::DeviceChanged)));
                }
                Async::NotReady => (),
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

pub enum ConfigurationEvent {
    SetDevice(spotify::Device),
}

#[derive(Clone)]
pub struct ConnectInterface {
    spotify: Arc<spotify::Spotify>,
    config_tx: mpsc::UnboundedSender<ConfigurationEvent>,
    device: Arc<RwLock<Option<spotify::Device>>>,
}

impl ConnectInterface {
    /// Get the current device.
    pub fn current_device(&self) -> Option<spotify::Device> {
        self.device.read().expect("poisoned").clone()
    }

    /// List all available devices.
    pub fn list_devices(&self) -> impl Future<Item = Vec<spotify::Device>, Error = failure::Error> {
        self.spotify.my_player_devices()
    }

    /// Set which device to perform playback from.
    pub fn set_device(&self, device: spotify::Device) {
        if let Err(_) = self
            .config_tx
            .unbounded_send(ConfigurationEvent::SetDevice(device))
        {
            log::error!("failed to configure device");
        }
    }
}
