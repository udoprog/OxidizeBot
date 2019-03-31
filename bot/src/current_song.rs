use crate::{player, template, utils};
use std::{fs::File, path::PathBuf, time};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CurrentSong {
    /// Path to render current song at.
    pub path: PathBuf,
    /// Message to render when a song is playing.
    template: template::Template,
    /// Message to show when no song is playing.
    #[serde(default)]
    not_playing: Option<String>,
    /// Update frequency.
    #[serde(default, deserialize_with = "utils::deserialize_duration")]
    update_interval: time::Duration,
}

impl CurrentSong {
    /// Either creates or truncates the current song file.
    fn create_or_truncate(&self) -> Result<File, failure::Error> {
        File::create(&self.path).map_err(Into::into)
    }

    /// Blank the current file.
    pub fn blank(&self) -> Result<(), failure::Error> {
        use std::io::Write as _;
        let mut f = self.create_or_truncate()?;

        if let Some(not_playing) = self.not_playing.as_ref() {
            write!(f, "{}", not_playing)?;
        } else {
            write!(f, "Not Playing")?;
        }

        Ok(())
    }

    /// Blank as a string.
    pub fn blank_to_string(&self) -> String {
        if let Some(not_playing) = self.not_playing.as_ref() {
            not_playing.to_string()
        } else {
            "Not Playing".to_string()
        }
    }

    /// Write the current song to a path.
    pub fn write(&self, song: &player::Song, paused: bool) -> Result<(), failure::Error> {
        let mut f = self.create_or_truncate()?;
        let data = song.data(paused)?;
        self.template.render(&mut f, &data)?;
        Ok(())
    }

    /// Format as string.
    pub fn write_to_string(
        &self,
        song: &player::Song,
        paused: bool,
    ) -> Result<String, failure::Error> {
        let data = song.data(paused)?;
        self.template.render_to_string(&data)
    }

    /// Get the current update frequency, if present.
    pub fn update_interval(&self) -> Option<&time::Duration> {
        if self.update_interval.as_secs() == 0 {
            return None;
        }

        Some(&self.update_interval)
    }
}
