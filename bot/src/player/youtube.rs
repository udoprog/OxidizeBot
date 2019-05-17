use crate::{bus, player, settings::ScopedSettings};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_core::reactor::Core;

/// Setup a player.
pub fn setup(
    core: &mut Core,
    bus: Arc<bus::Bus<bus::YouTube>>,
    settings: ScopedSettings,
) -> Result<YouTubePlayer, failure::Error> {
    let volume_scale = settings.sync_var(core, "volume-scale", 100)?;

    Ok(YouTubePlayer { bus, volume_scale })
}

pub struct YouTubePlayer {
    bus: Arc<bus::Bus<bus::YouTube>>,
    volume_scale: Arc<RwLock<u32>>,
}

impl YouTubePlayer {
    /// Update playback information.
    pub fn tick(&mut self, song: &player::Song, video_id: &str) {
        let event = bus::YouTubeEvent::Play {
            video_id: video_id.to_string(),
            elapsed: song.elapsed().as_secs(),
            duration: song.duration().as_secs(),
        };

        self.bus.send(bus::YouTube::YouTubeCurrent { event });
    }

    pub fn play(&mut self, song: &player::Song, video_id: &str) {
        let event = bus::YouTubeEvent::Play {
            video_id: video_id.to_string(),
            elapsed: song.elapsed().as_secs(),
            duration: song.duration().as_secs(),
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
