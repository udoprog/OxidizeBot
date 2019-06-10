mod base;
pub mod github;
pub mod nightbot;
pub mod setbac;
pub mod speedrun;
pub mod spotify;
pub mod twitch;
pub mod youtube;

pub use self::base::RequestBuilder;
pub use self::github::GitHub;
pub use self::nightbot::NightBot;
pub use self::setbac::SetBac;
pub use self::speedrun::Speedrun;
pub use self::spotify::Spotify;
pub use self::twitch::Twitch;
pub use self::youtube::YouTube;
