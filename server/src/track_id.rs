use librespot::core::spotify_id::SpotifyId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, diesel::FromSqlRow, diesel::AsExpression)]
#[sql_type = "diesel::sql_types::Text"]
pub struct TrackId(pub SpotifyId);

impl TrackId {
    /// Convert to a base 62 ID.
    pub fn to_base62(&self) -> String {
        self.0.to_base62()
    }

    /// Parse a track id from a URL or URI.
    pub fn from_url_or_uri(s: &str) -> Result<TrackId, failure::Error> {
        if let Ok(url) = str::parse::<url::Url>(s) {
            match url.host() {
                Some(host) => {
                    if host != url::Host::Domain("open.spotify.com") {
                        failure::bail!("bad host: {}", host);
                    }

                    let parts = url.path().split("/").collect::<Vec<_>>();

                    match parts.as_slice() {
                        &["", "track", id] => return str::parse(id),
                        _ => failure::bail!("bad path in url"),
                    }
                }
                None => {}
            }
        }

        if s.starts_with("spotify:track:") {
            return str::parse(s.trim_start_matches("spotify:track:"));
        }

        failure::bail!("bad track id");
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
        TrackId::from_url_or_uri(&s).map_err(serde::de::Error::custom)
    }
}
