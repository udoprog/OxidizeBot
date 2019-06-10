use crate::{api, player, prelude::*, settings::Settings, track_id::SpotifyId, utils::Futures};
use failure::{bail, Error};
use parking_lot::RwLock;
use std::sync::Arc;
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

/// Setup a player.
pub fn setup(
    futures: &mut Futures,
    spotify: Arc<api::Spotify>,
    settings: Settings,
) -> Result<(ConnectStream, ConnectPlayer, ConnectDevice), Error> {
    let (mut volume_stream, volume) = settings.stream("volume").or_with(50)?;
    let (mut volume_scale_stream, volume_scale) = settings.stream("volume-scale").or_with(100)?;
    let (mut device_stream, device) = settings.stream::<String>("device").optional()?;

    let device = Arc::new(RwLock::new(device));

    let mut scaled_volume = (volume * volume_scale) / 100u32;
    let volume = Arc::new(RwLock::new(volume));
    let volume_scale = Arc::new(RwLock::new(volume_scale));

    let (config_tx, config_rx) = mpsc::unbounded();

    let stream = ConnectStream { config_rx };

    let player = ConnectPlayer {
        spotify: spotify.clone(),
        device: device.clone(),
        settings: settings.clone(),
        volume_scale: volume_scale.clone(),
        volume: volume.clone(),
    };

    // Configuration interface.
    let interface = ConnectDevice {
        spotify,
        device: device.clone(),
        settings: settings.clone(),
    };

    let returned_player = player.clone();

    let future = async move {
        player.volume_update_log(scaled_volume).await;

        loop {
            futures::select! {
                update = device_stream.select_next_some() => {
                    *device.write() = update;

                    if let Err(_) = config_tx.unbounded_send(ConfigurationEvent::DeviceChanged) {
                        bail!("failed to send configuration event");
                    }
                }
                update = volume_scale_stream.select_next_some() => {
                    *volume_scale.write() = update;
                    scaled_volume = (*volume.read() * update) / 100u32;
                    player.volume_update_log(scaled_volume).await;
                }
                update = volume_stream.select_next_some() => {
                    *volume.write() = update;
                    scaled_volume = (update * *volume_scale.read()) / 100u32;
                    player.volume_update_log(scaled_volume).await;
                }
            }
        }
    };

    futures.push(future.boxed());
    Ok((stream, returned_player, interface))
}

#[derive(Debug, err_derive::Error)]
pub enum CommandError {
    #[error(display = "error when issuing {} command", _0)]
    Error(&'static str),
    #[error(display = "no device configured or available")]
    NoDevice,
    #[error(display = "other error")]
    Other(Error),
}

impl CommandError {
    fn handle(result: Result<bool, Error>, what: &'static str) -> Result<(), CommandError> {
        match result {
            Err(e) => {
                log_err!(e, "failed to issue {} command", what);
                Err(CommandError::Error(what))
            }
            Ok(true) => Ok(()),
            Ok(false) => Err(CommandError::NoDevice),
        }
    }
}

#[derive(Clone)]
pub struct ConnectPlayer {
    spotify: Arc<api::Spotify>,
    /// Currently configured device.
    device: Arc<RwLock<Option<String>>>,
    /// Access to settings.
    settings: Settings,
    /// Current volume scale for this player.
    volume_scale: Arc<RwLock<u32>>,
    /// Current volume for this player.
    volume: Arc<RwLock<u32>>,
}

impl ConnectPlayer {
    /// Play the specified song.
    pub async fn play(&self, elapsed: Duration, id: SpotifyId) -> Result<(), CommandError> {
        let track_uri = format!("spotify:track:{}", id.to_base62());
        let device_id = self.device.read().clone();

        let result = self
            .spotify
            .me_player_play(device_id, Some(track_uri), Some(elapsed.as_millis() as u64))
            .await;

        CommandError::handle(result, "play")
    }

    pub async fn pause(&self) -> Result<(), CommandError> {
        let device_id = self.device.read().clone();
        CommandError::handle(self.spotify.me_player_pause(device_id).await, "pause")
    }

    pub async fn stop(&self) -> Result<(), CommandError> {
        let device_id = self.device.read().clone();
        CommandError::handle(self.spotify.me_player_pause(device_id).await, "stop")
    }

    /// Update an unscaled volume.
    pub(crate) fn set_scaled_volume(&self, scaled_volume: u32) -> Result<u32, CommandError> {
        let volume_scale = *self.volume_scale.read();
        let update = u32::min((scaled_volume * 100) / volume_scale, 100);
        self.volume(player::ModifyVolume::Set(update))
    }

    pub fn volume(&self, modify: player::ModifyVolume) -> Result<u32, CommandError> {
        let mut volume = self.volume.write();
        let update = modify.apply(*volume);
        *volume = update;
        self.settings
            .set("volume", update)
            .map_err(CommandError::Other)?;
        Ok(update)
    }

    pub fn current_volume(&self) -> u32 {
        *self.volume.read()
    }

    async fn volume_update(&self, volume: u32) -> Result<(), CommandError> {
        let volume = (volume as f32) / 100f32;
        let device_id = self.device.read().clone();
        CommandError::handle(
            self.spotify.me_player_volume(device_id, volume).await,
            "volume",
        )
    }

    /// Same as volume update, but logs instead of errors.
    async fn volume_update_log(&self, volume: u32) {
        if let Err(e) = self.volume_update(volume).await {
            log::error!("Failed to update volume: {}", e);
        }
    }
}

pub struct ConnectStream {
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
}

impl Stream for ConnectStream {
    type Item = Result<player::IntegrationEvent, failure::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        use self::player::IntegrationEvent::*;

        loop {
            if let Poll::Ready(Some(e)) = Pin::new(&mut self.config_rx).poll_next(cx) {
                match e {
                    ConfigurationEvent::DeviceChanged => {
                        return Poll::Ready(Some(Ok(DeviceChanged)));
                    }
                }
            }

            return Poll::Pending;
        }
    }
}

impl stream::FusedStream for ConnectStream {
    fn is_terminated(&self) -> bool {
        false
    }
}

pub enum ConfigurationEvent {
    /// Indicate that the current device has been changed.
    DeviceChanged,
}

#[derive(Clone)]
pub struct ConnectDevice {
    spotify: Arc<api::Spotify>,
    pub device: Arc<RwLock<Option<String>>>,
    settings: Settings,
}

impl ConnectDevice {
    /// Synchronize the device.
    pub fn sync_device(&self, device: Option<api::spotify::Device>) -> Result<(), Error> {
        match (self.device.read().as_ref(), device.as_ref()) {
            (None, None) => return Ok(()),
            (Some(a), Some(b)) if *a == b.id => return Ok(()),
            _ => (),
        };

        self.settings.set("device", device.map(|d| d.id))?;
        Ok(())
    }

    /// Get the current device.
    pub fn current_device(&self) -> Option<String> {
        self.device.read().clone()
    }

    /// List all available devices.
    pub async fn list_devices(&self) -> Result<Vec<api::spotify::Device>, Error> {
        self.spotify.my_player_devices().await
    }

    /// Set which device to perform playback from.
    pub fn set_device(&self, device: Option<String>) -> Result<(), Error> {
        self.settings.set("device", device)?;
        Ok(())
    }
}
