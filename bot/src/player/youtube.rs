use crate::{bus, player, prelude::*, settings::Settings};
use anyhow::Error;
use std::{sync::Arc, time::Duration};

/// Setup a player.
pub(super) async fn setup(
    bus: Arc<bus::Bus<bus::YouTube>>,
    settings: Settings,
) -> Result<(YouTubePlayer, impl Future<Output = Result<(), Error>>), anyhow::Error> {
    let (mut volume_scale_stream, mut volume_scale) =
        settings.stream("volume-scale").or_with(100).await?;
    let (mut volume_stream, volume) = settings.stream("volume").or_with(50).await?;
    let mut scaled_volume = (volume * volume_scale) / 100u32;
    let volume = injector::Var::new(volume);

    let player = YouTubePlayer {
        bus,
        settings,
        volume: volume.clone(),
    };

    let returned_player = player.clone();

    let future = async move {
        player.volume_update(scaled_volume).await;

        loop {
            futures::select! {
                update = volume_scale_stream.select_next_some() => {
                    volume_scale = update;
                    scaled_volume = (volume.load().await * volume_scale) / 100u32;
                    player.volume_update(scaled_volume).await;
                }
                update = volume_stream.select_next_some() => {
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
    bus: Arc<bus::Bus<bus::YouTube>>,
    settings: Settings,
    volume: injector::Var<u32>,
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

    pub(super) async fn volume(&self, modify: player::ModifyVolume) -> Result<u32, Error> {
        let mut volume = self.volume.write().await;
        let update = modify.apply(*volume);
        *volume = update;
        self.settings.set("volume", update).await?;
        Ok(update)
    }

    pub(super) async fn current_volume(&self) -> u32 {
        self.volume.load().await
    }

    async fn volume_update(&self, volume: u32) {
        self.bus.send(bus::YouTube::YouTubeVolume { volume }).await;
    }
}
