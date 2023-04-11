use std::fmt;

use diesel::backend::{Backend, RawValue};
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::{backend, query_builder};
use diesel::{AsExpression, FromSqlRow};
use thiserror::Error;

use crate::models::SpotifyId;

static YOUTUBE_URL: &str = "https://youtu.be";
static SPOTIFY_URL: &str = "https://open.spotify.com/track";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, FromSqlRow, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum TrackId {
    /// A Spotify track.
    Spotify(SpotifyId),
    /// A YouTube track.
    YouTube(String),
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FromStrError {
    /// Requested a URI from a bad host, like youtube.com.
    #[error("bad host, expected: open.spotify.com")]
    BadHost(String),
    #[error(
        "bad URL, expected: \
                       https://open.spotify.com/track/<id>, \
                       https://youtube.com/watch?v=<id>, or \
                       https://youtu.be/<id>"
    )]
    BadUrl(String),
    /// Failed to parse an ID.
    #[error("bad spotify track id (expected base62): {}", _0)]
    BadBase62(String),
    #[error("missing uri prefix, expected youtube:video:<id>, or spotify:track:<id>")]
    MissingUriPrefix,
}

impl std::str::FromStr for TrackId {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("youtube:video:") {
            let id = s.trim_start_matches("youtube:video:");
            let video_id = TrackId::YouTube(id.to_string());
            return Ok(video_id);
        }

        if s.starts_with("spotify:track:") {
            let mut id = s.trim_start_matches("spotify:track:");
            //Trim parameters
            if let Some(index) = id.find('?') {
                id = &id[..index];
            }
            let id =
                SpotifyId::from_base62(id).map_err(|_| FromStrError::BadBase62(id.to_string()))?;
            return Ok(TrackId::Spotify(id));
        }

        Err(FromStrError::MissingUriPrefix)
    }
}

impl fmt::Display for TrackId {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TrackId::Spotify(ref id) => write!(fmt, "spotify:track:{}", id.to_base62()),
            TrackId::YouTube(ref id) => write!(fmt, "youtube:video:{}", id),
        }
    }
}

impl TrackId {
    /// Get the URL for this track.
    pub fn url(&self) -> String {
        match *self {
            TrackId::Spotify(ref id) => format!("{}/{}", SPOTIFY_URL, id.to_base62()),
            TrackId::YouTube(ref id) => format!("{}/{}", YOUTUBE_URL, id),
        }
    }

    /// Used to load records from the database since they don't have a prefix.
    pub(crate) fn parse_with_prefix_fallback(s: &str) -> Result<Self, FromStrError> {
        match str::parse::<Self>(s) {
            Err(FromStrError::MissingUriPrefix) => {
                let id = SpotifyId::from_base62(s)
                    .map_err(|_| FromStrError::BadBase62(s.to_string()))?;
                Ok(TrackId::Spotify(id))
            }
            other => other,
        }
    }

    /// Parse by trying  URL forms first.
    pub fn parse_with_urls(s: &str) -> Result<Self, FromStrError> {
        // Parse a track id from a URL or URI.
        if let Ok(url) = str::parse::<url::Url>(s) {
            match url.host() {
                Some(ref host) if *host == url::Host::Domain("open.spotify.com") => {
                    let parts = url.path().split('/').collect::<Vec<_>>();

                    let id = match parts.as_slice() {
                        ["", "track", id] => SpotifyId::from_base62(id)
                            .map_err(|_| FromStrError::BadBase62((*id).to_string()))?,
                        _ => return Err(FromStrError::BadUrl(url.to_string())),
                    };

                    return Ok(TrackId::Spotify(id));
                }
                Some(ref host) if is_long_youtube(host) => {
                    let parts = url.path().split('/').collect::<Vec<_>>();

                    if parts.as_slice() != ["", "watch"] {
                        return Err(FromStrError::BadUrl(url.to_string()));
                    }

                    let mut video_id = None;

                    for (n, value) in url.query_pairs() {
                        if n == "v" {
                            video_id = Some(value.to_string());
                        }
                    }

                    let video_id = match video_id {
                        Some(video_id) => video_id,
                        None => return Err(FromStrError::BadUrl(url.to_string())),
                    };

                    return Ok(TrackId::YouTube(video_id));
                }
                Some(ref host) if is_short_youtube(host) => {
                    let parts = url.path().split('/').collect::<Vec<_>>();

                    let video_id = match parts.as_slice() {
                        ["", video_id] => *video_id,
                        _ => return Err(FromStrError::BadUrl(url.to_string())),
                    };

                    return Ok(TrackId::YouTube(video_id.to_string()));
                }
                Some(..) => {
                    return Err(FromStrError::BadHost(url.to_string()));
                }
                None => (),
            }
        }

        return str::parse(s);

        #[inline]
        fn is_long_youtube(host: &url::Host<&str>) -> bool {
            matches!(
                host,
                url::Host::Domain("youtube.com") | url::Host::Domain("www.youtube.com")
            )
        }

        #[inline]
        fn is_short_youtube(host: &url::Host<&str>) -> bool {
            matches!(host, url::Host::Domain("youtu.be"))
        }
    }
}

impl<D> ToSql<Text, D> for TrackId
where
    D: backend::Backend,
    String: for<'a> Into<
        <backend::BindCollector<'a, D> as query_builder::BindCollector<'a, D>>::Buffer,
    >,
{
    fn to_sql(&self, out: &mut Output<'_, '_, D>) -> diesel::serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl<DB> diesel::deserialize::FromSql<Text, DB> for TrackId
where
    DB: Backend,
    String: diesel::deserialize::FromSql<Text, DB>,
{
    fn from_sql(bytes: RawValue<'_, DB>) -> diesel::deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(TrackId::parse_with_prefix_fallback(&s)?)
    }
}

impl serde::Serialize for TrackId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for TrackId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TrackId::parse_with_prefix_fallback(&s).map_err(serde::de::Error::custom)
    }
}
