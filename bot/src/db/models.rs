use super::schema::{
    after_streams, aliases, bad_words, balances, commands, promotions, script_keys, songs, themes,
};
use crate::track_id::TrackId;
use chrono::NaiveDateTime;

#[derive(serde::Serialize, serde::Deserialize, diesel::Queryable, diesel::Insertable)]
pub(crate) struct Balance {
    pub(crate) channel: String,
    pub(crate) user: String,
    #[serde(default)]
    pub(crate) amount: i64,
    #[serde(default)]
    pub(crate) watch_time: i64,
}

impl Balance {
    /// Helper function to convert into a checked balance.
    pub(crate) fn checked(self) -> Self {
        Self {
            channel: self.channel,
            user: super::user_id(&self.user),
            amount: self.amount,
            watch_time: self.watch_time,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub(crate) struct Command {
    /// The channel the command belongs to.
    pub(crate) channel: String,
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
pub(crate) struct UpdateCommand<'a> {
    pub(crate) count: Option<i32>,
    pub(crate) text: Option<&'a str>,
    pub(crate) group: Option<&'a str>,
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
#[diesel(table_name = aliases)]
pub(crate) struct Alias {
    /// The channel the alias belongs to.
    pub(crate) channel: String,
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Insertable)]
#[diesel(table_name = aliases)]
pub(crate) struct InsertAlias {
    pub(crate) channel: String,
    pub(crate) name: String,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[diesel(table_name = aliases)]
pub(crate) struct UpdateAlias<'a> {
    pub(crate) text: Option<&'a str>,
    pub(crate) group: Option<&'a str>,
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, diesel::Queryable)]
pub(crate) struct AfterStream {
    /// The unique identifier of the afterstream message.
    pub(crate) id: i32,
    /// The channel the afterstream message belongs to.
    pub(crate) channel: Option<String>,
    /// When the afterstream was added.
    pub(crate) added_at: NaiveDateTime,
    /// The user that added the afterstream.
    pub(crate) user: String,
    /// The text of the afterstream.
    pub(crate) text: String,
}

/// Insert model for afterstreams.
#[derive(diesel::Insertable)]
#[diesel(table_name = after_streams)]
pub(crate) struct InsertAfterStream {
    pub(crate) channel: Option<String>,
    pub(crate) user: String,
    pub(crate) text: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub(crate) struct BadWord {
    pub(crate) word: String,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, diesel::Queryable)]
pub(crate) struct Song {
    /// ID of the song request.
    pub(crate) id: i32,
    /// If the request already played or not.
    pub(crate) played: bool,
    /// If the request was deleted or not.
    pub(crate) deleted: bool,
    /// The track id of the song.
    pub(crate) track_id: TrackId,
    /// When the song was added.
    pub(crate) added_at: NaiveDateTime,
    /// Time at which the song was promoted.
    pub(crate) promoted_at: Option<NaiveDateTime>,
    /// The user that promoted the song last.
    pub(crate) promoted_by: Option<String>,
    /// The user that requested the song.
    pub(crate) user: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, diesel::Insertable)]
#[diesel(table_name = songs)]
pub(crate) struct AddSong {
    /// The track id of the song.
    pub(crate) track_id: TrackId,
    /// When the song was added.
    pub(crate) added_at: NaiveDateTime,
    /// The user that requested the song.
    pub(crate) user: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub(crate) struct Promotion {
    /// The channel the promotion belongs to.
    pub(crate) channel: String,
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
pub(crate) struct UpdatePromotion<'a> {
    pub(crate) frequency: Option<i32>,
    pub(crate) promoted_at: Option<&'a NaiveDateTime>,
    pub(crate) text: Option<&'a str>,
    pub(crate) group: Option<&'a str>,
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub(crate) struct Theme {
    /// The channel the theme belongs to.
    pub(crate) channel: String,
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
pub(crate) struct UpdateTheme<'a> {
    pub(crate) track_id: Option<&'a TrackId>,
    pub(crate) start: Option<i32>,
    pub(crate) end: i32,
    pub(crate) group: Option<&'a str>,
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, diesel::Queryable, diesel::Insertable)]
pub(crate) struct ScriptKey {
    pub(crate) channel: String,
    pub(crate) key: Vec<u8>,
    pub(crate) value: Vec<u8>,
}

#[derive(Debug, Clone, Default, diesel::AsChangeset)]
#[diesel(table_name = script_keys)]
pub(crate) struct SetScriptKeyValue<'a> {
    pub(crate) value: &'a [u8],
}
