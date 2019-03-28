use crate::{
    aliases, command, config,
    currency::Currency,
    db,
    features::{Feature, Features},
    module, oauth2, player, stream_info, twitch, utils,
    utils::BoxFuture,
};
use failure::format_err;
use futures::{
    future::{self, Future},
    stream::Stream,
};
use hashbrown::HashSet;
use irc::{
    client::{self, ext::ClientExt, Client, IrcClient, PackedIrcClient},
    proto::{
        command::{CapSubCommand, Command},
        message::{Message, Tag},
    },
};
use parking_lot::{Mutex, RwLock};
use setmod_notifier::{Notification, Notifier};
use std::{fmt, sync::Arc, time};
use tokio::timer;
use tokio_threadpool::ThreadPool;

mod admin;
mod after_stream;
mod bad_word;
mod clip;
mod command_admin;
mod counter;
mod eight_ball;
mod misc;
mod song;

const SERVER: &'static str = "irc.chat.twitch.tv";
const TWITCH_TAGS_CAP: &'static str = "twitch.tv/tags";
const TWITCH_COMMANDS_CAP: &'static str = "twitch.tv/commands";

/// Configuration for twitch integration.
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    bot: String,
    /// Cooldown for moderator actions.
    moderator_cooldown: Option<utils::Cooldown>,
    /// Cooldown for creating clips.
    #[serde(default = "default_cooldown")]
    clip_cooldown: utils::Cooldown,
    /// Cooldown for creating afterstream reminders.
    #[serde(default = "default_cooldown")]
    afterstream_cooldown: utils::Cooldown,
    /// Name of the channel to join.
    pub channel: Arc<String>,
    /// Whether or not to notify on currency rewards.
    #[serde(default)]
    notify_rewards: bool,
    /// Notify when bot starts.
    #[serde(default)]
    startup_message: Option<String>,
}

fn default_cooldown() -> utils::Cooldown {
    utils::Cooldown::from_duration(time::Duration::from_secs(15))
}

/// Helper struct to construct IRC integration.
pub struct Irc<'a> {
    pub core: &'a mut tokio_core::reactor::Core,
    pub db: db::Database,
    pub streamer_twitch: twitch::Twitch,
    pub bot_twitch: twitch::Twitch,
    pub config: &'a config::Config,
    pub irc_config: &'a Config,
    pub token: Arc<RwLock<oauth2::Token>>,
    pub commands: db::Commands<db::Database>,
    pub counters: db::Counters<db::Database>,
    pub bad_words: db::Words<db::Database>,
    pub notifier: Arc<Notifier>,
    pub player: Option<&'a player::Player>,
    pub modules: &'a [Box<dyn module::Module + 'static>],
    pub shutdown: utils::Shutdown,
}

impl Irc<'_> {
    pub fn run(
        self,
    ) -> Result<impl Future<Item = (), Error = failure::Error> + Send + 'static, failure::Error>
    {
        let Irc {
            core,
            db,
            streamer_twitch,
            bot_twitch,
            config,
            irc_config,
            token,
            commands,
            counters,
            bad_words,
            notifier,
            player,
            modules,
            shutdown,
            ..
        } = self;

        let access_token = token.read().access_token().to_string();

        let irc_client_config = client::data::config::Config {
            nickname: Some(irc_config.bot.clone()),
            channels: Some(vec![(*irc_config.channel).clone()]),
            password: Some(format!("oauth:{}", access_token)),
            server: Some(String::from(SERVER)),
            port: Some(6697),
            use_ssl: Some(true),
            ..client::data::config::Config::default()
        };

        let client = IrcClient::new_future(core.handle(), &irc_client_config)?;

        let PackedIrcClient(client, send_future) = core.run(client)?;
        client.identify()?;

        let sender = Sender::new(client.clone());
        sender.cap_req(TWITCH_TAGS_CAP);
        sender.cap_req(TWITCH_COMMANDS_CAP);

        if let Some(startup_message) = irc_config.startup_message.as_ref() {
            // greeting when bot joins
            sender.privmsg(irc_config.channel.as_str(), startup_message);
        }

        let mut handlers = module::Handlers::default();
        let mut futures = Vec::<BoxFuture<(), failure::Error>>::new();

        let stream_info = {
            let interval = time::Duration::from_secs(30);
            let (stream_info, future) =
                stream_info::setup(config.streamer.as_str(), interval, streamer_twitch.clone());
            futures.push(Box::new(future));
            stream_info
        };

        for module in modules {
            module.hook(module::HookContext {
                db: &db,
                handlers: &mut handlers,
                currency: config.currency.as_ref(),
                twitch: &bot_twitch,
                futures: &mut futures,
                stream_info: &stream_info,
            })?;
        }

        if let Some(currency) = config.currency.as_ref() {
            let reward = 10;
            let interval = 60 * 10;

            let future = reward_loop(
                irc_config,
                reward,
                interval,
                db.clone(),
                streamer_twitch.clone(),
                sender.clone(),
                currency,
            );

            futures.push(Box::new(future));
        }

        if let Some(player) = player {
            futures.push(Box::new(player_feedback_loop(
                irc_config,
                player,
                sender.clone(),
            )));

            handlers.insert(
                "song",
                song::Song {
                    player: player.client(),
                },
            );
        }

        if config.features.test(Feature::Admin) {
            handlers.insert(
                "title",
                misc::Title {
                    stream_info: stream_info.clone(),
                    twitch: streamer_twitch.clone(),
                },
            );

            handlers.insert(
                "game",
                misc::Game {
                    stream_info: stream_info.clone(),
                    twitch: streamer_twitch.clone(),
                },
            );

            handlers.insert(
                "uptime",
                misc::Uptime {
                    stream_info: stream_info.clone(),
                },
            );
        }

        if config.features.test(Feature::BadWords) {
            handlers.insert(
                "badword",
                bad_word::BadWord {
                    bad_words: bad_words.clone(),
                },
            );
        }

        if config.features.test(Feature::Counter) {
            handlers.insert(
                "counter",
                counter::Counter {
                    counters: counters.clone(),
                },
            );
        }

        if config.features.test(Feature::Command) {
            handlers.insert(
                "command",
                command_admin::Handler {
                    commands: commands.clone(),
                },
            );
        }

        if config.features.test(Feature::EightBall) {
            handlers.insert("8ball", eight_ball::EightBall {});
        }

        if config.features.test(Feature::Clip) {
            handlers.insert(
                "clip",
                clip::Clip {
                    stream_info: stream_info.clone(),
                    clip_cooldown: irc_config.clip_cooldown.clone(),
                    twitch: bot_twitch.clone(),
                },
            );
        }

        if config.features.test(Feature::AfterStream) {
            handlers.insert(
                "afterstream",
                after_stream::AfterStream {
                    cooldown: irc_config.afterstream_cooldown.clone(),
                    db: db.clone(),
                },
            );
        }

        handlers.insert("admin", admin::Admin {});

        futures.push(Box::new(send_future.map_err(failure::Error::from)));

        let mut handler = Handler {
            streamer: config.streamer.clone(),
            channel: irc_config.channel.clone(),
            db,
            sender: sender.clone(),
            moderators: HashSet::default(),
            whitelisted_hosts: config.whitelisted_hosts.clone(),
            currency: config.currency.clone(),
            commands,
            counters,
            bad_words,
            notifier,
            aliases: config.aliases.clone(),
            features: config.features.clone(),
            api_url: config.api_url.clone(),
            thread_pool: Arc::new(ThreadPool::new()),
            moderator_cooldown: irc_config.moderator_cooldown.clone(),
            handlers,
            shutdown,
        };

        futures.push(Box::new(
            client
                .stream()
                .map_err(failure::Error::from)
                .and_then(move |m| handler.handle(&m))
                // handle any errors.
                .or_else(|e| {
                    utils::log_err("failed to process message", e);
                    Ok(())
                })
                .for_each(|_| Ok(())),
        ));

        Ok(future::join_all(futures).map(|_| ()))
    }
}

/// Notifications from the player.
fn player_feedback_loop(
    config: &Config,
    player: &player::Player,
    sender: Sender,
) -> impl Future<Item = (), Error = failure::Error> + Send + 'static {
    player
        .add_rx()
        .map_err(|e| format_err!("failed to receive player update: {}", e))
        .for_each({
            let channel = config.channel.to_string();

            move |e| {
                match e {
                    player::Event::Playing(echo, item) => {
                        if !echo {
                            return Ok(());
                        }

                        let message = match item.user.as_ref() {
                            Some(user) => {
                                format!("Now playing: {}, requested by {}.", item.what(), user)
                            }
                            None => format!("Now playing: {}.", item.what(),),
                        };

                        sender.privmsg(channel.as_str(), message);
                    }
                    player::Event::Pausing => {
                        sender.privmsg(channel.as_str(), "Pausing playback.");
                    }
                    player::Event::Empty => {
                        sender.privmsg(
                            channel.as_str(),
                            format!(
                                "Song queue is empty (use !song request <spotify-id> to add more).",
                            ),
                        );
                    }
                    player::Event::NotConfigured => {
                        sender.privmsg(channel.as_str(), "Player has not been configured yet!");
                    }
                    // other event we don't care about
                    _ => {}
                }

                Ok(())
            }
        })
}

/// Set up a reward loop.
fn reward_loop(
    config: &Config,
    reward: i32,
    interval: u64,
    db: db::Database,
    twitch: twitch::Twitch,
    sender: Sender,
    currency: &Currency,
) -> impl Future<Item = (), Error = failure::Error> + Send + 'static {
    // Add currency timer.
    timer::Interval::new_interval(time::Duration::from_secs(interval))
        .map_err(Into::into)
        // fetch all users.
        .and_then({
            let channel = config.channel.to_string();

            move |_| {
                log::trace!("running reward loop");

                twitch.chatters(channel.as_str()).and_then(|chatters| {
                    let mut u = HashSet::new();
                    u.extend(chatters.viewers);
                    u.extend(chatters.moderators);
                    u.extend(chatters.broadcaster);

                    if u.is_empty() {
                        Err(format_err!("no chatters to reward"))
                    } else {
                        Ok(u)
                    }
                })
            }
        })
        // update database.
        .and_then({
            let channel = config.channel.to_string();

            move |u| db.balances_increment(channel.as_str(), u, reward)
        })
        .map({
            let notify_rewards = config.notify_rewards;
            let channel = config.channel.to_string();
            let currency = currency.clone();

            move |_| {
                if notify_rewards {
                    sender.privmsg(
                        channel.as_str(),
                        format!("/me has given {} {} to all viewers!", reward, currency.name),
                    );
                }
            }
        })
        // handle any errors.
        .or_else(|e| {
            utils::log_err("failed to reward users", e);
            Ok(())
        })
        .for_each(|_| Ok(()))
}

#[derive(Clone)]
pub struct Sender {
    client: IrcClient,
    thread_pool: Arc<ThreadPool>,
    limiter: Arc<Mutex<ratelimit::Limiter>>,
}

impl Sender {
    pub fn new(client: IrcClient) -> Sender {
        let limiter = ratelimit::Builder::new().frequency(10).capacity(95).build();

        Sender {
            client,
            thread_pool: Arc::new(ThreadPool::new()),
            limiter: Arc::new(Mutex::new(limiter)),
        }
    }

    /// Send a message.
    fn send(&self, m: impl Into<Message>) {
        let client = self.client.clone();
        let m = m.into();
        let limiter = Arc::clone(&self.limiter);

        self.thread_pool.spawn(future::lazy(move || {
            limiter.lock().wait();

            if let Err(e) = client.send(m) {
                utils::log_err("failed to send message", e.into());
            }

            Ok(())
        }));
    }

    /// Send a PRIVMSG.
    pub fn privmsg(&self, target: &str, f: impl fmt::Display) {
        self.send(Command::PRIVMSG(target.to_owned(), f.to_string()))
    }

    /// Send a capability request.
    pub fn cap_req(&self, cap: &str) {
        self.send(Command::CAP(
            None,
            CapSubCommand::REQ,
            Some(String::from(cap)),
            None,
        ))
    }
}

/// Handler for incoming messages.
struct Handler {
    /// Current Streamer.
    streamer: String,
    /// Currench channel.
    channel: Arc<String>,
    /// Database.
    db: db::Database,
    /// Queue for sending messages.
    sender: Sender,
    /// Moderators.
    moderators: HashSet<String>,
    /// Whitelisted hosts for links.
    whitelisted_hosts: HashSet<String>,
    /// Currency in use.
    currency: Option<Currency>,
    /// All registered commands.
    commands: db::Commands<db::Database>,
    /// All registered counters.
    counters: db::Counters<db::Database>,
    /// Bad words.
    bad_words: db::Words<db::Database>,
    /// For sending notifications.
    notifier: Arc<Notifier>,
    /// Aliases.
    aliases: aliases::Aliases,
    /// Enabled features.
    features: Features,
    /// Configured API URL.
    api_url: Option<String>,
    /// Thread pool used for driving futures.
    thread_pool: Arc<ThreadPool>,
    /// Active moderator cooldown.
    moderator_cooldown: Option<utils::Cooldown>,
    /// Handlers for specific commands like `!skip`.
    handlers: module::Handlers,
    /// Handler for shutting down the service.
    shutdown: utils::Shutdown,
}

impl Handler {
    /// Run as user.
    fn as_user<'m>(&self, tags: Tags<'m>, m: &'m Message) -> Result<User<'m>, failure::Error> {
        let name = m
            .source_nickname()
            .ok_or_else(|| format_err!("expected user info"))?;

        let target = m
            .response_target()
            .ok_or_else(|| format_err!("expected user info"))?;

        Ok(User {
            tags,
            sender: self.sender.clone(),
            name,
            target,
        })
    }

    /// Handle a command.
    pub fn process_command<'m>(
        &mut self,
        tags: Tags<'m>,
        command: &str,
        m: &'m Message,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        let user = self.as_user(tags, m)?;

        match command {
            "ping" => {
                user.respond("What do you want?");
                self.notifier.send(Notification::Ping)?;
            }
            other => {
                if let Some(handler) = self.handlers.get_mut(other) {
                    let ctx = command::Context {
                        api_url: self.api_url.as_ref().map(|s| s.as_str()),
                        streamer: self.streamer.as_str(),
                        sender: &self.sender,
                        moderators: &self.moderators,
                        moderator_cooldown: self.moderator_cooldown.as_mut(),
                        thread_pool: &self.thread_pool,
                        user,
                        it,
                        shutdown: &self.shutdown,
                    };

                    handler.handle(ctx)?;
                    return Ok(());
                }

                if let Some(currency) = self.currency.as_ref() {
                    if currency.name == other {
                        let balance = self.db.balance_of(user.name)?.unwrap_or(0);
                        user.respond(format!(
                            "You have {balance} {name}.",
                            balance = balance,
                            name = currency.name
                        ));
                    }
                }

                if self.features.test(Feature::Command) {
                    if let Some(command) = self.commands.get(user.target, other) {
                        let vars = TemplateVars {
                            name: &user.name,
                            target: &user.target,
                        };
                        let response = command.render(&vars)?;
                        self.sender.privmsg(user.target, response);
                    }
                }

                if self.features.test(Feature::Counter) {
                    if let Some(counter) = self.counters.get(user.target, other) {
                        self.counters.increment(&*counter)?;

                        let vars = CounterVars {
                            name: &user.name,
                            target: &user.target,
                            count: counter.count(),
                        };

                        let response = counter.render(&vars)?;
                        self.sender.privmsg(user.target, response);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract tags from message.
    fn tags<'local>(m: &'local Message) -> Tags<'local> {
        let mut id = None;
        let mut msg_id = None;

        if let Some(tags) = m.tags.as_ref() {
            for t in tags {
                match *t {
                    Tag(ref name, Some(ref value)) if name == "id" => {
                        id = Some(value.as_str());
                    }
                    Tag(ref name, Some(ref value)) if name == "msg-id" => {
                        msg_id = Some(value.as_str());
                    }
                    _ => {}
                }
            }
        }

        Tags { id, msg_id }
    }

    /// Delete the given message.
    fn delete_message<'local>(
        &mut self,
        source: &str,
        tags: Tags<'local>,
    ) -> Result<(), failure::Error> {
        let id = match tags.id {
            Some(id) => id,
            None => return Ok(()),
        };

        log::info!("Attempting to delete message: {}", id);

        self.sender.privmsg(source, format!("/delete {}", id));
        Ok(())
    }

    /// Test if the message should be deleted.
    fn should_be_deleted(&mut self, m: &Message, message: &str) -> bool {
        let user = m.source_nickname();

        // Moderators can say whatever they want.
        if user.map(|u| self.moderators.contains(u)).unwrap_or(false) {
            return false;
        }

        if self.features.test(Feature::BadWords) {
            if let Some(word) = self.test_bad_words(message) {
                if let (Some(why), Some(user), Some(target)) =
                    (word.why.as_ref(), user, m.response_target())
                {
                    let why = why.render_to_string(&TemplateVars {
                        name: user,
                        target: target,
                    });

                    match why {
                        Ok(why) => {
                            self.sender.privmsg(target, &why);
                        }
                        Err(e) => {
                            utils::log_err("failed to render response", e);
                        }
                    }
                }

                return true;
            }
        }

        if self.features.test(Feature::UrlWhitelist) {
            if self.has_bad_link(message) {
                return true;
            }
        }

        false
    }

    /// Test the message for bad words.
    fn test_bad_words(&self, message: &str) -> Option<Arc<db::Word>> {
        let tester = self.bad_words.tester();

        for word in utils::TrimmedWords::new(message) {
            if let Some(word) = tester.test(word) {
                return Some(word);
            }
        }

        None
    }

    /// Check if the given iterator has URLs that need to be
    fn has_bad_link(&mut self, message: &str) -> bool {
        for url in utils::Urls::new(message) {
            if let Some(host) = url.host_str() {
                if !self.whitelisted_hosts.contains(host) {
                    return true;
                }
            }
        }

        false
    }

    /// Handle the given command.
    pub fn handle<'local>(&mut self, m: &'local Message) -> Result<(), failure::Error> {
        match m.command {
            Command::PRIVMSG(ref source, ref message) => {
                let tags = Self::tags(&m);

                let mut it = utils::Words::new(message);

                // NB: needs to store locally to maintain a reference to it.
                let alias = self.aliases.lookup(it.clone());

                if let Some(alias) = alias.as_ref() {
                    it = utils::Words::new(alias.as_str());
                }

                if let Some(command) = it.next() {
                    if command.starts_with('!') {
                        let command = &command[1..];

                        if let Err(e) = self.process_command(tags.clone(), command, m, &mut it) {
                            utils::log_err("failed to process command", e);
                        }
                    }
                }

                if self.should_be_deleted(m, message) {
                    self.delete_message(source, tags)?;
                }
            }
            Command::CAP(_, CapSubCommand::ACK, _, ref what) => {
                match what.as_ref().map(|w| w.as_str()) {
                    // twitch commands capabilities have been acknowledged.
                    // do what needs to happen with them (like `/mods`).
                    Some(TWITCH_COMMANDS_CAP) => {
                        // request to get a list of moderators.
                        self.sender.privmsg(self.channel.as_str(), "/mods")
                    }
                    _ => {}
                }

                log::info!(
                    "Capability Acknowledged: {}",
                    what.as_ref().map(|w| w.as_str()).unwrap_or("*")
                );
            }
            Command::JOIN(ref channel, _, _) => {
                let user = m.source_nickname().unwrap_or("?");
                log::trace!("{} joined {}", user, channel);
            }
            Command::Response(..) => {
                log::trace!("Response: {}", m);
            }
            Command::PING(..) | Command::PONG(..) => {
                // ignore
            }
            Command::Raw(..) => {
                log::trace!("Raw: {:?}", m);
            }
            Command::NOTICE(_, ref message) => {
                let tags = Self::tags(&m);

                match tags.msg_id {
                    // Response to /mods request.
                    Some("room_mods") => {
                        self.moderators = parse_room_mods(message);
                        return Ok(());
                    }
                    _ => {
                        log::info!("unhandled notice: {:?}", m);
                    }
                }
            }
            _ => {
                log::info!("unhandled: {:?}", m);
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct OwnedUser {
    tags: OwnedTags,
    sender: Sender,
    name: String,
    target: String,
}

impl OwnedUser {
    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        self.sender
            .privmsg(self.target.as_str(), format!("{} -> {}", self.name, m));
    }
}

#[derive(Clone)]
pub struct User<'m> {
    pub tags: Tags<'m>,
    sender: Sender,
    pub name: &'m str,
    pub target: &'m str,
}

impl<'m> User<'m> {
    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        self.sender
            .privmsg(self.target, format!("{} -> {}", self.name, m));
    }

    /// Convert into an owned user.
    pub fn as_owned_user(&self) -> OwnedUser {
        OwnedUser {
            tags: self.tags.as_owned_tags(),
            sender: self.sender.clone(),
            name: self.name.to_owned(),
            target: self.target.to_owned(),
        }
    }
}

/// Struct of tags.
#[derive(Debug, Clone)]
pub struct Tags<'m> {
    /// contents of the id tag if present.
    id: Option<&'m str>,
    /// contents of the msg-id tag if present.
    msg_id: Option<&'m str>,
}

impl<'m> Tags<'m> {
    /// Convert into an owned set of tags.
    fn as_owned_tags(&self) -> OwnedTags {
        OwnedTags {
            id: self.id.map(|id| id.to_string()),
            msg_id: self.msg_id.map(|id| id.to_owned()),
        }
    }
}

/// Struct of tags.
#[derive(Debug, Clone)]
pub struct OwnedTags {
    id: Option<String>,
    msg_id: Option<String>,
}

#[derive(Debug)]
pub enum SenderThreadItem {
    Exit,
    Send(Message),
}

#[derive(serde::Serialize)]
pub struct TemplateVars<'a> {
    name: &'a str,
    target: &'a str,
}

#[derive(serde::Serialize)]
pub struct CounterVars<'a> {
    name: &'a str,
    target: &'a str,
    count: i32,
}

/// Parse the `room_mods` message.
fn parse_room_mods(message: &str) -> HashSet<String> {
    let mut out = HashSet::default();

    if let Some(index) = message.find(":") {
        let message = &message[(index + 1)..];
        out.extend(
            message
                .split(",")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from),
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::parse_room_mods;
    use hashbrown::HashSet;

    #[test]
    fn test_parse_room_mods() {
        assert_eq!(
            vec![String::from("foo"), String::from("bar")]
                .into_iter()
                .collect::<HashSet<String>>(),
            parse_room_mods("The moderators of this channel are: foo, bar")
        );

        assert_eq!(
            vec![String::from("a")]
                .into_iter()
                .collect::<HashSet<String>>(),
            parse_room_mods("The moderators of this channel are: a")
        );

        assert_eq!(
            vec![].into_iter().collect::<HashSet<String>>(),
            parse_room_mods("The moderators of this channel are:")
        );
    }
}
