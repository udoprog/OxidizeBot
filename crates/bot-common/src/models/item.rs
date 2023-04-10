use crate::display;
use crate::models::{Track, TrackId};

#[derive(Debug, Clone)]
pub struct Item {
    track_id: TrackId,
    track: Track,
    user: Option<String>,
    duration: std::time::Duration,
}

impl Item {
    pub fn new(
        track_id: TrackId,
        track: Track,
        user: Option<String>,
        duration: std::time::Duration,
    ) -> Self {
        Self {
            track_id,
            track,
            user,
            duration,
        }
    }

    /// Human readable version of playback item.
    pub fn what(&self) -> String {
        match &self.track {
            Track::Spotify { track } => {
                if let Some(artists) = display::human_artists(&track.artists) {
                    format!("\"{}\" by {}", track.name, artists)
                } else {
                    format!("\"{}\"", track.name)
                }
            }
            Track::YouTube { video } => match video.snippet.as_ref() {
                Some(snippet) => match snippet.channel_title.as_ref() {
                    Some(channel_title) => {
                        format!("\"{}\" from \"{}\"", snippet.title, channel_title)
                    }
                    None => format!("\"{}\"", snippet.title),
                },
                None => String::from("*Some YouTube Video*"),
            },
        }
    }

    /// Test if the given item is playable.
    pub fn is_playable(&self) -> bool {
        match &self.track {
            Track::Spotify { track } => track.is_playable.unwrap_or(true),
            Track::YouTube { video: _ } => true,
        }
    }

    /// Get the track identifier for the current song.
    #[inline]
    pub fn track_id(&self) -> &TrackId {
        &self.track_id
    }

    /// Get the track for the current song.
    #[inline]
    pub fn track(&self) -> &Track {
        &self.track
    }

    /// Get the name of the user that requested the song.
    #[inline]
    pub fn user(&self) -> Option<&String> {
        self.user.as_ref()
    }

    /// Duration of the current song.
    #[inline]
    pub fn duration(&self) -> std::time::Duration {
        self.duration
    }

    #[inline]
    pub fn set_duration(&mut self, duration: std::time::Duration) {
        self.duration = duration;
    }
}
