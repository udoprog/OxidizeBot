use crate::player;
use crate::track_id::TrackId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

pub trait Message: 'static + Clone + Send + Sync + serde::Serialize {
    /// The ID of a bussed message.
    fn id(&self) -> Option<&'static str> {
        None
    }
}

pub type Reader<T> = broadcast::Receiver<T>;

struct Inner<T>
where
    T: Clone,
{
    subs: broadcast::Sender<T>,
    /// Latest instances of all messages.
    latest: RwLock<HashMap<&'static str, T>>,
}

/// Bus system.
#[derive(Clone)]
pub struct Bus<T>
where
    T: Clone,
{
    inner: Arc<Inner<T>>,
}

impl<T> Bus<T>
where
    T: Clone,
{
    /// Create a new notifier.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                subs: broadcast::channel(64).0,
                latest: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Send a message through the bus.
    pub async fn send(&self, m: T)
    where
        T: Message,
    {
        if let Some(key) = m.id() {
            let mut latest = self.inner.latest.write().await;
            latest.insert(key, m.clone());
        }

        let _ = self.inner.subs.send(m);
    }

    /// Send a synced and cloneable message.
    pub fn send_sync(&self, m: T)
    where
        T: 'static + Clone + Send + Sync,
    {
        let _ = self.inner.subs.send(m);
    }

    /// Get the latest messages received.
    pub async fn latest(&self) -> Vec<T>
    where
        T: Clone,
    {
        let latest = self.inner.latest.read().await;
        latest.values().cloned().collect()
    }

    /// Create a receiver of the bus.
    pub fn subscribe(&self) -> Reader<T> {
        self.inner.subs.subscribe()
    }
}

impl<T> Default for Bus<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new()
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
                };
            }
        };

        Global::SongProgress {
            track_id: Some(song.item.track_id.clone()),
            elapsed: song.elapsed().as_secs(),
            duration: song.duration().as_secs(),
        }
    }

    /// Construct a message that the given song is running.
    pub fn song(song: Option<&player::Song>) -> Result<Self, anyhow::Error> {
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
                });
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

/// Events for running commands externally.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum Command {
    /// Run a raw command.
    #[serde(rename = "raw")]
    Raw { command: String },
}

impl Message for Command {
    /// Whether a message should be cached or not and under what key.
    fn id(&self) -> Option<&'static str> {
        None
    }
}
