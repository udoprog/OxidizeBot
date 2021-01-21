use crate::api::{self, twitch};
use crate::auth::{Auth, Role, Scope};
use crate::bus;
use crate::command;
use crate::currency::CurrencyBuilder;
use crate::db;
use crate::idle;
use crate::injector::{self, Injector, Key};
use crate::message_log::MessageLog;
use crate::module;
use crate::oauth2;
use crate::prelude::*;
use crate::script;
use crate::stream_info;
use crate::task;
use crate::utils::{self, Cooldown, Duration};
use anyhow::{anyhow, bail, Context as _, Error, Result};
use irc::client::{self, Client};
use irc::proto::command::{CapSubCommand, Command};
use irc::proto::message::{Message, Tag};
use leaky_bucket::LeakyBuckets;
use notify::{RecommendedWatcher, Watcher};
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

struct TwitchSetup {
    streamer_stream: injector::Stream<oauth2::SyncToken>,
    streamer: Option<oauth2::SyncToken>,
    streamer_user: Option<Arc<twitch::User>>,
    bot_stream: injector::Stream<oauth2::SyncToken>,
    bot: Option<oauth2::SyncToken>,
    bot_user: Option<Arc<twitch::User>>,
}

impl TwitchSetup {
    pub async fn setup(
        &mut self,
    ) -> Result<(
        Arc<twitch::User>,
        api::Twitch,
        Arc<twitch::User>,
        api::Twitch,
    )> {
        // loop to setup all necessary twitch authentication.
        loop {
            let (streamer_twitch, bot_twitch) = match (self.streamer.as_ref(), self.bot.as_ref()) {
                (Some(streamer_twitch), Some(bot_twitch)) => (streamer_twitch, bot_twitch),
                (_, _) => {
                    tokio::select! {
                        update = self.streamer_stream.select_next_some() => {
                            self.streamer = update;
                        },
                        update = self.bot_stream.select_next_some() => {
                            self.bot = update;
                        },
                    }

                    continue;
                }
            };

            let bot_twitch = api::Twitch::new(bot_twitch.clone())?;
            let streamer_twitch = api::Twitch::new(streamer_twitch.clone())?;

            let streamer = async {
                match streamer_twitch.user().await {
                    Ok(user) => Ok::<_, Error>(Some(user)),
                    Err(e) => {
                        streamer_twitch.token.force_refresh().await?;
                        log_warn!(e, "Failed to get streamer information");
                        Ok(None)
                    }
                }
            };

            let bot = async {
                match bot_twitch.user().await {
                    Ok(user) => Ok::<_, Error>(Some(user)),
                    Err(e) => {
                        bot_twitch.token.force_refresh().await?;
                        log_warn!(e, "Failed to get bot information");
                        Ok(None)
                    }
                }
            };

            let (bot, streamer) = match tokio::try_join!(bot, streamer)? {
                (Some(bot), Some(streamer)) => (bot, streamer),
                (bot, streamer) => {
                    if bot.is_none() {
                        self.bot = None;
                    }

                    if streamer.is_none() {
                        self.streamer = None;
                    }

                    continue;
                }
            };

            let bot = Arc::new(bot);
            let streamer = Arc::new(streamer);

            self.bot_user = Some(bot.clone());
            self.streamer_user = Some(streamer.clone());

            return Ok((bot, bot_twitch, streamer, streamer_twitch));
        }
    }

    /// Inner update helper function.
    async fn update_token_for(
        token_update: &mut Option<oauth2::SyncToken>,
        existing_user: Option<&Arc<twitch::User>>,
        token: Option<oauth2::SyncToken>,
    ) -> Result<bool> {
        *token_update = token;

        let token = match token_update.as_ref() {
            Some(token) => token,
            None => return Ok(true),
        };

        let old_user = match existing_user {
            Some(user) => user,
            None => return Ok(true),
        };

        let user = api::Twitch::new(token.clone())?.user().await?;
        Ok(user.id != old_user.id)
    }

    /// Update the bot token and force a restart in case it has changed.
    pub async fn update_streamer(&mut self, token: Option<oauth2::SyncToken>) -> Result<bool> {
        Self::update_token_for(&mut self.streamer, self.streamer_user.as_ref(), token).await
    }

    /// Update the bot token and force a restart in case it has changed.
    pub async fn update_bot(&mut self, token: Option<oauth2::SyncToken>) -> Result<bool> {
        Self::update_token_for(&mut self.bot, self.bot_user.as_ref(), token).await
    }
}

/// Helper struct to construct IRC integration.
pub struct Irc {
    pub db: db::Database,
    pub bad_words: db::Words,
    pub global_bus: Arc<bus::Bus<bus::Global>>,
    pub command_bus: Arc<bus::Bus<bus::Command>>,
    pub modules: Vec<Box<dyn module::Module>>,
    pub restart: utils::Restart,
    pub settings: settings::Settings,
    pub auth: Auth,
    pub global_channel: injector::Var<Option<String>>,
    pub injector: Injector,
    pub stream_state_tx: mpsc::Sender<stream_info::StreamState>,
    pub message_log: MessageLog,
    pub script_dirs: Vec<PathBuf>,
}

impl Irc {
    pub async fn run(self) -> Result<()> {
        let Irc {
            db,
            bad_words,
            global_bus,
            command_bus,
            modules,
            restart,
            settings,
            auth,
            global_channel,
            injector,
            stream_state_tx,
            message_log,
            script_dirs,
        } = self;

        let (streamer_stream, streamer) = injector
            .stream_key(&Key::<oauth2::SyncToken>::tagged(
                oauth2::TokenId::TwitchStreamer,
            )?)
            .await;

        let (bot_stream, bot) = injector
            .stream_key(&Key::<oauth2::SyncToken>::tagged(
                oauth2::TokenId::TwitchBot,
            )?)
            .await;

        let mut twitch_setup = TwitchSetup {
            streamer_stream,
            streamer,
            streamer_user: None,
            bot_stream,
            bot,
            bot_user: None,
        };

        'outer: loop {
            let (bot, bot_twitch, streamer, streamer_twitch) = twitch_setup.setup().await?;

            let channel = Arc::new(streamer_twitch.channel().await?);

            log::trace!("Channel: {:?}", channel);
            log::trace!("Streamer: {:?}", streamer);
            log::trace!("Bot: {:?}", bot);

            let chat_channel = format!("#{}", channel.name);
            *global_channel.write().await = Some(chat_channel.clone());

            let access_token = bot_twitch.token.read().await?.access_token().to_string();

            let irc_client_config = client::data::config::Config {
                nickname: Some(bot.name.to_string()),
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

            let nightbot = injector.var::<api::NightBot>().await?;

            let mut buckets = LeakyBuckets::new();

            let sender = Sender::new(
                sender_ty,
                chat_channel.clone(),
                client.sender(),
                nightbot.clone(),
                &buckets,
            )?;

            let mut futures = crate::utils::Futures::new();

            let coordinate = buckets.coordinate()?;

            let future = async move {
                coordinate.await?;
                Ok(())
            };

            futures.push(Box::pin(
                future.instrument(trace_span!(target: "futures", "buckets-coordinator",)),
            ));

            let stream_info = {
                let (stream_info, mut stream_state_rx, future) =
                    stream_info::setup(streamer.clone(), streamer_twitch.clone());

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

            let scripts = script::load_dir(channel.name.clone(), db.clone(), &script_dirs).await?;

            let (scripts_watch_tx, mut scripts_watch_rx) = sync::mpsc::unbounded_channel();

            let _watcher = if !script_dirs.is_empty() {
                let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |e| {
                    let _ = scripts_watch_tx.send(e);
                })?;

                for d in &script_dirs {
                    if d.is_dir() {
                        watcher.watch(d, notify::RecursiveMode::Recursive)?;
                    }
                }

                Some(watcher)
            } else {
                None
            };

            for module in modules.iter() {
                if log::log_enabled!(log::Level::Trace) {
                    log::trace!("initializing module: {}", module.ty());
                }

                let result = module
                    .hook(module::HookContext {
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
                    })
                    .await;

                result.with_context(|| anyhow!("failed to initialize module: {}", module.ty()))?;
            }

            let currency_handler = currency_admin::setup(&injector).await?;

            let future = currency_loop(
                streamer_twitch.clone(),
                channel.clone(),
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

            let (mut api_url_stream, api_url) =
                settings.stream("remote/api-url").optional().await?;

            let join_message = chat_settings.get::<String>("join-message").await?;

            let leave_message = chat_settings
                .get::<String>("leave-message")
                .await?
                .unwrap_or_else(|| String::from("Leaving chat... VoHiYo"));

            let mut chat_log_builder = chat_log::Builder::new(
                bot_twitch.clone(),
                &injector,
                message_log.clone(),
                settings.scoped("chat-log"),
            )
            .await?;

            let (mut commands_stream, commands) = injector.stream().await;
            let (mut aliases_stream, aliases) = injector.stream().await;

            let mut pong_timeout = Fuse::empty();

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
                api_url: Arc::new(api_url),
                moderator_cooldown,
                handlers,
                scripts,
                idle: &idle,
                pong_timeout: &mut pong_timeout,
                token: &bot_twitch.token,
                handler_shutdown: false,
                stream_info: &stream_info,
                auth: &auth,
                currency_handler,
                url_whitelist_enabled,
                bad_words_enabled,
                chat_log: chat_log_builder.build()?,
                channel,
                context_inner: Arc::new(command::ContextInner {
                    sender: sender.clone(),
                    scope_cooldowns: sync::Mutex::new(auth.scope_cooldowns()),
                    message_hooks: sync::RwLock::new(Default::default()),
                    restart: restart.clone(),
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
                                break 'outer;
                            }
                            Err(e) => {
                                log_warn!(e, "IRC component errored, restarting in 5 seconds");
                                tokio::time::sleep(time::Duration::from_secs(5)).await;
                                continue 'outer;
                            }
                        }
                    }
                    Some(update) = twitch_setup.streamer_stream.next() => {
                        if twitch_setup.update_streamer(update).await? {
                            leave.set(Fuse::new(tokio::time::sleep(time::Duration::from_secs(1))));
                        }
                    },
                    Some(update) = twitch_setup.bot_stream.next() => {
                        if twitch_setup.update_bot(update).await? {
                            leave.set(Fuse::new(tokio::time::sleep(time::Duration::from_secs(1))));
                        }
                    },
                    Some(update) = commands_stream.next() => {
                        handler.commands = update;
                    }
                    Some(update) = aliases_stream.next() => {
                        handler.aliases = update;
                    }
                    Some(cache) = chat_log_builder.cache_stream.next() => {
                        chat_log_builder.cache = cache;
                        handler.chat_log = chat_log_builder.build()?;
                    }
                    Some(update) = chat_log_builder.enabled_stream.next() => {
                        chat_log_builder.enabled = update;
                        chat_log_builder.message_log.enabled(update).await;
                        handler.chat_log = chat_log_builder.build()?;
                    }
                    Some(update) = chat_log_builder.emotes_enabled_stream.next() => {
                        chat_log_builder.emotes_enabled = update;
                        handler.chat_log = chat_log_builder.build()?;
                    }
                    Some(update) = api_url_stream.next() => {
                        handler.api_url = Arc::new(update);
                    }
                    Some(update) = moderator_cooldown_stream.next() => {
                        handler.moderator_cooldown = update;
                    }
                    _ = ping_interval.tick() => {
                        handler.send_ping()?;
                    }
                    _ = &mut *handler.pong_timeout => {
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
        }

        Ok(())
    }
}

/// Set up a reward loop.
async fn currency_loop(
    twitch: api::Twitch,
    channel: Arc<twitch::Channel>,
    sender: Sender,
    idle: idle::Idle,
    injector: Injector,
    chat_settings: settings::Settings,
    settings: settings::Settings,
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

    let mut builder = CurrencyBuilder::new(twitch, mysql_schema, injector.clone());
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
                Some(update) = interval_stream.next() => {
                    reward_interval = update;
                    timer = new_timer(&reward_interval, viewer_reward);
                }
                Some(update) = notify_rewards_stream.next() => {
                    notify_rewards = update;
                }
                Some(update) = db_stream.next() => {
                    builder.db = update;
                    currency = builder.build_and_inject().await;
                }
                Some(enabled) = enabled_stream.next() => {
                    builder.enabled = enabled;
                    currency = builder.build_and_inject().await;
                }
                Some(update) = ty_stream.next() => {
                    builder.ty = update;
                    currency = builder.build_and_inject().await;
                }
                Some(name) = name_stream.next() => {
                    builder.name = name.map(Arc::new);
                    currency = builder.build_and_inject().await;
                }
                Some(mysql_url) = mysql_url_stream.next() => {
                    builder.mysql_url = mysql_url;
                    currency = builder.build_and_inject().await;
                }
                Some(update) = mysql_schema_stream.next() => {
                    builder.mysql_schema = update;
                    currency = builder.build_and_inject().await;
                }
                Some(command_enabled) = command_enabled_stream.next() => {
                    builder.command_enabled = command_enabled;
                    currency = builder.build_and_inject().await;
                }
                Some(viewer_reward) = viewer_reward_stream.next() => {
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
                        .add_channel_all(&channel.name, reward, seconds)
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
    /// Information on the current channel.
    channel: Arc<twitch::Channel>,
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
    global_bus: &Arc<bus::Bus<bus::Global>>,
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
                    &self.global_bus,
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
                streamer: self.streamer.clone(),
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
                    let channel = self.channel.clone();
                    let name = name.clone();
                    let message = message.clone();

                    task::spawn(Box::pin(async move {
                        chat_log.observe(&tags, &*channel, &name, &*message).await;
                    }));
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
            .as_deref()
            .unwrap_or_else(|| self.name)
    }

    /// Respond to the user with a message.
    pub async fn respond(&self, m: impl fmt::Display) {
        self.sender
            .privmsg(format!("{} -> {}", self.display_name(), m))
            .await;
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
    pub async fn respond(&self, m: impl fmt::Display) {
        match self.display_name() {
            Some(name) => {
                self.inner
                    .sender
                    .privmsg(format!("{} -> {}", name, m))
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
