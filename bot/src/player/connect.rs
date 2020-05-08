use crate::{api, player, prelude::*, settings::Settings, track_id::SpotifyId};
use anyhow::{bail, Error};
use std::sync::Arc;
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use thiserror::Error;

/// Setup a player.
pub(super) async fn setup(
    spotify: Arc<api::Spotify>,
    settings: Settings,
) -> Result<
    (
        ConnectStream,
        ConnectPlayer,
        ConnectDevice,
        impl Future<Output = Result<(), Error>>,
    ),
    Error,
> {
    let (mut volume_stream, volume) = settings.stream("volume").or_with(50).await?;
    let (mut volume_scale_stream, volume_scale) =
        settings.stream("volume-scale").or_with(100).await?;
    let (mut device_stream, device) = settings.stream::<String>("device").optional().await?;

    // Locally scaled volume.
    let mut scaled_volume = (volume * volume_scale) / 100u32;

    let device = injector::Var::new(device);
    let volume = injector::Var::new(volume);
    let volume_scale = injector::Var::new(volume_scale);

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
        settings,
    };

    let returned_player = player.clone();

    let future = async move {
        player.volume_update_log(scaled_volume).await;

        loop {
            futures::select! {
                update = device_stream.select_next_some() => {
                    *device.write().await = update;

                    if config_tx.unbounded_send(ConfigurationEvent::DeviceChanged).is_err() {
                        bail!("failed to send configuration event");
                    }
                }
                update = volume_scale_stream.select_next_some() => {
                    *volume_scale.write().await = update;
                    scaled_volume = (volume.load().await * update) / 100u32;
                    player.volume_update_log(scaled_volume).await;
                }
                update = volume_stream.select_next_some() => {
                    *volume.write().await = update;
                    scaled_volume = (update * volume_scale.load().await) / 100u32;
                    player.volume_update_log(scaled_volume).await;
                }
            }
        }
    };

    Ok((stream, returned_player, interface, future))
}

#[derive(Debug, Error)]
pub(super) enum ConnectError {
    #[error("no device configured or available")]
    NoDevice,
    #[error("error when issuing {0} command")]
    Error(&'static str, #[source] Error),
}

impl ConnectError {
    fn handle(result: Result<bool, Error>, what: &'static str) -> Result<(), ConnectError> {
        match result {
            Err(e) => Err(ConnectError::Error(what, e.into())),
            Ok(true) => Ok(()),
            Ok(false) => Err(ConnectError::NoDevice),
        }
    }
}

#[derive(Clone)]
pub(super) struct ConnectPlayer {
    spotify: Arc<api::Spotify>,
    /// Currently configured device.
    device: injector::Var<Option<String>>,
    /// Access to settings.
    settings: Settings,
    /// Current volume scale for this player.
    volume_scale: injector::Var<u32>,
    /// Current volume for this player.
    volume: injector::Var<u32>,
}

impl ConnectPlayer {
    /// Play the specified song.
    pub(super) async fn play(
        &self,
        id: Option<SpotifyId>,
        elapsed: Option<Duration>,
    ) -> Result<(), ConnectError> {
        let track_uri = id.map(|id| format!("spotify:track:{}", id.to_base62()));
        let elapsed = elapsed.map(|elapsed| elapsed.as_millis() as u64);
        let device_id = self.device.load().await;

        let result = self
            .spotify
            .me_player_play(device_id, track_uri, elapsed)
            .await;

        ConnectError::handle(result, "play")
    }

    /// Enqueue the specified song to play next.
    pub(super) async fn queue(&self, id: SpotifyId) -> Result<(), ConnectError> {
        let track_uri = format!("spotify:track:{}", id.to_base62());
        let device_id = self.device.load().await;

        let result = self.spotify.me_player_queue(device_id, track_uri).await;

        ConnectError::handle(result, "queue")
    }

    pub(super) async fn next(&self) -> Result<(), ConnectError> {
        let device_id = self.device.load().await;
        let result = self.spotify.me_player_next(device_id).await;
        ConnectError::handle(result, "skip")
    }

    pub(super) async fn pause(&self) -> Result<(), ConnectError> {
        let device_id = self.device.load().await;
        ConnectError::handle(self.spotify.me_player_pause(device_id).await, "pause")
    }

    pub(super) async fn stop(&self) -> Result<(), ConnectError> {
        let device_id = self.device.load().await;
        ConnectError::handle(self.spotify.me_player_pause(device_id).await, "stop")
    }

    /// Update an unscaled volume.
    pub(super) async fn set_scaled_volume(&self, scaled_volume: u32) -> Result<u32, ConnectError> {
        let volume_scale = self.volume_scale.load().await;
        let update = u32::min((scaled_volume * 100) / volume_scale, 100);
        self.volume(player::ModifyVolume::Set(update)).await
    }

    pub(super) async fn volume(&self, modify: player::ModifyVolume) -> Result<u32, ConnectError> {
        let mut volume = self.volume.write().await;
        let update = modify.apply(*volume);
        *volume = update;
        self.settings
            .set("volume", update)
            .map_err(|e| ConnectError::Error("update volume settings", e.into()))
            .await?;
        Ok(update)
    }

    pub(super) async fn current_volume(&self) -> u32 {
        self.volume.load().await
    }

    async fn volume_update(&self, volume: u32) -> Result<(), ConnectError> {
        let volume = (volume as f32) / 100f32;
        let device_id = self.device.load().await;
        ConnectError::handle(
            self.spotify.me_player_volume(device_id, volume).await,
            "volume",
        )
    }

    /// Same as volume update, but logs instead of errors.
    async fn volume_update_log(&self, volume: u32) {
        if let Err(e) = self.volume_update(volume).await {
            log_warn!(e, "Failed to update volume");
        }
    }
}

pub(super) struct ConnectStream {
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
}

impl Stream for ConnectStream {
    type Item = Result<player::IntegrationEvent, anyhow::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        use self::player::IntegrationEvent::*;

        if let Poll::Ready(Some(e)) = Pin::new(&mut self.config_rx).poll_next(cx) {
            match e {
                ConfigurationEvent::DeviceChanged => {
                    return Poll::Ready(Some(Ok(DeviceChanged)));
                }
            }
        }

        Poll::Pending
    }
}

impl stream::FusedStream for ConnectStream {
    fn is_terminated(&self) -> bool {
        false
    }
}

pub(super) enum ConfigurationEvent {
    /// Indicate that the current device has been changed.
    DeviceChanged,
}

#[derive(Clone)]
pub(super) struct ConnectDevice {
    spotify: Arc<api::Spotify>,
    pub(super) device: injector::Var<Option<String>>,
    settings: Settings,
}

impl ConnectDevice {
    /// Synchronize the device.
    pub(super) async fn sync_device(
        &self,
        device: Option<api::spotify::Device>,
    ) -> Result<(), Error> {
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
    pub(super) async fn list_devices(&self) -> Result<Vec<api::spotify::Device>, Error> {
        self.spotify.my_player_devices().await
    }

    /// Set which device to perform playback from.
    pub(super) async fn set_device(&self, device: Option<String>) -> Result<(), Error> {
        self.settings.set("device", device).await?;
        Ok(())
    }
}
