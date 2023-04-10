pub mod base;

pub mod bttv;
pub use self::bttv::BetterTTV;

pub mod ffz;
pub use self::ffz::FrankerFaceZ;

pub mod github;
pub use self::github::GitHub;

pub mod nightbot;
pub use self::nightbot::NightBot;

pub mod open_weather_map;
pub use self::open_weather_map::OpenWeatherMap;

pub mod provider;
pub use self::provider::{twitch_clients_task, TwitchAndUser, User};

pub mod setbac;
pub use self::setbac::Setbac;

pub mod speedrun;
pub use self::speedrun::Speedrun;

pub mod spotify;
pub use self::spotify::Spotify;

pub mod tduva;
pub use self::tduva::Tduva;

pub mod token;
pub use self::token::Token;

pub mod twitch;
pub use self::twitch::Twitch;

pub mod youtube;
pub use self::youtube::YouTube;
