use crate::{player::Item, template};
use std::{fs::File, path::PathBuf};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CurrentSong {
    /// Path to render current song at.
    pub path: PathBuf,
    /// Message to render when a song is playing.
    template: template::Template,
    /// Message to show when no song is playing.
    #[serde(default)]
    not_playing: Option<String>,
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

    /// Write the current song to a path.
    pub fn write(&self, item: &Item, paused: bool) -> Result<(), failure::Error> {
        let mut f = self.create_or_truncate()?;
        let data = item.data(paused)?;
        self.template.render(&mut f, &data)?;
        Ok(())
    }
}
