use crate::{
    api,
    auth::{Auth, Role, Scope},
    bus, command, config,
    currency::{Currency, CurrencyBuilder},
    db, idle,
    injector::Injector,
    module, oauth2,
    prelude::*,
    settings, stream_info, template, timer,
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
use parking_lot::RwLock;
use std::{fmt, sync::Arc, time};
use tokio_threadpool::ThreadPool;

// re-exports
pub use self::sender::Sender;

mod currency_admin;
mod sender;

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
    #[serde(default)]
    pub clip_cooldown: Option<Cooldown>,
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
    pub nightbot: Arc<api::NightBot>,
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
    pub injector: Injector,
    pub stream_state_tx: mpsc::Sender<stream_info::StreamState>,
}

impl Irc {
    pub async fn run(self) -> Result<(), Error> {
        let Irc {
            db,
            youtube,
            nightbot,
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
            stream_state_tx,
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

            let mut vars = settings.scoped("irc").vars();
            let url_whitelist_enabled = vars.var("url-whitelist/enabled", true)?;
            let bad_words_enabled = vars.var("bad-words/enabled", false)?;
            let sender_ty = vars.var("sender-type", sender::Type::Chat)?;
            let threshold = vars.var("idle-detection/threshold", 5)?;
            let idle = idle::Idle::new(threshold);

            let sender = Sender::new(sender_ty, channel.clone(), client.clone(), nightbot.clone());

            let mut futures = Vec::<future::BoxFuture<'_, Result<(), Error>>>::new();
            futures.push(vars.run().boxed());

            let stream_info = {
                let (stream_info, mut stream_state_rx, future) =
                    stream_info::setup(streamer, streamer_twitch.clone());

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
                    after_streams: &after_streams,
                    youtube: &youtube,
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
            futures.push(send_future.compat().map_err(Error::from).boxed());

            if !config.whitelisted_hosts.is_empty() {
                if !settings
                    .get::<bool>("migration/whitelisted-hosts-migrated")?
                    .unwrap_or_default()
                {
                    log::warn!("Performing a one time migration of hosts from configuration.");
                    settings.set("irc/whitelisted-hosts", &config.whitelisted_hosts)?;
                    settings.set("migration/whitelisted-hosts-migrated", true)?;
                }
            }

            let (mut whitelisted_hosts_stream, whitelisted_hosts) =
                settings.stream("irc/whitelisted-hosts").or_default()?;

            let (mut moderator_cooldown_stream, moderator_cooldown) =
                settings.stream("irc/moderator-cooldown").optional()?;

            let (mut api_url_stream, api_url) = settings.stream("remote/api-url").optional()?;

            let startup_message = settings.get::<String>("irc/startup-message")?;

            let mut pong_timeout = None;

            let mut handler = Handler {
                streamer,
                sender: sender.clone(),
                moderators: HashSet::default(),
                vips: HashSet::default(),
                whitelisted_hosts,
                commands: &commands,
                bad_words: &bad_words,
                global_bus: &global_bus,
                aliases: &aliases,
                api_url,
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
                scope_cooldowns: auth.scope_cooldowns(),
                currency_handler,
                url_whitelist_enabled,
                bad_words_enabled,
                message_hooks: Default::default(),
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
    injector: &'a Injector,
    settings: &settings::Settings,
) -> Result<impl Future<Output = Result<(), Error>> + 'a, Error> {
    let reward = 10;
    let default_interval = Duration::seconds(60 * 10);

    let (mut interval_stream, mut interval) = settings
        .stream("irc/viewer-reward/interval")
        .or_with(default_interval)?;

    let mut variables = settings.vars();
    let reward_percentage = variables.var("irc/viewer-reward%", 100)?;
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

    let mut builder = CurrencyBuilder::new(db.clone(), twitch.clone(), mysql_schema);
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
    futures.push(variables.run().boxed());

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
}

/// Handler for incoming messages.
struct Handler<'a> {
    /// Current Streamer.
    streamer: &'a str,
    /// Queue for sending messages.
    sender: Sender,
    /// Moderators.
    moderators: HashSet<String>,
    /// VIPs.
    vips: HashSet<String>,
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
    currency_handler: currency_admin::Handler<'a>,
    bad_words_enabled: Arc<RwLock<bool>>,
    url_whitelist_enabled: Arc<RwLock<bool>>,
    /// A hook that can be installed to peek at all incoming messages.
    message_hooks: HashMap<String, Box<dyn command::MessageHook>>,
}

/// Handle a command.
pub async fn process_command<'a, 'b: 'a>(
    command: &'a str,
    mut ctx: command::Context<'a>,
    global_bus: &'a Arc<bus::Bus<bus::Global>>,
    currency_handler: &'a mut currency_admin::Handler<'b>,
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

impl<'a> Handler<'a> {
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
    fn delete_message(&self, user: &User<'_>) -> Result<(), Error> {
        let id = match user.tags.id {
            Some(id) => id,
            None => return Ok(()),
        };

        log::info!("Attempting to delete message: {}", id);
        user.sender.delete(id);
        Ok(())
    }

    /// Test if the message should be deleted.
    fn should_be_deleted(&self, user: &User<'_>, message: &str) -> bool {
        // Moderators can say whatever they want.
        if self.moderators.contains(user.name) {
            return false;
        }

        if *self.bad_words_enabled.read() {
            if let Some(word) = self.test_bad_words(message) {
                if let Some(why) = word.why.as_ref() {
                    let why = why.render_to_string(&BadWordsVars {
                        name: user.name,
                        target: user.target,
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

    /// Handle the given command.
    pub async fn handle(&mut self, m: Message) -> Result<(), Error> {
        match m.command {
            Command::PRIVMSG(_, ref message) => {
                let tags = Self::tags(&m);

                let name = m
                    .source_nickname()
                    .ok_or_else(|| format_err!("expected user info"))?;

                let target = m
                    .response_target()
                    .ok_or_else(|| format_err!("expected user info"))?;

                let user = User {
                    tags,
                    sender: self.sender.clone(),
                    name,
                    target,
                    streamer: self.streamer,
                    moderators: &self.moderators,
                    vips: &self.vips,
                    stream_info: &self.stream_info,
                    auth: &self.auth,
                };

                for (key, hook) in &mut self.message_hooks {
                    hook.peek(&user, message)
                        .with_context(|_| format_err!("hook `{}` failed", key))?;
                }

                // only non-moderators and non-streamer bumps the idle counter.
                if !self.moderators.contains(user.name) && user.name != self.streamer {
                    self.idle.seen();
                }

                let mut it = utils::Words::new(message);

                // NB: needs to store locally to maintain a reference to it.
                let a = self.aliases.lookup(user.target, it.clone());

                if let Some(a) = &a {
                    it = utils::Words::new(a.as_str());
                }

                if let Some(command) = it.next() {
                    if let Some(command) = self.commands.get(user.target, &command) {
                        if command.has_var("count") {
                            self.commands.increment(&*command)?;
                        }

                        let vars = CommandVars {
                            name: &user.name,
                            target: &user.target,
                            count: command.count(),
                            rest: it.rest(),
                        };

                        let response = command.render(&vars)?;
                        self.sender.privmsg(response);
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
                        self.moderators = parse_room_members(message);
                    }
                    Some("no_vips") => {
                        self.vips.clear();
                    }
                    // Response to /vips request.
                    Some("vips_success") => {
                        self.vips = parse_room_members(message);
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
pub struct User<'a> {
    pub tags: Tags<'a>,
    sender: Sender,
    pub name: &'a str,
    pub target: &'a str,
    pub streamer: &'a str,
    pub moderators: &'a HashSet<String>,
    pub vips: &'a HashSet<String>,
    pub stream_info: &'a stream_info::StreamInfo,
    pub auth: &'a Auth,
}

impl<'a> User<'a> {
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

    /// Test if streamer.
    fn is_streamer(&self) -> bool {
        self.name == self.streamer
    }

    /// Test if moderator.
    pub fn is_moderator(&self) -> bool {
        self.moderators.contains(self.name)
    }

    /// Test if subscriber.
    fn is_subscriber(&self) -> bool {
        self.is_streamer() || self.stream_info.is_subscriber(self.name)
    }

    /// Test if vip.
    fn is_vip(&self) -> bool {
        self.vips.contains(self.name)
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
