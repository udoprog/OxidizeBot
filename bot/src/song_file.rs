use crate::{player, template::Template, timer, utils};
use std::{fs::File, path::PathBuf};

/// Configuration which has been deprecated.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub template: Option<Template>,
    #[serde(default)]
    pub not_playing: Option<Template>,
    #[serde(default)]
    pub update_interval: utils::Duration,
}

#[derive(Debug, Clone, Default)]
pub struct SongFileBuilder {
    pub enabled: bool,
    pub path: Option<PathBuf>,
    pub template: Option<Template>,
    pub stopped_template: Option<Template>,
    pub update_interval: utils::Duration,
}

impl SongFileBuilder {
    /// Construct a new SongFile handler if all the necessary options are available.
    pub fn build(&self) -> Option<SongFile> {
        if !self.enabled {
            return None;
        }

        let path = self.path.as_ref()?;
        let template = self.template.as_ref()?;
        let update_interval = match self.update_interval.is_empty() {
            false => &self.update_interval,
            true => return None,
        };

        let update_interval = timer::Interval::new_interval(update_interval.as_std());

        Some(SongFile {
            path: path.clone(),
            template: template.clone(),
            stopped_template: self.stopped_template.clone(),
            update_interval,
        })
    }

    /// Initialize the given current song.
    pub fn init(&self, option: &mut Option<SongFile>) {
        match std::mem::replace(option, self.build()) {
            Some(old) => old.blank_log(),
            None => (),
        }
    }
}

pub struct SongFile {
    /// Path to render current song at.
    pub path: PathBuf,
    /// Message to render when a song is playing.
    template: Template,
    /// Message to show when no song is playing.
    stopped_template: Option<Template>,
    /// Update frequency.
    pub update_interval: timer::Interval,
}

impl SongFile {
    /// Either creates or truncates the current song file.
    fn create_or_truncate(&self) -> Result<File, failure::Error> {
        File::create(&self.path).map_err(Into::into)
    }

    /// Blank the current file.
    pub fn blank(&self) -> Result<(), failure::Error> {
        use std::io::Write as _;
        let mut f = self.create_or_truncate()?;

        if let Some(stopped_template) = self.stopped_template.as_ref() {
            write!(f, "{}", stopped_template)?;
        } else {
            write!(f, "Not Playing")?;
        }

        Ok(())
    }

    /// Write the current song to a path.
    pub fn write(&self, song: &player::Song, state: player::State) -> Result<(), failure::Error> {
        let mut f = self.create_or_truncate()?;
        let data = song.data(state)?;
        self.template.render(&mut f, &data)?;
        Ok(())
    }

    /// Clear the old log.
    pub fn blank_log(&self) {
        if let Err(e) = self.blank() {
            log::error!("Failed to blank file: {}: {}", self.path.display(), e);
        }
    }
}
