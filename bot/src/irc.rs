use crate::{
    api,
    auth::Auth,
    bus, command, config, currency, db,
    features::{Feature, Features},
    idle, injector, module, oauth2,
    prelude::*,
    settings, stream_info, template, timer,
    utils::{self, Cooldown, Duration},
};
use failure::{bail, format_err, Error, ResultExt as _};
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
#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    bot: Option<String>,
    /// Cooldown for moderator actions.
    #[serde(default)]
    moderator_cooldown: Option<Cooldown>,
    /// Cooldown for creating clips.
    #[serde(default = "default_cooldown")]
    clip_cooldown: Cooldown,
    /// Cooldown for creating afterstream reminders.
    #[serde(default = "default_cooldown")]
    afterstream_cooldown: Cooldown,
    /// Name of the channel to join.
    pub channel: Option<String>,
    /// Whether or not to notify on currency rewards.
    #[serde(default)]
    notify_rewards: Option<bool>,
    /// Notify when bot starts.
    #[serde(default)]
    startup_message: Option<String>,
}

fn default_cooldown() -> Cooldown {
    Cooldown::from_duration(Duration::seconds(15))
}

/// Helper struct to construct IRC integration.
pub struct Irc {
    pub db: db::Database,
    pub youtube: Arc<api::YouTube>,
    pub streamer_twitch: api::Twitch,
    pub bot_twitch: api::Twitch,
    pub config: Arc<config::Config>,
    pub token: oauth2::SyncToken,
    pub commands: db::Commands,
    pub aliases: db::Aliases,
    pub promotions: db::Promotions,
    pub themes: db::Themes,
    pub bad_words: db::Words,
    pub after_streams: db::AfterStreams,
    pub global_bus: Arc<bus::Bus<bus::Global>>,
    pub modules: Vec<Box<dyn module::Module>>,
    pub shutdown: utils::Shutdown,
    pub settings: settings::Settings,
    pub auth: Auth,
    pub global_channel: Arc<RwLock<Option<String>>>,
    pub injector: injector::Injector,
}

impl Irc {
    pub async fn run(self) -> Result<(), Error> {
        let Irc {
            db,
            youtube,
            streamer_twitch,
            bot_twitch,
            config,
            token,
            commands,
            aliases,
            promotions,
            themes,
            bad_words,
            after_streams,
            global_bus,
            modules,
            shutdown,
            settings,
            auth,
            global_channel,
            injector,
        } = self;

        if config.streamer.is_some() {
            log::warn!("`streamer` setting has been deprecated from the configuration");
        }

        if config.irc.bot.is_some() {
            log::warn!("`[irc] bot` setting has been deprecated from the configuration");
        }

        if config.irc.channel.is_some() {
            log::warn!("`[irc] channel` setting has been deprecated from the configuration");
        }

        if config.irc.moderator_cooldown.is_some() {
            log::warn!(
                "`[irc] moderator_cooldown` setting has been deprecated from the configuration"
            );
        }

        if config.irc.notify_rewards.is_some() {
            log::warn!("`[irc] notify_rewards` setting has been deprecated from the configuration");
        }

        if config.irc.startup_message.is_some() {
            log::warn!(
                "`[irc] startup_message` setting has been deprecated from the configuration"
            );
        }

        if config.currency.is_some() {
            log::warn!("`[currency]` setting has been deprecated from the configuration");
        }

        loop {
            log::trace!("Waiting for token to become ready");

            future::try_join(
                streamer_twitch.token.wait_until_ready(),
                token.wait_until_ready(),
            )
            .await?;

            let (bot_info, streamer_info) = future::try_join(
                bot_twitch.validate_token(),
                streamer_twitch.validate_token(),
            )
            .await?;

            let bot = bot_info.login.to_lowercase();
            let bot = bot.as_str();

            let streamer = streamer_info.login.to_lowercase();
            let streamer = streamer.as_str();

            let channel = Arc::new(format!("#{}", streamer));

            // TODO: remove this migration next major release.
            if !config.aliases.is_empty() {
                log::warn!("The [[aliases]] section in the configuration is now deprecated.");

                if !settings
                    .get::<bool>("migration/aliases-migrated")?
                    .unwrap_or_default()
                {
                    log::warn!("Performing a one time migration of aliases from configuration.");

                    for alias in &config.aliases {
                        let template = template::Template::compile(&alias.replace)?;
                        aliases.edit(channel.as_str(), &alias.r#match, template)?;
                    }

                    settings.set("migration/aliases-migrated", true)?;
                }
            }

            // TODO: remove this migration next major release.
            if !config.themes.themes.is_empty() {
                log::warn!("The [themes] section in the configuration is now deprecated.");

                if !settings
                    .get::<bool>("migration/themes-migrated")?
                    .unwrap_or_default()
                {
                    log::warn!("Performing a one time migration of themes from configuration.");

                    for (name, theme) in &config.themes.themes {
                        let track_id = theme.track.clone();
                        themes.edit(channel.as_str(), name.as_str(), track_id)?;
                        themes.edit_duration(
                            channel.as_str(),
                            name.as_str(),
                            theme.offset.clone(),
                            theme.end.clone(),
                        )?;
                    }

                    settings.set("migration/themes-migrated", true)?;
                }
            }

            *global_channel.write() = Some(channel.to_string());

            let access_token = token.read()?.access_token().to_string();

            let irc_client_config = client::data::config::Config {
                nickname: Some(bot.to_string()),
                channels: Some(vec![(*channel).clone()]),
                password: Some(format!("oauth:{}", access_token)),
                server: Some(String::from(SERVER)),
                port: Some(6697),
                use_ssl: Some(true),
                ..client::data::config::Config::default()
            };

            let client = IrcClient::new_future(irc_client_config)?;

            let PackedIrcClient(client, send_future) = client.compat().await?;
            client.identify()?;

            let sender = Sender::new(channel.clone(), client.clone());

            let mut futures = Vec::<future::BoxFuture<'_, Result<(), Error>>>::new();

            let stream_info = {
                let interval = time::Duration::from_secs(60 * 5);
                let (stream_info, future) =
                    stream_info::setup(streamer, interval, streamer_twitch.clone());
                futures.push(future.boxed());
                stream_info
            };

            let mut vars = settings.vars();
            let threshold = vars.var("irc/idle-detection/threshold", 5)?;
            let idle = idle::Idle::new(threshold);
            futures.push(vars.run().boxed());

            let mut handlers = module::Handlers::default();

            futures.push(refresh_mods_future(sender.clone()).boxed());

            for module in modules.iter() {
                if log::log_enabled!(log::Level::Trace) {
                    log::trace!("initializing module: {}", module.ty());
                }

                let result = module.hook(module::HookContext {
                    handlers: &mut handlers,
                    futures: &mut futures,
                    stream_info: &stream_info,
                    idle: &idle,
                    config: &*config,
                    db: &db,
                    commands: &commands,
                    aliases: &aliases,
                    promotions: &promotions,
                    themes: &themes,
                    youtube: &youtube,
                    twitch: &bot_twitch,
                    streamer_twitch: &streamer_twitch,
                    sender: &sender,
                    settings: &settings,
                    injector: &injector,
                });

                result.with_context(|_| {
                    format_err!("failed to initialize module: {}", module.ty())
                })?;
            }

            let (future, currency_handler) = currency_admin::setup(&injector, &db)?;

            futures.push(future.boxed());

            let future = currency_loop(
                &mut futures,
                db.clone(),
                streamer_twitch.clone(),
                channel.clone(),
                sender.clone(),
                idle.clone(),
                &injector,
                &settings,
            )?;

            futures.push(future.boxed());

            if config.features.test(Feature::Admin) {
                handlers.insert(
                    "title",
                    misc::Title {
                        stream_info: stream_info.clone(),
                        twitch: &streamer_twitch,
                    },
                );

                handlers.insert(
                    "game",
                    misc::Game {
                        stream_info: stream_info.clone(),
                        twitch: &streamer_twitch,
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
                        bad_words: &bad_words,
                    },
                );
            }

            if config.features.test(Feature::EightBall) {
                handlers.insert("8ball", eight_ball::EightBall);
            }

            if config.features.test(Feature::Clip) {
                handlers.insert(
                    "clip",
                    clip::Clip {
                        stream_info: stream_info.clone(),
                        clip_cooldown: config.irc.clip_cooldown.clone(),
                        twitch: &bot_twitch,
                    },
                );
            }

            if config.features.test(Feature::AfterStream) {
                handlers.insert(
                    "afterstream",
                    after_stream::AfterStream {
                        cooldown: config.irc.afterstream_cooldown.clone(),
                        after_streams: &after_streams,
                    },
                );
            }

            futures.push(send_future.compat().map_err(Error::from).boxed());

            if !settings
                .get::<bool>("migration/whitelisted-hosts-migrated")?
                .unwrap_or_default()
            {
                log::warn!("Performing a one time migration of aliases from configuration.");
                settings.set("irc/whitelisted-hosts", &config.whitelisted_hosts)?;
                settings.set("migration/whitelisted-hosts-migrated", true)?;
            }

            let (mut whitelisted_hosts_stream, whitelisted_hosts) =
                settings.stream("irc/whitelisted-hosts", HashSet::<String>::new())?;

            let (mut moderator_cooldown_stream, moderator_cooldown) =
                settings.stream_opt("irc/moderator-cooldown")?;

            let startup_message = settings.get::<String>("irc/startup-message")?;

            let mut pong_timeout = None;

            let mut handler = Handler {
                streamer,
                sender: sender.clone(),
                moderators: HashSet::default(),
                whitelisted_hosts,
                commands: &commands,
                bad_words: &bad_words,
                global_bus: &global_bus,
                aliases: &aliases,
                features: config.features.clone(),
                api_url: config.api_url.clone(),
                thread_pool: Arc::new(ThreadPool::new()),
                moderator_cooldown,
                handlers,
                shutdown: &shutdown,
                idle: &idle,
                pong_timeout: &mut pong_timeout,
                token: &token,
                handler_shutdown: false,
                stream_info: &stream_info,
                auth: &auth,
                currency_handler,
            };

            let mut client_stream = client.stream().compat().fuse();
            let mut ping_interval = timer::Interval::new_interval(time::Duration::from_secs(10));

            let future = async move {
                handler.sender.cap_req(TWITCH_TAGS_CAP);
                handler.sender.cap_req(TWITCH_COMMANDS_CAP);

                if let Some(startup_message) = startup_message.as_ref() {
                    // greeting when bot joins
                    handler.sender.privmsg(startup_message);
                }

                loop {
                    futures::select! {
                        update = moderator_cooldown_stream.select_next_some() => {
                            handler.moderator_cooldown = update;
                        }
                        _ = ping_interval.select_next_some() => {
                            handler.send_ping()?;
                        }
                        timeout = handler.pong_timeout.current() => {
                            bail!("server not responding");
                        }
                        update = whitelisted_hosts_stream.next() => {
                            if let Some(update) = update {
                                handler.whitelisted_hosts = update;
                            }
                        },
                        message = client_stream.next() => {
                            if let Some(m) = message.transpose()? {
                                if let Err(e) = handler.handle(&m) {
                                    log::error!("Failed to handle message: {}", e);
                                }
                            }

                            if handler.handler_shutdown {
                                bail!("handler forcibly shut down");
                            }
                        }
                    }
                }
            };

            match future::try_join(future, future::try_join_all(futures)).await {
                Ok(((), _)) => break,
                Err(e) => {
                    log::warn!("IRC component errored, restarting in 5 seconds: {}", e);
                    timer::Delay::new(time::Instant::now() + time::Duration::from_secs(5)).await?;
                    continue;
                }
            }
        }

        Ok(())
    }
}

/// Set up a reward loop.
fn currency_loop<'a>(
    futures: &mut utils::Futures,
    db: db::Database,
    twitch: api::Twitch,
    channel: Arc<String>,
    sender: Sender,
    idle: idle::Idle,
    injector: &'a injector::Injector,
    settings: &settings::Settings,
) -> Result<impl Future<Output = Result<(), Error>> + 'a, Error> {
    let reward = 10;
    let interval = 60 * 10;

    let mut variables = settings.vars();
    let reward_percentage = variables.var("irc/viewer-reward%", 100)?;
    let (mut notify_rewards_stream, mut notify_rewards) =
        settings.stream("currency/notify-rewards", true)?;

    let (mut enabled_stream, enabled) = settings.stream("currency/enabled", false)?;
    let (mut name_stream, name) = settings.stream_opt("currency/name")?;

    let mut currency_builder = CurrencyBuilder {
        enabled,
        name: None,
        db: db.clone(),
        twitch: twitch.clone(),
    };

    currency_builder.name = name.map(Arc::new);

    let mut currency = currency_builder.build();

    if let Some(currency) = currency.clone() {
        injector.update(currency);
    }

    futures.push(variables.run().boxed());

    return Ok(async move {
        let mut interval = timer::Interval::new_interval(time::Duration::from_secs(interval));

        loop {
            futures::select! {
                update = notify_rewards_stream.select_next_some() => {
                    notify_rewards = update;
                }
                enabled = enabled_stream.select_next_some() => {
                    currency_builder.enabled = enabled;
                    currency = currency_builder.build();

                    if let Some(currency) = currency.clone() {
                        injector.update(currency);
                    } else {
                        injector.clear::<currency::Currency>();
                    }
                }
                name = name_stream.select_next_some() => {
                    currency_builder.name = name.map(Arc::new);
                    currency = currency_builder.build();

                    if let Some(currency) = currency.clone() {
                        injector.update(currency);
                    } else {
                        injector.clear::<currency::Currency>();
                    }
                }
                i = interval.select_next_some() => {
                    let currency = match currency.as_ref() {
                        Some(currency) => currency,
                        None => continue,
                    };

                    let _ = i?;

                    log::trace!("running reward loop");

                    let reward = (reward * *reward_percentage.read() as i64) / 100i64;
                    let count = currency
                        .add_channel_all(channel.to_string(), reward)
                        .await?;

                    if notify_rewards && count > 0 && !idle.is_idle() {
                        sender.privmsg(format!(
                            "/me has given {} {} to all viewers!",
                            reward, currency.name
                        ));
                    }
                }
            }
        }
    });

    struct CurrencyBuilder {
        enabled: bool,
        name: Option<Arc<String>>,
        db: db::Database,
        twitch: api::Twitch,
    }

    impl CurrencyBuilder {
        fn build(&self) -> Option<currency::Currency> {
            if !self.enabled {
                return None;
            }

            let name = Arc::new(self.name.as_ref()?.to_string());

            Some(currency::Currency {
                name,
                db: self.db.clone(),
                twitch: self.twitch.clone(),
            })
        }
    }
}

struct SenderInner {
    target: Arc<String>,
    client: IrcClient,
    thread_pool: ThreadPool,
    limiter: Mutex<ratelimit::Limiter>,
}

#[derive(Clone)]
pub struct Sender {
    inner: Arc<SenderInner>,
}

impl Sender {
    pub fn new(target: Arc<String>, client: IrcClient) -> Sender {
        let limiter = ratelimit::Builder::new().frequency(10).capacity(95).build();

        Sender {
            inner: Arc::new(SenderInner {
                target,
                client,
                thread_pool: ThreadPool::new(),
                limiter: Mutex::new(limiter),
            }),
        }
    }

    /// Get the channel this sender is associated with.
    pub fn channel(&self) -> &str {
        self.inner.target.as_str()
    }

    /// Send an immediate message, without taking rate limiting into account.
    fn send_immediate(&self, m: impl Into<Message>) {
        if let Err(e) = self.inner.client.send(m) {
            log_err!(e, "failed to send message");
        }
    }

    /// Send a message.
    fn send(&self, m: impl Into<Message>) {
        let inner = self.inner.clone();
        let m = m.into();

        self.inner.thread_pool.spawn(future01::lazy(move || {
            inner.limiter.lock().wait();

            if let Err(e) = inner.client.send(m) {
                log_err!(e, "failed to send message");
            }

            Ok(())
        }));
    }

    /// Send a PRIVMSG.
    pub fn privmsg(&self, f: impl fmt::Display) {
        self.send(Command::PRIVMSG(
            (*self.inner.target).clone(),
            f.to_string(),
        ))
    }

    /// Send a PRIVMSG without rate limiting.
    pub fn privmsg_immediate(&self, f: impl fmt::Display) {
        self.send_immediate(Command::PRIVMSG(
            (*self.inner.target).clone(),
            f.to_string(),
        ))
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
struct Handler<'a: 'h, 'to, 'h> {
    /// Current Streamer.
    streamer: &'a str,
    /// Queue for sending messages.
    sender: Sender,
    /// Moderators.
    moderators: HashSet<String>,
    /// Whitelisted hosts for links.
    whitelisted_hosts: HashSet<String>,
    /// All registered commands.
    commands: &'a db::Commands,
    /// Bad words.
    bad_words: &'a db::Words,
    /// For sending notifications.
    global_bus: &'a Arc<bus::Bus<bus::Global>>,
    /// Aliases.
    aliases: &'a db::Aliases,
    /// Enabled features.
    features: Features,
    /// Configured API URL.
    api_url: Option<String>,
    /// Thread pool used for driving futures.
    thread_pool: Arc<ThreadPool>,
    /// Active moderator cooldown.
    moderator_cooldown: Option<Cooldown>,
    /// Handlers for specific commands like `!skip`.
    handlers: module::Handlers<'h>,
    /// Handler for shutting down the service.
    shutdown: &'a utils::Shutdown,
    /// Build idle detection.
    idle: &'a idle::Idle,
    /// Pong timeout currently running.
    pong_timeout: &'to mut Option<timer::Delay>,
    /// OAuth 2.0 Token used to authenticate with IRC.
    token: &'a oauth2::SyncToken,
    /// Force a shutdown.
    handler_shutdown: bool,
    /// Stream information.
    stream_info: &'a stream_info::StreamInfo,
    /// Information about auth.
    auth: &'a Auth,
    /// Handler for currencies.
    currency_handler: currency_admin::Handler<'a>,
}

impl Handler<'_, '_, '_> {
    /// Run as user.
    fn as_user<'m>(&self, tags: Tags<'m>, m: &'m Message) -> Result<User<'m>, Error> {
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
    ) -> Result<(), Error> {
        match command {
            "ping" => {
                user.respond("What do you want?");
                self.global_bus.send(bus::Global::Ping);
            }
            other => {
                let handler = match (other, self.currency_handler.currency_name()) {
                    (other, Some(ref name)) if other == **name => {
                        Some(&mut self.currency_handler as &mut (dyn command::Handler + Send))
                    }
                    (other, Some(..)) | (other, None) => self.handlers.get_mut(other),
                };

                if let Some(handler) = handler {
                    let ctx = command::Context {
                        api_url: self.api_url.as_ref().map(|s| s.as_str()),
                        streamer: self.streamer,
                        sender: &self.sender,
                        moderators: &self.moderators,
                        moderator_cooldown: self.moderator_cooldown.as_mut(),
                        thread_pool: &self.thread_pool,
                        user,
                        it,
                        shutdown: self.shutdown,
                        alias: command::Alias { alias },
                        stream_info: &self.stream_info,
                        auth: &self.auth,
                    };

                    let scope = handler.scope();

                    if log::log_enabled!(log::Level::Trace) {
                        log::trace!("Auth: {:?} against {:?}", scope, ctx.roles());
                    }

                    // Test if user has the required scope to run the given
                    // command.
                    if let Some(scope) = scope {
                        if !ctx.has_scope(scope) {
                            if ctx.is_moderator() {
                                ctx.respond("You are not allowed to run that command");
                            } else {
                                ctx.privmsg(format!(
                                    "Do you think this is a democracy {name}? LUL",
                                    name = ctx.user.name
                                ));
                            }

                            return Ok(());
                        }
                    }

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
    fn delete_message<'local>(&mut self, tags: Tags<'local>) -> Result<(), Error> {
        let id = match tags.id {
            Some(id) => id,
            None => return Ok(()),
        };

        log::info!("Attempting to delete message: {}", id);

        self.sender.privmsg_immediate(format!("/delete {}", id));

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
                            self.sender.privmsg(&why);
                        }
                        Err(e) => {
                            log_err!(e, "failed to render response");
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

    /// Send a ping to the remote server.
    fn send_ping(&mut self) -> Result<(), Error> {
        self.sender
            .send_immediate(Command::PING(String::from(SERVER), None));

        *self.pong_timeout = Some(timer::Delay::new(
            time::Instant::now() + time::Duration::from_secs(5),
        ));

        Ok(())
    }

    /// Handle the given command.
    pub fn handle<'local>(&mut self, m: &'local Message) -> Result<(), Error> {
        match m.command {
            Command::PRIVMSG(_, ref message) => {
                let tags = Self::tags(&m);
                let user = self.as_user(tags.clone(), m)?;

                // only non-moderators and non-streamer bumps the idle counter.
                if !self.moderators.contains(user.name) && user.name != self.streamer {
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
                            self.sender.privmsg(response);
                        }
                    }

                    if command.starts_with('!') {
                        let command = &command[1..];

                        if let Err(e) = self.process_command(command, user, &mut it, alias) {
                            log_err!(e, "failed to process command");
                        }
                    }
                }

                if self.should_be_deleted(m, message) {
                    self.delete_message(tags)?;
                }
            }
            Command::CAP(_, CapSubCommand::ACK, _, ref what) => {
                match what.as_ref().map(|w| w.as_str()) {
                    // twitch commands capabilities have been acknowledged.
                    // do what needs to happen with them (like `/mods`).
                    Some(TWITCH_COMMANDS_CAP) => {
                        // request to get a list of moderators.
                        self.sender.privmsg("/mods")
                    }
                    _ => {}
                }

                log::trace!(
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
            Command::PING(ref server, ref other) => {
                log::trace!("Received PING, responding with PONG");
                self.sender
                    .send_immediate(Command::PONG(server.clone(), other.clone()));
            }
            Command::PONG(..) => {
                log::trace!("Received PONG, clearing PING timeout");
                *self.pong_timeout = None;
            }
            Command::Raw(..) => {
                log::trace!("Raw: {:?}", m);
            }
            Command::NOTICE(_, ref message) => {
                let tags = Self::tags(&m);

                match tags.msg_id {
                    _ if message == "Login authentication failed" => {
                        self.token.force_refresh()?;
                        self.handler_shutdown = true;
                    }
                    Some("no_mods") => {
                        self.moderators.clear();
                    }
                    // Response to /mods request.
                    Some("room_mods") => {
                        self.moderators = parse_room_mods(message);
                    }
                    Some(msg_id) => {
                        log::info!("unhandled notice w/ msg_id: {:?}: {:?}", msg_id, m);
                    }
                    None => {
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
    pub name: String,
    pub target: String,
}

impl OwnedUser {
    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        self.sender.privmsg(format!("{} -> {}", self.name, m));
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
        self.sender.privmsg(format!("{} -> {}", self.name, m));
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
async fn refresh_mods_future(sender: Sender) -> Result<(), Error> {
    let mut interval = timer::Interval::new_interval(time::Duration::from_secs(60 * 5));

    while let Some(i) = interval.next().await {
        let _ = i?;
        log::trace!("refreshing mods");
        sender.privmsg_immediate("/mods");
    }

    Ok(())
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
