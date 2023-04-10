use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use crate::display;
use crate::models::{Track, Item, TrackId, SpotifyId, PlayerKind, State};

/// Information on current song.
#[derive(Clone)]
pub struct Song {
    item: Arc<Item>,
    /// Since the last time it was unpaused, what was the initial elapsed duration.
    elapsed: std::time::Duration,
    /// When the current song started playing.
    started_at: Option<Instant>,
}

impl fmt::Debug for Song {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Song")
            .field("track_id", &self.item.track_id())
            .field("user", &self.item.user())
            .field("duration", &self.item.duration())
            .field("elapsed", &self.elapsed)
            .field("started_at", &self.started_at)
            .finish_non_exhaustive()
    }
}

impl Song {
    /// Create a new current song.
    pub fn new(item: Arc<Item>, elapsed: std::time::Duration) -> Self {
        Self {
            item,
            elapsed,
            started_at: None,
        }
    }

    /// Get the deadline for when this song will end, assuming it is currently playing.
    #[inline]
    pub fn deadline(&self) -> Instant {
        Instant::now() + self.remaining()
    }

    /// Access the item corresponding the song.
    #[inline]
    pub fn item(&self) -> &Arc<Item> {
        &self.item
    }

    /// Convert song into underlying item.
    #[inline]
    pub fn into_item(self) -> Arc<Item> {
        self.item
    }

    /// Elapsed time on current song.
    ///
    /// Elapsed need to take started at into account.
    pub fn elapsed(&self) -> std::time::Duration {
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
    pub fn remaining(&self) -> std::time::Duration {
        self.item
            .duration()
            .checked_sub(self.elapsed())
            .unwrap_or_default()
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
        match self.item.track_id() {
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
    fn take_started_at(&mut self) -> std::time::Duration {
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
