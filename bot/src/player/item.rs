use super::track::Track;
use crate::{track_id::TrackId, utils};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Item {
    pub track_id: TrackId,
    pub track: Track,
    pub user: Option<String>,
    pub duration: Duration,
}

impl Item {
    /// Human readable version of playback item.
    pub fn what(&self) -> String {
        match self.track {
            Track::Spotify { ref track } => {
                if let Some(artists) = utils::human_artists(&track.artists) {
                    format!("\"{}\" by {}", track.name, artists)
                } else {
                    format!("\"{}\"", track.name)
                }
            }
            Track::YouTube { ref video } => match video.snippet.as_ref() {
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

    pub fn is_playable(&self) -> bool {
        match self.track {
            Track::Spotify { ref track } => {
                match track.is_playable {
                    Some(is_playable) => return is_playable,
                    None => return false,
                };
            }
            Track::YouTube { video: _ } => {
                return true;
            }
        }
    }
}
