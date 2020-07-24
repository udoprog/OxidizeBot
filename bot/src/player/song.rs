use crate::api;
use crate::player::{Item, PlayerKind, State, Track};
use crate::spotify_id::SpotifyId;
use crate::track_id::TrackId;
use crate::utils;
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Information on current song.
#[derive(Debug, Clone)]
pub struct Song {
    pub item: Arc<Item>,
    /// Since the last time it was unpaused, what was the initial elapsed duration.
    elapsed: Duration,
    /// When the current song started playing.
    started_at: Option<Instant>,
}

impl Song {
    /// Create a new current song.
    pub fn new(item: Arc<Item>, elapsed: Duration) -> Self {
        Self {
            item,
            elapsed,
            started_at: None,
        }
    }

    /// Test if the two songs reference roughly the same song.
    pub fn is_same(&self, song: &Self) -> bool {
        if self.item.track_id != song.item.track_id {
            return false;
        }

        let a = self.elapsed();
        let b = song.elapsed();
        let diff = if a > b { a - b } else { b - a };

        if diff.as_secs() > 5 {
            return false;
        }

        true
    }

    /// Convert a playback information into a Song struct.
    pub fn from_playback(playback: &api::spotify::FullPlayingContext) -> Option<Self> {
        let progress_ms = playback.progress_ms.unwrap_or_default();

        let track = match playback.item.clone() {
            Some(track) => track,
            _ => {
                log::warn!("No playback item in current playback");
                return None;
            }
        };

        let track_id = match &track.id {
            Some(track_id) => track_id,
            None => {
                log::warn!("Current playback doesn't have a track id");
                return None;
            }
        };

        let track_id = match SpotifyId::from_base62(&track_id) {
            Ok(spotify_id) => TrackId::Spotify(spotify_id),
            Err(e) => {
                log::warn!(
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
            track: Track::Spotify { track },
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
    pub fn deadline(&self) -> Instant {
        Instant::now() + self.remaining()
    }

    /// Duration of the current song.
    pub fn duration(&self) -> Duration {
        self.item.duration
    }

    /// Elapsed time on current song.
    ///
    /// Elapsed need to take started at into account.
    pub fn elapsed(&self) -> Duration {
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
    pub fn remaining(&self) -> Duration {
        self.item
            .duration
            .checked_sub(self.elapsed())
            .unwrap_or_default()
    }

    /// Get serializable data for this item.
    pub fn data(&self, state: State) -> Result<CurrentData<'_>> {
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
    pub fn state(&self) -> State {
        if self.started_at.is_some() {
            State::Playing
        } else {
            State::Paused
        }
    }

    /// Get the player kind for the current song.
    pub fn player(&self) -> PlayerKind {
        match self.item.track_id {
            TrackId::Spotify(..) => PlayerKind::Spotify,
            TrackId::YouTube(..) => PlayerKind::YouTube,
        }
    }

    /// Set the started_at time to now.
    /// For safety, update the current `elapsed` time based on any prior `started_at`.
    pub fn play(&mut self) {
        let duration = self.take_started_at();
        self.elapsed += duration;
        self.started_at = Some(Instant::now());
    }

    /// Update the elapsed time based on when this song was started.
    pub fn pause(&mut self) {
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
pub struct CurrentData<'a> {
    paused: bool,
    track_id: &'a TrackId,
    name: String,
    artists: Option<String>,
    user: Option<&'a str>,
    duration: String,
    elapsed: String,
}
