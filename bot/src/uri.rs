/// Tools to deal with URIs.
///
/// URIs are strings that identify a single resource, like a track or a playlist.
use crate::spotify_id::SpotifyId;
use diesel::backend::{Backend, RawValue};
use diesel::serialize::IsNull;
use diesel::sqlite::Sqlite;
use std::fmt;
use std::str::FromStr as _;
use thiserror::Error;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, diesel::FromSqlRow, diesel::AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub(crate) enum Uri {
    /// A Spotify track.
    SpotifyTrack(SpotifyId),
    /// A Spotify playlist.
    SpotifyPlaylist(SpotifyId),
    /// A YouTube video.
    YouTubeVideo(String),
}

#[derive(Debug, Error)]
pub(crate) enum ParseUriError {
    /// Failed to parse an ID.
    #[error("bad spotify track id (expected base62): {}", _0)]
    BadBase62(String),
    #[error("missing uri prefix, expected youtube:video:<id>, or spotify:track:<id>")]
    BadURIPrefix,
}

impl std::str::FromStr for Uri {
    type Err = ParseUriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.split(':');

        match it.next() {
            Some("youtube") => {
                if let (Some("video"), Some(id)) = (it.next(), it.next()) {
                    let video_id = Uri::YouTubeVideo(id.to_string());
                    return Ok(video_id);
                }
            }
            Some("spotify") => match (it.next(), it.next()) {
                (Some("track"), Some(id)) => {
                    let id = SpotifyId::from_base62(id)
                        .map_err(|_| ParseUriError::BadBase62(id.to_string()))?;
                    return Ok(Uri::SpotifyTrack(id));
                }
                (Some("playlist"), Some(id)) => {
                    let id = SpotifyId::from_base62(id)
                        .map_err(|_| ParseUriError::BadBase62(id.to_string()))?;
                    return Ok(Uri::SpotifyPlaylist(id));
                }
                _ => (),
            },
            _ => (),
        }

        Err(ParseUriError::BadURIPrefix)
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Uri::SpotifyTrack(ref id) => write!(fmt, "spotify:track:{}", id.to_base62()),
            Uri::SpotifyPlaylist(ref id) => write!(fmt, "spotify:playlist:{}", id.to_base62()),
            Uri::YouTubeVideo(ref id) => write!(fmt, "youtube:video:{}", id),
        }
    }
}

impl diesel::serialize::ToSql<diesel::sql_types::Text, Sqlite> for Uri {
    fn to_sql(
        &self,
        out: &mut diesel::serialize::Output<'_, '_, Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for Uri
where
    DB: Backend,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: RawValue<'_, DB>) -> diesel::deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(Uri::from_str(&s)?)
    }
}

impl serde::Serialize for Uri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Uri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Uri::from_str(&s).map_err(serde::de::Error::custom)
    }
}
