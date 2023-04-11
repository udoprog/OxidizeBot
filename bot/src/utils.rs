mod respond;
pub(crate) use self::respond::respond;

use std::fmt;
use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{oneshot, Mutex};

pub(crate) trait Driver<'a> {
    /// Drive the given future.
    fn drive<F>(&mut self, future: F)
    where
        F: 'a + Send + Future<Output = Result<()>>;
}

impl<'a> Driver<'a> for Vec<common::BoxFuture<'a, Result<()>>> {
    fn drive<F>(&mut self, future: F)
    where
        F: 'a + Send + Future<Output = Result<()>>,
    {
        self.push(Box::pin(future));
    }
}

pub(crate) struct Urls<'a> {
    message: &'a str,
}

impl<'a> Urls<'a> {
    /// Extract all URLs from the given message.
    pub(crate) fn new(message: &'a str) -> Urls<'a> {
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

#[derive(Clone, Copy)]
pub(crate) struct Percentage(u32, u32);

/// Format the given part and whole as a percentage.
pub(crate) fn percentage(part: u32, total: u32) -> Percentage {
    Percentage(part, total)
}

impl fmt::Display for Percentage {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Percentage(part, total) = *self;

        let total = match total {
            0 => return write!(fmt, "0%"),
            total => total,
        };

        let p = (part * 10_000) / total;
        write!(fmt, "{}", p / 100)?;

        match p % 100 {
            0 => (),
            n => write!(fmt, ".{}", n)?,
        };

        fmt.write_str("%")
    }
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

/// Helper to handle shutdowns.
#[derive(Clone)]
pub(crate) struct Restart {
    sender: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl Restart {
    /// Construct a new shutdown handler.
    pub(crate) fn new() -> (Self, oneshot::Receiver<()>) {
        let (tx, rx) = oneshot::channel();
        (
            Self {
                sender: Arc::new(Mutex::new(Some(tx))),
            },
            rx,
        )
    }

    /// Execute the shutdown handler.
    pub(crate) async fn restart(&self) -> bool {
        if let Some(sender) = self.sender.lock().await.take() {
            sender.send(()).expect("no listener");
            return true;
        }

        tracing::warn!("Shutdown already called");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::Urls;

    #[test]
    pub(crate) fn test_urls() {
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
