#![recursion_limit = "256"]
#![type_length_limit = "4194304"]
#![cfg_attr(feature = "windows", windows_subsystem = "windows")]
#![cfg_attr(backtrace, feature(backtrace))]

use anyhow::{anyhow, bail, Context, Result};
use backoff::backoff::Backoff as _;
use oxidize::api;
use oxidize::auth;
use oxidize::bus;
use oxidize::db;
use oxidize::injector::{Injector, Key};
use oxidize::irc;
use oxidize::message_log;
use oxidize::module;
use oxidize::oauth2;
use oxidize::player;
use oxidize::storage;
use oxidize::stream_info;
use oxidize::sys;
use oxidize::tags;
use oxidize::tracing_utils;
use oxidize::updater;
use oxidize::utils;
use oxidize::web;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time;
use tokio::sync::mpsc;
use tokio_stream::StreamExt as _;
use tracing::trace_span;
use tracing_futures::Instrument as _;

const OLD_CONFIG_DIR: &str = "SetMod";
const CONFIG_DIR: &str = "OxidizeBot";
const FILE: &str = "file";
#[cfg(not(feature = "windows"))]
const STDOUT: &str = "stdout";
const PACKAGE: &str = env!("CARGO_PKG_NAME");

#[cfg(feature = "windows")]
mod internal {
    use super::FILE;
    use log4rs::config::runtime::{ConfigBuilder, LoggerBuilder, RootBuilder};
    use log4rs::config::{Config, Logger, Root};

    pub(crate) fn logger_builder() -> LoggerBuilder {
        Logger::builder().appender(FILE).additive(false)
    }

    pub(crate) fn root_builder() -> RootBuilder {
        Root::builder()
    }

    pub(crate) fn config_builder() -> ConfigBuilder {
        Config::builder()
    }
}

#[cfg(not(feature = "windows"))]
mod internal {
    use super::{FILE, STDOUT};
    use log4rs::config::runtime::{ConfigBuilder, LoggerBuilder, RootBuilder};
    use log4rs::config::{Appender, Config, Logger, Root};

    pub(crate) fn logger_builder() -> LoggerBuilder {
        Logger::builder()
            .appender(STDOUT)
            .appender(FILE)
            .additive(false)
    }

    pub(crate) fn root_builder() -> RootBuilder {
        Root::builder().appender(STDOUT)
    }

    pub(crate) fn config_builder() -> ConfigBuilder {
        use log4rs::append::console::ConsoleAppender;
        use log4rs::encode::pattern::PatternEncoder;

        let pattern =
            PatternEncoder::new("{d(%Y-%m-%dT%H:%M:%S%.3f%Z)} {highlight({l:5.5})} {t} - {m}{n}");

        Config::builder().appender(
            Appender::builder().build(
                STDOUT,
                Box::new(
                    ConsoleAppender::builder()
                        .encoder(Box::new(pattern))
                        .build(),
                ),
            ),
        )
    }
}

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
        log_config: Option<PathBuf>,
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
    /// File to use for reading log configuration.
    ["--log-config", #[os] path] => {
        log_config = Some(PathBuf::from(path));
    }
    /// Configure a different stack size to use.
    ["--stack-size", size] => {
        stack_size = Some(str::parse(&size)?);
    }
}

/// Setup tracing.
fn tracing_config() -> Result<()> {
    tracing::subscriber::set_global_default(tracing_utils::Subscriber::new())?;
    Ok(())
}

/// Setup a default logging configuration if none is specified.
fn default_log_config(
    log_path: &Path,
    trace: bool,
    modules: &[String],
) -> Result<log4rs::config::Config> {
    use self::internal::{config_builder, logger_builder, root_builder};
    use log::LevelFilter;
    use log4rs::{append::file::FileAppender, config::Appender, encode::pattern::PatternEncoder};

    let pattern = PatternEncoder::new("{d(%Y-%m-%dT%H:%M:%S%.3f%Z)} {l:5.5} {t} - {m}{n}");

    let mut config = config_builder().appender(
        Appender::builder().build(
            FILE,
            Box::new(
                FileAppender::builder()
                    .encoder(Box::new(pattern))
                    .build(log_path)?,
            ),
        ),
    );

    // special case: trace everything
    if trace {
        return Ok(config.build(root_builder().build(LevelFilter::Trace))?);
    }

    let mut panic_configured = false;
    let mut package_configured = false;

    for module in modules {
        let (level, module) = match module.find('=').map(|i| module.split_at(i)) {
            Some((module, level)) => {
                let level = match &level[1..] {
                    "error" => LevelFilter::Error,
                    "warn" => LevelFilter::Warn,
                    "info" => LevelFilter::Info,
                    "debug" => LevelFilter::Debug,
                    "trace" => LevelFilter::Trace,
                    other => bail!("invalid log level: {}", other),
                };

                (level, module)
            }
            _ => (LevelFilter::Info, module.as_str()),
        };

        if module == "panic" {
            panic_configured = true;
        }

        if module == PACKAGE {
            package_configured = true;
        }

        config = config.logger(logger_builder().build(module, level));
    }

    if !package_configured {
        config = config.logger(logger_builder().build(PACKAGE, LevelFilter::Info));
    }

    // make sure panic logger is configured.
    if !panic_configured {
        config = config.logger(logger_builder().build("panic", LevelFilter::Info));
    }

    Ok(config.build(root_builder().build(LevelFilter::Off))?)
}

/// Configure logging.
fn setup_logs(
    root: &Path,
    log_config: Option<PathBuf>,
    default_log_file: &Path,
    trace: bool,
    modules: &[String],
) -> Result<()> {
    let file = log_config.unwrap_or_else(|| root.join("log4rs.yaml"));

    if !file.is_file() {
        let config = default_log_config(default_log_file, trace, modules)?;
        log4rs::init_config(config)?;
    } else {
        log4rs::init_file(file, Default::default())?;
    }

    tracing_config()?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum Intent {
    Shutdown,
    Restart,
}

fn main() -> Result<()> {
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

    let default_log_file = root.join("oxidize.log");

    setup_logs(
        &root,
        args.log_config,
        &default_log_file,
        args.trace,
        &args.log,
    )
    .context("failed to setup logs")?;

    oxidize::panic_logger();

    if !root.is_dir() {
        log::info!("Creating config directory: {}", root.display());
        std::fs::create_dir_all(&root)?;
    }

    let system = sys::setup(&root, &default_log_file)?;

    let mut error_backoff = backoff::ExponentialBackoff::default();
    error_backoff.current_interval = time::Duration::from_secs(5);
    error_backoff.initial_interval = time::Duration::from_secs(5);
    error_backoff.max_elapsed_time = None;

    if !args.silent {
        let startup = sys::Notification::new(format!("Started Oxidize {}", oxidize::VERSION));
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

    let mut script_dirs = Vec::new();
    script_dirs.push(root.join("scripts"));
    script_dirs.push(PathBuf::from("scripts"));

    loop {
        let runtime = {
            let mut runtime = tokio::runtime::Builder::new_multi_thread();
            runtime.enable_all();

            if let Some(size) = args.stack_size {
                runtime.thread_stack_size(size);
            }

            runtime.build()?
        };

        let future = {
            try_main(&system, &root, &script_dirs, &db, &storage)
                .instrument(trace_span!(target: "futures", "main",))
        };

        system.clear();

        let backoff = match runtime.block_on(future) {
            Err(e) => {
                let backoff = error_backoff.next_backoff().unwrap_or_default();
                system.error(String::from("Bot crashed, see log for more details."));
                oxidize::log_error!(e, "Bot crashed");
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
                    utils::compact_duration(backoff)
                );

                let n = sys::Notification::new(message)
                    .title("Bot Crashed!")
                    .icon(sys::NotificationIcon::Error);

                system.notification(n);
            }

            log::info!("Restarting in {}...", utils::compact_duration(backoff));

            let intent = runtime.block_on(async {
                let wait_for_shutdown = system.wait_for_shutdown();
                tokio::pin!(wait_for_shutdown);

                let wait_for_restart = system.wait_for_restart();
                tokio::pin!(wait_for_restart);

                tokio::select! {
                    _ = wait_for_shutdown => Intent::Shutdown,
                    _ = wait_for_restart => Intent::Restart,
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

    log::info!("exiting...");
    system.join()?;
    Ok(())
}

/// Actual main function, running the application loop.
async fn try_main(
    system: &sys::System,
    root: &Path,
    script_dirs: &Vec<PathBuf>,
    db: &db::Database,
    storage: &storage::Storage,
) -> Result<Intent> {
    log::info!("Starting Oxidize Bot Version {}", oxidize::VERSION);

    if !root.is_dir() {
        std::fs::create_dir_all(&root)
            .with_context(|| anyhow!("failed to create root: {}", root.display()))?;
    }

    let injector = Injector::new();

    let mut modules = Vec::<Box<dyn module::Module>>::new();
    let mut futures = oxidize::utils::Futures::new();

    injector.update(db.clone()).await;

    let scopes_schema = auth::Schema::load_static()?;
    let auth = db.auth(scopes_schema).await?;
    injector.update(auth.clone()).await;

    let settings_schema = oxidize::load_schema()?;
    let settings = db.settings(settings_schema)?;

    let drive = settings.clone();

    futures.push(Box::pin(
        async { drive.drive().await.map_err(Into::into) }
            .instrument(trace_span!(target: "futures", "settings-driver")),
    ));

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

    futures.push(Box::pin(
        system_loop(settings.scoped("system"), system.clone())
            .instrument(trace_span!(target: "futures", "system-loop",)),
    ));

    injector.update(storage.cache()?).await;

    let (latest, future) = updater::run(&injector);
    futures.push(Box::pin(
        future.instrument(trace_span!(target: "futures", "remote-updates",)),
    ));

    let message_log = message_log::MessageLog::builder()
        .bus(message_bus.clone())
        .limit(512)
        .build();
    injector.update(message_log.clone()).await;

    let (web, future) = web::setup(
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

    futures.push(Box::pin(
        async {
            future.await;
            Err::<(), _>(anyhow!("web server exited unexpectedly"))
        }
        .instrument(trace_span!(target: "futures", "web")),
    ));

    if settings.get::<bool>("first-run").await?.unwrap_or(true) {
        log::info!("Opening {} for the first time", web::URL);

        if let Err(e) = webbrowser::open(web::URL) {
            log::error!("failed to open browser: {}", e);
        }

        settings.set("first-run", false).await?;
    }

    log::info!("Listening on: {}", web::URL);

    let token_settings = settings.scoped("secrets/oauth2");

    let spotify_setup = {
        let s = token_settings.scoped("spotify");
        let key = Key::tagged(tags::Token::Spotify)?;
        oauth2::build(
            "spotify",
            "Spotify",
            &settings,
            s,
            injector.clone(),
            key,
            web.clone(),
        )
    };

    let youtube_setup = {
        let s = token_settings.scoped("youtube");
        let key = Key::tagged(tags::Token::YouTube)?;
        oauth2::build(
            "youtube",
            "YouTube",
            &settings,
            s,
            injector.clone(),
            key,
            web.clone(),
        )
    };

    let nightbot_setup = {
        let s = token_settings.scoped("nightbot");
        let key = Key::tagged(tags::Token::NightBot)?;
        oauth2::build(
            "nightbot",
            "NightBot",
            &settings,
            s,
            injector.clone(),
            key,
            web.clone(),
        )
    };

    let streamer_setup = {
        let s = token_settings.scoped("twitch-streamer");
        let key = Key::tagged(tags::Token::Twitch(tags::Twitch::Streamer))?;
        oauth2::build(
            "twitch-streamer",
            "Twitch Streamer",
            &settings,
            s,
            injector.clone(),
            key,
            web.clone(),
        )
    };

    let bot_setup = {
        let s = token_settings.scoped("twitch-bot");
        let key = Key::tagged(tags::Token::Twitch(tags::Twitch::Bot))?;
        oauth2::build(
            "twitch-bot",
            "Twitch Bot",
            &settings,
            s,
            injector.clone(),
            key,
            web.clone(),
        )
    };

    let (
        (spotify_token, spotify_future),
        (youtube_token, youtube_future),
        (_, nightbot_future),
        (_, streamer_future),
        (_, bot_future),
    ) = tokio::try_join!(
        spotify_setup,
        youtube_setup,
        nightbot_setup,
        streamer_setup,
        bot_setup
    )?;

    futures.push(Box::pin(api::twitch::pubsub::connect(&settings, &injector)));
    futures.push(Box::pin(api::twitch_clients_task(injector.clone())));

    futures.push(Box::pin(
        spotify_future.instrument(trace_span!(target: "futures", "spotify-token",)),
    ));

    futures.push(Box::pin(
        youtube_future.instrument(trace_span!(target: "futures", "youtube-token",)),
    ));

    futures.push(Box::pin(
        nightbot_future.instrument(trace_span!(target: "futures", "nightbot-token",)),
    ));

    futures.push(Box::pin(
        streamer_future.instrument(trace_span!(target: "futures", "streamer-token",)),
    ));

    futures.push(Box::pin(
        bot_future.instrument(trace_span!(target: "futures", "bot-token",)),
    ));

    futures.push(Box::pin(
        api::open_weather_map::setup(settings.clone(), injector.clone())
            .await?
            .instrument(trace_span!(target: "futures", "open-weather-map",)),
    ));

    let (restart, restart_rx) = utils::Restart::new();
    injector.update(restart).await;

    let spotify = Arc::new(api::Spotify::new(spotify_token.clone())?);
    let youtube = Arc::new(api::YouTube::new(youtube_token.clone())?);
    injector.update(youtube.clone()).await;

    futures
        .push(Box::pin(api::NightBot::run(injector.clone()).instrument(
            trace_span!(target: "futures", "nightbot-client"),
        )));

    injector.update(api::Speedrun::new()?).await;

    let future = player::run(
        &injector,
        db.clone(),
        spotify.clone(),
        youtube.clone(),
        global_bus.clone(),
        youtube_bus.clone(),
        settings.clone(),
    )
    .await?;

    futures.push(Box::pin(
        future.instrument(trace_span!(target: "futures", "player",)),
    ));

    futures.push(Box::pin(
        api::setbac::run(&settings, &injector, global_bus.clone())
            .await?
            .instrument(trace_span!(target: "futures", "setbac.tv",)),
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
    futures.push(Box::pin(
        notify_after_streams.instrument(trace_span!(target: "futures", "notify-after-streams",)),
    ));

    let irc = irc::Irc {
        modules,
        injector: injector.clone(),
        stream_state_tx,
        script_dirs: script_dirs.clone(),
    };

    futures.push(Box::pin(
        irc.run().instrument(trace_span!(target: "futures", "irc",)),
    ));

    tokio::select! {
        Some(result) = futures.next() => {
            result.map(|_| Intent::Shutdown)
        }
        _ = system.wait_for_shutdown() => {
            log::info!("shutdown triggered by system");
            Ok(Intent::Shutdown)
        },
        _ = system.wait_for_restart() => {
            log::info!("restart triggered by system");
            Ok(Intent::Restart)
        },
        _ = restart_rx => {
            log::info!("restart triggered by bot");
            Ok(Intent::Restart)
        },
        _ = tokio::signal::ctrl_c() => {
            log::info!("shutdown triggered by signal");
            Ok(Intent::Shutdown)
        },
    }
}

/// Notify if there are any after streams.
///
/// If this is clicked, open the after-streams page.
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
                        log::info!("Stream started");
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
async fn system_loop(settings: oxidize::Settings, system: sys::System) -> Result<()> {
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
