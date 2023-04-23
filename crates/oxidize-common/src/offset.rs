use std::fmt;
use std::num::ParseIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FromStrError {
    #[error("{0}")]
    ParseIntError(#[from] ParseIntError),
}

/// An offset with millisecond precision.
///
/// Stored field is in milliseconds.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset(u32);

impl std::str::FromStr for Offset {
    type Err = FromStrError;

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
    pub fn as_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.0 as u64)
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

#[cfg(test)]
mod tests {
    use super::Offset;

    #[test]
    pub(crate) fn test_offset() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(Offset::milliseconds(1_000), str::parse::<Offset>("1")?);
        assert_eq!(Offset::milliseconds(1_000), str::parse::<Offset>("01")?);
        assert_eq!(Offset::milliseconds(61_000), str::parse::<Offset>("01:01")?);
        assert_eq!(
            Offset::milliseconds(61_123),
            str::parse::<Offset>("01:01.123")?
        );
        Ok(())
    }
}
