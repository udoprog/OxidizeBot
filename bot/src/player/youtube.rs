use crate::{bus, player, prelude::*, settings::Settings, utils::Futures};
use failure::Error;
use parking_lot::RwLock;
use std::{sync::Arc, time::Duration};

/// Setup a player.
pub fn setup(
    futures: &mut Futures,
    bus: Arc<bus::Bus<bus::YouTube>>,
    settings: Settings,
) -> Result<YouTubePlayer, failure::Error> {
    let (mut volume_scale_stream, mut volume_scale) =
        settings.stream("volume-scale").or_with(100)?;
    let (mut volume_stream, volume) = settings.stream("volume").or_with(50)?;
    let mut scaled_volume = (volume * volume_scale) / 100u32;
    let volume = Arc::new(RwLock::new(volume));

    let player = YouTubePlayer {
        bus,
        settings,
        volume: volume.clone(),
    };

    let returned_player = player.clone();

    let future = async move {
        player.volume_update(scaled_volume);

        loop {
            futures::select! {
                update = volume_scale_stream.select_next_some() => {
                    volume_scale = update;
                    scaled_volume = (*volume.read() * volume_scale) / 100u32;
                    player.volume_update(scaled_volume);
                }
                update = volume_stream.select_next_some() => {
                    *volume.write() = update;
                    scaled_volume = (*volume.read() * volume_scale) / 100u32;
                    player.volume_update(scaled_volume);
                }
            }
        }
    };

    futures.push(future.boxed());
    Ok(returned_player)
}

#[derive(Clone)]
pub struct YouTubePlayer {
    bus: Arc<bus::Bus<bus::YouTube>>,
    settings: Settings,
    volume: Arc<RwLock<u32>>,
}

impl YouTubePlayer {
    /// Update playback information.
    pub fn tick(&self, elapsed: Duration, duration: Duration, video_id: String) {
        let event = bus::YouTubeEvent::Play {
            video_id,
            elapsed: elapsed.as_secs(),
            duration: duration.as_secs(),
        };

        self.bus.send(bus::YouTube::YouTubeCurrent { event });
    }

    pub fn play(&self, elapsed: Duration, duration: Duration, video_id: String) {
        let event = bus::YouTubeEvent::Play {
            video_id,
            elapsed: elapsed.as_secs(),
            duration: duration.as_secs(),
        };

        self.bus.send(bus::YouTube::YouTubeCurrent { event });
    }

    pub fn pause(&self) {
        let event = bus::YouTubeEvent::Pause;
        self.bus.send(bus::YouTube::YouTubeCurrent { event });
    }

    pub fn stop(&self) {
        let event = bus::YouTubeEvent::Stop;
        self.bus.send(bus::YouTube::YouTubeCurrent { event });
    }

    pub fn volume(&self, modify: player::ModifyVolume) -> Result<u32, Error> {
        let mut volume = self.volume.write();
        let update = modify.apply(*volume);
        *volume = update;
        self.settings.set("volume", update)?;
        Ok(update)
    }

    pub fn current_volume(&self) -> u32 {
        *self.volume.read()
    }

    fn volume_update(&self, volume: u32) {
        self.bus.send(bus::YouTube::YouTubeVolume { volume });
    }
}
