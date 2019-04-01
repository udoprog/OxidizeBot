use super::schema::{after_streams, bad_words, balances, commands, counters, set_values, songs};
use crate::player::TrackId;
use chrono::NaiveDateTime;

#[derive(diesel::Queryable, diesel::Insertable)]
pub struct Balance {
    pub channel: String,
    pub user: String,
    pub amount: i32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct Command {
    pub channel: String,
    pub name: String,
    /// The number of times the counter has been invoked.
    pub count: i32,
    pub text: String,
}

#[derive(diesel::Insertable)]
pub struct AfterStream {
    pub user: String,
    pub text: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct BadWord {
    pub word: String,
    pub why: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct Counter {
    pub channel: String,
    /// The name of the counter.
    pub name: String,
    /// The number of times the counter has been invoked.
    pub count: i32,
    /// The text of the count. A mustache template.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, diesel::Queryable)]
pub struct Song {
    /// ID of the song request.
    pub id: i32,
    /// If the request is deleted or not.
    pub deleted: bool,
    /// The track id of the song.
    pub track_id: TrackId,
    /// When the song was added.
    pub added_at: NaiveDateTime,
    /// Time at which the song was promoted.
    pub promoted_at: Option<NaiveDateTime>,
    /// The user that promoted the song last.
    pub promoted_by: Option<String>,
    /// The user that requested the song.
    pub user: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, diesel::Insertable)]
#[table_name = "songs"]
pub struct AddSong {
    /// The track id of the song.
    pub track_id: TrackId,
    /// When the song was added.
    pub added_at: NaiveDateTime,
    /// The user that requested the song.
    pub user: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, diesel::Queryable, diesel::Insertable)]
pub struct SetValue {
    pub channel: String,
    /// The kind of the value.
    pub kind: String,
    pub value: String,
}
