use crate::{track_id::TrackId, utils::Offset};
use std::sync::Arc;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(transparent)]
pub struct Themes {
    themes: Vec<Arc<Theme>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Theme {
    pub name: String,
    pub track: TrackId,
    #[serde(default)]
    pub offset: Offset,
}

impl Themes {
    /// Lookup the specified theme song.
    pub fn lookup<'a>(&'a self, name: &str) -> Option<Arc<Theme>> {
        self.themes
            .iter()
            .find(|t| t.name.as_str() == name)
            .cloned()
    }
}
