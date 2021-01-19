use std::fmt;

/// Used to consistently format an IRC response.
pub(crate) fn respond<'a, M: 'a>(name: &'a str, m: M) -> impl fmt::Display + 'a
where
    M: fmt::Display,
{
    Respond { name, m }
}

struct Respond<'a, M> {
    name: &'a str,
    m: M,
}

impl<M> fmt::Display for Respond<'_, M>
where
    M: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.name, self.m)
    }
}
