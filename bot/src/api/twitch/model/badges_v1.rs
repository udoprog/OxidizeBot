use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Badge {
    pub image_url_1x: String,
    pub image_url_2x: String,
    pub image_url_4x: String,
    pub description: String,
    pub title: String,
    pub click_action: String,
    pub click_url: String,
    #[serde(default)]
    pub last_updated: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BadgeSet {
    pub versions: HashMap<String, Badge>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BadgesDisplay {
    pub badge_sets: HashMap<String, BadgeSet>,
}
