use crate::api;
use parking_lot::Mutex;
use std::{borrow, fmt, mem, sync::Arc, time};
use url::percent_encoding::PercentDecode;

mod duration;

pub use self::duration::Duration;

/// Helper type for futures.
pub type BoxFuture<T, E> = Box<dyn futures::Future<Item = T, Error = E> + Send + 'static>;
pub type BoxStream<T, E> = Box<dyn futures::Stream<Item = T, Error = E> + Send + 'static>;

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

struct DurationParts {
    seconds: u64,
    minutes: u64,
    hours: u64,
    days: u64,
}

/// Partition the given duration into time components.
#[inline(always)]
fn partition(seconds: u64) -> DurationParts {
    let rest = seconds as u64;
    let days = rest / (3600 * 24);
    let rest = rest % (3600 * 24);
    let hours = rest / 3600;
    let rest = rest % 3600;
    let minutes = rest / 60;
    let seconds = rest % 60;

    DurationParts {
        seconds,
        minutes,
        hours,
        days,
    }
}

/// Format the given number of seconds as a compact human time.
pub fn compact_duration(duration: time::Duration) -> String {
    let mut parts = Vec::new();

    let p = partition(duration.as_secs());

    parts.extend(match p.hours {
        0 => None,
        n => Some(format!("{:02}h", n)),
    });

    parts.extend(match p.minutes {
        0 => None,
        n => Some(format!("{:02}m", n)),
    });

    parts.extend(match p.seconds {
        0 => None,
        n => Some(format!("{:02}s", n)),
    });

    if parts.is_empty() {
        return String::from("0s");
    }

    parts.join(" ")
}

/// Format the given number of seconds as a long human time.
pub fn long_duration(duration: &time::Duration) -> String {
    let mut parts = Vec::new();

    let p = partition(duration.as_secs());

    parts.extend(match p.hours {
        0 => None,
        1 => Some(format!("one hour")),
        n => Some(format!("{} hours", english_num(n))),
    });

    parts.extend(match p.minutes {
        0 => None,
        1 => Some(format!("one minute")),
        n => Some(format!("{} minutes", english_num(n))),
    });

    parts.extend(match p.seconds {
        0 => None,
        1 => Some(format!("one second")),
        n => Some(format!("{} seconds", english_num(n))),
    });

    if parts.is_empty() {
        return String::from("0 seconds");
    }

    parts.join(", ")
}

/// Format the given number of seconds as a digital duration.
pub fn digital_duration(duration: &time::Duration) -> String {
    let mut parts = Vec::new();

    let p = partition(duration.as_secs());

    parts.extend(match p.hours {
        0 => None,
        n => Some(format!("{:02}", n)),
    });

    parts.push(format!("{:02}", p.minutes));
    parts.push(format!("{:02}", p.seconds));

    parts.join(":")
}

/// Format the given number as a string according to english conventions.
pub fn english_num(n: u64) -> borrow::Cow<'static, str> {
    let n = match n {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        n => return borrow::Cow::from(n.to_string()),
    };

    borrow::Cow::Borrowed(n)
}

/// Render artists in a human readable form INCLUDING an oxford comma.
pub fn human_artists(artists: &[api::spotify::SimplifiedArtist]) -> Option<String> {
    if artists.is_empty() {
        return None;
    }

    let mut it = artists.iter();
    let mut artists = String::new();

    if let Some(artist) = it.next() {
        artists.push_str(artist.name.as_str());
    }

    let back = it.next_back();

    while let Some(artist) = it.next() {
        artists.push_str(", ");
        artists.push_str(artist.name.as_str());
    }

    if let Some(artist) = back {
        artists.push_str(", and ");
        artists.push_str(artist.name.as_str());
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

/// An offset with millisecond precision.
///
/// Stored field is in milliseconds.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset(u32);

impl std::str::FromStr for Offset {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (s, ms) = match s.rfind('.') {
            Some(i) => (&s[..i], str::parse::<u32>(&s[(i + 1)..])?),
            None => (s, 0),
        };

        let (s, seconds) = match s.rfind(':') {
            Some(i) => (&s[..i], str::parse::<u32>(&s[(i + 1)..])? * 1_000),
            None => ("", str::parse::<u32>(s)? * 1_000),
        };

        let minutes = match s {
            "" => 0,
            s => str::parse::<u32>(s)? * 60_000,
        };

        Ok(Offset(
            ms.checked_add(seconds)
                .and_then(|t| t.checked_add(minutes))
                .unwrap_or_default(),
        ))
    }
}

impl<'de> serde::Deserialize<'de> for Offset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        str::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for Offset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl Offset {
    /// An offset from milliseconds.
    pub fn milliseconds(ms: u32) -> Self {
        Offset(ms)
    }

    /// Convert to seconds.
    pub fn as_milliseconds(&self) -> u32 {
        self.0
    }

    /// Treat offset as duration.
    pub fn as_duration(&self) -> time::Duration {
        time::Duration::from_millis(self.0 as u64)
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rest = self.0;
        let ms = rest % 1_000;
        let rest = rest / 1_000;
        let seconds = rest % 60;
        let minutes = rest / 60;

        if ms > 0 {
            write!(fmt, "{:02}:{:02}.{:03}", minutes, seconds, ms)
        } else {
            write!(fmt, "{:02}:{:02}", minutes, seconds)
        }
    }
}

/// A cooldown implementation that prevents an action from being executed too frequently.
#[derive(Debug, Clone)]
pub struct Cooldown {
    last_action_at: Option<time::Instant>,
    cooldown: Duration,
}

impl Cooldown {
    /// Create a cooldown from the given duration.
    pub fn from_duration(duration: Duration) -> Self {
        Self {
            last_action_at: None,
            cooldown: duration,
        }
    }

    /// Test if we are allowed to perform the action based on the cooldown in effect.
    pub fn is_open(&mut self) -> bool {
        let now = time::Instant::now();

        if let Some(last_action_at) = self.last_action_at.as_ref() {
            if now - *last_action_at < self.cooldown.as_std() {
                return false;
            }
        }

        self.last_action_at = Some(now);
        return true;
    }
}

impl serde::Serialize for Cooldown {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.cooldown.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Cooldown {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let duration = Duration::deserialize(deserializer)?;
        Ok(Cooldown::from_duration(duration))
    }
}

/// Helper to handle shutdowns.
#[derive(Clone)]
pub struct Shutdown {
    sender: Arc<Mutex<Option<futures::sync::oneshot::Sender<()>>>>,
}

impl Shutdown {
    /// Construct a new shutdown handler.
    pub fn new() -> (Self, futures::sync::oneshot::Receiver<()>) {
        let (tx, rx) = futures::sync::oneshot::channel();
        (
            Self {
                sender: Arc::new(Mutex::new(Some(tx))),
            },
            rx,
        )
    }

    /// Execute the shutdown handler.
    pub fn shutdown(&self) -> bool {
        if let Some(sender) = self.sender.lock().take() {
            sender.send(()).expect("no listener");
            return true;
        }

        log::warn!("shutdown already called");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{Offset, TrimmedWords, Urls, Words};

    #[test]
    pub fn test_offset() -> Result<(), failure::Error> {
        assert_eq!(Offset::milliseconds(1_000), str::parse::<Offset>("1")?);
        assert_eq!(Offset::milliseconds(1_000), str::parse::<Offset>("01")?);
        assert_eq!(Offset::milliseconds(61_000), str::parse::<Offset>("01:01")?);
        assert_eq!(
            Offset::milliseconds(61_123),
            str::parse::<Offset>("01:01.123")?
        );
        Ok(())
    }

    #[test]
    pub fn test_trimmed_words() {
        let out = TrimmedWords::new("hello, do you feel alive?").collect::<Vec<_>>();
        assert_eq!(out, vec!["hello", "do", "you", "feel", "alive"]);
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
}
