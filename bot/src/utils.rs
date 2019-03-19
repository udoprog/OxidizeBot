use std::{mem, time::Duration};
use url::percent_encoding::PercentDecode;

/// Helper type for futures.
pub type BoxFuture<T, E> = Box<dyn futures::Future<Item = T, Error = E> + Send + 'static>;

pub struct Urls<'a> {
    message: &'a str,
}

impl<'a> Urls<'a> {
    /// Extract all URLs from the given message.
    pub fn new(message: &'a str) -> Urls<'a> {
        Urls {
            message: message.trim(),
        }
    }
}

impl<'a> Iterator for Urls<'a> {
    type Item = url::Url;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.message.is_empty() {
            let index = match self.message.find("http") {
                Some(index) => index,
                None => break,
            };

            let m = &self.message[index..];

            let end = match m.find(|c| !is_url_character(c)) {
                Some(index) => index,
                None => m.len(),
            };

            let (url, rest) = m.split_at(end);
            self.message = rest.trim();

            if let Ok(url) = str::parse::<url::Url>(url) {
                return Some(url);
            }
        }

        None
    }
}

/// Decode a query string.
pub fn query_pairs(query: &str) -> QueryPairs<'_> {
    QueryPairs { query }
}

pub struct QueryPairs<'a> {
    query: &'a str,
}

impl<'a> Iterator for QueryPairs<'a> {
    type Item = (PercentDecode<'a>, Option<PercentDecode<'a>>);

    fn next(&mut self) -> Option<Self::Item> {
        use std::mem;
        use url::percent_encoding::percent_decode;

        while !self.query.is_empty() {
            let s = match self.query.find('&') {
                Some(index) => {
                    let (s, rest) = self.query.split_at(index);
                    self.query = &rest[1..];
                    s
                }
                None => mem::replace(&mut self.query, ""),
            };

            match s.find('=') {
                Some(index) => {
                    let (s, rest) = s.split_at(index);
                    let key = percent_decode(s.as_bytes());
                    let value = percent_decode(rest[1..].as_bytes());
                    return Some((key, Some(value)));
                }
                None => {
                    let s = percent_decode(s.as_bytes());
                    return Some((s, None));
                }
            }
        }

        None
    }
}

#[derive(Clone, Debug)]
pub struct Words<'a> {
    string: &'a str,
}

impl<'a> Words<'a> {
    /// Split the commandline.
    pub fn new(string: &str) -> Words<'_> {
        Words {
            string: string.trim_start_matches(char::is_whitespace),
        }
    }

    /// The rest of the input.
    pub fn rest(&self) -> &'a str {
        self.string
    }
}

impl<'a> Iterator for Words<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }

        let (out, rest) = match self.string.find(char::is_whitespace) {
            Some(n) => self.string.split_at(n),
            None => return Some(mem::replace(&mut self.string, "")),
        };

        self.string = rest.trim_start_matches(char::is_whitespace);
        Some(out)
    }
}

#[derive(Debug)]
pub struct TrimmedWords<'a> {
    string: &'a str,
}

impl<'a> TrimmedWords<'a> {
    /// Split the commandline.
    pub fn new(string: &str) -> TrimmedWords<'_> {
        TrimmedWords {
            string: string.trim_start_matches(is_not_alphanum),
        }
    }
}

impl<'a> Iterator for TrimmedWords<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }

        let (out, rest) = match self.string.find(is_not_alphanum) {
            Some(n) => self.string.split_at(n),
            None => return Some(mem::replace(&mut self.string, "")),
        };

        self.string = rest.trim_start_matches(is_not_alphanum);
        Some(out)
    }
}

/// Test if char is not alphanumeric.
fn is_not_alphanum(c: char) -> bool {
    match c {
        'a'..='z' => false,
        'A'..='Z' => false,
        '0'..='9' => false,
        _ => true,
    }
}

/// Format the given number of seconds as a human time.
pub fn human_time(seconds: i64) -> String {
    let mut parts = Vec::new();

    if seconds < 0 {
        return String::from("negative time?");
    }

    let rest = seconds as u64;
    let hours = rest / 3600;
    let rest = rest % 3600;
    let minutes = rest / 60;
    let seconds = rest % 60;

    parts.extend(match hours {
        0 => None,
        1 => Some(format!("1 hour")),
        n => Some(format!("{} hours", n)),
    });

    parts.extend(match minutes {
        0 => None,
        1 => Some(format!("1 minute")),
        n => Some(format!("{} minutes", n)),
    });

    parts.extend(match seconds {
        0 => None,
        1 => Some(format!("1 second")),
        n => Some(format!("{} seconds", n)),
    });

    parts.join(", ")
}

/// Format the given number of seconds as a human time.
pub fn compact_time(seconds: u64) -> String {
    let mut time = String::new();

    let rest = seconds as u64;
    let hours = rest / 3600;
    let rest = rest % 3600;
    let minutes = rest / 60;
    let seconds = rest % 60;

    time.extend(match hours {
        0 => None,
        n => Some(format!("{}h", n)),
    });

    time.extend(match minutes {
        0 => None,
        n => Some(format!("{}m", n)),
    });

    time.extend(match seconds {
        0 => None,
        n => Some(format!("{}s", n)),
    });

    time
}

/// Render artists in a human readable form INCLUDING an oxford comma.
pub fn human_artists(artists: &[String]) -> Option<String> {
    if artists.is_empty() {
        return None;
    }

    let mut it = artists.iter();
    let mut artists = String::new();

    if let Some(artist) = it.next() {
        artists.push_str(artist);
    }

    let back = it.next_back();

    while let Some(artist) = it.next() {
        artists.push_str(", ");
        artists.push_str(artist);
    }

    if let Some(artist) = back {
        artists.push_str(", and ");
        artists.push_str(artist);
    }

    Some(artists)
}

/// Test if character is a URL character.
fn is_url_character(c: char) -> bool {
    match c {
        'a'..='z' => true,
        'A'..='Z' => true,
        // url-safe characters
        '$' | '-' | '_' | '.' | '+' | '!' | '*' | '\'' | '(' | ')' => true,
        // control characters.
        ';' | '/' | '?' | ':' | '@' | '=' | '&' => true,
        _ => false,
    }
}

/// Offset.
///
/// Stored field is in milliseconds.
#[derive(Debug, Clone, Default)]
pub struct Offset(u32);

impl std::str::FromStr for Offset {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.split(':').rev();

        let seconds: Option<u32> = it.next().map(str::parse).transpose()?;
        let minutes: Option<u32> = it.next().map(str::parse).transpose()?;

        let seconds = match seconds {
            Some(seconds) => seconds.checked_mul(1000),
            None => None,
        }
        .unwrap_or_default();

        let minutes = match minutes {
            Some(minutes) => minutes.checked_mul(1000 * 60),
            None => None,
        }
        .unwrap_or_default();

        Ok(Offset(seconds.checked_add(minutes).unwrap_or_default()))
    }
}

impl<'de> serde::Deserialize<'de> for Offset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        str::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Offset {
    /// Treat offset as duration.
    pub fn as_duration(&self) -> Duration {
        Duration::from_millis(self.0 as u64)
    }
}

/// Parse a human-readable duration, like `5m 1s`.
pub fn parse_duration(s: &str) -> Result<Duration, failure::Error> {
    let mut ms = 0u64;

    for p in s.split(' ') {
        let p = p.trim();

        if p.is_empty() {
            continue;
        }

        let (s, e) = p.split_at(p.len() - 1);

        match e {
            "s" => {
                let n = str::parse::<u64>(s)?;
                ms += n * 1000;
            }
            "m" => {
                let n = str::parse::<u64>(s)?;
                ms += n * 1000 * 60;
            }
            "h" => {
                let n = str::parse::<u64>(s)?;
                ms += n * 1000 * 60 * 60;
            }
            o => {
                failure::bail!("bad unit: {}", o);
            }
        }
    }

    Ok(Duration::from_millis(ms))
}

/// Deserialize an optional duration.
pub fn deserialize_optional_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize as _;
    let s = <Option<String>>::deserialize(deserializer)?;

    match s {
        Some(s) => Ok(Some(parse_duration(&s).map_err(serde::de::Error::custom)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::{human_artists, parse_duration, TrimmedWords, Urls, Words};

    #[test]
    pub fn test_trimmed_words() {
        let out = TrimmedWords::new("are, you a cherry? fucker?").collect::<Vec<_>>();
        assert_eq!(out, vec!["are", "you", "a", "cherry", "fucker"]);
    }

    #[test]
    pub fn test_split_escape() {
        let out = Words::new("   foo bar   baz   ").collect::<Vec<_>>();
        assert_eq!(out, vec!["foo", "bar", "baz"]);
    }

    #[test]
    pub fn test_urls() {
        let u: Vec<url::Url> =
            Urls::new("here:https://google.se/test+this, and this:http://example.com").collect();

        assert_eq!(
            u,
            vec![
                str::parse("https://google.se/test+this").unwrap(),
                str::parse("http://example.com").unwrap()
            ],
        );
    }

    #[test]
    pub fn test_human_artists() {
        let artists = vec![String::from("foo"), String::from("bar")];
        assert_eq!("foo, and bar", human_artists(&artists).expect("artists"));

        let artists = vec![
            String::from("foo"),
            String::from("bar"),
            String::from("baz"),
        ];
        assert_eq!(
            "foo, bar, and baz",
            human_artists(&artists).expect("artists")
        );
    }

    #[test]
    pub fn test_parse_duration() {
        use std::time::Duration;

        assert_eq!(
            Duration::from_millis(1000),
            parse_duration("1s").expect("duration")
        );
        assert_eq!(
            Duration::from_millis(2000),
            parse_duration("2s").expect("duration")
        );
        assert_eq!(
            Duration::from_millis(60 * 1000 + 3000),
            parse_duration("1m 3s").expect("duration")
        );
    }
}
