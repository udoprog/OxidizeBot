use super::schema::{
    after_streams, aliases, bad_words, balances, commands, counters, promotions, set_values, songs,
};
use crate::player::TrackId;
use chrono::NaiveDateTime;

#[derive(serde::Serialize, serde::Deserialize, diesel::Queryable, diesel::Insertable)]
pub struct Balance {
    pub channel: String,
    pub user: String,
    pub amount: i64,
}

impl Balance {
    /// Helper function to convert into a checked balance.
    pub fn checked(self) -> Self {
        Self {
            channel: self.channel,
            user: super::user_id(&self.user),
            amount: self.amount,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct Command {
    pub channel: String,
    pub name: String,
    /// The number of times the counter has been invoked.
    pub count: i32,
    pub text: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
#[table_name = "aliases"]
pub struct Alias {
    pub channel: String,
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, diesel::Queryable)]
pub struct AfterStream {
    pub id: i32,
    pub channel: Option<String>,
    pub added_at: NaiveDateTime,
    pub user: String,
    pub text: String,
}

#[derive(diesel::Insertable)]
#[table_name = "after_streams"]
pub struct InsertAfterStream {
    pub channel: Option<String>,
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct Promotion {
    pub channel: String,
    pub name: String,
    /// The frequency in seconds at which the promotion is posted.
    pub frequency: i32,
    /// The last time the promoted was promoted.
    pub promoted_at: Option<NaiveDateTime>,
    /// The promotion template to run.
    pub text: String,
}
