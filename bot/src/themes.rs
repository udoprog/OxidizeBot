use crate::{track_id::TrackId, utils::Offset};
use hashbrown::HashMap;
use std::sync::Arc;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(transparent)]
pub struct Themes {
    themes: HashMap<String, Arc<Theme>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Theme {
    pub track: TrackId,
    #[serde(default)]
    pub offset: Offset,
}

impl Themes {
    /// Lookup the specified theme song.
    pub fn lookup<'a>(&'a self, name: &str) -> Option<Arc<Theme>> {
        self.themes.get(name).cloned()
    }
}
