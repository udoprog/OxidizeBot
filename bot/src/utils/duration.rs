use crate::utils;
use anyhow::bail;
use std::fmt;
use std::time;

/// A duration with second precision.
///
/// Stored field is in seconds.
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(u64);

impl Duration {
    /// Construct a duration from the given number of seconds.
    pub fn seconds(seconds: u64) -> Self {
        Duration(seconds)
    }

    /// Construct a duration from the given number of hours.
    pub fn hours(hours: u64) -> Self {
        Duration(hours * 3600)
    }

    /// Test if the duration is empty.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Convert into a standard duration.
    #[inline]
    pub fn as_std(&self) -> time::Duration {
        time::Duration::from_secs(self.0)
    }

    /// Convert into a chrono duration.
    #[inline]
    pub fn as_chrono(&self) -> chrono::Duration {
        chrono::Duration::seconds(self.0 as i64)
    }

    /// Subtract another duration from this duration.
    ///
    /// This will saturate on overflows.
    pub fn saturating_sub(&self, other: Self) -> Self {
        Duration(self.0.saturating_sub(other.0))
    }

    /// Convert into a digital digit representation.
    ///
    /// Like `01:30:44`.
    pub fn as_digital(&self) -> String {
        let mut parts = Vec::new();

        let p = utils::partition(time::Duration::from_secs(self.0));

        parts.extend(match p.days {
            0 => None,
            n => Some(format!("{:02}", n)),
        });

        parts.extend(match p.hours {
            0 => None,
            n => Some(format!("{:02}", n)),
        });

        parts.push(format!("{:02}", p.minutes));
        parts.push(format!("{:02}", p.seconds));

        parts.join(":")
    }

    /// Convert into seconds.
    pub fn num_seconds(&self) -> u64 {
        self.0
    }
}

impl std::ops::Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Duration(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut nothing = true;
        let mut s = self.0;

        if s >= 3_600u64 * 24u64 {
            nothing = false;
            write!(fmt, "{}d", s / (3_600u64 * 24u64))?;
            s %= 3_600u64 * 24u64;
        }

        if s >= 3_600u64 {
            nothing = false;
            write!(fmt, "{}h", s / 3_600)?;
            s %= 3_600;
        }

        if s >= 60u64 {
            nothing = false;
            write!(fmt, "{}m", s / 60)?;
            s %= 60;
        }

        if s != 0u64 || nothing {
            write!(fmt, "{}s", s)?;
        }

        Ok(())
    }
}

impl std::str::FromStr for Duration {
    type Err = anyhow::Error;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let mut seconds = 0u64;

        while !s.is_empty() {
            match s.find(|c: char| !c.is_numeric()) {
                Some(i) if s[i..].starts_with('d') => {
                    let n = str::parse::<u64>(&s[..i])?;
                    seconds += n * 60 * 60 * 24;
                    s = &s[(i + 1)..];
                }
                Some(i) if s[i..].starts_with('h') => {
                    let n = str::parse::<u64>(&s[..i])?;
                    seconds += n * 60 * 60;
                    s = &s[(i + 1)..];
                }
                Some(i) if s[i..].starts_with('m') => {
                    let n = str::parse::<u64>(&s[..i])?;
                    seconds += n * 60;
                    s = &s[(i + 1)..];
                }
                Some(i) if s[i..].starts_with('s') => {
                    let n = str::parse::<u64>(&s[..i])?;
                    seconds += n;
                    s = &s[(i + 1)..];
                }
                Some(i) => {
                    bail!("bad suffix: {}", &s[i..]);
                }
                _ => bail!("unexpected end"),
            }
        }

        Ok(Duration(seconds))
    }
}

impl<'de> serde::Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let duration = String::deserialize(deserializer)?;
        str::parse(&duration).map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Duration;

    #[test]
    pub fn test_parse_duration() {
        assert_eq!(Duration::seconds(1), str::parse("1s").expect("duration"));
        assert_eq!(Duration::seconds(2), str::parse("2s").expect("duration"));
        assert_eq!(
            Duration::seconds(60 + 3),
            str::parse("1m3s").expect("duration")
        );
    }

    #[test]
    pub fn test_format_duration() {
        assert_eq!("0s", Duration::default().to_string());
        assert_eq!("2s", Duration::seconds(2).to_string());
        assert_eq!("2m1s", Duration::seconds(2 * 60 + 1).to_string());
        assert_eq!(
            "5h2m1s",
            Duration::seconds(5 * 3600 + 2 * 60 + 1).to_string()
        );
        assert_eq!(
            "6d5h2m1s",
            Duration::seconds(6 * 3600 * 24 + 5 * 3600 + 2 * 60 + 1).to_string()
        );
        assert_eq!(
            "1d12h",
            str::parse::<Duration>("1d12h").unwrap().to_string()
        );
        assert_eq!("1d12h", str::parse::<Duration>("36h").unwrap().to_string());

        assert_eq!("2h10m", str::parse::<Duration>("130m").unwrap().to_string());
    }
}
