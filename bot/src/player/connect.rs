use crate::{api, player, prelude::*, settings::Settings, track_id::SpotifyId, utils::Futures};
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
) -> Result<(ConnectPlayer, ConnectDevice), failure::Error> {
    let device = Arc::new(RwLock::new(None));

    let mut vars = settings.vars();
    let volume_scale = vars.var("volume-scale", 100)?;
    futures.push(vars.run().boxed());

    let (config_tx, config_rx) = mpsc::unbounded();

    let player = ConnectPlayer {
        spotify: spotify.clone(),
        device: device.clone(),
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
    /// Receiver for configuration events.
    config_rx: mpsc::UnboundedReceiver<ConfigurationEvent>,
    /// Scale to use for volume.
    volume_scale: Arc<RwLock<u32>>,
}

#[derive(Debug, Clone, Copy, err_derive::Error)]
pub enum CommandError {
    #[error(display = "error when issuing {} command", _0)]
    Error(&'static str),
    #[error(display = "no device configured")]
    NoDevice,
}

impl CommandError {
    fn handle(
        result: Result<bool, failure::Error>,
        what: &'static str,
    ) -> Result<(), CommandError> {
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

impl ConnectPlayer {
    /// Play the specified song.
    pub async fn play(&self, elapsed: Duration, id: SpotifyId) -> Result<(), CommandError> {
        let track_uri = format!("spotify:track:{}", id.to_base62());
        let device_id = self.device.read().as_ref().map(|d| d.id.to_string());

        let result = self
            .spotify
            .me_player_play(device_id, Some(track_uri), Some(elapsed.as_millis() as u64))
            .await;

        CommandError::handle(result, "play")
    }

    pub async fn pause(&self) -> Result<(), CommandError> {
        let device_id = self.device.read().as_ref().map(|d| d.id.to_string());
        CommandError::handle(self.spotify.me_player_pause(device_id).await, "pause")
    }

    pub async fn stop(&self) -> Result<(), CommandError> {
        let device_id = self.device.read().as_ref().map(|d| d.id.to_string());
        CommandError::handle(self.spotify.me_player_pause(device_id).await, "stop")
    }

    pub async fn volume(&self, volume: u32) -> Result<(), CommandError> {
        let volume = (volume * *self.volume_scale.read()) / 100u32;
        let volume = (volume as f32) / 100f32;
        let device_id = self.device.read().as_ref().map(|d| d.id.to_string());
        CommandError::handle(
            self.spotify.me_player_volume(device_id, volume).await,
            "volume",
        )
    }
}

impl Stream for ConnectPlayer {
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

impl stream::FusedStream for ConnectPlayer {
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
    config_tx: mpsc::UnboundedSender<ConfigurationEvent>,
    pub device: Arc<RwLock<Option<api::spotify::Device>>>,
}

impl ConnectDevice {
    /// Get the current device.
    pub fn current_device(&self) -> Option<api::spotify::Device> {
        self.device.read().clone()
    }

    /// List all available devices.
    pub async fn list_devices(&self) -> Result<Vec<api::spotify::Device>, failure::Error> {
        self.spotify.my_player_devices().await
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
