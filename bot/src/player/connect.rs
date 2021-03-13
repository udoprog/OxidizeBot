use crate::api;
use crate::player;
use crate::prelude::*;
use crate::track_id::SpotifyId;
use anyhow::{bail, Error, Result};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Setup a player.
pub(super) async fn setup(
    spotify: Arc<api::Spotify>,
    settings: crate::Settings,
) -> Result<(
    ConnectStream,
    ConnectPlayer,
    ConnectDevice,
    impl Future<Output = Result<()>>,
)> {
    let (mut volume_stream, volume) = settings.stream("volume").or_with(50).await?;
    let (mut volume_scale_stream, volume_scale) =
        settings.stream("volume-scale").or_with(100).await?;
    let (mut device_stream, device) = settings.stream::<String>("device").optional().await?;

    // Locally scaled volume.
    let mut scaled_volume = (volume * volume_scale) / 100u32;

    let device = settings::Var::new(device);
    let volume = settings::Var::new(volume);
    let volume_scale = settings::Var::new(volume_scale);

    let (config_tx, config_rx) = mpsc::unbounded_channel();

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
        settings,
    };

    let returned_player = player.clone();

    let future = async move {
        warn_on_error(player.volume_update(scaled_volume).await);

        loop {
            tokio::select! {
                update = device_stream.recv() => {
                    *device.write().await = update;

                    if config_tx.send(ConfigurationEvent::DeviceChanged).is_err() {
                        bail!("failed to send configuration event");
                    }
                }
                update = volume_scale_stream.recv() => {
                    *volume_scale.write().await = update;
                    scaled_volume = (volume.load().await * update) / 100u32;
                    warn_on_error(player.volume_update(scaled_volume).await);
                }
                update = volume_stream.recv() => {
                    *volume.write().await = update;
                    scaled_volume = (update * volume_scale.load().await) / 100u32;
                    warn_on_error(player.volume_update(scaled_volume).await);
                }
            }
        }
    };

    Ok((stream, returned_player, interface, future))
}

#[derive(Debug, Error)]
pub(super) enum ConnectError {
    #[error("{0}: no device configured or available")]
    NoDevice(&'static str),
    #[error("{0}: error")]
    Error(&'static str, #[source] Error),
}

impl ConnectError {
    fn handle(result: Result<bool>, what: &'static str) -> Result<(), ConnectError> {
        match result {
            Err(e) => Err(ConnectError::Error(what, e.into())),
            Ok(true) => Ok(()),
            Ok(false) => Err(ConnectError::NoDevice(what)),
        }
    }
}

#[derive(Clone)]
pub(super) struct ConnectPlayer {
    spotify: Arc<api::Spotify>,
    /// Currently configured device.
    device: settings::Var<Option<String>>,
    /// Access to settings.
    settings: crate::Settings,
    /// Current volume scale for this player.
    volume_scale: settings::Var<u32>,
    /// Current volume for this player.
    volume: settings::Var<u32>,
}

impl ConnectPlayer {
    /// Play the specified song. Or just starting playing if the id of the song
    /// is unspecified.
    pub(super) async fn play(&self, id: Option<SpotifyId>, elapsed: Option<Duration>) {
        let track_uri = id.map(|id| format!("spotify:track:{}", id.to_base62()));
        let elapsed = elapsed.map(|elapsed| elapsed.as_millis() as u64);
        let device_id = self.device.load().await;

        let result = self
            .spotify
            .me_player_play(device_id.as_deref(), track_uri.as_deref(), elapsed)
            .await;

        warn_on_error(ConnectError::handle(result, "play"));
    }

    /// Play the next song.
    pub(super) async fn next(&self) {
        let device_id = self.device.load().await;
        let result = self.spotify.me_player_next(device_id.as_deref()).await;
        warn_on_error(ConnectError::handle(result, "skip"));
    }

    /// Pause playback.
    pub(super) async fn pause(&self) {
        let device_id = self.device.load().await;

        warn_on_error(ConnectError::handle(
            self.spotify.me_player_pause(device_id.as_deref()).await,
            "pause",
        ));
    }

    /// Stop playback.
    pub(super) async fn stop(&self) {
        let device_id = self.device.load().await;

        warn_on_error(ConnectError::handle(
            self.spotify.me_player_pause(device_id.as_deref()).await,
            "stop",
        ));
    }

    /// Update a scaled volume.
    pub(super) async fn set_scaled_volume(&self, scaled_volume: u32) {
        let volume_scale = self.volume_scale.load().await;
        let update = u32::min((scaled_volume * 100) / volume_scale, 100);
        self.volume(player::ModifyVolume::Set(update)).await;
    }

    /// Get the current volume of the player.
    pub(super) async fn current_volume(&self) -> u32 {
        self.volume.load().await
    }

    /// Enqueue the specified song to play next.
    pub(super) async fn queue(&self, id: SpotifyId) -> Result<(), ConnectError> {
        let track_uri = format!("spotify:track:{}", id.to_base62());
        let device_id = self.device.load().await;

        let result = self
            .spotify
            .me_player_queue(device_id.as_deref(), &track_uri)
            .await;

        ConnectError::handle(result, "queue")
    }

    /// Internal function to modify the volume of the player.
    pub(super) async fn volume(&self, modify: player::ModifyVolume) -> u32 {
        let volume = self.volume.load().await;
        let update = modify.apply(volume);

        let result = self
            .settings
            .set("volume", update)
            .await
            .map_err(|e| ConnectError::Error("update volume settings", e.into()));

        if let Err(e) = result {
            log_error!(e, "failed to store updated volume in settings");
        }

        update
    }

    async fn volume_update(&self, volume: u32) -> Result<(), ConnectError> {
        let volume = (volume as f32) / 100f32;
        let device_id = self.device.load().await;
        ConnectError::handle(
            self.spotify
                .me_player_volume(device_id.as_deref(), volume)
                .await,
            "volume",
        )
    }
}

pub(super) struct ConnectStream {
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
}

impl ConnectStream {
    pub(crate) async fn recv(&mut self) -> player::IntegrationEvent {
        loop {
            if let Some(event) = self.config_rx.recv().await {
                return match event {
                    ConfigurationEvent::DeviceChanged => player::IntegrationEvent::DeviceChanged,
                };
            }
        }
    }
}

pub(super) enum ConfigurationEvent {
    /// Indicate that the current device has been changed.
    DeviceChanged,
}

#[derive(Clone)]
pub(super) struct ConnectDevice {
    spotify: Arc<api::Spotify>,
    pub(super) device: settings::Var<Option<String>>,
    settings: crate::Settings,
}

impl ConnectDevice {
    /// Synchronize the device.
    pub(super) async fn sync_device(&self, device: Option<api::spotify::Device>) -> Result<()> {
        match (self.device.read().await.as_ref(), device.as_ref()) {
            (None, None) => return Ok(()),
            (Some(a), Some(b)) if *a == b.id => return Ok(()),
            _ => (),
        };

        self.settings.set("device", device.map(|d| d.id)).await?;
        Ok(())
    }

    /// Get the current device.
    pub(super) async fn current_device(&self) -> Option<String> {
        self.device.load().await
    }

    /// List all available devices.
    pub(super) async fn list_devices(&self) -> Result<Vec<api::spotify::Device>> {
        self.spotify.my_player_devices().await
    }

    /// Set which device to perform playback from.
    pub(super) async fn set_device(&self, device: Option<String>) -> Result<()> {
        self.settings.set("device", device).await?;
        Ok(())
    }
}

/// Discards a result and log a warning on errors.
fn warn_on_error<T, E>(result: Result<T, E>)
where
    Error: From<E>,
{
    if let Err(e) = result {
        log_warn!(e, "failed to issue connect command");
    }
}
