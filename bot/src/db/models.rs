use super::schema::{
    after_streams, aliases, bad_words, balances, commands, promotions, script_keys, songs, themes,
};
use crate::track_id::TrackId;
use chrono::NaiveDateTime;

#[derive(serde::Serialize, serde::Deserialize, diesel::Queryable, diesel::Insertable)]
pub struct Balance {
    pub channel: String,
    pub user: String,
    #[serde(default)]
    pub amount: i64,
    #[serde(default)]
    pub watch_time: i64,
}

impl Balance {
    /// Helper function to convert into a checked balance.
    pub fn checked(self) -> Self {
        Self {
            channel: self.channel,
            user: super::user_id(&self.user),
            amount: self.amount,
            watch_time: self.watch_time,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct Command {
    /// The channel the command belongs to.
    pub channel: String,
    /// The name of the command.
    pub name: String,
    /// The regular expression pattern to match for the given command.
    pub pattern: Option<String>,
    /// The number of times the counter has been invoked.
    pub count: i32,
    /// The text of the command.
    pub text: String,
    /// The group the promotion is part of, if any.
    pub group: Option<String>,
    /// If the command is disabled.
    pub disabled: bool,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[table_name = "commands"]
pub struct UpdateCommand<'a> {
    pub count: Option<i32>,
    pub text: Option<&'a str>,
    pub group: Option<&'a str>,
    pub disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
#[table_name = "aliases"]
pub struct Alias {
    /// The channel the alias belongs to.
    pub channel: String,
    /// The name of the alias.
    pub name: String,
    /// The regular expression pattern to match for the given alias.
    pub pattern: Option<String>,
    /// The text of the alias.
    pub text: String,
    /// The group the promotion is part of, if any.
    pub group: Option<String>,
    /// If the promotion is disabled.
    pub disabled: bool,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Insertable)]
#[table_name = "aliases"]
pub struct InsertAlias {
    pub channel: String,
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[table_name = "aliases"]
pub struct UpdateAlias<'a> {
    pub text: Option<&'a str>,
    pub group: Option<&'a str>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, diesel::Queryable)]
pub struct AfterStream {
    /// The unique identifier of the afterstream message.
    pub id: i32,
    /// The channel the afterstream message belongs to.
    pub channel: Option<String>,
    /// When the afterstream was added.
    pub added_at: NaiveDateTime,
    /// The user that added the afterstream.
    pub user: String,
    /// The text of the afterstream.
    pub text: String,
}

/// Insert model for afterstreams.
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

#[derive(Debug, Clone, PartialEq, Eq, diesel::Queryable)]
pub struct Song {
    /// ID of the song request.
    pub id: i32,
    /// If the request already played or not.
    pub played: bool,
    /// If the request was deleted or not.
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct Promotion {
    /// The channel the promotion belongs to.
    pub channel: String,
    /// The name of the promotion.
    pub name: String,
    /// The frequency in seconds at which the promotion is posted.
    pub frequency: i32,
    /// The last time the promoted was promoted.
    pub promoted_at: Option<NaiveDateTime>,
    /// The promotion template to run.
    pub text: String,
    /// The group the promotion is part of, if any.
    pub group: Option<String>,
    /// If the promotion is disabled.
    pub disabled: bool,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[table_name = "promotions"]
pub struct UpdatePromotion<'a> {
    pub frequency: Option<i32>,
    pub promoted_at: Option<&'a NaiveDateTime>,
    pub text: Option<&'a str>,
    pub group: Option<&'a str>,
    pub disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct Theme {
    /// The channel the theme belongs to.
    pub channel: String,
    /// The name of the theme.
    pub name: String,
    /// If track id of the theme.
    pub track_id: TrackId,
    /// The start of the theme in seconds.
    pub start: i32,
    /// The end of the theme in seconds.
    pub end: Option<i32>,
    /// The group the theme is part of, if any.
    pub group: Option<String>,
    /// If the theme is disabled.
    pub disabled: bool,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[table_name = "themes"]
pub struct UpdateTheme<'a> {
    pub track_id: Option<&'a TrackId>,
    pub start: Option<i32>,
    pub end: i32,
    pub group: Option<&'a str>,
    pub disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub struct ScriptKey {
    pub channel: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[table_name = "script_keys"]
pub struct SetScriptKeyValue<'a> {
    pub value: &'a [u8],
}
