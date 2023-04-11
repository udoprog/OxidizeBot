use std::{borrow::Cow, fmt};

use thiserror::Error;

/// Used to consistently format an IRC response.
pub fn respond<'a, M: 'a>(name: &'a str, m: M) -> impl fmt::Display + 'a
where
    M: fmt::Display,
{
    NameRespond { name, m }
}

struct NameRespond<'a, M> {
    name: &'a str,
    m: M,
}

impl<M> fmt::Display for NameRespond<'_, M>
where
    M: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.name, self.m)
    }
}

/// En error type we can propagate to generate a response, which is useful for
/// aborting operations.
#[derive(Debug, Error)]
pub enum RespondErr {
    /// Response already sent.
    #[error("Command failed")]
    Empty,
    /// A literal message.
    #[error("Command failed with: {0}")]
    Message(Cow<'static, str>),
}
