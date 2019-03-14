use std::mem;
use url::percent_encoding::PercentDecode;

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

    for artist in (&mut it).take(artists.len().saturating_sub(2)) {
        artists.push_str(", ");
        artists.push_str(artist);
    }

    if let Some(artist) = it.next() {
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

#[cfg(test)]
mod tests {
    use super::{TrimmedWords, Urls, Words};

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
}
