use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Badge {
    pub(crate) image_url_1x: String,
    pub(crate) image_url_2x: String,
    pub(crate) image_url_4x: String,
    pub(crate) description: String,
    pub(crate) title: String,
    pub(crate) click_action: String,
    pub(crate) click_url: String,
    #[serde(default)]
    pub(crate) last_updated: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct BadgeSet {
    pub(crate) versions: HashMap<String, Badge>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct BadgesDisplay {
    pub(crate) badge_sets: HashMap<String, BadgeSet>,
}
