use crate::{
    api::{self, twitch},
    auth::{Auth, Role, Scope},
    bus, command,
    currency::{Currency, CurrencyBuilder},
    db, idle,
    injector::{Injector, Key},
    message_log::MessageLog,
    module, oauth2,
    prelude::*,
    settings, stream_info, timer,
    utils::{self, Cooldown, Duration},
};
use failure::{bail, format_err, Error, ResultExt as _};
use hashbrown::{HashMap, HashSet};
use irc::{
    client::{self, ext::ClientExt, Client, IrcClient, PackedIrcClient},
    proto::{
        command::{CapSubCommand, Command},
        message::{Message, Tag},
    },
};
use leaky_bucket::LeakyBuckets;
use parking_lot::RwLock;
use std::{fmt, sync::Arc, time};
use tokio_threadpool::ThreadPool;

// re-exports
pub use self::sender::Sender;

mod chat_log;
mod currency_admin;
mod sender;

const SERVER: &'static str = "irc.chat.twitch.tv";
const TWITCH_TAGS_CAP: &'static str = "twitch.tv/tags";
const TWITCH_COMMANDS_CAP: &'static str = "twitch.tv/commands";

/// Helper struct to construct IRC integration.
pub struct Irc {
    pub bad_words: db::Words,
    pub global_bus: Arc<bus::Bus<bus::Global>>,
    pub command_bus: Arc<bus::Bus<bus::Command>>,
    pub modules: Vec<Box<dyn module::Module>>,
    pub shutdown: utils::Shutdown,
    pub settings: settings::Settings,
    pub auth: Auth,
    pub global_channel: Arc<RwLock<Option<String>>>,
    pub injector: Injector,
    pub stream_state_tx: mpsc::Sender<stream_info::StreamState>,
    pub message_log: MessageLog,
}

impl Irc {
    pub async fn run(self) -> Result<(), Error> {
        let Irc {
            bad_words,
            global_bus,
            command_bus,
            modules,
            shutdown,
            settings,
            auth,
            global_channel,
            injector,
            stream_state_tx,
            message_log,
        } = self;

        let (mut streamer_twitch_stream, mut streamer_twitch_opt) =
            injector.stream_key(&Key::<oauth2::SyncToken>::tagged(
                oauth2::TokenId::TwitchStreamer,
            )?);

        let (mut bot_twitch_stream, mut bot_twitch_opt) =
            injector.stream_key(&Key::<oauth2::SyncToken>::tagged(
                oauth2::TokenId::TwitchBot,
            )?);

        'outer: loop {
            let (streamer_twitch, bot_twitch) =
                match (streamer_twitch_opt.as_ref(), bot_twitch_opt.as_ref()) {
                    (Some(streamer_twitch), Some(bot_twitch)) => (streamer_twitch, bot_twitch),
                    (_, _) => {
                        futures::select! {
                            update = streamer_twitch_stream.select_next_some() => {
                                streamer_twitch_opt = update;
                            },
                            update = bot_twitch_stream.select_next_some() => {
                                bot_twitch_opt = update;
                            },
                        }

                        continue;
                    }
                };

            let bot_twitch = api::Twitch::new(bot_twitch.clone())?;
            let streamer_twitch = api::Twitch::new(streamer_twitch.clone())?;

            let (bot, streamer) =
                future::try_join(bot_twitch.user(), streamer_twitch.user()).await?;

            let bot = Arc::new(bot);
            let streamer = Arc::new(streamer);

            let channel = Arc::new(streamer_twitch.channel().await?);

            log::trace!("Channel: {:?}", channel);
            log::trace!("Streamer: {:?}", streamer);
            log::trace!("Bot: {:?}", bot);

            let chat_channel = format!("#{}", channel.name);
            *global_channel.write() = Some(chat_channel.clone());

            let access_token = bot_twitch.token.read()?.access_token().to_string();

            let irc_client_config = client::data::config::Config {
                nickname: Some(bot.name.to_string()),
                channels: Some(vec![chat_channel.clone()]),
                password: Some(format!("oauth:{}", access_token)),
                server: Some(String::from(SERVER)),
                port: Some(6697),
                use_ssl: Some(true),
                ..client::data::config::Config::default()
            };

            let client = IrcClient::new_future(irc_client_config)?;

            let PackedIrcClient(client, send_future) = client.compat().await?;
            client.identify()?;

            let irc_settings = settings.scoped("irc");
            let url_whitelist_enabled = irc_settings.var("url-whitelist/enabled", true)?;
            let bad_words_enabled = irc_settings.var("bad-words/enabled", false)?;
            let sender_ty = irc_settings.var("sender-type", sender::Type::Chat)?;
            let threshold = irc_settings.var("idle-detection/threshold", 5)?;
            let idle = idle::Idle::new(threshold);

            let nightbot = injector.var::<Arc<api::NightBot>>()?;

            let buckets = LeakyBuckets::new();

            let sender = Sender::new(
                sender_ty,
                chat_channel.clone(),
                client.clone(),
                nightbot.clone(),
                &buckets,
            )?;

            let mut futures = Vec::<future::BoxFuture<'_, Result<(), Error>>>::new();

            let future = async move {
                buckets.coordinate().await?;
                Ok(())
            };

            futures.push(future.boxed());

            let stream_info = {
                let (stream_info, mut stream_state_rx, future) =
                    stream_info::setup(streamer.clone(), streamer_twitch.clone());

                let mut stream_state_tx = stream_state_tx.clone();

                let forward = async move {
                    loop {
                        let m = stream_state_rx.select_next_some().await;
                        stream_state_tx
                            .send(m)
                            .await
                            .map_err(|_| format_err!("failed to send"))?;
                    }
                };

                futures.push(forward.boxed());
                futures.push(future.boxed());

                stream_info
            };

            futures.push(refresh_mods_future(sender.clone()).boxed());

            let mut handlers = module::Handlers::default();

            for module in modules.iter() {
                if log::log_enabled!(log::Level::Trace) {
                    log::trace!("initializing module: {}", module.ty());
                }

                let result = module.hook(module::HookContext {
                    handlers: &mut handlers,
                    futures: &mut futures,
                    stream_info: &stream_info,
                    idle: &idle,
                    twitch: &bot_twitch,
                    streamer_twitch: &streamer_twitch,
                    sender: &sender,
                    settings: &settings,
                    injector: &injector,
                    auth: &auth,
                });

                result.with_context(|_| {
                    format_err!("failed to initialize module: {}", module.ty())
                })?;
            }

            let (future, currency_handler) = currency_admin::setup(&injector)?;

            futures.push(future.boxed());

            let future = currency_loop(
                streamer_twitch.clone(),
                channel.clone(),
                sender.clone(),
                idle.clone(),
                &injector,
                &settings,
            )?;

            futures.push(future.boxed());
            futures.push(send_future.compat().map_err(Error::from).boxed());

            let (mut whitelisted_hosts_stream, whitelisted_hosts) =
                settings.stream("irc/whitelisted-hosts").or_default()?;

            let (mut moderator_cooldown_stream, moderator_cooldown) =
                settings.stream("irc/moderator-cooldown").optional()?;

            let (mut api_url_stream, api_url) = settings.stream("remote/api-url").optional()?;

            let join_message = settings.get::<String>("irc/join-message")?;
            let leave_message = settings.get::<String>("irc/leave-message")?;

            let mut chat_log_builder = chat_log::Builder::new(
                bot_twitch.clone(),
                &injector,
                message_log.clone(),
                settings.scoped("chat-log"),
            )?;

            let (mut commands_stream, commands) = injector.stream();
            let (mut aliases_stream, aliases) = injector.stream();

            let mut pong_timeout = None;

            let mut handler = Handler {
                streamer,
                sender: sender.clone(),
                moderators: Default::default(),
                vips: Default::default(),
                whitelisted_hosts,
                commands,
                bad_words: &bad_words,
                global_bus: &global_bus,
                aliases,
                api_url,
                thread_pool: Arc::new(ThreadPool::new()),
                moderator_cooldown,
                handlers,
                shutdown: &shutdown,
                idle: &idle,
                pong_timeout: &mut pong_timeout,
                token: &bot_twitch.token,
                handler_shutdown: false,
                stream_info: &stream_info,
                auth: &auth,
                scope_cooldowns: auth.scope_cooldowns(),
                currency_handler,
                url_whitelist_enabled,
                bad_words_enabled,
                message_hooks: Default::default(),
                chat_log: chat_log_builder.build()?,
                channel,
            };

            let mut client_stream = client.stream().compat().fuse();
            let mut ping_interval = timer::Interval::new_interval(time::Duration::from_secs(10));

            let mut futures = future::try_join_all(futures).fuse();

            handler.sender.cap_req(TWITCH_TAGS_CAP);
            handler.sender.cap_req(TWITCH_COMMANDS_CAP);

            if let Some(join_message) = join_message.as_ref() {
                // greeting when bot joins
                handler.sender.privmsg_immediate(join_message);
            }

            let mut commands = command_bus.add_rx().compat().fuse();

            let mut leave = false;

            while !leave {
                futures::select! {
                    command = commands.select_next_some() => {
                        match command? {
                            bus::Command::Raw { command } => {
                                log::trace!("Raw command: {}", command);

                                if let Err(e) = handler.raw(&command).await {
                                    log::error!("Failed to handle message: {}", e);
                                }
                            }
                        }
                    }
                    future = futures => {
                        match future {
                            Ok(..) => break 'outer,
                            Err(e) => {
                                log::warn!("IRC component errored, restarting in 5 seconds: {}", e);
                                timer::Delay::new(time::Instant::now() + time::Duration::from_secs(5)).await?;
                                continue 'outer;
                            }
                        }
                    }
                    update = streamer_twitch_stream.select_next_some() => {
                        streamer_twitch_opt = update;
                        leave = true;
                    },
                    update = bot_twitch_stream.select_next_some() => {
                        bot_twitch_opt = update;
                        leave = true;
                    },
                    update = commands_stream.select_next_some() => {
                        handler.commands = update;
                    }
                    update = aliases_stream.select_next_some() => {
                        handler.aliases = update;
                    }
                    cache = chat_log_builder.cache_stream.select_next_some() => {
                        chat_log_builder.cache = cache;
                        handler.chat_log = chat_log_builder.build()?;
                    }
                    update = chat_log_builder.enabled_stream.select_next_some() => {
                        chat_log_builder.enabled = update;
                        chat_log_builder.message_log.enabled(update);
                        handler.chat_log = chat_log_builder.build()?;
                    }
                    update = chat_log_builder.emotes_enabled_stream.select_next_some() => {
                        chat_log_builder.emotes_enabled = update;
                        handler.chat_log = chat_log_builder.build()?;
                    }
                    update = api_url_stream.select_next_some() => {
                        handler.api_url = update;
                    }
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
                            if let Err(e) = handler.handle(m).await {
                                log::error!("Failed to handle message: {}", e);
                            }
                        }

                        if handler.handler_shutdown {
                            bail!("handler forcibly shut down");
                        }
                    }
                }
            }

            // NB: probably not being sent right now. Figure out a better way to do graceful shutdown.
            if let Some(leave_message) = leave_message.as_ref() {
                // greeting when bot leaves
                handler.sender.privmsg_immediate(leave_message);
            }
        }

        Ok(())
    }
}

/// Set up a reward loop.
fn currency_loop<'a>(
    twitch: api::Twitch,
    channel: Arc<twitch::Channel>,
    sender: Sender,
    idle: idle::Idle,
    injector: &'a Injector,
    settings: &settings::Settings,
) -> Result<impl Future<Output = Result<(), Error>> + 'a, Error> {
    let reward = 10;
    let default_interval = Duration::seconds(60 * 10);

    let (mut interval_stream, mut interval) = settings
        .stream("irc/viewer-reward/interval")
        .or_with(default_interval)?;

    let reward_percentage = settings.var("irc/viewer-reward%", 100)?;
    let (mut viewer_reward_stream, viewer_reward) = settings
        .stream("irc/viewer-reward/enabled")
        .or_with(false)?;
    let (mut notify_rewards_stream, mut notify_rewards) =
        settings.stream("currency/notify-rewards").or_with(true)?;

    let (mut ty_stream, ty) = settings.stream("currency/type").or_default()?;
    let (mut enabled_stream, enabled) = settings.stream("currency/enabled").or_default()?;
    let (mut name_stream, name) = settings.stream("currency/name").optional()?;
    let (mut command_enabled_stream, command_enabled) =
        settings.stream("currency/command-enabled").or_with(true)?;
    let (mut mysql_url_stream, mysql_url) = settings.stream("currency/mysql/url").optional()?;
    let (mut mysql_schema_stream, mysql_schema) =
        settings.stream("currency/mysql/schema").or_default()?;

    let (mut db_stream, db) = injector.stream::<db::Database>();

    let mut builder = CurrencyBuilder::new(twitch.clone(), mysql_schema);
    builder.db = db;
    builder.ty = ty;
    builder.enabled = enabled;
    builder.command_enabled = command_enabled;
    builder.name = name.map(Arc::new);
    builder.mysql_url = mysql_url;

    let build = |injector: &Injector, builder: &CurrencyBuilder| match builder.build() {
        Some(currency) => {
            injector.update(currency.clone());
            Some(currency)
        }
        None => {
            injector.clear::<Currency>();
            None
        }
    };

    let mut currency = build(injector, &builder);

    return Ok(async move {
        let new_timer = |interval: &Duration, viewer_reward: bool| match viewer_reward {
            true if !interval.is_empty() => Some(timer::Interval::new_interval(interval.as_std())),
            _ => None,
        };

        let mut timer = new_timer(&interval, viewer_reward);

        loop {
            futures::select! {
                update = interval_stream.select_next_some() => {
                    interval = update;
                    timer = new_timer(&interval, viewer_reward);
                }
                update = notify_rewards_stream.select_next_some() => {
                    notify_rewards = update;
                }
                update = db_stream.select_next_some() => {
                    builder.db = update;
                    currency = build(injector, &builder);
                }
                enabled = enabled_stream.select_next_some() => {
                    builder.enabled = enabled;
                    currency = build(injector, &builder);
                }
                update = ty_stream.select_next_some() => {
                    builder.ty = update;
                    currency = build(injector, &builder);
                }
                name = name_stream.select_next_some() => {
                    builder.name = name.map(Arc::new);
                    currency = build(injector, &builder);
                }
                mysql_url = mysql_url_stream.select_next_some() => {
                    builder.mysql_url = mysql_url;
                    currency = build(injector, &builder);
                }
                update = mysql_schema_stream.select_next_some() => {
                    builder.mysql_schema = update;
                    currency = build(injector, &builder);
                }
                command_enabled = command_enabled_stream.select_next_some() => {
                    builder.command_enabled = command_enabled;
                    currency = build(injector, &builder);
                }
                viewer_reward = viewer_reward_stream.select_next_some() => {
                    timer = new_timer(&interval, viewer_reward);
                }
                i = timer.select_next_some() => {
                    let currency = match currency.as_ref() {
                        Some(currency) => currency,
                        None => continue,
                    };

                    let _ = i?;

                    log::trace!("running reward loop");

                    let reward = (reward * *reward_percentage.read() as i64) / 100i64;
                    let count = currency
                        .add_channel_all(&channel.name, reward)
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
}

/// Handler for incoming messages.
struct Handler<'a> {
    /// Current Streamer.
    streamer: Arc<twitch::User>,
    /// Queue for sending messages.
    sender: Sender,
    /// Moderators.
    moderators: Arc<RwLock<HashSet<String>>>,
    /// VIPs.
    vips: Arc<RwLock<HashSet<String>>>,
    /// Whitelisted hosts for links.
    whitelisted_hosts: HashSet<String>,
    /// All registered commands.
    commands: Option<db::Commands>,
    /// Bad words.
    bad_words: &'a db::Words,
    /// For sending notifications.
    global_bus: &'a Arc<bus::Bus<bus::Global>>,
    /// Aliases.
    aliases: Option<db::Aliases>,
    /// Configured API URL.
    api_url: Option<String>,
    /// Thread pool used for driving futures.
    thread_pool: Arc<ThreadPool>,
    /// Active moderator cooldown.
    moderator_cooldown: Option<Cooldown>,
    /// Handlers for specific commands like `!skip`.
    handlers: module::Handlers<'a>,
    /// Handler for shutting down the service.
    shutdown: &'a utils::Shutdown,
    /// Build idle detection.
    idle: &'a idle::Idle,
    /// Pong timeout currently running.
    pong_timeout: &'a mut Option<timer::Delay>,
    /// OAuth 2.0 Token used to authenticate with IRC.
    token: &'a oauth2::SyncToken,
    /// Force a shutdown.
    handler_shutdown: bool,
    /// Stream information.
    stream_info: &'a stream_info::StreamInfo,
    /// Information about auth.
    auth: &'a Auth,
    /// Active scope cooldowns.
    scope_cooldowns: HashMap<Scope, Cooldown>,
    /// Handler for currencies.
    currency_handler: currency_admin::Handler,
    bad_words_enabled: Arc<RwLock<bool>>,
    url_whitelist_enabled: Arc<RwLock<bool>>,
    /// A hook that can be installed to peek at all incoming messages.
    message_hooks: HashMap<String, Box<dyn command::MessageHook>>,
    /// Handler for chat logs.
    chat_log: Option<chat_log::ChatLog>,
    /// Information on the current channel.
    channel: Arc<twitch::Channel>,
}

/// Handle a command.
pub async fn process_command<'a, 'b: 'a>(
    command: &'a str,
    ctx: command::Context<'a>,
    global_bus: &'a Arc<bus::Bus<bus::Global>>,
    currency_handler: &'a mut currency_admin::Handler,
    handlers: &'a mut module::Handlers<'b>,
) -> Result<(), Error> {
    match command {
        "ping" => {
            ctx.user.respond("What do you want?");
            global_bus.send(bus::Global::Ping);
        }
        other => {
            log::trace!("Testing command: {}", other);

            let handler = match (other, currency_handler.command_name()) {
                (other, Some(ref name)) if other == **name => {
                    Some(currency_handler as &mut (dyn command::Handler + Send))
                }
                (other, Some(..)) | (other, None) => handlers.get_mut(other),
            };

            if let Some(handler) = handler {
                let scope = handler.scope();

                if log::log_enabled!(log::Level::Trace) {
                    log::trace!("Auth: {:?} against {:?}", scope, ctx.user.roles());
                }

                // Test if user has the required scope to run the given
                // command.
                if let Some(scope) = scope {
                    if !ctx.user.has_scope(scope) {
                        if ctx.user.is_moderator() {
                            ctx.respond("You are not allowed to run that command");
                        } else {
                            if let Some(display_name) = ctx.user.display_name() {
                                ctx.privmsg(format!(
                                    "Do you think this is a democracy {name}? LUL",
                                    name = display_name
                                ));
                            }
                        }

                        return Ok(());
                    }
                }

                handler.handle(ctx).await?;
                return Ok(());
            }
        }
    }

    Ok(())
}

impl<'a> Handler<'a> {
    /// Delete the given message.
    fn delete_message(&self, user: &User) -> Result<(), Error> {
        let id = match &user.inner.tags.id {
            Some(id) => id,
            None => return Ok(()),
        };

        log::info!("Attempting to delete message: {}", id);
        user.inner.sender.delete(id);
        Ok(())
    }

    /// Test if the message should be deleted.
    fn should_be_deleted(&self, user: &User, message: &str) -> bool {
        // Moderators can say whatever they want.
        if user.is_moderator() {
            return false;
        }

        if *self.bad_words_enabled.read() {
            if let Some(word) = self.test_bad_words(message) {
                if let Some(why) = word.why.as_ref() {
                    let why = why.render_to_string(&BadWordsVars {
                        name: user.display_name(),
                        target: user.channel(),
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

        if !user.has_scope(Scope::ChatBypassUrlWhitelist) && *self.url_whitelist_enabled.read() {
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
    fn has_bad_link(&self, message: &str) -> bool {
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

    /// Process the given command.
    pub async fn process_message(&mut self, user: &User, message: &str) -> Result<(), Error> {
        for (key, hook) in &mut self.message_hooks {
            hook.peek(&user, message)
                .with_context(|_| format_err!("hook `{}` failed", key))?;
        }

        // only non-moderators and non-streamer bumps the idle counter.
        if !user.is_streamer() {
            self.idle.seen();
        }

        let mut it = utils::Words::new(message);
        // NB: declared here to be in scope.
        let a;

        if let Some(aliases) = self.aliases.as_ref() {
            // NB: needs to store locally to maintain a reference to it.
            a = aliases.lookup(user.channel(), it.clone());

            if let Some(a) = &a {
                it = utils::Words::new(a.as_str());
            }
        }

        if let Some(command) = it.next() {
            if let Some(commands) = self.commands.as_ref() {
                if let Some(command) = commands.get(user.channel(), &command) {
                    if command.has_var("count") {
                        commands.increment(&*command)?;
                    }

                    let vars = CommandVars {
                        name: user.display_name(),
                        target: user.channel(),
                        count: command.count(),
                        rest: it.rest(),
                    };

                    let response = command.render(&vars)?;
                    self.sender.privmsg(response);
                }
            }

            if command.starts_with('!') {
                let command = &command[1..];

                let ctx = command::Context {
                    api_url: self.api_url.as_ref().map(|s| s.as_str()),
                    sender: &self.sender,
                    thread_pool: &self.thread_pool,
                    user: user.clone(),
                    it,
                    shutdown: self.shutdown,
                    scope_cooldowns: &mut self.scope_cooldowns,
                    message_hooks: &mut self.message_hooks,
                };

                let result = process_command(
                    command,
                    ctx,
                    &self.global_bus,
                    &mut self.currency_handler,
                    &mut self.handlers,
                );

                if let Err(e) = result.await {
                    log_err!(e, "failed to process command");
                }
            }
        }

        if self.should_be_deleted(&user, message) {
            self.delete_message(&user)?;
        }

        Ok(())
    }

    /// Run the given raw command.
    pub async fn raw(&mut self, message: &str) -> Result<(), Error> {
        let tags = Tags::default();

        let user = User {
            inner: Arc::new(UserInner {
                tags,
                sender: self.sender.clone(),
                principal: Principal::Injected,
                streamer: self.streamer.clone(),
                moderators: self.moderators.clone(),
                vips: self.vips.clone(),
                stream_info: self.stream_info.clone(),
                auth: self.auth.clone(),
            }),
        };

        self.process_message(&user, message).await
    }

    /// Handle the given command.
    pub async fn handle(&mut self, mut m: Message) -> Result<(), Error> {
        match m.command {
            Command::PRIVMSG(_, ref message) => {
                let tags = Tags::from_tags(m.tags.take());

                let name = m
                    .source_nickname()
                    .ok_or_else(|| format_err!("expected user info"))?
                    .to_string();

                if let Some(chat_log) = self.chat_log.as_ref().cloned() {
                    let tags = tags.clone();
                    let channel = self.channel.clone();
                    let name = name.clone();
                    let message = message.clone();

                    let future = async move {
                        chat_log.observe(&tags, &*channel, &name, &message).await;
                    };

                    self.thread_pool
                        .spawn(Compat::new(Box::pin(future.unit_error())));
                }

                let user = User {
                    inner: Arc::new(UserInner {
                        tags,
                        sender: self.sender.clone(),
                        principal: Principal::User { name },
                        streamer: self.streamer.clone(),
                        moderators: self.moderators.clone(),
                        vips: self.vips.clone(),
                        stream_info: self.stream_info.clone(),
                        auth: self.auth.clone(),
                    }),
                };

                self.process_message(&user, message).await?;
            }
            Command::CAP(_, CapSubCommand::ACK, _, ref what) => {
                match what.as_ref().map(|w| w.as_str()) {
                    // twitch commands capabilities have been acknowledged.
                    // do what needs to happen with them (like `/mods`).
                    Some(TWITCH_COMMANDS_CAP) => {
                        // request to get a list of moderators and vips.
                        self.sender.mods();
                        self.sender.vips();
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
            Command::NOTICE(_, ref message) => {
                let tags = Tags::from_tags(m.tags.take());

                match tags.msg_id.as_ref().map(|id| id.as_str()) {
                    _ if message == "Login authentication failed" => {
                        self.token.force_refresh()?;
                        self.handler_shutdown = true;
                    }
                    Some("no_mods") => {
                        self.moderators.write().clear();
                    }
                    // Response to /mods request.
                    Some("room_mods") => {
                        *self.moderators.write() = parse_room_members(message);
                    }
                    Some("no_vips") => {
                        self.vips.write().clear();
                    }
                    // Response to /vips request.
                    Some("vips_success") => {
                        *self.vips.write() = parse_room_members(message);
                    }
                    Some(msg_id) => {
                        log::info!("unhandled notice w/ msg_id: {:?}: {:?}", msg_id, m);
                    }
                    None => {
                        log::info!("unhandled notice: {:?}", m);
                    }
                }
            }
            Command::Raw(ref command, _, ref tail) => match command.as_str() {
                "CLEARMSG" => {
                    if let Some(chat_log) = self.chat_log.as_ref() {
                        if let Some(tags) = ClearMsgTags::from_tags(m.tags) {
                            chat_log.message_log.delete_by_id(&tags.target_msg_id);
                        }
                    }
                }
                "CLEARCHAT" => {
                    if let Some(chat_log) = self.chat_log.as_ref() {
                        match tail {
                            Some(user) => {
                                chat_log.message_log.delete_by_user(user);
                            }
                            None => {
                                chat_log.message_log.delete_all();
                            }
                        }
                    }
                }
                _ => {
                    log::trace!("Raw: {:?}", m);
                }
            },
            _ => {
                log::info!("unhandled: {:?}", m);
            }
        }

        Ok(())
    }
}

/// Struct representing a real user.
///
/// For example, an injected command does not have a real user associated with it.
pub struct RealUser<'a> {
    tags: &'a Tags,
    sender: &'a Sender,
    name: &'a str,
    streamer: &'a twitch::User,
    moderators: &'a RwLock<HashSet<String>>,
    vips: &'a RwLock<HashSet<String>>,
    stream_info: &'a stream_info::StreamInfo,
    auth: &'a Auth,
}

impl<'a> RealUser<'a> {
    /// Get the channel the user is associated with.
    pub fn channel(&self) -> &str {
        self.sender.channel()
    }

    /// Get the name of the user.
    pub fn name(&self) -> &'a str {
        &self.name
    }

    /// Get the display name of the user.
    pub fn display_name(&self) -> &'a str {
        self.tags
            .display_name
            .as_ref()
            .map(|d| d.as_str())
            .unwrap_or_else(|| self.name)
    }

    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        self.sender
            .privmsg(format!("{} -> {}", self.display_name(), m));
    }

    /// Test if the current user is the given user.
    pub fn is(&self, name: &str) -> bool {
        self.name == name.to_lowercase()
    }

    /// Test if streamer.
    fn is_streamer(&self) -> bool {
        self.name == self.streamer.name
    }

    /// Test if moderator.
    fn is_moderator(&self) -> bool {
        self.moderators.read().contains(self.name)
    }

    /// Test if user is a subscriber.
    fn is_subscriber(&self) -> bool {
        self.is_streamer() || self.stream_info.is_subscriber(self.name)
    }

    /// Test if vip.
    fn is_vip(&self) -> bool {
        self.vips.read().contains(self.name)
    }

    /// Get a list of all roles the current requester belongs to.
    pub fn roles(&self) -> smallvec::SmallVec<[Role; 4]> {
        let mut roles = smallvec::SmallVec::new();

        if self.is_streamer() {
            roles.push(Role::Streamer);
        }

        if self.is_moderator() {
            roles.push(Role::Moderator);
        }

        if self.is_subscriber() {
            roles.push(Role::Subscriber);
        }

        if self.is_vip() {
            roles.push(Role::Vip);
        }

        roles.push(Role::Everyone);
        roles
    }

    /// Test if the current user has the given scope.
    pub fn has_scope(&self, scope: Scope) -> bool {
        self.auth.test_any(scope, self.name, self.roles())
    }
}

/// Information about the user.
pub enum Principal {
    User { name: String },
    Injected,
}

/// Inner struct for User to make it cheaper to clone.
struct UserInner {
    tags: Tags,
    sender: Sender,
    principal: Principal,
    streamer: Arc<twitch::User>,
    moderators: Arc<RwLock<HashSet<String>>>,
    vips: Arc<RwLock<HashSet<String>>>,
    stream_info: stream_info::StreamInfo,
    auth: Auth,
}

#[derive(Clone)]
pub struct User {
    inner: Arc<UserInner>,
}

impl User {
    /// Access the user as a real user.
    pub fn real(&self) -> Option<RealUser<'_>> {
        match self.inner.principal {
            Principal::User { ref name } => Some(RealUser {
                tags: &self.inner.tags,
                sender: &self.inner.sender,
                name,
                streamer: &*self.inner.streamer,
                moderators: &*self.inner.moderators,
                vips: &*self.inner.vips,
                stream_info: &self.inner.stream_info,
                auth: &self.inner.auth,
            }),
            Principal::Injected => None,
        }
    }

    /// Get the channel the user is associated with.
    pub fn channel(&self) -> &str {
        self.inner.sender.channel()
    }

    /// Get the name of the user.
    pub fn name(&self) -> Option<&str> {
        match self.inner.principal {
            Principal::User { ref name, .. } => Some(name),
            Principal::Injected => None,
        }
    }

    /// Get the display name of the user.
    pub fn display_name(&self) -> Option<&str> {
        self.inner
            .tags
            .display_name
            .as_ref()
            .map(|d| d.as_str())
            .or_else(|| self.name())
    }

    /// Get tags associated with the message.
    pub fn tags(&self) -> &Tags {
        &self.inner.tags
    }

    /// Access the sender associated with the user.
    pub fn sender(&self) -> &Sender {
        &self.inner.sender
    }

    /// Get the name of the streamer.
    pub fn streamer(&self) -> &twitch::User {
        &*self.inner.streamer
    }

    /// Test if the current user is the given user.
    pub fn is(&self, name: &str) -> bool {
        self.real().map(|u| u.is(name)).unwrap_or(false)
    }

    /// Test if streamer.
    fn is_streamer(&self) -> bool {
        self.real().map(|u| u.is_streamer()).unwrap_or(true)
    }

    /// Test if moderator.
    fn is_moderator(&self) -> bool {
        self.real().map(|u| u.is_moderator()).unwrap_or(true)
    }

    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        match self.display_name() {
            Some(name) => {
                self.inner.sender.privmsg(format!("{} -> {}", name, m));
            }
            None => {
                self.inner.sender.privmsg(m);
            }
        }
    }

    /// Pretty render the results.
    pub fn respond_lines<F>(&self, results: impl IntoIterator<Item = F>, empty: &str)
    where
        F: fmt::Display,
    {
        let mut output = partition_response(results, 360, " | ");

        if let Some(line) = output.next() {
            let count = output.count();

            if count > 0 {
                self.respond(format!("{} ... {} line(s) not shown", line, count,));
            } else {
                self.respond(line);
            }
        } else {
            self.respond(empty);
        }
    }

    /// Get a list of all roles the current requester belongs to.
    pub fn roles(&self) -> smallvec::SmallVec<[Role; 4]> {
        match self.real().map(|u| u.roles()) {
            Some(roles) => roles,
            None => {
                let mut roles = smallvec::SmallVec::<[Role; 4]>::default();
                roles.push(Role::Streamer);
                roles.push(Role::Moderator);
                roles.push(Role::Subscriber);
                roles.push(Role::Vip);
                roles
            }
        }
    }

    /// Test if the current user has the given scope.
    pub fn has_scope(&self, scope: Scope) -> bool {
        self.real().map(|u| u.has_scope(scope)).unwrap_or(true)
    }
}

struct PartitionResponse<'a, I> {
    iter: I,
    width: usize,
    sep: &'a str,
    // composition of current line.
    line_buf: String,
    // buffer for current item.
    item_buf: String,
}

impl<F, I> Iterator for PartitionResponse<'_, I>
where
    I: Iterator<Item = F>,
    F: fmt::Display,
{
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        use std::fmt::Write as _;
        const TAIL: &'static str = "...";

        self.line_buf.clear();
        // length of current line.
        let mut len = 0;

        while let Some(result) = self.iter.next() {
            self.item_buf.clear();
            write!(&mut self.item_buf, "{}", result)
                .expect("a Display implementation returned an error unexpectedly");

            loop {
                if len + self.item_buf.len() <= self.width {
                    if len > 0 {
                        self.line_buf.push_str(self.sep);
                    }

                    self.line_buf.push_str(&self.item_buf);
                    len += self.item_buf.len() + self.sep.len();
                    break;
                }

                // we don't have a choice, force an entry even if it's too wide.
                if self.line_buf.is_empty() {
                    let mut index = usize::min(self.item_buf.len(), self.width - TAIL.len());

                    while index > 0 && !self.item_buf.is_char_boundary(index) {
                        index -= 1;
                    }

                    return Some(format!("{}{}", &self.item_buf[..index], TAIL));
                }

                let output = self.line_buf.clone();
                self.line_buf.clear();
                return Some(output);
            }
        }

        if !self.line_buf.is_empty() {
            let output = self.line_buf.clone();
            self.line_buf.clear();
            return Some(output);
        }

        None
    }
}

/// Partition the results to fit the given width, using a separator defined in `part`.
fn partition_response<'a, I, F>(
    iter: I,
    width: usize,
    sep: &'a str,
) -> PartitionResponse<'a, I::IntoIter>
where
    I: IntoIterator<Item = F>,
    F: fmt::Display,
{
    PartitionResponse {
        iter: iter.into_iter(),
        width,
        sep,
        line_buf: String::new(),
        item_buf: String::new(),
    }
}

/// Struct of tags.
#[derive(Debug, Clone, Default)]
pub struct Tags {
    /// Contents of the id tag if present.
    pub id: Option<String>,
    /// Contents of the msg-id tag if present.
    pub msg_id: Option<String>,
    /// The display name of the user.
    pub display_name: Option<String>,
    /// The ID of the user.
    pub user_id: Option<String>,
    /// Color of the user.
    pub color: Option<String>,
    /// Emotes part of the message.
    pub emotes: Option<String>,
    /// Badges part of the message.
    pub badges: Option<String>,
}

impl Tags {
    /// Extract tags from message.
    fn from_tags(tags: Option<Vec<Tag>>) -> Tags {
        let mut id = None;
        let mut msg_id = None;
        let mut display_name = None;
        let mut user_id = None;
        let mut color = None;
        let mut emotes = None;
        let mut badges = None;

        if let Some(tags) = tags {
            for t in tags {
                match t {
                    Tag(name, Some(value)) => match name.as_str() {
                        "id" => id = Some(value),
                        "msg-id" => msg_id = Some(value),
                        "display-name" => display_name = Some(value),
                        "user-id" => user_id = Some(value),
                        "color" => color = Some(value),
                        "emotes" => emotes = Some(value),
                        "badges" => badges = Some(value),
                        _ => (),
                    },
                    _ => (),
                }
            }
        }

        Tags {
            id,
            msg_id,
            display_name,
            user_id,
            color,
            emotes,
            badges,
        }
    }
}

/// Tags associated with a CLEARMSG.
struct ClearMsgTags {
    target_msg_id: String,
}

impl ClearMsgTags {
    /// Extract tags from message.
    fn from_tags(tags: Option<Vec<Tag>>) -> Option<ClearMsgTags> {
        let mut target_msg_id = None;

        if let Some(tags) = tags {
            for t in tags {
                match t {
                    Tag(name, Some(value)) => match name.as_str() {
                        "target-msg-id" => target_msg_id = Some(value),
                        _ => (),
                    },
                    _ => (),
                }
            }
        }

        Some(ClearMsgTags {
            target_msg_id: target_msg_id?,
        })
    }
}

#[derive(Debug)]
pub enum SenderThreadItem {
    Exit,
    Send(Message),
}

#[derive(serde::Serialize)]
pub struct BadWordsVars<'a> {
    name: Option<&'a str>,
    target: &'a str,
}

#[derive(serde::Serialize)]
pub struct CommandVars<'a> {
    name: Option<&'a str>,
    target: &'a str,
    count: i32,
    rest: &'a str,
}

// Future to refresh moderators every 5 minutes.
async fn refresh_mods_future(sender: Sender) -> Result<(), Error> {
    let mut interval = timer::Interval::new_interval(time::Duration::from_secs(60 * 5));

    while let Some(i) = interval.next().await {
        let _ = i?;
        log::trace!("refreshing mods and vips");
        sender.mods();
        sender.vips();
    }

    Ok(())
}

/// Parse the `room_mods` message.
fn parse_room_members(message: &str) -> HashSet<String> {
    let mut out = HashSet::default();

    if let Some(index) = message.find(":") {
        let message = &message[(index + 1)..];
        let message = message.trim_end_matches('.');

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
    use super::parse_room_members;
    use hashbrown::HashSet;

    #[test]
    fn test_parse_room_mods() {
        assert_eq!(
            vec![String::from("foo"), String::from("bar")]
                .into_iter()
                .collect::<HashSet<String>>(),
            parse_room_members("The moderators of this channel are: foo, bar")
        );

        assert_eq!(
            vec![String::from("a")]
                .into_iter()
                .collect::<HashSet<String>>(),
            parse_room_members("The moderators of this channel are: a")
        );

        assert_eq!(
            vec![].into_iter().collect::<HashSet<String>>(),
            parse_room_members("The moderators of this channel are:")
        );
    }
}
