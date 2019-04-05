use crate::{
    currency, current_song, features, irc, module, player, secrets, settings, themes, web,
};
use hashbrown::HashSet;
use relative_path::RelativePathBuf;
use std::{marker, sync::Arc};

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    /// The username of the streamer.
    /// TODO: get from twitch token.
    pub streamer: String,
    pub irc: Option<irc::Config>,
    #[serde(default)]
    pub twitch: Oauth2Config<TwitchDefaults>,
    #[serde(default)]
    pub spotify: Oauth2Config<SpotifyDefaults>,
    #[serde(default)]
    pub database_url: Option<String>,
    #[serde(default)]
    pub bad_words: Option<RelativePathBuf>,
    /// Where secrets are stored.
    #[serde(default)]
    pub secrets: Option<RelativePathBuf>,
    /// Themes that can be played.
    #[serde(default)]
    pub themes: Arc<themes::Themes>,
    /// Player configuration file.
    #[serde(default)]
    pub player: Option<player::Config>,
    /// Aliases in use for channels.
    #[serde(default)]
    pub aliases: Vec<DeprecatedAlias>,
    /// Features enabled for bot.
    #[serde(default)]
    pub features: features::Features,
    #[serde(default)]
    pub whitelisted_hosts: HashSet<String>,
    /// Write the current song to the specified path.
    #[serde(default)]
    pub current_song: Option<Arc<current_song::CurrentSong>>,
    /// API URL to use for pushing updates.
    #[serde(default)]
    pub api_url: Option<String>,
    /// Loyalty currency in use.
    #[serde(default)]
    pub currency: Option<currency::Currency>,
    /// Modules to load.
    #[serde(default)]
    pub modules: Vec<module::Config>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct DeprecatedAlias {
    pub r#match: String,
    pub replace: String,
}

#[derive(Debug)]
pub struct SpotifyDefaults;

impl Oauth2Defaults for SpotifyDefaults {
    const SECRETS_KEY: &'static str = "spotify::oauth2";

    fn new_flow_builder(
        web: web::Server,
        secrets_config: Arc<crate::oauth2::SecretsConfig>,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error> {
        crate::oauth2::spotify(web, secrets_config)
    }
}

#[derive(Debug)]
pub struct TwitchDefaults;

impl Oauth2Defaults for TwitchDefaults {
    const SECRETS_KEY: &'static str = "twitch::oauth2";

    fn new_flow_builder(
        web: web::Server,
        secrets_config: Arc<crate::oauth2::SecretsConfig>,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error> {
        crate::oauth2::twitch(web, secrets_config)
    }
}

/// Define defaults for fields.
pub trait Oauth2Defaults {
    const SECRETS_KEY: &'static str;

    fn new_flow_builder(
        web: web::Server,
        secrets_config: Arc<crate::oauth2::SecretsConfig>,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error>;
}

#[derive(Debug, serde::Deserialize)]
pub struct Oauth2Config<T>
where
    T: Oauth2Defaults,
{
    #[serde(default)]
    marker: marker::PhantomData<T>,
}

impl<T> Oauth2Config<T>
where
    T: Oauth2Defaults,
{
    /// Construct a new flow builder with the given configuration.
    pub fn new_flow_builder(
        &self,
        web: web::Server,
        name: &str,
        settings: &settings::ScopedSettings,
        secrets: &secrets::Secrets,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error> {
        let secrets = secrets.load(T::SECRETS_KEY)?;
        let settings = settings.scoped(&[name]);
        Ok(T::new_flow_builder(web, secrets)?.with_settings(settings.clone()))
    }
}

impl<T> Default for Oauth2Config<T>
where
    T: Oauth2Defaults,
{
    fn default() -> Oauth2Config<T> {
        Oauth2Config {
            marker: Default::default(),
        }
    }
}
