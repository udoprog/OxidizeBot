use chrono::NaiveDateTime;
use common::{OwnedChannel};
use common::models::TrackId;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::{
    after_streams, aliases, bad_words, balances, commands, promotions, script_keys, songs, themes,
};

#[derive(Serialize, Deserialize, Queryable, Insertable)]
pub struct Balance {
    pub channel: OwnedChannel,
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Queryable, Insertable)]
pub struct Command {
    /// The channel the command belongs to.
    pub(crate) channel: OwnedChannel,
    /// The name of the command.
    pub(crate) name: String,
    /// The regular expression pattern to match for the given command.
    pub(crate) pattern: Option<String>,
    /// The number of times the counter has been invoked.
    pub(crate) count: i32,
    /// The text of the command.
    pub(crate) text: String,
    /// The group the promotion is part of, if any.
    pub(crate) group: Option<String>,
    /// If the command is disabled.
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[diesel(table_name = commands)]
pub struct UpdateCommand<'a> {
    pub(crate) count: Option<i32>,
    pub(crate) text: Option<&'a str>,
    pub(crate) group: Option<&'a str>,
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Queryable, Insertable)]
#[diesel(table_name = aliases)]
pub struct Alias {
    /// The channel the alias belongs to.
    pub(crate) channel: OwnedChannel,
    /// The name of the alias.
    pub(crate) name: String,
    /// The regular expression pattern to match for the given alias.
    pub(crate) pattern: Option<String>,
    /// The text of the alias.
    pub(crate) text: String,
    /// The group the promotion is part of, if any.
    pub(crate) group: Option<String>,
    /// If the promotion is disabled.
    pub(crate) disabled: bool,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Insertable)]
#[diesel(table_name = aliases)]
pub struct InsertAlias {
    pub(crate) channel: OwnedChannel,
    pub(crate) name: String,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[diesel(table_name = aliases)]
pub struct UpdateAlias<'a> {
    pub(crate) text: Option<&'a str>,
    pub(crate) group: Option<&'a str>,
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable)]
pub struct AfterStream {
    /// The unique identifier of the afterstream message.
    pub(crate) id: i32,
    /// The channel the afterstream message belongs to.
    pub(crate) channel: Option<OwnedChannel>,
    /// When the afterstream was added.
    pub(crate) added_at: NaiveDateTime,
    /// The user that added the afterstream.
    pub(crate) user: String,
    /// The text of the afterstream.
    pub(crate) text: String,
}

/// Insert model for afterstreams.
#[derive(Insertable)]
#[diesel(table_name = after_streams)]
pub struct InsertAfterStream {
    pub(crate) channel: Option<String>,
    pub(crate) user: String,
    pub(crate) text: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Queryable, Insertable)]
pub struct BadWord {
    pub(crate) word: String,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Queryable)]
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

#[derive(Debug, Clone, PartialEq, Eq, Insertable)]
#[diesel(table_name = songs)]
pub struct AddSong {
    /// The track id of the song.
    pub track_id: TrackId,
    /// When the song was added.
    pub added_at: NaiveDateTime,
    /// The user that requested the song.
    pub user: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Queryable, Insertable)]
pub struct Promotion {
    /// The channel the promotion belongs to.
    pub(crate) channel: OwnedChannel,
    /// The name of the promotion.
    pub(crate) name: String,
    /// The frequency in seconds at which the promotion is posted.
    pub(crate) frequency: i32,
    /// The last time the promoted was promoted.
    pub(crate) promoted_at: Option<NaiveDateTime>,
    /// The promotion template to run.
    pub(crate) text: String,
    /// The group the promotion is part of, if any.
    pub(crate) group: Option<String>,
    /// If the promotion is disabled.
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[diesel(table_name = promotions)]
pub struct UpdatePromotion<'a> {
    pub(crate) frequency: Option<i32>,
    pub(crate) promoted_at: Option<&'a NaiveDateTime>,
    pub(crate) text: Option<&'a str>,
    pub(crate) group: Option<&'a str>,
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Queryable, Insertable)]
pub struct Theme {
    /// The channel the theme belongs to.
    pub(crate) channel: OwnedChannel,
    /// The name of the theme.
    pub(crate) name: String,
    /// If track id of the theme.
    pub(crate) track_id: TrackId,
    /// The start of the theme in seconds.
    pub(crate) start: i32,
    /// The end of the theme in seconds.
    pub(crate) end: Option<i32>,
    /// The group the theme is part of, if any.
    pub(crate) group: Option<String>,
    /// If the theme is disabled.
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[diesel(table_name = themes)]
pub struct UpdateTheme<'a> {
    pub(crate) track_id: Option<&'a TrackId>,
    pub(crate) start: Option<i32>,
    pub(crate) end: i32,
    pub(crate) group: Option<&'a str>,
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Queryable, Insertable)]
pub struct ScriptKey {
    pub(crate) channel: OwnedChannel,
    pub(crate) key: Vec<u8>,
    pub(crate) value: Vec<u8>,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[diesel(table_name = script_keys)]
pub struct SetScriptKeyValue<'a> {
    pub(crate) value: &'a [u8],
}
