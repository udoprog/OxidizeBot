use crate::{spotify, utils::BoxFuture};
use failure::format_err;
use futures::{sync::mpsc, Async, Future, Poll, Stream};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::sync::Arc;
use tokio::timer;

/// Setup a player.
pub fn setup(
    spotify: Arc<spotify::Spotify>,
) -> Result<(ConnectPlayer, ConnectDevice), failure::Error> {
    let device = Arc::new(RwLock::new(None));

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
    let interface = ConnectDevice {
        spotify,
        config_tx,
        device,
    };

    Ok((player, interface))
}

#[derive(Debug)]
pub enum Event {
    /// We've reached the end of a track.
    EndOfTrack,
    /// Indicate that the current device changed.
    DeviceChanged,
    /// We've filtered a player event.
    Filtered,
    /// Indicate that we have successfully issued a playing command to the player.
    Playing(super::Source),
    /// Indicate that we have successfully issued a pause command to the player.
    Pausing(super::Source),
    /// Indicate that we have successfully issued a volume command to the player.
    Volume(super::Source, u32),
}

pub struct ConnectPlayerWithDevice<'a> {
    device: MappedRwLockReadGuard<'a, spotify::Device>,
    spotify: &'a spotify::Spotify,
    /// Last play command.
    play: &'a mut Option<(super::Source, BoxFuture<(), failure::Error>)>,
    /// Last pause command.
    pause: &'a mut Option<(super::Source, BoxFuture<(), failure::Error>)>,
    /// Last volume command.
    volume: &'a mut Option<(super::Source, BoxFuture<(), failure::Error>, u32)>,
    /// Timeout for end of song.
    timeout: &'a mut Option<timer::Delay>,
}

impl ConnectPlayerWithDevice<'_> {
    /// Play the specified song.
    pub fn play(&mut self, kind: super::Source, song: &super::Song) {
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

    pub fn pause(&mut self, kind: super::Source) {
        *self.pause = Some((
            kind,
            Box::new(self.spotify.me_player_pause(&self.device.id)),
        ));
        *self.timeout = None;
    }

    pub fn volume(&mut self, kind: super::Source, volume: u32) {
        let future = Box::new(
            self.spotify
                .me_player_volume(&self.device.id, (volume as f32) / 100f32),
        );
        *self.volume = Some((kind, future, volume));
    }
}

pub struct ConnectPlayer {
    spotify: Arc<spotify::Spotify>,
    device: Arc<RwLock<Option<spotify::Device>>>,
    /// Last play command.
    play: Option<(super::Source, BoxFuture<(), failure::Error>)>,
    /// Last pause command.
    pause: Option<(super::Source, BoxFuture<(), failure::Error>)>,
    /// Last volume command.
    volume: Option<(super::Source, BoxFuture<(), failure::Error>, u32)>,
    /// Timeout for end of song.
    timeout: Option<timer::Delay>,
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
}

impl ConnectPlayer {
    /// Access the current connect player, testing that we have access to a device.
    pub fn with_device(&mut self) -> Result<ConnectPlayerWithDevice<'_>, super::NotConfigured> {
        let device = self.device.read();

        let device = match RwLockReadGuard::try_map(device, |d| d.as_ref()) {
            Ok(device) => device,
            Err(_) => return Err(super::NotConfigured),
        };

        Ok(ConnectPlayerWithDevice {
            device,
            spotify: &self.spotify,
            play: &mut self.play,
            pause: &mut self.pause,
            volume: &mut self.volume,
            timeout: &mut self.timeout,
        })
    }

    /// Synchronize the state of the player with the given song.
    pub fn play_sync(&mut self, song: Option<&super::Song>) {
        self.timeout = match song {
            Some(song) if song.is_playing() => Some(timer::Delay::new(song.deadline())),
            _ => None,
        };
    }
}

impl Stream for ConnectPlayer {
    type Item = Event;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Event>, failure::Error> {
        loop {
            if let Some(timeout) = self.timeout.as_mut() {
                match timeout.poll()? {
                    Async::NotReady => (),
                    Async::Ready(()) => {
                        self.timeout = None;
                        return Ok(Async::Ready(Some(Event::EndOfTrack)));
                    }
                }
            }

            if let Some(kind) = handle_future(&mut self.play, "play command") {
                return Ok(Async::Ready(Some(Event::Playing(kind))));
            }

            if let Some(kind) = handle_future(&mut self.pause, "pause command") {
                return Ok(Async::Ready(Some(Event::Pausing(kind))));
            }

            if let Some((kind, future, volume)) = self.volume.as_mut() {
                match future.poll() {
                    Ok(Async::NotReady) => (),
                    Err(e) => {
                        log_err!(e, "failed to issue volume command");
                    }
                    Ok(Async::Ready(())) => {
                        let kind = *kind;
                        let volume = *volume;
                        self.volume = None;
                        return Ok(Async::Ready(Some(Event::Volume(kind, volume))));
                    }
                }
            }

            match self
                .config_rx
                .poll()
                .map_err(|_| format_err!("failed to receive configuration event"))?
            {
                Async::NotReady => (),
                Async::Ready(None) => failure::bail!("configuration received ended"),
                Async::Ready(Some(ConfigurationEvent::DeviceChanged)) => {
                    return Ok(Async::Ready(Some(Event::DeviceChanged)));
                }
            }

            return Ok(Async::NotReady);
        }
    }
}

fn handle_future(
    future: &mut Option<(super::Source, BoxFuture<(), failure::Error>)>,
    what: &'static str,
) -> Option<super::Source> {
    let (kind, pollable) = match future.as_mut() {
        Some(future) => future,
        None => return None,
    };

    let result = match pollable.poll() {
        Ok(Async::Ready(())) => Some(*kind),
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
    spotify: Arc<spotify::Spotify>,
    config_tx: mpsc::UnboundedSender<ConfigurationEvent>,
    pub device: Arc<RwLock<Option<spotify::Device>>>,
}

impl ConnectDevice {
    /// Get the current device.
    pub fn current_device(&self) -> Option<spotify::Device> {
        self.device.read().clone()
    }

    /// List all available devices.
    pub fn list_devices(&self) -> impl Future<Item = Vec<spotify::Device>, Error = failure::Error> {
        self.spotify.my_player_devices()
    }

    /// Set which device to perform playback from.
    pub fn set_device(&self, device: Option<spotify::Device>) -> Option<spotify::Device> {
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
