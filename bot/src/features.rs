#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, fixed_map::Key)]
pub enum Feature {
    /// Song features.
    #[serde(rename = "song")]
    Song,
    /// Custom commands.
    #[serde(rename = "command")]
    Command,
    /// Counter commands.
    #[serde(rename = "counter")]
    Counter,
    /// Add afterstream notifications.
    #[serde(rename = "afterstream")]
    AfterStream,
    /// Feature to remove messages which matches a bad words filter.
    #[serde(rename = "bad-words")]
    BadWords,
    /// If URL-whitelisting is enabled.
    #[serde(rename = "url-whitelist")]
    UrlWhitelist,
    /// Admin features.
    #[serde(rename = "admin")]
    Admin,
    /// Clip feature.
    #[serde(rename = "clip")]
    Clip,
    /// !8ball feature.
    #[serde(rename = "8ball")]
    EightBall,
}

/// By-channel features that are enabled.
#[derive(Default, Debug, Clone)]
pub struct Features(fixed_map::Set<Feature>);

impl<'de> serde::Deserialize<'de> for Features {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut features = fixed_map::Set::default();

        for feature in Vec::<Feature>::deserialize(deserializer)? {
            features.insert(feature);
        }

        Ok(Features(features))
    }
}

impl Features {
    /// Test if there are any features configured.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
