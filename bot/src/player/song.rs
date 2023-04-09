use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::api;
use crate::player::{Item, PlayerKind, State, Track};
use crate::spotify_id::SpotifyId;
use crate::track_id::TrackId;
use crate::utils;

/// Information on current song.
#[derive(Clone)]
pub(crate) struct Song {
    pub(crate) item: Arc<Item>,
    /// Since the last time it was unpaused, what was the initial elapsed duration.
    elapsed: Duration,
    /// When the current song started playing.
    started_at: Option<Instant>,
}

impl fmt::Debug for Song {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Song")
            .field("track_id", &self.item.track_id)
            .field("user", &self.item.user)
            .field("duration", &self.item.duration)
            .field("elapsed", &self.elapsed)
            .field("started_at", &self.started_at)
            .finish_non_exhaustive()
    }
}

impl Song {
    /// Create a new current song.
    pub(crate) fn new(item: Arc<Item>, elapsed: Duration) -> Self {
        Self {
            item,
            elapsed,
            started_at: None,
        }
    }

    /// Convert a playback information into a Song struct.
    pub(crate) fn from_playback(playback: &api::spotify::FullPlayingContext) -> Option<Self> {
        let progress_ms = playback.progress_ms.unwrap_or_default();

        let track = match playback.item.clone() {
            Some(track) => track,
            _ => {
                tracing::warn!("No playback item in current playback");
                return None;
            }
        };

        let track_id = match &track.id {
            Some(track_id) => track_id,
            None => {
                tracing::warn!("Current playback doesn't have a track id");
                return None;
            }
        };

        let track_id = match SpotifyId::from_base62(track_id) {
            Ok(spotify_id) => TrackId::Spotify(spotify_id),
            Err(e) => {
                tracing::warn!(
                    "Failed to parse track id from current playback: {}: {}",
                    track_id,
                    e
                );
                return None;
            }
        };

        let elapsed = Duration::from_millis(progress_ms as u64);
        let duration = Duration::from_millis(track.duration_ms.into());

        let item = Arc::new(Item {
            track_id,
            track: Track::Spotify {
                track: Box::new(track),
            },
            user: None,
            duration,
        });

        let mut song = Song::new(item, elapsed);

        if playback.is_playing {
            song.play();
        } else {
            song.pause();
        }

        Some(song)
    }

    /// Get the deadline for when this song will end, assuming it is currently playing.
    pub(crate) fn deadline(&self) -> Instant {
        Instant::now() + self.remaining()
    }

    /// Duration of the current song.
    pub(crate) fn duration(&self) -> Duration {
        self.item.duration
    }

    /// Elapsed time on current song.
    ///
    /// Elapsed need to take started at into account.
    pub(crate) fn elapsed(&self) -> Duration {
        let when = self
            .started_at
            .and_then(|started_at| {
                let now = Instant::now();

                if now > started_at {
                    Some(now - started_at)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        when.checked_add(self.elapsed).unwrap_or_default()
    }

    /// Remaining time of the current song.
    pub(crate) fn remaining(&self) -> Duration {
        self.item
            .duration
            .checked_sub(self.elapsed())
            .unwrap_or_default()
    }

    /// Get serializable data for this item.
    pub(crate) fn data(&self, state: State) -> Result<CurrentData<'_>> {
        let artists = self.item.track.artists();

        Ok(CurrentData {
            paused: state != State::Playing,
            track_id: &self.item.track_id,
            name: self.item.track.name(),
            artists,
            user: self.item.user.as_deref(),
            duration: utils::digital_duration(self.item.duration),
            elapsed: utils::digital_duration(self.elapsed()),
        })
    }

    /// Check if the song is currently playing.
    pub(crate) fn state(&self) -> State {
        if self.started_at.is_some() {
            State::Playing
        } else {
            State::Paused
        }
    }

    /// Get the player kind for the current song.
    pub(crate) fn player(&self) -> PlayerKind {
        match self.item.track_id {
            TrackId::Spotify(..) => PlayerKind::Spotify,
            TrackId::YouTube(..) => PlayerKind::YouTube,
        }
    }

    /// Set the started_at time to now.
    /// For safety, update the current `elapsed` time based on any prior `started_at`.
    pub(crate) fn play(&mut self) {
        let duration = self.take_started_at();
        self.elapsed += duration;
        self.started_at = Some(Instant::now());
    }

    /// Update the elapsed time based on when this song was started.
    pub(crate) fn pause(&mut self) {
        let duration = self.take_started_at();
        self.elapsed += duration;
    }

    /// Take the current started_at as a duration and leave it as None.
    fn take_started_at(&mut self) -> Duration {
        let started_at = match self.started_at.take() {
            Some(started_at) => started_at,
            None => return Default::default(),
        };

        let now = Instant::now();

        if now < started_at {
            return Default::default();
        }

        now - started_at
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct CurrentData<'a> {
    paused: bool,
    track_id: &'a TrackId,
    name: String,
    artists: Option<String>,
    user: Option<&'a str>,
    duration: String,
    elapsed: String,
}
