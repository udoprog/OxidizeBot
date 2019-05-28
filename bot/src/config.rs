use crate::{
    features, irc, module, player, secrets, settings, song_file, track_id::TrackId, utils::Offset,
    web,
};
use hashbrown::{HashMap, HashSet};
use relative_path::RelativePathBuf;
use std::sync::Arc;

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    pub streamer: Option<String>,
    #[serde(default)]
    pub irc: irc::Config,
    #[serde(default)]
    pub database_url: Option<String>,
    #[serde(default)]
    pub bad_words: Option<RelativePathBuf>,
    /// Where secrets are stored.
    #[serde(default)]
    pub secrets: Option<RelativePathBuf>,
    /// Themes that can be played.
    #[serde(default)]
    pub themes: Themes,
    /// Player configuration file.
    #[serde(default)]
    pub player: Arc<player::Config>,
    /// Aliases in use for channels.
    #[serde(default)]
    pub aliases: Vec<DeprecatedAlias>,
    /// Features enabled for bot.
    #[serde(default)]
    pub features: features::Features,
    #[serde(default)]
    pub whitelisted_hosts: HashSet<String>,
    /// Deprecated current_song configuration.
    #[serde(default)]
    pub current_song: song_file::Config,
    /// API URL to use for pushing updates.
    #[serde(default)]
    pub api_url: Option<String>,
    /// Loyalty currency in use.
    #[serde(default)]
    pub currency: Option<serde_json::Value>,
    /// Modules to load.
    #[serde(default)]
    pub modules: Vec<module::Config>,
    #[serde(default)]
    pub obs: Option<serde_json::Value>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct DeprecatedAlias {
    pub r#match: String,
    pub replace: String,
}

#[derive(Debug)]
pub struct Spotify;

impl OAuth2Params for Spotify {
    const SECRETS_KEY: &'static str = "spotify::oauth2";

    fn new_flow_builder(
        web: web::Server,
        settings: settings::Settings,
        secrets_config: Arc<crate::oauth2::SecretsConfig>,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error> {
        crate::oauth2::spotify(web, settings, secrets_config)
    }
}

#[derive(Debug)]
pub struct Twitch;

impl OAuth2Params for Twitch {
    const SECRETS_KEY: &'static str = "twitch::oauth2";

    fn new_flow_builder(
        web: web::Server,
        settings: settings::Settings,
        secrets_config: Arc<crate::oauth2::SecretsConfig>,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error> {
        crate::oauth2::twitch(web, settings, secrets_config)
    }
}

/// Define defaults for fields.
pub trait OAuth2Params {
    const SECRETS_KEY: &'static str;

    fn new_flow_builder(
        web: web::Server,
        settings: settings::Settings,
        secrets_config: Arc<crate::oauth2::SecretsConfig>,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error>;
}

/// Create a new flow based on a statis configuration.
pub fn new_oauth2_flow<T>(
    web: web::Server,
    name: &str,
    settings: &settings::Settings,
    secrets: &secrets::Secrets,
) -> Result<crate::oauth2::FlowBuilder, failure::Error>
where
    T: OAuth2Params,
{
    let secrets = secrets.load(T::SECRETS_KEY)?;
    let settings = settings.scoped(&[name]);
    Ok(T::new_flow_builder(web, settings, secrets)?)
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(transparent)]
pub struct Themes {
    pub themes: HashMap<String, Arc<Theme>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Theme {
    pub track: TrackId,
    #[serde(default)]
    pub offset: Offset,
    #[serde(default)]
    pub end: Option<Offset>,
}
