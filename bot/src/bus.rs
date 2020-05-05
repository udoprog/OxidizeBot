use crate::{player, track_id::TrackId};
use futures::channel::mpsc;
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};

pub trait Message: 'static + Clone + Send + Sync + serde::Serialize {
    /// The ID of a bussed message.
    fn id(&self) -> Option<&'static str> {
        None
    }
}

pub type Reader<T> = mpsc::Receiver<T>;

struct Inner<T> {
    subs: Vec<mpsc::Sender<T>>,
    /// Latest instances of all messages.
    latest: HashMap<&'static str, T>,
}

/// Bus system.
#[derive(Clone)]
pub struct Bus<T> {
    inner: Arc<Mutex<Inner<T>>>,
}

impl<T> Bus<T> {
    /// Create a new notifier.
    pub fn new() -> Self {
        Bus {
            inner: Arc::new(Mutex::new(Inner {
                subs: Vec::new(),
                latest: HashMap::new(),
            })),
        }
    }

    /// Send a message to the bus.
    pub fn send(&self, m: T)
    where
        T: Message,
    {
        let mut inner = self.inner.lock();

        if let Some(key) = m.id() {
            inner.latest.insert(key, m.clone());
        }

        self.send_inner(&mut inner, m);
    }

    /// Send a synced and cloneable message.
    pub fn send_sync(&self, m: T)
    where
        T: 'static + Clone + Send + Sync,
    {
        let mut inner = self.inner.lock();
        self.send_inner(&mut inner, m);
    }

    /// Send a synced and cloneable message.
    fn send_inner(&self, inner: &mut Inner<T>, m: T)
    where
        T: 'static + Clone + Send + Sync,
    {
        let mut remove = smallvec::SmallVec::<[usize; 16]>::new();

        for (i, s) in inner.subs.iter_mut().enumerate() {
            match s.try_send(m.clone()) {
                Err(e) if e.is_disconnected() => {
                    remove.push(i);
                }
                Err(e) if e.is_full() => (),
                Err(e) => {
                    log::error!("Error sending: {}", e);
                }
                Ok(()) => (),
            }
        }

        for i in remove.into_iter().rev() {
            inner.subs.swap_remove(i);
        }
    }

    /// Get the latest messages received.
    pub fn latest(&self) -> Vec<T>
    where
        T: Clone,
    {
        let inner = self.inner.lock();
        inner.latest.values().cloned().collect()
    }

    /// Create a receiver of the bus.
    pub fn add_rx(&self) -> Reader<T> {
        let mut inner = self.inner.lock();
        let (tx, rx) = mpsc::channel(1024);
        inner.subs.push(tx);
        rx
    }
}

impl<T> Default for Bus<T> {
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
