use std::{fmt, num::ParseIntError};

use serde::{de, ser, Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FromStrError {
    #[error("{0}")]
    ParseIntError(#[from] ParseIntError),
}

/// PT-formatted duration.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct PtDuration(std::time::Duration);

impl PtDuration {
    /// Access the inner duration.
    pub fn as_std(&self) -> std::time::Duration {
        self.0
    }

    /// Convert into inner duration.
    pub fn into_std(self) -> std::time::Duration {
        self.0
    }
}

impl std::str::FromStr for PtDuration {
    type Err = FromStrError;

    fn from_str(duration: &str) -> Result<Self, Self::Err> {
        let duration = duration.trim_start_matches("PT");

        let (duration, hours) = match duration.find('H') {
            Some(index) => {
                let hours = str::parse::<u64>(&duration[..index])?;
                (&duration[(index + 1)..], hours)
            }
            None => (duration, 0u64),
        };

        let (duration, minutes) = match duration.find('M') {
            Some(index) => {
                let minutes = str::parse::<u64>(&duration[..index])?;
                (&duration[(index + 1)..], minutes)
            }
            None => (duration, 0u64),
        };

        let mut milliseconds = 0;

        let (_, mut seconds) = match duration.find('S') {
            Some(index) => {
                let seconds = &duration[..index];

                let seconds = match seconds.find('.') {
                    Some(dot) => {
                        let (seconds, tail) = seconds.split_at(dot);
                        milliseconds = str::parse::<u64>(&tail[1..])?;
                        seconds
                    }
                    None => seconds,
                };

                let seconds = str::parse::<u64>(seconds)?;
                (&duration[(index + 1)..], seconds)
            }
            None => (duration, 0u64),
        };

        seconds += minutes * 60;
        seconds += hours * 3600;
        milliseconds += seconds * 1000;

        Ok(PtDuration(std::time::Duration::from_millis(milliseconds)))
    }
}

impl fmt::Display for PtDuration {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = crate::duration::partition(self.0);

        write!(fmt, "PT")?;

        if p.hours > 0 {
            write!(fmt, "{}H", p.hours)?;
        }

        if p.minutes > 0 {
            write!(fmt, "{}M", p.minutes)?;
        }

        if p.seconds > 0 {
            write!(fmt, "{}S", p.seconds)?;
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for PtDuration {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        str::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for PtDuration {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.collect_str(self)
    }
}
