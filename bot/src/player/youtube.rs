use crate::{bus, settings::ScopedSettings};
use parking_lot::RwLock;
use std::{sync::Arc, time::Duration};

/// Setup a player.
pub fn setup(
    bus: Arc<bus::Bus<bus::YouTube>>,
    settings: ScopedSettings,
) -> Result<YouTubePlayer, failure::Error> {
    let volume_scale = settings.sync_var("volume-scale", 100)?;

    Ok(YouTubePlayer { bus, volume_scale })
}

pub struct YouTubePlayer {
    bus: Arc<bus::Bus<bus::YouTube>>,
    volume_scale: Arc<RwLock<u32>>,
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

    pub fn play(&mut self, elapsed: Duration, duration: Duration, video_id: String) {
        let event = bus::YouTubeEvent::Play {
            video_id,
            elapsed: elapsed.as_secs(),
            duration: duration.as_secs(),
        };

        self.bus.send(bus::YouTube::YouTubeCurrent { event });
    }

    pub fn pause(&mut self) {
        let event = bus::YouTubeEvent::Pause;
        self.bus.send(bus::YouTube::YouTubeCurrent { event });
    }

    pub fn stop(&mut self) {
        let event = bus::YouTubeEvent::Stop;
        self.bus.send(bus::YouTube::YouTubeCurrent { event });
    }

    pub fn volume(&mut self, volume: u32) {
        let volume = (volume * *self.volume_scale.read()) / 100u32;
        self.bus.send(bus::YouTube::YouTubeVolume { volume });
    }
}
