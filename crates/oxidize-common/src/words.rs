use std::ops::Deref;
use std::sync::Arc;

/// Construct an iterator over words in a string.
pub fn split(string: impl Into<WordsStorage>) -> Split {
    Split::new(string)
}

/// Trimmed words.
pub fn trimmed(string: &str) -> Trimmed<'_> {
    Trimmed::new(string)
}

#[derive(Debug, Clone)]
pub enum WordsStorage {
    Shared(Arc<String>),
    Static(&'static str),
}

impl Deref for WordsStorage {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            Self::Shared(s) => s,
            Self::Static(s) => s,
        }
    }
}

impl From<&'static str> for WordsStorage {
    fn from(value: &'static str) -> Self {
        Self::Static(value)
    }
}

impl From<Arc<String>> for WordsStorage {
    fn from(value: Arc<String>) -> Self {
        Self::Shared(value)
    }
}

/// An iterator over words in a string.
#[derive(Debug, Clone)]
pub struct Split {
    string: WordsStorage,
    off: usize,
    /// one character lookahead.
    b0: Option<(usize, char)>,
    buffer: String,
}

impl Split {
    /// Split the given string.
    pub(crate) fn new(string: impl Into<WordsStorage>) -> Split {
        let string = string.into();
        let mut it = string.char_indices();
        let b0 = it.next();
        let off = it.next().map(|(n, _)| n).unwrap_or_else(|| string.len());

        Split {
            string,
            off,
            b0,
            buffer: String::new(),
        }
    }

    /// Access the underlying string.
    pub fn string(&self) -> &str {
        &self.string
    }

    /// Take the next character.
    pub(crate) fn take(&mut self) -> Option<(usize, char)> {
        let s = &self.string[self.off..];
        let mut it = s.char_indices();
        let next = it.next().map(|(_, c)| (self.off, c));
        let out = std::mem::replace(&mut self.b0, next);
        self.off += match it.next() {
            Some((n, _)) => n,
            None => s.len(),
        };
        out
    }

    /// Look at the next character.
    pub(crate) fn peek(&self) -> Option<(usize, char)> {
        self.b0
    }

    /// The rest of the input.
    pub fn rest(&self) -> &str {
        self.b0.map(|(n, _)| &self.string[n..]).unwrap_or_default()
    }

    /// Process an escape.
    fn escape(&mut self) {
        let c = match self.take() {
            Some((_, c)) => c,
            None => return,
        };

        match c {
            't' => self.buffer.push('\t'),
            'r' => self.buffer.push('\r'),
            'n' => self.buffer.push('\n'),
            o => self.buffer.push(o),
        }
    }
}

impl Iterator for Split {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }

        while let Some((_, c)) = self.take() {
            match c {
                ' ' | '\t' | '\r' | '\n' => {
                    // Consume all whitespace so that `rest` behaves better.
                    while let Some((_, c)) = self.peek() {
                        match c {
                            ' ' | '\t' | '\r' | '\n' => {
                                self.take();
                            }
                            _ => break,
                        }
                    }

                    if !self.buffer.is_empty() {
                        let ret = self.buffer.clone();
                        self.buffer.clear();
                        return Some(ret);
                    }

                    continue;
                }
                '\\' => self.escape(),
                // parse string
                '"' => {
                    while let Some((_, c)) = self.take() {
                        match c {
                            '\\' => self.escape(),
                            '"' => break,
                            o => self.buffer.push(o),
                        }
                    }
                }
                o => self.buffer.push(o),
            }
        }

        if !self.buffer.is_empty() {
            let ret = self.buffer.clone();
            self.buffer.clear();
            return Some(ret);
        }

        None
    }
}

#[derive(Debug)]
pub struct Trimmed<'a> {
    string: &'a str,
}

impl<'a> Trimmed<'a> {
    /// Split the commandline.
    fn new(string: &str) -> Trimmed<'_> {
        Trimmed {
            string: string.trim_start_matches(is_trim_separator),
        }
    }
}

impl<'a> Iterator for Trimmed<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }

        let (out, rest) = match self.string.find(is_trim_separator) {
            Some(n) => self.string.split_at(n),
            None => return Some(std::mem::take(&mut self.string)),
        };

        self.string = rest.trim_start_matches(is_trim_separator);
        Some(out)
    }
}

fn is_trim_separator(c: char) -> bool {
    char::is_whitespace(c) || char::is_ascii_punctuation(&c)
}

#[cfg(test)]
mod tests {
    use super::{Split, Trimmed};

    #[test]
    pub(crate) fn test_trimmed_words() {
        let out = Trimmed::new("hello, do you feel alive?").collect::<Vec<_>>();
        assert_eq!(out, vec!["hello", "do", "you", "feel", "alive"]);
    }

    #[test]
    pub(crate) fn test_trimmed_words_unicode() {
        let it = Trimmed::new(" 👌👌 foo");

        assert_eq!(
            vec![String::from("👌👌"), String::from("foo")],
            it.collect::<Vec<_>>(),
        );
    }

    #[test]
    pub(crate) fn test_split_escape() {
        let out = Split::new("   foo bar   baz   ").collect::<Vec<_>>();
        assert_eq!(out, vec!["foo", "bar", "baz"]);
    }

    #[test]
    pub(crate) fn test_split_quoted() {
        let out = Split::new("   foo bar   \"baz  biz\" ").collect::<Vec<_>>();
        assert_eq!(out, vec!["foo", "bar", "baz  biz"]);

        let out = Split::new("   foo\"baz  biz\" ").collect::<Vec<_>>();
        assert_eq!(out, vec!["foobaz  biz"]);

        let out = Split::new("   foo\\\"baz  biz").collect::<Vec<_>>();
        assert_eq!(out, vec!["foo\"baz", "biz"]);

        // test that rest kinda works.
        let mut it = Split::new("   foo\\\"baz  biz \"is good\"");
        assert_eq!(it.next().as_deref(), Some("foo\"baz"));
        assert_eq!(it.rest(), "biz \"is good\"");
    }

    #[test]
    pub(crate) fn test_unicode() {
        let it = Split::new("👌👌 foo");

        assert_eq!(
            vec![String::from("👌👌"), String::from("foo")],
            it.collect::<Vec<_>>(),
        );
    }
}
