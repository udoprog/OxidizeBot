use crate::{
    bus, command, config, currency, db,
    features::{Feature, Features},
    idle, module, oauth2, player, settings, stream_info, twitch, utils,
    utils::BoxFuture,
};
use failure::{format_err, ResultExt as _};
use futures::{
    future::{self, Future},
    stream::Stream,
    Async, Poll,
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
use std::{fmt, sync::Arc, time};
use tokio::timer;
use tokio_threadpool::ThreadPool;

mod after_stream;
mod bad_word;
mod clip;
mod currency_admin;
mod eight_ball;
mod misc;

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
    utils::Cooldown::from_duration(utils::Duration::seconds(15))
}

/// Helper struct to construct IRC integration.
pub struct Irc<'a> {
    pub core: &'a mut tokio_core::reactor::Core,
    pub db: db::Database,
    pub streamer_twitch: twitch::Twitch,
    pub bot_twitch: twitch::Twitch,
    pub config: &'a config::Config,
    pub irc_config: &'a Config,
    pub currency: Option<currency::Currency>,
    pub token: Arc<RwLock<oauth2::Token>>,
    pub commands: db::Commands,
    pub aliases: db::Aliases,
    pub promotions: db::Promotions,
    pub bad_words: db::Words,
    pub after_streams: db::AfterStreams,
    pub global_bus: Arc<bus::Bus>,
    pub modules: &'a [Box<dyn module::Module>],
    pub shutdown: utils::Shutdown,
    pub settings: settings::Settings,
    pub player: Option<&'a player::Player>,
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
            aliases,
            promotions,
            bad_words,
            after_streams,
            global_bus,
            modules,
            shutdown,
            settings,
            player,
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

        futures.push(Box::new(refresh_mods_future(
            sender.clone(),
            irc_config.channel.clone(),
        )));

        let stream_info = {
            let interval = time::Duration::from_secs(30);
            let (stream_info, future) =
                stream_info::setup(config.streamer.as_str(), interval, streamer_twitch.clone());
            futures.push(Box::new(future));
            stream_info
        };

        let threshold = settings.sync_var(core, "irc/idle-detection/threshold", 5)?;

        let idle = idle::Idle::new(threshold);

        for module in modules {
            let result = module.hook(module::HookContext {
                core,
                config,
                irc_config,
                db: &db,
                commands: &commands,
                aliases: &aliases,
                promotions: &promotions,
                handlers: &mut handlers,
                currency: self.currency.as_ref(),
                twitch: &bot_twitch,
                futures: &mut futures,
                stream_info: &stream_info,
                sender: &sender,
                settings: &settings,
                idle: &idle,
                player,
            });

            result.with_context(|_| {
                failure::format_err!("failed to initialize module: {}", module.ty())
            })?;
        }

        if let Some(currency) = self.currency.as_ref() {
            handlers.insert(
                &*currency.name,
                currency_admin::Handler {
                    currency: currency.clone(),
                    db: db.clone(),
                },
            );

            let reward = 10;
            let interval = 60 * 10;

            let reward_percentage = settings.sync_var(core, "irc/viewer-reward%", 100)?;

            let future = reward_loop(
                irc_config,
                reward,
                interval,
                sender.clone(),
                currency,
                &idle,
                reward_percentage,
            );

            futures.push(Box::new(future));
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
                    after_streams,
                },
            );
        }

        futures.push(Box::new(send_future.map_err(failure::Error::from)));

        if !settings
            .get::<bool>("migration/whitelisted-hosts-migrated")?
            .unwrap_or_default()
        {
            log::warn!("Performing a one time migration of aliases from configuration.");
            settings.set("irc/whitelisted-hosts", &config.whitelisted_hosts)?;
            settings.set("migration/whitelisted-hosts-migrated", true)?;
        }

        let (whitelisted_hosts_stream, whitelisted_hosts) =
            settings.init_and_stream("irc/whitelisted-hosts", HashSet::<String>::new())?;

        let handler = Handler {
            streamer: config.streamer.clone(),
            channel: irc_config.channel.clone(),
            sender: sender.clone(),
            moderators: HashSet::default(),
            whitelisted_hosts,
            commands,
            bad_words,
            global_bus,
            aliases,
            features: config.features.clone(),
            api_url: config.api_url.clone(),
            thread_pool: Arc::new(ThreadPool::new()),
            moderator_cooldown: irc_config.moderator_cooldown.clone(),
            handlers,
            shutdown,
            idle,
        };

        futures.push(Box::new(IrcFuture {
            handler,
            whitelisted_hosts_stream,
            client_stream: client.stream(),
        }));

        Ok(future::join_all(futures).map(|_| ()))
    }
}

/// Set up a reward loop.
fn reward_loop(
    config: &Config,
    reward: i64,
    interval: u64,
    sender: Sender,
    currency: &currency::Currency,
    idle: &idle::Idle,
    reward_percentage: Arc<RwLock<u32>>,
) -> impl Future<Item = (), Error = failure::Error> + Send + 'static {
    // Add currency timer.
    timer::Interval::new_interval(time::Duration::from_secs(interval))
        .map_err(Into::into)
        // fetch all users.
        .and_then({
            let channel = config.channel.to_string();
            let currency = currency.clone();

            move |_| {
                let reward = (reward * *reward_percentage.read() as i64) / 100i64;
                log::trace!("running reward loop");
                currency
                    .add_channel_all(channel.as_str(), reward)
                    .map(move |count| (count, reward))
            }
        })
        .map({
            let idle = idle.clone();
            let notify_rewards = config.notify_rewards;
            let channel = config.channel.to_string();
            let currency = currency.clone();

            move |(count, reward)| {
                if notify_rewards && count > 0 && !idle.is_idle() {
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
    /// Queue for sending messages.
    sender: Sender,
    /// Moderators.
    moderators: HashSet<String>,
    /// Whitelisted hosts for links.
    whitelisted_hosts: HashSet<String>,
    /// All registered commands.
    commands: db::Commands,
    /// Bad words.
    bad_words: db::Words,
    /// For sending notifications.
    global_bus: Arc<bus::Bus>,
    /// Aliases.
    aliases: db::Aliases,
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
    /// Build idle detection.
    idle: idle::Idle,
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
        command: &str,
        user: User<'m>,
        it: &mut utils::Words<'m>,
        alias: Option<(&str, &str)>,
    ) -> Result<(), failure::Error> {
        match command {
            "ping" => {
                user.respond("What do you want?");
                self.global_bus.send(bus::Message::Ping);
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
                        alias: command::Alias { alias },
                    };

                    handler.handle(ctx)?;
                    return Ok(());
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
                    let why = why.render_to_string(&BadWordsVars {
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
                let user = self.as_user(tags.clone(), m)?;

                // only non-moderators and non-streamer bumps the idle counter.
                if !self.moderators.contains(user.name) || user.name != self.streamer {
                    self.idle.seen();
                }

                let mut it = utils::Words::new(message);

                // NB: needs to store locally to maintain a reference to it.
                let mut alias = None;
                let a = self.aliases.lookup(user.target, it.clone());

                if let Some((m, a)) = a.as_ref() {
                    it = utils::Words::new(a.as_str());
                    alias = Some((*m, a.as_str()));
                }

                if let Some(command) = it.next() {
                    if self.features.test(Feature::Command) {
                        if let Some(command) = self.commands.get(user.target, command) {
                            if command.has_var("count") {
                                self.commands.increment(&*command)?;
                            }

                            let vars = CommandVars {
                                name: &user.name,
                                target: &user.target,
                                count: command.count(),
                            };

                            let response = command.render(&vars)?;
                            self.sender.privmsg(user.target, response);
                        }
                    }

                    if command.starts_with('!') {
                        let command = &command[1..];

                        if let Err(e) = self.process_command(command, user, &mut it, alias) {
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
                    Some("no_mods") => {
                        self.moderators.clear();
                    }
                    // Response to /mods request.
                    Some("room_mods") => {
                        self.moderators = parse_room_mods(message);
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

struct IrcFuture {
    handler: Handler,
    whitelisted_hosts_stream: settings::Stream<HashSet<String>>,
    client_stream: client::ClientStream,
}

impl Future for IrcFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let mut not_ready = true;

            let m = self
                .client_stream
                .poll()
                .with_context(|_| failure::format_err!("failed to poll for IRC message"))?;

            match m {
                Async::NotReady => (),
                Async::Ready(None) => {
                    failure::bail!("irc stream ended");
                }
                Async::Ready(Some(m)) => {
                    if let Err(e) = self.handler.handle(&m) {
                        log::error!("failed to handle message: {}", e);
                    }

                    not_ready = false;
                }
            }

            if let Async::Ready(whitelisted_hosts) = self.whitelisted_hosts_stream.poll()? {
                self.handler.whitelisted_hosts = whitelisted_hosts;
                not_ready = false;
            }

            if not_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}

#[derive(Clone)]
pub struct OwnedUser {
    tags: OwnedTags,
    sender: Sender,
    pub name: String,
    pub target: String,
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
    /// Test if the current user is the given user.
    pub fn is(&self, name: &str) -> bool {
        self.name == name.to_lowercase()
    }

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
pub struct BadWordsVars<'a> {
    name: &'a str,
    target: &'a str,
}

#[derive(serde::Serialize)]
pub struct CommandVars<'a> {
    name: &'a str,
    target: &'a str,
    count: i32,
}

// Future to refresh moderators every 5 minutes.
fn refresh_mods_future(
    sender: Sender,
    channel: Arc<String>,
) -> impl Future<Item = (), Error = failure::Error> + Send + 'static {
    let interval = timer::Interval::new_interval(time::Duration::from_secs(60 * 5));

    interval
        .map_err(|_| failure::format_err!("failed to refresh mods"))
        .for_each(move |_| {
            log::trace!("refreshing mods");
            sender.privmsg(channel.as_str(), "/mods");
            Ok(())
        })
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
