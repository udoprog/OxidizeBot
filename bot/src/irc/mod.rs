use crate::api;
use crate::auth::{Auth, Role, Scope};
use crate::bus;
use crate::command;
use crate::currency::CurrencyBuilder;
use crate::db;
use crate::idle;
use crate::message_log::MessageLog;
use crate::module;
use crate::oauth2;
use crate::prelude::*;
use crate::script;
use crate::stream_info;
use crate::tags;
use crate::task;
use crate::utils::{self, Cooldown, Duration};
use anyhow::{anyhow, bail, Context as _, Result};
use irc::client::{self, Client};
use irc::proto::command::{CapSubCommand, Command};
use irc::proto::message::{Message, Tag};
use notify::{recommended_watcher, RecommendedWatcher, Watcher};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::fmt;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::time;
use tokio::sync;
use tracing::trace_span;
use tracing_futures::Instrument as _;

// re-exports
pub use self::sender::Sender;

mod chat_log;
mod currency_admin;
mod sender;

const SERVER: &str = "irc.chat.twitch.tv";
const TWITCH_TAGS_CAP: &str = "twitch.tv/tags";
const TWITCH_COMMANDS_CAP: &str = "twitch.tv/commands";

/// Helper struct to construct IRC integration.
pub struct Irc {
    pub injector: Injector,
    pub modules: Vec<Box<dyn module::Module>>,
    pub script_dirs: Vec<PathBuf>,
    pub stream_state_tx: mpsc::Sender<stream_info::StreamState>,
}

impl Irc {
    pub async fn run(self) -> Result<()> {
        use backoff::backoff::Backoff as _;

        let mut provider = Setup::provider(&self.injector).await?;

        let mut error_backoff = backoff::ExponentialBackoff::default();
        error_backoff.current_interval = time::Duration::from_secs(5);
        error_backoff.initial_interval = time::Duration::from_secs(5);
        error_backoff.max_elapsed_time = None;

        loop {
            while let Some(setup) = provider.build() {
                let irc_loop = IrcLoop {
                    setup,
                    provider: &mut provider,
                    irc: &self,
                };

                match irc_loop.run().await {
                    Ok(()) => {
                        error_backoff.reset();
                    }
                    Err(e) => {
                        let backoff = error_backoff.next_backoff().unwrap_or_default();
                        log_error!(e, "chat component crashed, restarting in {:?}", backoff);
                        tokio::time::sleep(backoff).await;
                        continue;
                    }
                }
            }

            provider.wait().await;
        }
    }
}

#[derive(Provider)]
struct Setup {
    #[dependency]
    db: db::Database,
    #[dependency(tag = "tags::Twitch::Bot")]
    bot: api::TwitchAndUser,
    #[dependency(tag = "tags::Twitch::Streamer")]
    streamer: api::TwitchAndUser,
    #[dependency]
    auth: Auth,
    #[dependency]
    bad_words: db::Words,
    #[dependency]
    message_log: MessageLog,
    #[dependency]
    command_bus: bus::Bus<bus::Command>,
    #[dependency]
    global_bus: bus::Bus<bus::Global>,
    #[dependency]
    settings: crate::Settings,
    #[dependency]
    restart: utils::Restart,
}

struct IrcLoop<'a> {
    setup: Setup,
    provider: &'a mut SetupProvider,
    irc: &'a Irc,
}

impl IrcLoop<'_> {
    async fn run(self) -> Result<()> {
        let Self {
            setup,
            provider,
            irc,
        } = self;

        let Setup {
            db,
            bot,
            streamer,
            auth,
            bad_words,
            message_log,
            command_bus,
            global_bus,
            settings,
            restart,
        } = setup;

        let Irc {
            injector,
            modules,
            script_dirs,
            stream_state_tx,
            ..
        } = irc;

        let streamer_channel = match &streamer.channel {
            Some(channel) => channel,
            None => {
                bail!("missing channel information for streamer");
            }
        };

        log::trace!("Channel: {:?}", streamer_channel);
        log::trace!("Streamer: {:?}", streamer.user.display_name);
        log::trace!("Bot: {:?}", bot.user.display_name);

        let chat_channel = format!("#{}", streamer.user.login);
        injector
            .update_key(Key::tagged(tags::Globals::Channel)?, chat_channel.clone())
            .await;

        let access_token = bot.client.token.read().await?.access_token().to_string();

        let irc_client_config = client::data::config::Config {
            nickname: Some(bot.user.login.to_string()),
            channels: vec![chat_channel.clone()],
            password: Some(format!("oauth:{}", access_token)),
            server: Some(String::from(SERVER)),
            port: Some(6697),
            use_tls: Some(true),
            ..client::data::config::Config::default()
        };

        let mut client = Client::from_config(irc_client_config).await?;
        client.identify()?;

        let chat_settings = settings.scoped("chat");

        let url_whitelist_enabled = chat_settings.var("url-whitelist/enabled", true).await?;
        let bad_words_enabled = chat_settings.var("bad-words/enabled", false).await?;
        let sender_ty = chat_settings.var("sender-type", sender::Type::Chat).await?;
        let threshold = chat_settings.var("idle-detection/threshold", 5).await?;
        let idle = idle::Idle::new(threshold);

        let nightbot = injector.var::<api::NightBot>().await;

        let sender = Sender::new(sender_ty, chat_channel.clone(), client.sender(), nightbot)?;

        let mut futures = crate::utils::Futures::new();

        let stream_info = {
            let (stream_info, mut stream_state_rx, future) =
                stream_info::setup(streamer.user.clone(), streamer.client.clone());

            let stream_state_tx = stream_state_tx.clone();

            let forward = async move {
                while let Some(m) = stream_state_rx.recv().await {
                    stream_state_tx
                        .send(m)
                        .await
                        .map_err(|_| anyhow!("failed to send"))?;
                }

                Ok(())
            };

            futures.push(Box::pin(
                forward.instrument(trace_span!(target: "futures", "stream-info-forward",)),
            ));
            futures.push(Box::pin(
                future.instrument(trace_span!(target: "futures", "stream-info-refresh",)),
            ));

            stream_info
        };

        futures.push(Box::pin(
            refresh_mods_future(sender.clone())
                .instrument(trace_span!(target: "futures", "refresh-mods",)),
        ));

        let mut handlers = module::Handlers::default();

        let scripts =
            script::load_dir(streamer.user.login.clone(), db.clone(), script_dirs).await?;

        let (scripts_watch_tx, mut scripts_watch_rx) = sync::mpsc::unbounded_channel();

        let _watcher = if !script_dirs.is_empty() {
            let mut watcher: RecommendedWatcher = recommended_watcher(move |e| {
                let _ = scripts_watch_tx.send(e);
            })?;

            for d in script_dirs {
                if d.is_dir() {
                    watcher.watch(d, notify::RecursiveMode::Recursive)?;
                }
            }

            Some(watcher)
        } else {
            None
        };

        for module in modules {
            if log::log_enabled!(log::Level::Trace) {
                log::trace!("initializing module: {}", module.ty());
            }

            let result = module
                .hook(module::HookContext {
                    handlers: &mut handlers,
                    futures: &mut futures,
                    stream_info: &stream_info,
                    idle: &idle,
                    twitch: &bot.client,
                    streamer_twitch: &streamer.client,
                    sender: &sender,
                    settings: &settings,
                    injector,
                })
                .await;

            result.with_context(|| anyhow!("failed to initialize module: {}", module.ty()))?;
        }

        let currency_handler = currency_admin::setup(injector).await?;

        let future = currency_loop(
            streamer.clone(),
            sender.clone(),
            idle.clone(),
            injector.clone(),
            chat_settings.clone(),
            settings.clone(),
        )
        .await?;

        futures.push(Box::pin(
            future.instrument(trace_span!(target: "futures", "currency-loop",)),
        ));

        let (mut whitelisted_hosts_stream, whitelisted_hosts) = chat_settings
            .stream("whitelisted-hosts")
            .or_default()
            .await?;

        let (mut moderator_cooldown_stream, moderator_cooldown) = chat_settings
            .stream("moderator-cooldown")
            .optional()
            .await?;

        let (mut api_url_stream, api_url) = settings.stream("remote/api-url").optional().await?;

        let join_message = chat_settings.get::<String>("join-message").await?;

        let leave_message = chat_settings
            .get::<String>("leave-message")
            .await?
            .unwrap_or_else(|| String::from("Leaving chat... VoHiYo"));

        let mut chat_log_builder = chat_log::Builder::new(
            bot.client.clone(),
            injector,
            message_log.clone(),
            settings.scoped("chat-log"),
        )
        .await?;

        let (mut commands_stream, commands) = injector.stream().await;
        let (mut aliases_stream, aliases) = injector.stream().await;

        let mut pong_timeout = Fuse::empty();

        let mut handler = Handler {
            streamer: &streamer,
            sender: sender.clone(),
            moderators: Default::default(),
            vips: Default::default(),
            whitelisted_hosts,
            commands,
            bad_words: &bad_words,
            global_bus: &global_bus,
            aliases,
            api_url: Arc::new(api_url),
            moderator_cooldown,
            handlers,
            scripts,
            idle: &idle,
            pong_timeout: &mut pong_timeout,
            token: &bot.client.token,
            handler_shutdown: false,
            stream_info: &stream_info,
            auth: &auth,
            currency_handler,
            url_whitelist_enabled,
            bad_words_enabled,
            chat_log: chat_log_builder.build()?,
            context_inner: Arc::new(command::ContextInner {
                sender: sender.clone(),
                scope_cooldowns: sync::Mutex::new(auth.scope_cooldowns()),
                message_hooks: sync::RwLock::new(Default::default()),
                restart,
            }),
        };

        let mut outgoing = client
            .outgoing()
            .ok_or_else(|| anyhow!("missing outgoing future for irc client"))?;

        let mut client_stream = client.stream()?;

        let mut ping_interval = tokio::time::interval(time::Duration::from_secs(10));
        let mut commands = command_bus.subscribe();

        let leave = Fuse::empty();
        tokio::pin!(leave);

        let sender = handler.sender.clone();

        // Things to do when joining.
        let join_task = Fuse::new(async move {
            sender.cap_req(TWITCH_TAGS_CAP).await;
            sender.cap_req(TWITCH_COMMANDS_CAP).await;

            if let Some(join_message) = join_message.as_ref() {
                // greeting when bot joins.
                sender.privmsg_immediate(join_message);
            }
        });
        tokio::pin!(join_task);

        #[allow(clippy::unnecessary_mut_passed)]
        while leave.is_empty() {
            tokio::select! {
                _ = &mut join_task => {
                    log::trace!("Done sending capabilities request and join message");
                }
                Some(ev) = scripts_watch_rx.recv() => {
                    if let Ok(ev) = ev {
                        if let Err(e) = handler.handle_script_filesystem_event(ev) {
                            log_error!(e, "failed to handle script filesystem event");
                        }
                    }
                }
                command = commands.recv() => {
                    let command = command?;

                    match command {
                        bus::Command::Raw { command } => {
                            log::trace!("Raw command: {}", command);

                            if let Err(e) = handler.raw(command).await {
                                log_error!(e, "Failed to handle message");
                            }
                        }
                    }
                }
                Some(future) = futures.next() => {
                    match future {
                        Ok(..) => {
                            log::warn!("IRC component exited, exiting...");
                            return Ok(());
                        }
                        Err(e) => {
                            log_warn!(e, "IRC component errored, restarting in 5 seconds");
                            tokio::time::sleep(time::Duration::from_secs(5)).await;
                            return Ok(());
                        }
                    }
                }
                _ = provider.wait_for_update() => {
                    // If configuration state changes, force a reconnect.
                    leave.set(Fuse::new(tokio::time::sleep(time::Duration::from_secs(1))));
                }
                commands = commands_stream.recv() => {
                    handler.commands = commands;
                }
                aliases = aliases_stream.recv() => {
                    handler.aliases = aliases;
                }
                chat_log = chat_log_builder.update() => {
                    handler.chat_log = chat_log?;
                }
                api_url = api_url_stream.recv() => {
                    handler.api_url = Arc::new(api_url);
                }
                moderator_cooldown = moderator_cooldown_stream.recv() => {
                    handler.moderator_cooldown = moderator_cooldown;
                }
                _ = ping_interval.tick() => {
                    handler.send_ping()?;
                }
                _ = &mut *handler.pong_timeout => {
                    bail!("server not responding");
                }
                update = whitelisted_hosts_stream.recv() => {
                    handler.whitelisted_hosts = update;
                },
                message = client_stream.next() => {
                    if let Some(m) = message.transpose()? {
                        if let Err(e) = handler.handle(m).await {
                            log_error!(e, "Failed to handle message");
                        }
                    }

                    if handler.handler_shutdown {
                        bail!("handler forcibly shut down");
                    }
                }
                _ = &mut outgoing => {
                    bail!("outgoing future ended unexpectedly");
                }
                _ = &mut leave => {
                    break;
                }
            }
        }

        handler.sender.privmsg_immediate(leave_message);

        #[allow(clippy::never_loop, clippy::unnecessary_mut_passed)]
        loop {
            tokio::select! {
                _ = &mut outgoing => {
                    bail!("outgoing future ended unexpectedly");
                }
                _ = &mut leave => {
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Set up a reward loop.
async fn currency_loop(
    streamer: api::TwitchAndUser,
    sender: Sender,
    idle: idle::Idle,
    injector: Injector,
    chat_settings: crate::Settings,
    settings: crate::Settings,
) -> Result<impl Future<Output = Result<()>>> {
    log::trace!("Setting up currency loop");

    let reward = 10;
    let default_interval = Duration::seconds(60 * 10);

    let (mut interval_stream, mut reward_interval) = chat_settings
        .stream("viewer-reward/interval")
        .or_with(default_interval)
        .await?;

    let reward_percentage = chat_settings.var("viewer-reward%", 100).await?;
    let (mut viewer_reward_stream, viewer_reward) = chat_settings
        .stream("viewer-reward/enabled")
        .or_with(false)
        .await?;
    let (mut notify_rewards_stream, mut notify_rewards) = settings
        .stream("currency/notify-rewards")
        .or_with(true)
        .await?;

    let (mut ty_stream, ty) = settings.stream("currency/type").or_default().await?;
    let (mut enabled_stream, enabled) = settings.stream("currency/enabled").or_default().await?;
    let (mut name_stream, name) = settings.stream("currency/name").optional().await?;
    let (mut command_enabled_stream, command_enabled) = settings
        .stream("currency/command-enabled")
        .or_with(true)
        .await?;
    let (mut mysql_url_stream, mysql_url) =
        settings.stream("currency/mysql/url").optional().await?;
    let (mut mysql_schema_stream, mysql_schema) = settings
        .stream("currency/mysql/schema")
        .or_default()
        .await?;

    let (mut db_stream, db) = injector.stream::<db::Database>().await;

    let mut builder = CurrencyBuilder::new(streamer.client.clone(), mysql_schema, injector.clone());

    builder.db = db;
    builder.ty = ty;
    builder.enabled = enabled;
    builder.command_enabled = command_enabled;
    builder.name = name.map(Arc::new);
    builder.mysql_url = mysql_url;

    let mut currency = builder.build_and_inject().await;

    Ok(async move {
        let new_timer = |interval: &Duration, viewer_reward: bool| {
            if viewer_reward && !interval.is_empty() {
                Fuse::new(tokio::time::interval(interval.as_std()))
            } else {
                Fuse::empty()
            }
        };

        let mut timer = new_timer(&reward_interval, viewer_reward);

        loop {
            tokio::select! {
                update = interval_stream.recv() => {
                    reward_interval = update;
                    timer = new_timer(&reward_interval, viewer_reward);
                }
                update = notify_rewards_stream.recv() => {
                    notify_rewards = update;
                }
                update = db_stream.recv() => {
                    builder.db = update;
                    currency = builder.build_and_inject().await;
                }
                enabled = enabled_stream.recv() => {
                    builder.enabled = enabled;
                    currency = builder.build_and_inject().await;
                }
                update = ty_stream.recv() => {
                    builder.ty = update;
                    currency = builder.build_and_inject().await;
                }
                name = name_stream.recv() => {
                    builder.name = name.map(Arc::new);
                    currency = builder.build_and_inject().await;
                }
                mysql_url = mysql_url_stream.recv() => {
                    builder.mysql_url = mysql_url;
                    currency = builder.build_and_inject().await;
                }
                update = mysql_schema_stream.recv() => {
                    builder.mysql_schema = update;
                    currency = builder.build_and_inject().await;
                }
                command_enabled = command_enabled_stream.recv() => {
                    builder.command_enabled = command_enabled;
                    currency = builder.build_and_inject().await;
                }
                viewer_reward = viewer_reward_stream.recv() => {
                    timer = new_timer(&reward_interval, viewer_reward);
                }
                _ = timer.as_pin_mut().poll_inner(|mut i, cx| i.poll_tick(cx)) => {
                    let currency = match currency.as_ref() {
                        Some(currency) => currency,
                        None => continue,
                    };

                    let seconds = reward_interval.num_seconds() as i64;

                    log::trace!("running reward loop");

                    let reward = (reward * reward_percentage.load().await as i64) / 100i64;
                    let count = currency
                        .add_channel_all(&streamer.user.login, reward, seconds)
                        .await?;

                    if notify_rewards && count > 0 && !idle.is_idle().await {
                        sender.privmsg(format!(
                            "/me has given {} {} to all viewers!",
                            reward, currency.name
                        )).await;
                    }
                }
            }
        }
    })
}

/// Handler for incoming messages.
struct Handler<'a> {
    /// Current Streamer.
    streamer: &'a api::TwitchAndUser,
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
    global_bus: &'a bus::Bus<bus::Global>,
    /// Aliases.
    aliases: Option<db::Aliases>,
    /// Configured API URL.
    api_url: Arc<Option<String>>,
    /// Active moderator cooldown.
    moderator_cooldown: Option<Cooldown>,
    /// Handlers for specific commands like `!skip`.
    handlers: module::Handlers,
    /// Dynamic handlers.
    scripts: script::Scripts,
    /// Build idle detection.
    idle: &'a idle::Idle,
    /// Pong timeout currently running.
    pong_timeout: &'a mut Fuse<Pin<Box<tokio::time::Sleep>>>,
    /// OAuth 2.0 Token used to authenticate with IRC.
    token: &'a oauth2::SyncToken,
    /// Force a shutdown.
    handler_shutdown: bool,
    /// Stream information.
    stream_info: &'a stream_info::StreamInfo,
    /// Information about auth.
    auth: &'a Auth,
    /// Handler for currencies.
    currency_handler: Arc<currency_admin::Handler>,
    bad_words_enabled: settings::Var<bool>,
    url_whitelist_enabled: settings::Var<bool>,
    /// Handler for chat logs.
    chat_log: Option<chat_log::ChatLog>,
    /// Shared context paramters.
    context_inner: Arc<command::ContextInner>,
}

impl Handler<'_> {
    /// Handle filesystem event.
    fn handle_script_filesystem_event(&mut self, ev: notify::Event) -> Result<()> {
        use notify::event::{CreateKind, EventKind::*, ModifyKind, RemoveKind, RenameMode};

        log::trace!("filesystem event: {:?}", ev);

        let kind = match ev.kind {
            Create(CreateKind::File) => Kind::Load,
            Create(CreateKind::Any) => Kind::Load,
            Modify(ModifyKind::Data(..)) => Kind::Load,
            Modify(ModifyKind::Name(RenameMode::From)) => Kind::Remove,
            Modify(ModifyKind::Name(RenameMode::To)) => Kind::Load,
            Modify(ModifyKind::Any) => Kind::Load,
            Remove(RemoveKind::File) => Kind::Remove,
            Remove(RemoveKind::Any) => Kind::Remove,
            _ => return Ok(()),
        };

        match kind {
            Kind::Load => {
                for p in &ev.paths {
                    log::info!("loading script: {}", p.display());

                    let p = p.canonicalize()?;

                    if let Err(e) = self.scripts.reload(&p) {
                        log_error!(e, "failed to reload: {}", p.display());
                    }
                }
            }
            Kind::Remove => {
                for p in &ev.paths {
                    log::info!("unloading script: {}", p.display());

                    let p = p.canonicalize()?;
                    self.scripts.unload(&p);
                }
            }
        }

        return Ok(());

        #[derive(Debug, Clone, Copy)]
        enum Kind {
            Load,
            Remove,
        }
    }
}

/// Handle a command.
async fn process_command(
    command: &str,
    mut ctx: command::Context,
    global_bus: &bus::Bus<bus::Global>,
    currency_handler: &Arc<currency_admin::Handler>,
    handlers: &module::Handlers,
    scripts: &script::Scripts,
) -> Result<()> {
    match command {
        "ping" => {
            respond!(ctx, "What do you want?");
            global_bus.send(bus::Global::Ping).await;
        }
        other => {
            log::trace!("Testing command: {}", other);

            // TODO: store currency name locally to match against.
            let currency_command = currency_handler.command_name().await;

            let handler = match (other, currency_command) {
                (other, Some(ref name)) if other == **name => {
                    Some(currency_handler.clone() as Arc<dyn command::Handler>)
                }
                (other, Some(..)) | (other, None) => handlers.get(other),
            };

            if let Some(handler) = handler {
                let scope = handler.scope();

                if log::log_enabled!(log::Level::Trace) {
                    log::trace!("Auth: {:?} against {:?}", scope, ctx.user.roles());
                }

                // Test if user has the required scope to run the given
                // command.
                if let Some(scope) = scope {
                    if !ctx.user.has_scope(scope).await {
                        if ctx.user.is_moderator() {
                            respond!(ctx, "You are not allowed to run that command");
                        } else {
                            respond!(ctx, "Do you think this is a democracy? LUL");
                        }

                        return Ok(());
                    }
                }

                task::spawn(async move {
                    if let Err(e) = handler.handle(&mut ctx).await {
                        if let Some(command::Respond(respond)) = e.downcast_ref() {
                            respond!(ctx, respond);
                        } else {
                            respond!(ctx, "Sorry, something went wrong :(");
                            log_error!(e, "Error when processing command");
                        }
                    }
                });

                return Ok(());
            }

            if let Some(handler) = scripts.get(other) {
                if let Err(e) = handler.call(ctx.clone()).await {
                    ctx.respond("Sorry, something went wrong :(").await;
                    log_error!(e, "Error when processing command");
                }

                return Ok(());
            }
        }
    }

    Ok(())
}

impl<'a> Handler<'a> {
    /// Delete the given message.
    fn delete_message(&self, user: &User) -> Result<()> {
        let id = match &user.inner.tags.id {
            Some(id) => id,
            None => return Ok(()),
        };

        log::info!("Attempting to delete message: {}", id);
        user.inner.sender.delete(id);
        Ok(())
    }

    /// Test if the message should be deleted.
    async fn should_be_deleted(&self, user: &User, message: &str) -> bool {
        // Moderators can say whatever they want.
        if user.is_moderator() {
            return false;
        }

        if self.bad_words_enabled.load().await {
            if let Some(word) = self.test_bad_words(message).await {
                if let Some(why) = word.why.as_ref() {
                    let why = why.render_to_string(&BadWordsVars {
                        name: user.display_name(),
                        target: user.channel(),
                    });

                    match why {
                        Ok(why) => {
                            self.sender.privmsg(&why).await;
                        }
                        Err(e) => {
                            log_error!(e, "failed to render response");
                        }
                    }
                }

                return true;
            }
        }

        #[allow(clippy::collapsible_if)]
        {
            if !user.has_scope(Scope::ChatBypassUrlWhitelist).await
                && self.url_whitelist_enabled.load().await
            {
                if self.has_bad_link(message) {
                    return true;
                }
            }
        }

        false
    }

    /// Test the message for bad words.
    async fn test_bad_words(&self, message: &str) -> Option<Arc<db::Word>> {
        let tester = self.bad_words.tester().await;

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
    fn send_ping(&mut self) -> Result<()> {
        self.sender
            .send_immediate(Command::PING(String::from(SERVER), None));

        self.pong_timeout
            .set(Box::pin(tokio::time::sleep(time::Duration::from_secs(5))));
        Ok(())
    }

    /// Process the given command.
    pub async fn process_message(&mut self, user: &User, mut message: Arc<String>) -> Result<()> {
        // Run message hooks.
        let _ = task::spawn({
            let user = user.clone();
            let context_inner = self.context_inner.clone();
            let message = message.clone();

            async move {
                let message_hooks = context_inner.message_hooks.read().await;

                for (key, hook) in &*message_hooks {
                    if let Err(e) = hook.peek(&user, &*message).await {
                        log_error!(e, "Hook `{}` failed", key);
                    }
                }
            }
        });

        // only non-moderators and non-streamer bumps the idle counter.
        if !user.is_streamer() {
            self.idle.seen();
        }

        // NB: declared here to be in scope.
        let mut seen = HashSet::new();
        let mut path = Vec::new();

        if let Some(aliases) = self.aliases.as_ref() {
            while let Some((key, next)) = aliases.resolve(user.channel(), message.clone()).await {
                path.push(key.to_string());

                if !seen.insert(key.clone()) {
                    respond!(
                        user,
                        "Recursion found in alias expansion: {} :(",
                        path.join(" -> ")
                    );
                    return Ok(());
                }

                message = Arc::new(next);
            }
        }

        let mut it = utils::Words::new(message.clone());
        let first = it.next();

        if let Some(commands) = self.commands.as_ref() {
            if let Some((command, captures)) = commands
                .resolve(user.channel(), first.as_deref(), &it)
                .await
            {
                if command.has_var("count") {
                    commands.increment(&*command).await?;
                }

                let vars = CommandVars {
                    name: user.display_name(),
                    target: user.channel(),
                    count: command.count(),
                    captures,
                };

                let response = command.render(&vars)?;
                self.sender.privmsg(response).await;
            }
        }

        if let Some(command) = first {
            if command.starts_with('!') {
                let command = &command[1..];

                let ctx = command::Context {
                    api_url: self.api_url.clone(),
                    user: user.clone(),
                    it,
                    inner: self.context_inner.clone(),
                };

                let result = process_command(
                    command,
                    ctx,
                    self.global_bus,
                    &self.currency_handler,
                    &self.handlers,
                    &self.scripts,
                );

                if let Err(e) = result.await {
                    log_error!(e, "failed to process command");
                }
            }
        }

        if self.should_be_deleted(&user, &*message).await {
            self.delete_message(&user)?;
        }

        Ok(())
    }

    /// Run the given raw command.
    pub async fn raw(&mut self, message: String) -> Result<()> {
        let tags = Tags::default();

        let user = User {
            inner: Arc::new(UserInner {
                tags,
                sender: self.sender.clone(),
                principal: Principal::Injected,
                streamer: self.streamer.user.clone(),
                moderators: self.moderators.clone(),
                vips: self.vips.clone(),
                stream_info: self.stream_info.clone(),
                auth: self.auth.clone(),
            }),
        };

        self.process_message(&user, Arc::new(message)).await
    }

    /// Handle the given command.
    pub async fn handle(&mut self, mut m: Message) -> Result<()> {
        match m.command {
            Command::PRIVMSG(_, ref mut message) => {
                let message = Arc::new(mem::replace(message, String::new()));
                let tags = Tags::from_tags(m.tags.take());

                let name = m
                    .source_nickname()
                    .ok_or_else(|| anyhow!("expected user info"))?
                    .to_string();

                if let Some(chat_log) = self.chat_log.as_ref().cloned() {
                    let tags = tags.clone();
                    let user = self.streamer.user.clone();
                    let name = name.clone();
                    let message = message.clone();

                    task::spawn(Box::pin(async move {
                        chat_log.observe(&tags, &*user, &name, &*message).await;
                    }));
                }

                let user = User {
                    inner: Arc::new(UserInner {
                        tags,
                        sender: self.sender.clone(),
                        principal: Principal::User { name },
                        streamer: self.streamer.user.clone(),
                        moderators: self.moderators.clone(),
                        vips: self.vips.clone(),
                        stream_info: self.stream_info.clone(),
                        auth: self.auth.clone(),
                    }),
                };

                self.process_message(&user, message).await?;
            }
            Command::CAP(_, CapSubCommand::ACK, _, ref what) => {
                #[allow(clippy::single_match)]
                match what.as_deref() {
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
                self.pong_timeout.clear();
            }
            Command::NOTICE(_, ref message) => {
                let tags = Tags::from_tags(m.tags.take());

                match tags.msg_id.as_deref() {
                    _ if message == "Login authentication failed" => {
                        self.token.force_refresh().await?;
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
            Command::Raw(ref command, ref tail) => match command.as_str() {
                "CLEARMSG" => {
                    if let Some(chat_log) = self.chat_log.as_ref() {
                        if let Some(tags) = ClearMsgTags::from_tags(m.tags) {
                            chat_log.message_log.delete_by_id(&tags.target_msg_id).await;
                        }
                    }
                }
                "CLEARCHAT" => {
                    if let Some(chat_log) = self.chat_log.as_ref() {
                        match tail.first() {
                            Some(user) => {
                                chat_log.message_log.delete_by_user(user).await;
                            }
                            None => {
                                chat_log.message_log.delete_all().await;
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
    streamer: &'a api::User,
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
            .as_deref()
            .unwrap_or_else(|| self.name)
    }

    /// Respond to the user with a message.
    pub async fn respond(&self, m: impl fmt::Display) {
        self.sender
            .privmsg(crate::utils::respond(self.display_name(), m))
            .await;
    }

    /// Test if the current user is the given user.
    pub fn is(&self, name: &str) -> bool {
        self.name == name.to_lowercase()
    }

    /// Test if streamer.
    fn is_streamer(&self) -> bool {
        self.name == self.streamer.login
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
    pub async fn has_scope(&self, scope: Scope) -> bool {
        self.auth.test_any(scope, self.name, self.roles()).await
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
    streamer: Arc<api::User>,
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
            .as_deref()
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
    pub fn streamer(&self) -> &api::User {
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
    pub async fn respond(&self, m: impl fmt::Display) {
        match self.display_name() {
            Some(name) => {
                self.inner
                    .sender
                    .privmsg(crate::utils::respond(name, m))
                    .await;
            }
            None => {
                self.inner.sender.privmsg(m).await;
            }
        }
    }

    /// Render an iterable of results, that implements display.
    pub async fn respond_lines<I>(&self, results: I, empty: &str)
    where
        I: IntoIterator,
        I::Item: fmt::Display,
    {
        let mut output = partition_response(results, 360, " | ");

        if let Some(line) = output.next() {
            let count = output.count();

            if count > 0 {
                self.respond(format!("{} ... {} line(s) not shown", line, count))
                    .await;
            } else {
                self.respond(line).await;
            }
        } else {
            self.respond(empty).await;
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
    pub async fn has_scope(&self, scope: Scope) -> bool {
        let user = match self.real() {
            Some(user) => user,
            None => return false,
        };

        user.has_scope(scope).await
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

impl<I> Iterator for PartitionResponse<'_, I>
where
    I: Iterator,
    I::Item: fmt::Display,
{
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        use std::fmt::Write as _;
        const TAIL: &str = "...";

        self.line_buf.clear();
        // length of current line.
        let mut len = 0;

        while let Some(result) = self.iter.next() {
            self.item_buf.clear();
            write!(&mut self.item_buf, "{}", result)
                .expect("a Display implementation returned an error unexpectedly");

            #[allow(clippy::never_loop)]
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
fn partition_response<I>(iter: I, width: usize, sep: &str) -> PartitionResponse<'_, I::IntoIter>
where
    I: IntoIterator,
    I::Item: fmt::Display,
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
    #[allow(clippy::single_match)]
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
    #[allow(clippy::single_match)]
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
    #[serde(flatten)]
    captures: db::Captures<'a>,
}

// Future to refresh moderators every 5 minutes.
async fn refresh_mods_future(sender: Sender) -> Result<()> {
    let mut interval = tokio::time::interval(time::Duration::from_secs(60 * 5));

    loop {
        interval.tick().await;
        log::trace!("refreshing mods and vips");
        sender.mods();
        sender.vips();
    }
}

/// Parse the `room_mods` message.
fn parse_room_members(message: &str) -> HashSet<String> {
    let mut out = HashSet::default();

    if let Some(index) = message.find(':') {
        let message = &message[(index + 1)..];
        let message = message.trim_end_matches('.');

        out.extend(
            message
                .split(',')
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
    use std::collections::HashSet;

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
