use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AlbumType {
    Album,
    Single,
    AppearsOn,
    Compilation,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Type {
    Artist,
    Album,
    Track,
    Playlist,
    User,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TimeRange {
    LongTerm,
    MediumTerm,
    ShortTerm,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepeatState {
    Off,
    Track,
    Context,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SearchType {
    Artist,
    Album,
    Track,
    Playlist,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum DeviceType {
    Computer,
    Smartphone,
    Speaker,
    CastAudio,
    #[serde(other)]
    Unknown,
}
