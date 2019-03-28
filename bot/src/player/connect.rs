use crate::{
    spotify,
    utils::{self, BoxFuture},
};
use failure::format_err;
use futures::{sync::mpsc, Async, Future, Poll, Stream};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
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
) -> Result<(Player, ConnectInterface), failure::Error> {
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

    let player = Player {
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

pub struct PlayerClient<'a> {
    device: MappedRwLockReadGuard<'a, spotify::Device>,
    spotify: &'a spotify::Spotify,
    /// Last play command.
    play: &'a mut Option<(super::EventKind, BoxFuture<(), failure::Error>)>,
    /// Last pause command.
    pause: &'a mut Option<(super::EventKind, BoxFuture<(), failure::Error>)>,
    /// Last volume command.
    volume: &'a mut Option<(super::EventKind, BoxFuture<(), failure::Error>, u32)>,
    /// Timeout for end of song.
    timeout: &'a mut Option<timer::Delay>,
}

impl PlayerClient<'_> {
    /// Play the specified song.
    pub fn play(&mut self, kind: super::EventKind, song: &super::Song) {
        let track_uri = format!("spotify:track:{}", song.item.track_id.to_base62());

        *self.play = Some((
            kind,
            Box::new(self.spotify.me_player_play(
                &self.device.id,
                Some(track_uri.as_str()),
                Some(song.elapsed().as_millis() as u64),
            )),
        ));

        *self.timeout = Some(timer::Delay::new(song.deadline()));
    }

    pub fn pause(&mut self, kind: super::EventKind) {
        *self.pause = Some((
            kind,
            Box::new(self.spotify.me_player_pause(&self.device.id)),
        ));
        *self.timeout = None;
    }

    pub fn volume(&mut self, kind: super::EventKind, volume: u32) {
        let future = Box::new(
            self.spotify
                .me_player_volume(&self.device.id, (volume as f32) / 100f32),
        );
        *self.volume = Some((kind, future, volume));
    }
}

pub struct Player {
    spotify: Arc<spotify::Spotify>,
    device: Arc<RwLock<Option<spotify::Device>>>,
    /// Last play command.
    play: Option<(super::EventKind, BoxFuture<(), failure::Error>)>,
    /// Last pause command.
    pause: Option<(super::EventKind, BoxFuture<(), failure::Error>)>,
    /// Last volume command.
    volume: Option<(super::EventKind, BoxFuture<(), failure::Error>, u32)>,
    /// Timeout for end of song.
    timeout: Option<timer::Delay>,
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
}

impl Player {
    /// Access the current player client.
    pub fn client(&mut self) -> Result<PlayerClient<'_>, super::NotConfigured> {
        let device = self.device.read();

        let device = match RwLockReadGuard::try_map(device, |d| d.as_ref()) {
            Ok(device) => device,
            Err(_) => return Err(super::NotConfigured),
        };

        Ok(PlayerClient {
            device,
            spotify: &self.spotify,
            play: &mut self.play,
            pause: &mut self.pause,
            volume: &mut self.volume,
            timeout: &mut self.timeout,
        })
    }
}

impl Stream for Player {
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

            if let Some(kind) = handle_future(&mut self.play, &mut not_ready, "play command") {
                return Ok(Async::Ready(Some(super::PlayerEvent::Playing(kind))));
            }

            if let Some(kind) = handle_future(&mut self.pause, &mut not_ready, "pause command") {
                return Ok(Async::Ready(Some(super::PlayerEvent::Pausing(kind))));
            }

            if let Some((kind, future, volume)) = self.volume.as_mut() {
                match future.poll() {
                    Ok(Async::Ready(())) => {
                        let kind = *kind;
                        let volume = *volume;
                        self.volume = None;
                        return Ok(Async::Ready(Some(super::PlayerEvent::Volume(kind, volume))));
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
                    *self.device.write() = Some(device);
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
    future: &mut Option<(super::EventKind, BoxFuture<(), failure::Error>)>,
    not_ready: &mut bool,
    what: &'static str,
) -> Option<super::EventKind> {
    let (kind, pollable) = match future.as_mut() {
        Some(future) => future,
        None => return None,
    };

    let result = match pollable.poll() {
        Ok(Async::Ready(())) => Some(*kind),
        Ok(Async::NotReady) => return None,
        Err(e) => {
            utils::log_err(format!("failed to issue {what}", what = what), e);
            None
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
        self.device.read().clone()
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
