use crate::spotify_id::SpotifyId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, diesel::FromSqlRow, diesel::AsExpression)]
#[sql_type = "diesel::sql_types::Text"]
pub struct TrackId(pub SpotifyId);

#[derive(Debug, err_derive::Error)]
pub enum ParseTrackIdError {
    /// Requested a URI from a bad host, like youtube.com.
    #[error(display = "bad host, expected: open.spotify.com")]
    BadHost(String),
    #[error(display = "bad URL, expected: https://open.spotify.com/track/<id>")]
    BadUrl(String),
    /// Argument had a bad URI.
    #[error(display = "bad URI, expected: spotify:tracks:<id>")]
    BadUri(String),
    /// Failed to parse an ID.
    #[error(display = "bad track id: {}", _0)]
    BadId(String),
}

impl ParseTrackIdError {
    /// Test if a youtube track was requested.
    pub fn is_bad_host_youtube(&self) -> bool {
        match *self {
            ParseTrackIdError::BadHost(ref host) => match host.as_str() {
                "youtube.com" => true,
                "youtu.be" => true,
                "www.youtube.com" => true,
                "www.youtu.be" => true,
                _ => false,
            },
            _ => false,
        }
    }
}

impl TrackId {
    /// Convert to a base 62 ID.
    pub fn to_base62(&self) -> String {
        self.0.to_base62()
    }

    pub fn parse(s: &str) -> Result<Self, ParseTrackIdError> {
        // Parse a track id from a URL or URI.
        if let Ok(url) = str::parse::<url::Url>(s) {
            match url.host() {
                Some(host) => {
                    if host != url::Host::Domain("open.spotify.com") {
                        return Err(ParseTrackIdError::BadHost(host.to_string()));
                    }

                    let parts = url.path().split("/").collect::<Vec<_>>();

                    match parts.as_slice() {
                        &["", "track", id] => {
                            return str::parse(id)
                                .map_err(|_| ParseTrackIdError::BadId(id.to_string()))
                        }
                        _ => return Err(ParseTrackIdError::BadUrl(url.to_string())),
                    }
                }
                None => {}
            }
        }

        if s.starts_with("spotify:track:") {
            let id = s.trim_start_matches("spotify:track:");
            return str::parse(id).map_err(|_| ParseTrackIdError::BadId(id.to_string()));
        }

        Err(ParseTrackIdError::BadUri(s.to_string()))
    }
}

impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for TrackId
where
    DB: diesel::backend::Backend,
    String: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<W>(&self, out: &mut diesel::serialize::Output<W, DB>) -> diesel::serialize::Result
    where
        W: std::io::Write,
    {
        self.0.to_base62().to_sql(out)
    }
}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for TrackId
where
    DB: diesel::backend::Backend,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> diesel::deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(str::parse(&s)?)
    }
}

impl serde::Serialize for TrackId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.to_base62().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for TrackId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TrackId::parse(&s).map_err(serde::de::Error::custom)
    }
}
