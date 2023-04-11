use std::path::{Path, PathBuf};
use std::time;

use anyhow::{anyhow, Context, Result};
use async_injector::{Injector, Key};
use common::backoff;
use common::display;
use common::stream::StreamExt;
use common::tags;
use tokio::sync::mpsc;

use crate::irc;
use crate::module;
use crate::stream_info;
use crate::sys;
use crate::updater;
use crate::utils;

const OLD_CONFIG_DIR: &str = "SetMod";
const CONFIG_DIR: &str = "OxidizeBot";
const LOG: &str = "oxidize.log";

argwerk::define! {
    /// Oxidize Twitch Bot
    /// Site: https://setbac.tv
    ///
    /// Made with ❤️
    ///   ... by John-John Tedro <udoprog@tedro.se>,
    ///       and OxidizeBot Developers!
    #[usage = "oxidize"]
    struct Args {
        help: bool,
        trace: bool,
        silent: bool,
        root: Option<PathBuf>,
        config: Option<PathBuf>,
        log: Vec<String>,
        stack_size: Option<usize>,
    }
    /// Show this help.
    ["--help" | "-h"] => {
        println!("{}", HELP);
        help = true;
    }
    /// If we should enable tracing in all logs.
    ["--trace"] => {
        trace = true;
    }
    /// Suppress desktop notifications.
    ["--silent"] => {
        silent = true;
    }
    /// Configuration directory to use.
    ["--root", #[os] path] => {
        root = Some(PathBuf::from(path));
    }
    /// Configuration file to use.
    ["--config", #[os] path] => {
        config = Some(PathBuf::from(path));
    }
    /// Additionally enable logging for the specified modules. Example: --log irc=trace
    ["--log", spec] => {
        log.push(spec);
    }
    /// Configure a different stack size to use.
    ["--stack-size", size] => {
        stack_size = Some(str::parse(&size)?);
    }
}

/// Configure logging.
fn setup_logs(root: &Path, trace: bool, modules: &[String]) -> Result<(impl Drop, PathBuf)> {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, Registry};

    // Crates to enable logging for, by default.
    const CRATES: [&str; 16] = [
        "bot_api",
        "bot_auth",
        "bot_bus",
        "bot_common",
        "bot_currency",
        "bot_db",
        "bot_emotes",
        "bot_messagelog",
        "bot_oauth2",
        "bot_player",
        "bot_settings",
        "bot_storage",
        "bot_template",
        "bot_web",
        "oxidize",
        "panic",
    ];

    let (capture, _) = crate::tracing::capture();

    let mut env_filter = tracing_subscriber::EnvFilter::from_default_env();

    if trace {
        for name in CRATES {
            env_filter = env_filter.add_directive(format!("{name}=trace").parse()?);
        }
    } else {
        for name in CRATES {
            env_filter = env_filter.add_directive(format!("{name}=info").parse()?);
        }
    };

    for module in modules {
        env_filter = env_filter.add_directive(module.parse()?);
    }

    let file_appender = tracing_appender::rolling::daily(root, LOG);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = Registry::default()
        .with(capture)
        .with(env_filter)
        .with(
            fmt::Layer::default()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(fmt::Layer::default().with_writer(std::io::stdout));

    tracing::subscriber::set_global_default(subscriber)?;
    Ok((guard, root.join(LOG)))
}

#[derive(Debug, Clone, Copy)]
enum Intent {
    Shutdown,
    Restart,
}

/// Entrypoint.
pub fn main() -> Result<()> {
    let args = Args::args()?;

    if args.help {
        return Ok(());
    }

    if let Some(size) = args.stack_size {
        let thread = std::thread::Builder::new()
            .name(format!("main-with-stack-{}", size))
            .spawn(move || inner_main(args))?;

        thread.join().expect("thread shouldn't panic")
    } else {
        inner_main(args)
    }
}

fn inner_main(args: Args) -> Result<()> {
    let (old_root, root) = match args.root {
        Some(root) => (None, root),
        None => {
            let base = dirs::config_dir()
                .ok_or_else(|| anyhow!("no standard configuration directory available"))?;
            let old = base.join(OLD_CONFIG_DIR);
            let new = base.join(CONFIG_DIR);
            (Some(old), new)
        }
    };

    let (_guard, log_file) =
        setup_logs(&root, args.trace, &args.log).context("failed to setup logs")?;

    crate::panic_logger::panic_logger();

    if !root.is_dir() {
        tracing::info!("Creating config directory: {}", root.display());
        std::fs::create_dir_all(&root)?;
    }

    let system = sys::setup(&root, &log_file)?;

    let mut error_backoff = backoff::Exponential::new(time::Duration::from_secs(5));

    if !args.silent {
        let startup = sys::Notification::new(format!("Started Oxidize {}", crate::VERSION));
        system.notification(startup);
    }

    let database_path = {
        let new = root.join("oxidize.sql");

        if let Some(old) = old_root {
            let old = old.join("setmod.sql");

            if old.is_file() && !new.is_file() {
                std::fs::copy(&old, &new).with_context(|| {
                    anyhow!(
                        "failed to copy database: {} to {}",
                        old.display(),
                        new.display()
                    )
                })?;
            }
        }

        new
    };

    let db = db::Database::open(&database_path)
        .with_context(|| anyhow!("failed to open database at: {}", database_path.display()))?;

    let storage = storage::Storage::open(&root.join("storage"))?;

    let script_dirs = vec![root.join("scripts"), PathBuf::from("scripts")];

    loop {
        let runtime = {
            let mut runtime = tokio::runtime::Builder::new_multi_thread();
            runtime.enable_all();

            if let Some(size) = args.stack_size {
                runtime.thread_stack_size(size);
            }

            runtime.build()?
        };

        let future = try_main(&system, &root, &script_dirs, &db, &storage);

        system.clear();

        let backoff = match runtime.block_on(future) {
            Err(e) => {
                let backoff = error_backoff.failed();
                system.error(String::from("Bot crashed, see log for more details."));
                common::log_error!(e, "Bot crashed");
                Some(backoff)
            }
            Ok(Intent::Shutdown) => {
                break;
            }
            Ok(Intent::Restart) => {
                error_backoff.reset();
                None
            }
        };

        if let Some(backoff) = backoff {
            if !args.silent {
                let message = format!(
                    "Restart in {}.\nSee log for more details.",
                    display::compact_duration(backoff)
                );

                let n = sys::Notification::new(message)
                    .title("Bot Crashed!")
                    .icon(sys::NotificationIcon::Error);

                system.notification(n);
            }

            tracing::info!("Restarting in {}...", display::compact_duration(backoff));

            let intent = runtime.block_on(async {
                tokio::select! {
                    _ = system.wait_for_shutdown() => Intent::Shutdown,
                    _ = system.wait_for_restart() => Intent::Restart,
                    _ = tokio::signal::ctrl_c() => Intent::Shutdown,
                    _ = tokio::time::sleep(backoff) => Intent::Restart,
                }
            });

            if let Intent::Shutdown = intent {
                break;
            }
        }

        if !args.silent {
            let n =
                sys::Notification::new("Restarted OxidizeBot").icon(sys::NotificationIcon::Warning);
            system.notification(n);
        }
    }

    if !args.silent {
        let shutdown = sys::Notification::new("Exiting OxidizeBot");
        system.notification(shutdown);
    }

    tracing::info!("Exiting...");
    system.join()?;
    Ok(())
}

/// Actual main function, running the application loop.
async fn try_main(
    system: &sys::System,
    root: &Path,
    script_dirs: &[PathBuf],
    db: &db::Database,
    storage: &storage::Storage,
) -> Result<Intent> {
    tracing::info!("Starting Oxidize Bot Version {}", crate::VERSION);

    if !root.is_dir() {
        std::fs::create_dir_all(root)
            .with_context(|| anyhow!("failed to create root: {}", root.display()))?;
    }

    let injector = Injector::new();

    let mut modules = Vec::<Box<dyn module::Module>>::new();
    let mut futures = common::Futures::new();

    injector.update(db.clone()).await;

    let auth_schema = auth::Schema::load_static(crate::AUTH_SCHEMA)?;
    let auth = auth::Auth::new(db.clone(), auth_schema).await?;
    injector.update(auth.clone()).await;

    let settings_schema = settings::Schema::load_bytes(crate::SETTINGS_SCHEMA)?;
    let settings = settings::Settings::new(db.clone(), settings_schema);

    let drive = settings.clone();

    futures.push(Box::pin({
        let future = drive.drive();
        async move { Ok(future.await?) }
    }));

    settings
        .run_migrations()
        .await
        .context("failed to run settings migrations")?;

    injector.update(settings.clone()).await;

    let bad_words = db::Words::load(db.clone()).await?;
    injector.update(bad_words).await;

    injector
        .update(db::AfterStreams::load(db.clone()).await?)
        .await;
    injector.update(db::Commands::load(db.clone()).await?).await;
    injector.update(db::Aliases::load(db.clone()).await?).await;
    injector
        .update(db::Promotions::load(db.clone()).await?)
        .await;
    injector.update(db::Themes::load(db.clone()).await?).await;

    let message_bus = bus::Bus::new();
    injector.update(message_bus.clone()).await;
    let global_bus = bus::Bus::new();
    injector.update(global_bus.clone()).await;
    let youtube_bus = bus::Bus::new();
    injector.update(youtube_bus.clone()).await;
    let command_bus = bus::Bus::new();
    injector.update(command_bus.clone()).await;

    futures.push(Box::pin(system_loop(
        settings.scoped("system"),
        system.clone(),
    )));

    injector.update(storage.cache()?).await;

    let (latest, future) = updater::updater(&injector);
    futures.push(Box::pin(future));

    let message_log = messagelog::MessageLog::builder()
        .bus(message_bus.clone())
        .limit(512)
        .build();
    injector.update(message_log.clone()).await;

    let (web, future) = web::run(
        crate::VERSION,
        &injector,
        message_log.clone(),
        message_bus.clone(),
        global_bus.clone(),
        youtube_bus.clone(),
        command_bus.clone(),
        auth.clone(),
        latest.clone(),
    )
    .await?;

    futures.push(Box::pin(future));

    if settings.get::<bool>("first-run").await?.unwrap_or(true) {
        tracing::info!("Opening {} for the first time", web::URL);

        if let Err(e) = webbrowser::open(web::URL) {
            tracing::error!("Failed to open browser: {}", e);
        }

        settings.set("first-run", false).await?;
    }

    tracing::info!("Listening on: {}", web::URL);

    let token_settings = settings.scoped("secrets/oauth2");

    let integration = WebIntegration(web.clone());

    futures.push(Box::pin({
        let s = token_settings.scoped("spotify");
        let key = Key::tagged(tags::Token::Spotify)?;
        oauth2::setup(
            "spotify",
            settings.clone(),
            s,
            injector.clone(),
            key,
            integration.clone(),
        )
    }));

    futures.push(Box::pin({
        let s = token_settings.scoped("youtube");
        let key = Key::tagged(tags::Token::YouTube)?;
        oauth2::setup(
            "youtube",
            settings.clone(),
            s,
            injector.clone(),
            key,
            integration.clone(),
        )
    }));

    futures.push(Box::pin({
        let s = token_settings.scoped("nightbot");
        let key = Key::tagged(tags::Token::NightBot)?;
        oauth2::setup(
            "nightbot",
            settings.clone(),
            s,
            injector.clone(),
            key,
            integration.clone(),
        )
    }));

    futures.push(Box::pin({
        let s = token_settings.scoped("twitch-streamer");
        let key = Key::tagged(tags::Token::Twitch(tags::Twitch::Streamer))?;
        oauth2::setup(
            "twitch-streamer",
            settings.clone(),
            s,
            injector.clone(),
            key,
            integration.clone(),
        )
    }));

    futures.push(Box::pin({
        let s = token_settings.scoped("twitch-bot");
        let key = Key::tagged(tags::Token::Twitch(tags::Twitch::Bot))?;
        oauth2::setup(
            "twitch-bot",
            settings.clone(),
            s,
            injector.clone(),
            key,
            integration.clone(),
        )
    }));

    futures.push(Box::pin(api::twitch::pubsub::connect(&settings, &injector)));
    futures.push(Box::pin(api::twitch_clients_task(
        crate::USER_AGENT,
        injector.clone(),
    )));

    futures.push(Box::pin(
        api::open_weather_map::setup(crate::USER_AGENT, settings.clone(), injector.clone()).await?,
    ));

    let (restart, restart_rx) = utils::Restart::new();
    injector.update(restart).await;

    futures.push(Box::pin(api::NightBot::run(
        crate::USER_AGENT,
        injector.clone(),
    )));

    injector
        .update(api::Speedrun::new(crate::USER_AGENT)?)
        .await;

    let future = player::setup(
        crate::USER_AGENT,
        injector.clone(),
        db.clone(),
        global_bus.clone(),
        youtube_bus.clone(),
        settings.clone(),
    );

    futures.push(Box::pin(crate::song_file::setup(
        injector.clone(),
        settings.scoped("player/song-file"),
    )));

    futures.push(Box::pin(future));

    futures.push(Box::pin(
        crate::setbac::run(&settings, &injector, global_bus.clone()).await?,
    ));

    modules.push(Box::new(module::time::Module));
    modules.push(Box::new(module::song::Module));
    modules.push(Box::new(module::command_admin::Module));
    modules.push(Box::new(module::admin::Module));
    modules.push(Box::new(module::alias_admin::Module));
    modules.push(Box::new(module::theme_admin::Module));
    modules.push(Box::new(module::promotions::Module));
    modules.push(Box::new(module::swearjar::Module));
    modules.push(Box::new(module::countdown::Module));
    modules.push(Box::new(module::gtav::Module));
    modules.push(Box::new(module::water::Module));
    modules.push(Box::new(module::misc::Module));
    modules.push(Box::new(module::after_stream::Module));
    modules.push(Box::new(module::clip::Module));
    modules.push(Box::new(module::eight_ball::Module));
    modules.push(Box::new(module::speedrun::Module));
    modules.push(Box::new(module::auth::Module));
    modules.push(Box::new(module::poll::Module));
    modules.push(Box::new(module::weather::Module));
    modules.push(Box::new(module::help::Module));

    let (stream_state_tx, stream_state_rx) = mpsc::channel(64);

    let notify_after_streams = notify_after_streams(&injector, stream_state_rx, system.clone());
    futures.push(Box::pin(notify_after_streams));

    let irc = irc::Irc {
        modules,
        injector: injector.clone(),
        stream_state_tx,
        script_dirs: script_dirs.to_vec(),
    };

    futures.push(Box::pin(irc.run()));

    tokio::select! {
        Some(result) = futures.next() => {
            result.map(|_| Intent::Shutdown)
        }
        _ = system.wait_for_shutdown() => {
            tracing::info!("Shutdown triggered by system");
            Ok(Intent::Shutdown)
        },
        _ = system.wait_for_restart() => {
            tracing::info!("Restart triggered by system");
            Ok(Intent::Restart)
        },
        _ = restart_rx => {
            tracing::info!("Restart triggered by bot");
            Ok(Intent::Restart)
        },
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown triggered by signal");
            Ok(Intent::Shutdown)
        },
    }
}

/// Notify if there are any after streams.
///
/// If this is clicked, open the after-streams page.
#[tracing::instrument(skip_all)]
async fn notify_after_streams(
    injector: &Injector,
    mut rx: mpsc::Receiver<stream_info::StreamState>,
    system: sys::System,
) -> Result<()> {
    let (mut after_streams_stream, mut after_streams) = injector.stream::<db::AfterStreams>().await;

    loop {
        tokio::select! {
            update = after_streams_stream.recv() => {
                after_streams = update;
            }
            Some(update) = rx.recv() => {
                match update {
                    stream_info::StreamState::Started => {
                        tracing::info!("Stream started");
                    }
                    stream_info::StreamState::Stopped => {
                        let after_streams = match after_streams.as_ref() {
                            Some(after_streams) => after_streams,
                            None => continue,
                        };

                        let list = after_streams.list().await?;

                        if !list.is_empty() {
                            let reminder = sys::Notification::new(format!(
                                "You have {} afterstream messages.\nClick to open...",
                                list.len()
                            ));

                            let reminder = reminder.on_click(|| {
                                webbrowser::open(&format!("{}/after-streams", web::URL))?;
                                Ok(())
                            });

                            system.notification(reminder);
                        }
                    }
                }
            }
        }
    }
}

/// Run the loop that handles installing this as a service.
#[tracing::instrument(skip_all)]
async fn system_loop(
    settings: settings::Settings<::auth::Scope>,
    system: sys::System,
) -> Result<()> {
    settings
        .set("run-on-startup", system.is_installed()?)
        .await?;

    let (mut run_on_startup_stream, _) = settings.stream("run-on-startup").or_with(false).await?;

    let build = move |run_on_startup: bool| match (run_on_startup, system.is_installed()?) {
        (true, true) | (false, false) => Ok(()),
        (true, false) => system.install(),
        (false, true) => system.uninstall(),
    };

    loop {
        let update = run_on_startup_stream.recv().await;
        build(update)?;
    }
}

#[derive(Clone)]
struct WebIntegration(web::Server);

impl oauth2::ConnectionIntegration for WebIntegration {
    fn clear_connection(&self, id: &str) {
        self.0.clear_connection(id);
    }

    fn update_connection(&self, id: &str, meta: oauth2::ConnectionIntegrationMeta) {
        self.0.update_connection(
            id,
            api::setbac::ConnectionMeta {
                id: meta.id,
                title: meta.title,
                description: meta.description,
                hash: meta.hash,
            },
        );
    }
}
