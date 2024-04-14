use std::path::{Path, PathBuf};
use std::pin::{pin, Pin};
use std::sync::Arc;
use std::time;

use anyhow::{anyhow, bail, Context as _, Result};
use async_fuse::Fuse;
use async_injector::{Injector, Key, Provider};
use auth::{Auth, Role, Scope};
use common::backoff;
use common::irc::Tags;
use common::stream::{Stream, StreamExt};
use common::{tags, Channel, Cooldown};
use irc::client::{self, Client};
use irc::proto::command::Command;
use irc::proto::message::{Message, Tag};
use irc::proto::Prefix;
use notify::{recommended_watcher, RecommendedWatcher, Watcher};
use serde::Serialize;
use std::collections::HashSet;
use std::fmt;
use tokio::sync::{mpsc, Notify};

use crate::chat_log;
use crate::command;
use crate::currency_admin;
use crate::idle;
use crate::messages;
use crate::module;
use crate::reward_loop;
use crate::script;
use crate::sender;
use crate::stream_info;
use crate::task;
use crate::utils;

const SERVER: &str = "irc.chat.twitch.tv";
const TWITCH_TAGS_CAP: &str = "twitch.tv/tags";
const TWITCH_COMMANDS_CAP: &str = "twitch.tv/commands";

/// The type of a pending command.
type PendingOutput<'a> = (Result<()>, command::Context<'a>);
type HookOutput<'a> = Result<()>;

/// Helper struct to construct Chat integration.
pub struct Configuration {
    user_agent: &'static str,
    injector: Injector,
    stream_state_tx: mpsc::Sender<stream_info::StreamState>,
    restart: Arc<Notify>,
    modules: Vec<Box<dyn module::Module>>,
    script_dirs: Vec<PathBuf>,
}

impl Configuration {
    /// Construct a new chat configuration.
    pub fn new(
        user_agent: &'static str,
        injector: Injector,
        stream_state_tx: mpsc::Sender<stream_info::StreamState>,
        restart: Arc<Notify>,
    ) -> Self {
        Self {
            user_agent,
            injector,
            stream_state_tx,
            restart,
            modules: Vec::new(),
            script_dirs: Vec::new(),
        }
    }

    /// Add a script directory to watch.
    pub fn script_dir<P>(&mut self, script_dir: P)
    where
        P: AsRef<Path>,
    {
        self.script_dirs.push(script_dir.as_ref().to_owned());
    }

    /// Add a module.
    pub fn module<M>(&mut self, module: M)
    where
        M: module::Module,
    {
        self.modules.push(Box::new(module));
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(self) -> Result<()> {
        tracing::trace!("Waiting for everything to be ready");

        let mut provider = Setup::provider(&self.injector).await?;

        let mut error_backoff = backoff::Exponential::new(time::Duration::from_millis(100));

        loop {
            let Some(setup) = provider.build() else {
                provider.wait().await;
                continue;
            };

            let irc_loop = ChatLoop {
                setup,
                provider: &mut provider,
                irc: &self,
            };

            match irc_loop.run().await {
                Ok(()) => {
                    error_backoff.reset();
                }
                Err(e) => {
                    let backoff = error_backoff.failed();
                    common::log_error!(e, "Chat component crashed, restarting in {:?}", backoff);
                    tokio::time::sleep(backoff).await;
                    continue;
                }
            }
        }
    }
}

#[derive(Provider)]
struct Setup {
    #[dependency]
    db: db::Database,
    #[dependency(tag = tags::Twitch::Bot)]
    bot: api::TwitchAndUser,
    #[dependency(tag = tags::Twitch::Streamer)]
    streamer: api::TwitchAndUser,
    #[dependency]
    auth: Auth,
    #[dependency]
    bad_words: db::Words,
    #[dependency]
    message_log: messagelog::MessageLog,
    #[dependency]
    command_bus: bus::Bus<bus::Command>,
    #[dependency]
    global_bus: bus::Bus<bus::Global>,
    #[dependency]
    settings: settings::Settings<::auth::Scope>,
}

struct ChatLoop<'a> {
    setup: Setup,
    provider: &'a mut SetupProvider,
    irc: &'a Configuration,
}

impl ChatLoop<'_> {
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
        } = setup;

        let Configuration {
            user_agent,
            injector,
            stream_state_tx,
            restart,
            modules,
            script_dirs,
            ..
        } = irc;

        tracing::info!("Starting to chat");
        tracing::trace!("Streamer: {:?}", streamer.user.display_name);
        tracing::trace!("Bot: {:?}", bot.user.display_name);

        let chat_channel = format!("#{}", streamer.user.login);
        injector
            .update_key(Key::tagged(tags::Globals::Channel)?, chat_channel.clone())
            .await;

        let access_token = bot.client.token().read().context("missing bot token")?.0;

        let irc_client_config = client::data::config::Config {
            nickname: Some(bot.user.login.to_string()),
            channels: vec![chat_channel.clone()],
            password: Some(format!("oauth:{}", access_token.as_str())),
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

        let sender =
            sender::Sender::new(sender_ty, chat_channel.clone(), client.sender(), nightbot)?;

        let (stream_info, stream_info_future) =
            stream_info::setup(streamer.clone(), stream_state_tx.clone());

        let context_inner = Arc::new(command::ContextInner::new(
            sender.clone(),
            auth.scope_cooldowns(),
            restart.clone(),
        ));

        let refresh_roles_future = refresh_roles(context_inner.clone(), streamer.clone());

        let mut handlers = module::Handlers::default();

        let channel = Channel::from_string(&streamer.user.login);

        let mut scripts = script::load_dir(channel.as_ref(), db.clone(), script_dirs).await?;

        let (scripts_watch_tx, mut scripts_watch_rx) = mpsc::unbounded_channel();

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

        let mut hook_futures = Vec::new();

        for module in modules {
            tracing::trace!("Initializing module: {}", module.ty());

            let result = module
                .hook(module::HookContext {
                    handlers: &mut handlers,
                    tasks: &mut hook_futures,
                    stream_info: &stream_info,
                    idle: &idle,
                    streamer: &streamer,
                    sender: &sender,
                    settings: &settings,
                    injector,
                })
                .await;

            result.with_context(|| anyhow!("failed to initialize module: {}", module.ty()))?;
        }

        let currency_handler = currency_admin::setup(injector).await?;

        let reward_loop_future = reward_loop::setup(
            streamer.clone(),
            sender.clone(),
            idle.clone(),
            injector.clone(),
            chat_settings.clone(),
            settings.clone(),
        );

        let (mut whitelisted_hosts_stream, whitelisted_hosts) = chat_settings
            .stream("whitelisted-hosts")
            .or_default()
            .await?;

        let (mut moderator_cooldown_stream, moderator_cooldown) = chat_settings
            .stream("moderator-cooldown")
            .optional()
            .await?;

        let (mut api_url_stream, api_url) = settings.stream("remote/api-url").optional().await?;

        let mut chat_log_builder = chat_log::Builder::new(
            user_agent,
            streamer.clone(),
            injector,
            message_log.clone(),
            settings.scoped("chat-log"),
        )
        .await?;

        let (mut commands_stream, commands) = injector.stream().await;
        let (mut aliases_stream, aliases) = injector.stream().await;

        let mut pong_timeout = Fuse::empty();

        let (messages, messages_future) = messages::setup(&settings).await?;

        // Join all local tasks we are performing.
        common::local_join! {
            futures =>
            stream_info_future,
            refresh_roles_future,
            reward_loop_future,
            messages_future
        }

        for future in &mut hook_futures {
            futures.push(future.as_mut());
        }

        // We maintain separate collections of local futures which we add
        // potentially user-defined tasks to, to allow them to .await
        // concurrently with the main chat task.
        let mut pending = common::Futures::default();
        let mut hooks = common::Futures::default();

        let mut handler = Handler {
            streamer: &streamer,
            sender: sender.clone(),
            whitelisted_hosts,
            commands,
            bad_words: &bad_words,
            global_bus: &global_bus,
            aliases,
            api_url: Arc::new(api_url),
            moderator_cooldown,
            handlers: &handlers,
            scripts: &mut scripts,
            idle: &idle,
            pong_timeout: &mut pong_timeout,
            bot: &bot,
            handler_shutdown: false,
            stream_info: &stream_info,
            auth: &auth,
            currency_handler: &currency_handler,
            url_whitelist_enabled,
            bad_words_enabled,
            chat_log: chat_log_builder.build()?,
            messages: &messages,
            context_inner: &context_inner,
        };

        let mut outgoing = client
            .outgoing()
            .ok_or_else(|| anyhow!("missing outgoing future for irc client"))?;

        let mut client_stream = client.stream()?;

        let mut ping_interval = tokio::time::interval(time::Duration::from_secs(10));
        let mut commands = command_bus.subscribe();

        let mut leave = pin!(Fuse::empty());

        let sender = handler.sender.clone();

        // Things to do when joining.
        let mut join_task = pin!(Fuse::new(async {
            sender.cap_req(TWITCH_TAGS_CAP).await;
            sender.cap_req(TWITCH_COMMANDS_CAP).await;

            if let Some(m) = messages.try_get(messages::JOIN_CHAT).await {
                sender.privmsg_immediate(m);
            }
        }));

        while leave.is_empty() {
            tokio::select! {
                _ = &mut join_task => {
                    tracing::trace!("Done sending capabilities request and join message");
                }
                Some(ev) = scripts_watch_rx.recv() => {
                    if let Ok(ev) = ev {
                        if let Err(e) = handler.handle_script_filesystem_event(ev) {
                            common::log_error!(e, "Failed to handle script filesystem event");
                        }
                    }
                }
                command = commands.recv() => {
                    let command = command?;

                    match command {
                        bus::Command::Raw { command } => {
                            tracing::trace!("Raw command: {}", command);

                            if let Err(e) = handler.raw(command, &mut pending, &mut hooks).await {
                                common::log_error!(e, "Failed to handle message");
                            }
                        }
                    }
                }
                Some(future) = futures.next() => {
                    match future {
                        Ok(..) => {
                            tracing::warn!("Chat component exited, exiting...");
                            return Ok(());
                        }
                        Err(e) => {
                            common::log_warn!(e, "Chat component errored, restarting in 5 seconds");
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
                    bail!("Server not responding");
                }
                update = whitelisted_hosts_stream.recv() => {
                    handler.whitelisted_hosts = update;
                },
                message = client_stream.next() => {
                    if let Some(m) = message.transpose()? {
                        if let Err(e) = handler.handle(m, &mut pending, &mut hooks).await {
                            common::log_error!(e, "Failed to handle message");
                        }
                    }

                    if handler.handler_shutdown {
                        bail!("handler forcibly shut down");
                    }
                }
                Some(result) = pending.next(), if !pending.is_empty() => {
                    if let (Err(error), ctx) = result {
                        if let Some(respond) = error.downcast_ref::<crate::RespondErr>() {
                            if let crate::RespondErr::Message(respond) = respond {
                                respond!(ctx, respond);
                            }
                        } else {
                            respond!(ctx, "Sorry, something went wrong :(");
                            common::log_error!(error, "Error when processing command");
                        }
                    }
                }
                Some(result) = hooks.next(), if !hooks.is_empty() => {
                    if let Err(error) = result {
                        common::log_error!(error, "Hooks failed to run");
                    }
                }
                _ = &mut outgoing => {
                    bail!("Outgoing future ended unexpectedly");
                }
                _ = &mut leave => {
                    break;
                }
            }
        }

        if let Some(m) = handler.messages.try_get(messages::LEAVE_CHAT).await {
            handler.sender.privmsg_immediate(m);
        }

        tokio::select! {
            _ = outgoing => {
                bail!("outgoing future ended unexpectedly");
            }
            _ = leave => {
            }
        }

        Ok(())
    }
}

/// Handler for incoming messages.
struct Handler<'a> {
    /// Current Streamer.
    streamer: &'a api::TwitchAndUser,
    /// Queue for sending messages.
    sender: sender::Sender,
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
    handlers: &'a module::Handlers,
    /// Dynamic handlers.
    scripts: &'a mut script::Scripts,
    /// Build idle detection.
    idle: &'a idle::Idle,
    /// Pong timeout currently running.
    pong_timeout: &'a mut Fuse<Pin<Box<tokio::time::Sleep>>>,
    /// OAuth 2.0 Token used to authenticate with Chat.
    bot: &'a api::TwitchAndUser,
    /// Force a shutdown.
    handler_shutdown: bool,
    /// Stream information.
    stream_info: &'a stream_info::StreamInfo,
    /// Information about auth.
    auth: &'a Auth,
    /// Handler for currencies.
    currency_handler: &'a currency_admin::Handler,
    bad_words_enabled: settings::Var<bool>,
    url_whitelist_enabled: settings::Var<bool>,
    /// Handler for chat logs.
    chat_log: Option<chat_log::ChatLog>,
    /// Messages.
    messages: &'a messages::Messages,
    /// Shared context paramters.
    pub(crate) context_inner: &'a Arc<command::ContextInner>,
}

impl Handler<'_> {
    /// Handle filesystem event.
    fn handle_script_filesystem_event(&mut self, ev: notify::Event) -> Result<()> {
        use notify::event::{CreateKind, EventKind::*, ModifyKind, RemoveKind, RenameMode};

        tracing::trace!("Filesystem event: {:?}", ev);

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
                    tracing::info!("Loading script: {}", p.display());

                    let p = p.canonicalize()?;

                    if let Err(e) = self.scripts.reload(&p) {
                        common::log_error!(e, "Failed to reload: {}", p.display());
                    }
                }
            }
            Kind::Remove => {
                for p in &ev.paths {
                    tracing::info!("Unloading script: {}", p.display());

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
async fn process_command<'a>(
    command: &str,
    mut ctx: command::Context<'a>,
    global_bus: &'a bus::Bus<bus::Global>,
    currency_handler: &'a currency_admin::Handler,
    handlers: &'a module::Handlers,
    scripts: &script::Scripts,
    pending: &mut common::Futures<'a, PendingOutput<'a>>,
) -> Result<()> {
    match command {
        "ping" => {
            respond!(ctx, "What do you want?");
            global_bus.send(bus::Global::Ping).await;
        }
        other => {
            tracing::trace!("Testing command: {}", other);

            // TODO: store currency name locally to match against.
            let currency_command = currency_handler.command_name().await;

            let handler = match (other, currency_command) {
                (other, Some(name)) if other == name.as_ref() => {
                    Some(currency_handler as &dyn command::Handler)
                }
                (other, Some(..)) | (other, None) => handlers.get(other),
            };

            if let Some(handler) = handler {
                let scope = handler.scope();

                // Test if user has the required scope to run the given
                // command.
                if let Some(scope) = scope {
                    tracing::info! {
                        ?scope,
                        roles = ?ctx.user.roles(),
                        principal = ?ctx.user.principal(),
                        "Testing handler scope"
                    };

                    if !ctx.user.has_scope(scope).await {
                        if ctx.user.is_moderator() {
                            let m = ctx.messages.get(messages::AUTH_FAILED).await;
                            ctx.respond(m).await;
                        } else {
                            let m = ctx.messages.get(messages::AUTH_FAILED_RUDE).await;
                            ctx.respond(m).await;
                        }

                        return Ok(());
                    }
                }

                pending.push(Box::pin(async move {
                    let result = handler.handle(&mut ctx).await;
                    (result, ctx)
                }));

                return Ok(());
            }

            if let Some(handler) = scripts.get(other) {
                if let Err(e) = handler.call(ctx.clone()).await {
                    ctx.respond("Sorry, something went wrong :(").await;
                    common::log_error!(e, "Error when processing command");
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

        tracing::info!("Attempting to delete message: {}", id);
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
                    let why = why.render_to_string(BadWordsVars {
                        name: user.display_name(),
                        target: &self.streamer.user.login,
                    });

                    match why {
                        Ok(why) => {
                            self.sender.privmsg(&why).await;
                        }
                        Err(e) => {
                            common::log_error!(e, "Failed to render response");
                        }
                    }
                }

                return true;
            }
        }

        if !user.has_scope(Scope::ChatBypassUrlWhitelist).await
            && self.url_whitelist_enabled.load().await
            && self.has_bad_link(message)
        {
            return true;
        }

        false
    }

    /// Test the message for bad words.
    async fn test_bad_words(&self, message: &str) -> Option<Arc<db::Word>> {
        let tester = self.bad_words.tester().await;

        for word in common::words::trimmed(message) {
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
    pub(crate) async fn process_message(
        &mut self,
        user: &User,
        mut message: Arc<String>,
        pending: &mut common::Futures<'a, PendingOutput<'a>>,
        hooks: &mut common::Futures<'a, HookOutput<'a>>,
    ) -> Result<()> {
        // Run message hooks.
        hooks.push({
            let user = user.clone();
            let context_inner = self.context_inner;
            let message = message.clone();

            Box::pin(async move {
                let message_hooks = context_inner.message_hooks.read().await;

                for (key, hook) in &*message_hooks {
                    if let Err(e) = hook.peek(&user, &message).await {
                        common::log_error!(e, "Hook `{}` failed", key);
                    }
                }

                Ok(())
            })
        });

        // only non-moderators and non-streamer bumps the idle counter.
        if !user.is_streamer() {
            self.idle.seen();
        }

        // NB: declared here to be in scope.
        let mut seen = HashSet::new();
        let mut path = Vec::new();

        if let Some(aliases) = self.aliases.as_ref() {
            tracing::trace!(?message, channel = ?user.sender().channel(), "Resolving aliases");

            while let Some((key, next)) = aliases
                .resolve(user.sender().channel(), message.clone())
                .await
            {
                path.push(key.to_string());

                if !seen.insert(key.clone()) {
                    tracing::error!(?message, ?path, "Recursion found in alias expansion");

                    respond!(
                        user,
                        "Recursion found in alias expansion: {} :(",
                        path.join(" -> ")
                    );
                    return Ok(());
                }

                tracing::trace!(?message, ?next, "Resolved alias");
                message = Arc::new(next);
            }
        }

        let mut it = common::words::split(message.clone());
        let first = it.next();

        if let Some(commands) = self.commands.as_ref() {
            if let Some((command, captures)) = commands
                .resolve(user.sender().channel(), first.as_deref(), &it)
                .await
            {
                if command.has_var("count") {
                    commands.increment(&command).await?;
                }

                let vars = CommandVars {
                    name: user.display_name(),
                    target: &self.streamer.user.login,
                    count: command.count(),
                    captures,
                };

                let response = command.render(&vars)?;
                self.sender.privmsg(response).await;
            }
        }

        if let Some(command) = first {
            if let Some(command) = command.strip_prefix('!') {
                let ctx = command::Context {
                    api_url: self.api_url.clone(),
                    user: user.clone(),
                    it,
                    messages: self.messages,
                    inner: self.context_inner,
                };

                let result = process_command(
                    command,
                    ctx,
                    self.global_bus,
                    self.currency_handler,
                    self.handlers,
                    self.scripts,
                    pending,
                )
                .await;

                if let Err(e) = result {
                    common::log_error!(e, "Failed to process command");
                }
            }
        }

        if self.should_be_deleted(user, &message).await {
            self.delete_message(user)?;
        }

        Ok(())
    }

    /// Run the given raw command.
    pub(crate) async fn raw(
        &mut self,
        message: String,
        pending: &mut common::Futures<'a, PendingOutput<'a>>,
        hook: &mut common::Futures<'a, HookOutput<'a>>,
    ) -> Result<()> {
        let tags = Tags::default();

        let user = User {
            inner: Arc::new(UserInner {
                tags,
                sender: self.sender.clone(),
                principal: Principal::Injected,
                streamer_login: self.streamer.user.login.clone(),
                stream_info: self.stream_info.clone(),
                auth: self.auth.clone(),
                context: self.context_inner.clone(),
            }),
        };

        self.process_message(&user, Arc::new(message), pending, hook)
            .await?;
        Ok(())
    }

    /// Handle the given command.
    #[tracing::instrument(skip_all)]
    pub(crate) async fn handle(
        &mut self,
        m: Message,
        pending: &mut common::Futures<'a, PendingOutput<'a>>,
        hooks: &mut common::Futures<'a, HookOutput<'a>>,
    ) -> Result<()> {
        match m.command {
            Command::PRIVMSG(_, message) => {
                let message = Arc::new(message);

                let tags = Tags::from_tags(m.tags.iter().flat_map(|tag| {
                    tag.iter().flat_map(|tag| match tag {
                        Tag(key, Some(value)) => Some((key, value)),
                        _ => None,
                    })
                }));

                let Some(Prefix::Nickname(login, _, _)) = m.prefix else {
                    bail!("Missing nickname");
                };

                let login = Box::<str>::from(login);

                if let Some(chat_log) = self.chat_log.as_ref().cloned() {
                    let tags = tags.clone();
                    let user = self.streamer.user.clone();
                    let login = login.clone();
                    let message = message.clone();

                    task::spawn(Box::pin(async move {
                        chat_log.observe(&tags, &user, &login, &message).await;
                        Ok(())
                    }));
                }

                let user = User {
                    inner: Arc::new(UserInner {
                        tags,
                        sender: self.sender.clone(),
                        principal: Principal::User { login },
                        streamer_login: self.streamer.user.login.clone(),
                        stream_info: self.stream_info.clone(),
                        auth: self.auth.clone(),
                        context: self.context_inner.clone(),
                    }),
                };

                self.process_message(&user, message, pending, hooks).await?;
            }
            Command::JOIN(channel, _, _) => {
                let user = match &m.prefix {
                    Some(Prefix::Nickname(user, _, _)) => user,
                    _ => "?",
                };

                tracing::trace!("{} joined {}", user, channel);
            }
            Command::PING(server, other) => {
                tracing::trace!("Received PING, responding with PONG");
                self.sender.send_immediate(Command::PONG(server, other));
            }
            Command::PONG(..) => {
                tracing::trace!("Received PONG, clearing PING timeout");
                self.pong_timeout.clear();
            }
            Command::NOTICE(_, message) => {
                let tags = Tags::from_tags(m.tags.iter().flat_map(|tag| {
                    tag.iter().flat_map(|tag| match tag {
                        Tag(key, Some(value)) => Some((key, value)),
                        _ => None,
                    })
                }));

                match tags.msg_id.as_deref() {
                    _ if message == "Login authentication failed" => {
                        tracing::trace!("Authentication failed");
                        self.bot.client.token().force_refresh();
                        self.handler_shutdown = true;
                    }
                    msg_id => {
                        tracing::trace!(?msg_id, message, "Unhandled notice");
                    }
                }
            }
            Command::Response(response, tail) => {
                tracing::trace!(?response, ?tail, "Response");
            }
            Command::Raw(raw, tail) => match raw.as_str() {
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
                    tracing::trace!(?raw, ?tail, "Unhandled raw command");
                }
            },
            _ => {
                tracing::trace!(?m, "Unhandled",);
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
    sender: &'a sender::Sender,
    login: &'a str,
    streamer_login: &'a str,
    stream_info: &'a stream_info::StreamInfo,
    auth: &'a Auth,
    context: &'a command::ContextInner,
}

impl<'a> RealUser<'a> {
    /// Get the login of the user.
    pub fn login(&self) -> &'a str {
        self.login
    }

    /// Get the display name of the user.
    pub fn display_name(&self) -> &'a str {
        self.tags.display_name.as_deref().unwrap_or(self.login)
    }

    /// Respond to the user with a message.
    pub async fn respond(&self, m: impl fmt::Display) {
        self.sender
            .privmsg(crate::respond(self.display_name(), m))
            .await;
    }

    /// Test if the current user is the given user.
    pub fn is(&self, name: &str) -> bool {
        self.login == name.to_lowercase()
    }

    /// Test if streamer.
    pub(crate) fn is_streamer(&self) -> bool {
        self.login == self.streamer_login
    }

    /// Test if moderator.
    fn is_moderator(&self) -> bool {
        self.context.moderators.read().contains(self.login)
    }

    /// Test if user is a subscriber.
    fn is_subscriber(&self) -> bool {
        self.is_streamer() || self.stream_info.is_subscriber(self.login)
    }

    /// Test if vip.
    fn is_vip(&self) -> bool {
        self.context.vips.read().contains(self.login)
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
    pub async fn has_scope<S>(&self, scope: S) -> bool
    where
        S: AsRef<Scope>,
    {
        self.auth.test_any(scope, self.login, self.roles()).await
    }
}

/// Information about the user.
#[derive(Debug)]
pub enum Principal {
    /// A real user based on its login.
    User { login: Box<str> },
    /// An injected user command.
    Injected,
}

/// Inner struct for User to make it cheaper to clone.
struct UserInner {
    tags: Tags,
    sender: sender::Sender,
    principal: Principal,
    streamer_login: String,
    stream_info: stream_info::StreamInfo,
    auth: Auth,
    context: Arc<command::ContextInner>,
}

#[derive(Clone)]
pub struct User {
    inner: Arc<UserInner>,
}

impl User {
    /// Access the user as a real user.
    pub fn real(&self) -> Option<RealUser<'_>> {
        match &self.inner.principal {
            Principal::User { login } => Some(RealUser {
                tags: &self.inner.tags,
                sender: &self.inner.sender,
                login: login.as_ref(),
                streamer_login: &self.inner.streamer_login,
                stream_info: &self.inner.stream_info,
                auth: &self.inner.auth,
                context: &self.inner.context,
            }),
            Principal::Injected => None,
        }
    }

    /// Get the name of the user.
    pub fn name(&self) -> Option<&str> {
        match &self.inner.principal {
            Principal::User { login: name, .. } => Some(name),
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

    /// Access the sender associated with the user.
    pub fn sender(&self) -> &sender::Sender {
        &self.inner.sender
    }

    /// Test if the current user is the given user.
    pub fn is(&self, name: &str) -> bool {
        self.real().map(|u| u.is(name)).unwrap_or(false)
    }

    /// Get the principal of the current user.
    pub fn principal(&self) -> &Principal {
        &self.inner.principal
    }

    /// Test if streamer.
    pub(crate) fn is_streamer(&self) -> bool {
        self.real().map(|u| u.is_streamer()).unwrap_or(true)
    }

    /// Test if moderator.
    pub(crate) fn is_moderator(&self) -> bool {
        self.real().map(|u| u.is_moderator()).unwrap_or(true)
    }

    /// Respond to the user with a message.
    pub async fn respond(&self, m: impl fmt::Display) {
        match self.display_name() {
            Some(name) => {
                self.inner.sender.privmsg(crate::respond(name, m)).await;
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
    pub async fn has_scope<S>(&self, scope: S) -> bool
    where
        S: AsRef<Scope>,
    {
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

        for result in self.iter.by_ref() {
            self.item_buf.clear();
            write!(&mut self.item_buf, "{}", result)
                .expect("a Display implementation returned an error unexpectedly");

            if len + self.item_buf.len() <= self.width {
                if len > 0 {
                    self.line_buf.push_str(self.sep);
                }

                self.line_buf.push_str(&self.item_buf);
                len += self.item_buf.len() + self.sep.len();
                continue;
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
                let Tag(name, Some(value)) = t else {
                    continue;
                };

                if name.as_str() == "target-msg-id" {
                    target_msg_id = Some(value);
                }
            }
        }

        Some(ClearMsgTags {
            target_msg_id: target_msg_id?,
        })
    }
}

#[derive(Serialize)]
pub(crate) struct BadWordsVars<'a> {
    name: Option<&'a str>,
    target: &'a str,
}

#[derive(Serialize)]
pub(crate) struct CommandVars<'a> {
    name: Option<&'a str>,
    target: &'a str,
    count: i32,
    #[serde(flatten)]
    captures: db::Captures<'a>,
}

// Future to populate moderators and VIPs.
#[tracing::instrument(skip_all)]
async fn refresh_roles(
    context: Arc<command::ContextInner>,
    streamer: api::TwitchAndUser,
) -> Result<()> {
    async fn update_set<S>(
        what: &str,
        f: S,
        out: &parking_lot::RwLock<HashSet<String>>,
    ) -> Result<()>
    where
        S: Stream<Item = Result<api::twitch::model::Chatter>>,
    {
        tracing::trace!(?what, "Refreshing");

        let mut stream = pin!(f);
        let mut set = HashSet::new();

        while let Some(chatter) = stream.next().await.transpose()? {
            set.insert(chatter.user_login);
        }

        if *out.read() != set {
            tracing::trace!(?set, ?what, "Changed");
            *out.write() = set;
        }

        Ok(())
    }

    let mut interval = tokio::time::interval(time::Duration::from_secs(60 * 5));

    let mut mods_need = pin!(context.notify().refresh_mods.notified());
    mods_need.as_mut().enable();

    let mut vips_need = pin!(context.notify().refresh_vips.notified());
    vips_need.as_mut().enable();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                context.notify().refresh_mods.notify_one();
                context.notify().refresh_vips.notify_one();
            }
            _ = mods_need.as_mut() => {
                let future = streamer.client.moderators(&streamer.user.id);

                if let Err(error) = update_set("Moderators", future, &context.moderators).await {
                    common::log_error!(error, "Failed to update Moderators");
                }

                // Remove interest since we just updated to avoid duplicates.
                mods_need.set(context.notify().refresh_mods.notified());
            }
            _ = vips_need.as_mut() => {
                let future = streamer.client.vips(&streamer.user.id);

                if let Err(error) = update_set("VIPs", future, &context.vips).await {
                    common::log_error!(error, "Failed to update VIPs");
                }

                // Remove interest since we just updated to avoid duplicates.
                vips_need.set(context.notify().refresh_vips.notified());
            }
        }
    }
}
