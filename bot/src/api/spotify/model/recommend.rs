//! All objects related to recommendation
use super::track::SimplifiedTrack;
use serde::{Deserialize, Serialize};

///[recommendations object](https://developer.spotify.com/web-api/object-model/#recommendations-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Recommendations {
    pub(crate) seeds: Vec<RecommendationsSeed>,
    pub(crate) tracks: Vec<SimplifiedTrack>,
}

///[recommendations seed object](https://developer.spotify.com/web-api/object-model/#recommendations-seed-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RecommendationsSeed {
    #[serde(rename = "afterFilteringSize")]
    pub(crate) after_filtering_size: u32,
    #[serde(rename = "afterRelinkingSize")]
    pub(crate) after_relinking_size: u32,
    pub(crate) href: Option<String>,
    pub(crate) id: String,
    #[serde(rename = "initialPoolSize")]
    pub(crate) initial_pool_size: u32,
    #[serde(rename = "type")]
    pub(crate) _type: RecommendationsSeedType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum RecommendationsSeedType {
    #[serde(rename = "ARTIST")]
    Artist,
    #[serde(rename = "TRACK")]
    Track,
    #[serde(rename = "GENRE")]
    Genre,
}
