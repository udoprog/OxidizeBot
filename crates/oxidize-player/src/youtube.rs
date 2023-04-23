use std::future::Future;
use std::time::Duration;

use anyhow::Result;

/// Setup a player.
pub(super) async fn setup(
    bus: bus::Bus<bus::YouTube>,
    settings: settings::Settings<::auth::Scope>,
) -> Result<(YouTubePlayer, impl Future<Output = Result<()>>)> {
    tracing::trace!("Setting up YouTube connection");

    let (mut volume_scale_stream, mut volume_scale) =
        settings.stream("volume-scale").or_with(100).await?;
    let (mut volume_stream, volume) = settings.stream("volume").or_with(50).await?;
    let mut scaled_volume = (volume * volume_scale) / 100u32;
    let volume = settings::Var::new(volume);

    let player = YouTubePlayer {
        bus,
        settings,
        volume: volume.clone(),
    };

    let returned_player = player.clone();

    let future = async move {
        player.volume_update(scaled_volume).await;

        loop {
            tokio::select! {
                update = volume_scale_stream.recv() => {
                    volume_scale = update;
                    scaled_volume = (volume.load().await * volume_scale) / 100u32;
                    player.volume_update(scaled_volume).await;
                }
                update = volume_stream.recv() => {
                    *volume.write().await = update;
                    scaled_volume = (volume.load().await * volume_scale) / 100u32;
                    player.volume_update(scaled_volume).await;
                }
            }
        }
    };

    Ok((returned_player, future))
}

#[derive(Clone)]
pub(super) struct YouTubePlayer {
    bus: bus::Bus<bus::YouTube>,
    settings: settings::Settings<::auth::Scope>,
    volume: settings::Var<u32>,
}

impl YouTubePlayer {
    /// Update playback information.
    pub(super) async fn tick(&self, elapsed: Duration, duration: Duration, video_id: String) {
        let event = bus::YouTubeEvent::Play {
            video_id,
            elapsed: elapsed.as_secs(),
            duration: duration.as_secs(),
        };

        self.bus.send(bus::YouTube::YouTubeCurrent { event }).await;
    }

    pub(super) async fn play(&self, elapsed: Duration, duration: Duration, video_id: String) {
        let event = bus::YouTubeEvent::Play {
            video_id,
            elapsed: elapsed.as_secs(),
            duration: duration.as_secs(),
        };

        self.bus.send(bus::YouTube::YouTubeCurrent { event }).await;
    }

    pub(super) async fn pause(&self) {
        let event = bus::YouTubeEvent::Pause;
        self.bus.send(bus::YouTube::YouTubeCurrent { event }).await;
    }

    pub(super) async fn stop(&self) {
        let event = bus::YouTubeEvent::Stop;
        self.bus.send(bus::YouTube::YouTubeCurrent { event }).await;
    }

    pub(super) async fn volume(&self, modify: crate::ModifyVolume) -> u32 {
        let mut volume = self.volume.write().await;
        let update = modify.apply(*volume);
        *volume = update;
        let result = self.settings.set("volume", update).await;

        if let Err(e) = result {
            common::log_error!(e, "Failed to store updated volume in settings");
        }

        update
    }

    pub(super) async fn current_volume(&self) -> u32 {
        self.volume.load().await
    }

    async fn volume_update(&self, volume: u32) {
        self.bus.send(bus::YouTube::YouTubeVolume { volume }).await;
    }
}
