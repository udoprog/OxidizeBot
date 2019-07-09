use crate::{player, track_id::TrackId};
use hashbrown::HashMap;
use parking_lot::Mutex;
use std::sync::Arc;

pub trait Message: 'static + Clone + Send + Sync + serde::Serialize {
    /// The ID of a bussed message.
    fn id(&self) -> Option<&'static str> {
        None
    }
}

pub type Reader<T> = tokio_bus::BusReader<T>;

struct Inner<T>
where
    T: Message,
{
    bus: tokio_bus::Bus<T>,
    /// Latest instances of all messages.
    latest: HashMap<&'static str, T>,
}

/// Bus system.
pub struct Bus<T>
where
    T: Message,
{
    bus: Mutex<Inner<T>>,
}

impl<T> Bus<T>
where
    T: Message,
{
    /// Create a new notifier.
    pub fn new() -> Self {
        Bus {
            bus: Mutex::new(Inner {
                bus: tokio_bus::Bus::new(1024),
                latest: HashMap::new(),
            }),
        }
    }

    /// Send a message to the bus.
    pub fn send(&self, m: T) {
        let mut inner = self.bus.lock();

        if let Some(key) = m.id() {
            inner.latest.insert(key, m.clone());
        }

        if let Err(_) = inner.bus.try_broadcast(m) {
            log::error!("failed to send notification: bus is full");
        }
    }

    /// Get the latest messages received.
    pub fn latest(&self) -> Vec<T> {
        let inner = self.bus.lock();
        inner.latest.values().cloned().collect()
    }

    /// Create a receiver of the bus.
    pub fn add_rx(self: Arc<Self>) -> Reader<T> {
        self.bus.lock().bus.add_rx()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum YouTubeEvent {
    /// Play a new song.
    #[serde(rename = "play")]
    Play {
        video_id: String,
        elapsed: u64,
        duration: u64,
    },
    /// Pause the player.
    #[serde(rename = "pause")]
    Pause,
    /// Stop the player.
    #[serde(rename = "stop")]
    Stop,
}

/// Events for driving the YouTube player.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum YouTube {
    #[serde(rename = "youtube/current")]
    YouTubeCurrent { event: YouTubeEvent },
    #[serde(rename = "youtube/volume")]
    YouTubeVolume { volume: u32 },
}

impl Message for YouTube {
    /// Whether a message should be cached or not and under what key.
    fn id(&self) -> Option<&'static str> {
        use self::YouTube::*;

        match *self {
            YouTubeCurrent { .. } => Some("youtube/current"),
            YouTubeVolume { .. } => Some("youtube/volume"),
        }
    }
}

/// Messages that go on the global bus.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum Global {
    #[serde(rename = "firework")]
    Firework,
    #[serde(rename = "ping")]
    Ping,
    /// Progress of current song.
    #[serde(rename = "song/progress")]
    SongProgress {
        track_id: Option<TrackId>,
        elapsed: u64,
        duration: u64,
    },
    #[serde(rename = "song/current")]
    SongCurrent {
        track_id: Option<TrackId>,
        track: Option<player::Track>,
        user: Option<String>,
        is_playing: bool,
        elapsed: u64,
        duration: u64,
    },
    #[serde(rename = "song/modified")]
    SongModified,
}

impl Message for Global {
    /// Whether a message should be cached or not and under what key.
    fn id(&self) -> Option<&'static str> {
        use self::Global::*;

        match *self {
            SongProgress { .. } => Some("song/progress"),
            SongCurrent { .. } => Some("song/current"),
            _ => None,
        }
    }
}

impl Global {
    /// Construct a message about song progress.
    pub fn song_progress(song: Option<&player::Song>) -> Self {
        let song = match song {
            Some(song) => song,
            None => {
                return Global::SongProgress {
                    track_id: None,
                    elapsed: 0,
                    duration: 0,
                }
            }
        };

        Global::SongProgress {
            track_id: Some(song.item.track_id.clone()),
            elapsed: song.elapsed().as_secs(),
            duration: song.duration().as_secs(),
        }
    }

    /// Construct a message that the given song is running.
    pub fn song(song: Option<&player::Song>) -> Result<Self, failure::Error> {
        let song = match song {
            Some(song) => song,
            None => {
                return Ok(Global::SongCurrent {
                    track_id: None,
                    track: None,
                    user: None,
                    is_playing: false,
                    elapsed: 0,
                    duration: 0,
                })
            }
        };

        Ok(Global::SongCurrent {
            track_id: Some(song.item.track_id.clone()),
            track: Some(song.item.track.clone()),
            user: song.item.user.clone(),
            is_playing: song.state() == player::State::Playing,
            elapsed: song.elapsed().as_secs(),
            duration: song.duration().as_secs(),
        })
    }
}
