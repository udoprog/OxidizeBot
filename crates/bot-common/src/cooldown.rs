use serde::{Serialize, Deserialize, de, ser};

use crate::Duration;

/// A cooldown implementation that prevents an action from being executed too frequently.
#[derive(Debug, Clone, Default)]
pub struct Cooldown {
    last_action_at: Option<std::time::Instant>,
    pub(crate) cooldown: Duration,
}

impl Cooldown {
    /// Create a cooldown from the given duration.
    pub fn from_duration(cooldown: Duration) -> Self {
        Self {
            last_action_at: None,
            cooldown,
        }
    }

    /// Test if we are allowed to perform the action based on the cooldown in effect.
    pub(crate) fn is_open(&mut self) -> bool {
        let now = std::time::Instant::now();

        match self.check(now) {
            None => {
                self.poke(now);
                true
            }
            Some(..) => false,
        }
    }

    /// Test how much time remains until cooldown is open.
    pub(crate) fn check(&mut self, now: std::time::Instant) -> Option<std::time::Duration> {
        if let Some(last_action_at) = self.last_action_at.as_ref() {
            let since_last_action = now - *last_action_at;
            let cooldown = self.cooldown.as_std();

            if since_last_action < cooldown {
                return Some(cooldown - since_last_action);
            }
        }

        None
    }

    /// Poke the cooldown with the current time
    pub(crate) fn poke(&mut self, now: std::time::Instant) {
        self.last_action_at = Some(now);
    }
}

impl Serialize for Cooldown {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        self.cooldown.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Cooldown {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let duration = Duration::deserialize(deserializer)?;
        Ok(Cooldown::from_duration(duration))
    }
}
