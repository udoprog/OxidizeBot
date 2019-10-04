#![recursion_limit = "256"]
#![cfg_attr(feature = "windows", windows_subsystem = "windows")]

use backoff::backoff::Backoff as _;
use failure::{bail, format_err, Error, ResultExt};
use oxidize::{
    api, auth, bus, db, injector, irc, message_log, module, oauth2, player, prelude::*, settings,
    storage, stream_info, sys, tracing_utils, updater, utils, web,
};
use parking_lot::RwLock;
use std::{
    env,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time,
};
use tracing::trace_span;
use tracing_futures::Instrument as _;

const OLD_CONFIG_DIR: &'static str = "SetMod";
const CONFIG_DIR: &'static str = "OxidizeBot";
const FILE: &str = "file";
#[cfg(not(feature = "windows"))]
const STDOUT: &str = "stdout";
const PACKAGE: &str = env!("CARGO_PKG_NAME");

#[cfg(feature = "windows")]
mod internal {
    use super::FILE;
    use log4rs::config::{Config, ConfigBuilder, Logger, LoggerBuilder, Root, RootBuilder};

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
    use log4rs::config::{
        Appender, Config, ConfigBuilder, Logger, LoggerBuilder, Root, RootBuilder,
    };

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
        use log4rs::{append::console::ConsoleAppender, encode::pattern::PatternEncoder};

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

fn opts() -> clap::App<'static, 'static> {
    clap::App::new("Oxidize Bot")
        .version(oxidize::VERSION)
        .author("John-John Tedro <udoprog@tedro.se>")
        .about("Oxidize Twitch Bot.")
        .arg(
            clap::Arg::with_name("root")
                .long("root")
                .value_name("root")
                .help("Configuration directory to use.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("config")
                .long("config")
                .value_name("file")
                .help("Configuration file to use.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("trace")
                .long("trace")
                .help("If we should enable tracing in all logs."),
        )
        .arg(
            clap::Arg::with_name("log")
                .long("log")
                .takes_value(true)
                .multiple(true)
                .help("Additionally enable logging for the specified modules. Example: --log irc=trace"),
        )
        .arg(
            clap::Arg::with_name("log-config")
                .long("log-config")
                .value_name("file")
                .help("File to use for reading log configuration.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("silent")
                .long("silent")
                .help("Start without sending a notification."),
        )
}

/// Setup tracing.
fn tracing_config() -> Result<(), Error> {
    tracing::subscriber::set_global_default(tracing_utils::Subscriber::new())?;
    Ok(())
}

/// Setup a default logging configuration if none is specified.
fn default_log_config(
    log_path: &Path,
    trace: bool,
    modules: &Vec<&str>,
) -> Result<log4rs::config::Config, Error> {
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

    for module in modules.iter().copied() {
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
            _ => (LevelFilter::Info, module),
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
    modules: Vec<&str>,
) -> Result<(), Error> {
    let file = log_config.unwrap_or_else(|| root.join("log4rs.yaml"));

    if !file.is_file() {
        let config = default_log_config(default_log_file, trace, &modules)?;
        log4rs::init_config(config)?;
    } else {
        log4rs::init_file(file, Default::default())?;
    }

    tracing_config()?;
    Ok(())
}

/// Test if this is a watchdog process and return all arguments except for the
/// watchdog arguments.
fn is_watchdog() -> bool {
    if let Some(first) = env::args_os().skip(1).next() {
        return first == OsStr::new("--watchdog");
    }

    false
}

/// Run the underlying process being watchdogged.
fn run_watchdog() -> io::Result<()> {
    let exe = env::current_exe()?;
    let args = env::args_os().skip(2).collect::<Vec<_>>();

    println!("Running watchdog (pid: {})", process::id());

    loop {
        let mut child = process::Command::new(&exe).args(&args).spawn()?;
        println!("Started child process (pid: {})", child.id());
        let status = child.wait()?;

        // gracefully shut down.
        if status.success() {
            return Ok(());
        }

        println!("Child process exited unexpectedly: {}", status);
    }
}

fn main() -> Result<(), Error> {
    if is_watchdog() {
        if let Err(e) = run_watchdog() {
            eprintln!("Watched process failed: {}", e);
        }

        return Ok(());
    }

    let opts = opts();
    let m = opts.get_matches();

    let (old_root, root) = match m.value_of("root") {
        Some(root) => (None, PathBuf::from(root)),
        None => {
            let base = dirs::config_dir()
                .ok_or_else(|| format_err!("no standard configuration directory available"))?;
            let old = base.join(OLD_CONFIG_DIR);
            let new = base.join(CONFIG_DIR);
            (Some(old), new)
        }
    };

    let trace = m.is_present("trace");

    let log_modules = match m.values_of("log") {
        Some(modules) => modules.collect(),
        None => vec![],
    };

    let log_config = m.value_of("log-config").map(PathBuf::from);
    let default_log_file = root.join("oxidize.log");

    setup_logs(&root, log_config, &default_log_file, trace, log_modules)
        .context("failed to setup logs")?;

    oxidize::panic_logger();

    if !root.is_dir() {
        log::info!("Creating config directory: {}", root.display());
        std::fs::create_dir_all(&root)?;
    }

    let system = sys::setup(&root, &default_log_file)?;

    let mut error_backoff = backoff::ExponentialBackoff::default();
    error_backoff.current_interval = time::Duration::from_secs(30);
    error_backoff.initial_interval = time::Duration::from_secs(30);

    let mut current_backoff;
    let mut errored = false;

    if !m.is_present("silent") {
        let startup = sys::Notification::new(format!("Started Oxidize {}", oxidize::VERSION));
        system.notification(startup);
    }

    loop {
        if !system.is_running() {
            break;
        }

        if errored {
            system.clear();
            errored = false;
        }

        let future = {
            let system = system.clone();
            let old_root = old_root.clone();
            let root = root.clone();

            try_main(system, old_root, root).instrument(trace_span!(target: "futures", "main",))
        };

        let runtime = tokio::runtime::Runtime::new()?;
        let result = runtime.block_on(future);

        match result {
            Err(e) => {
                let backoff = error_backoff.next_backoff().unwrap_or_default();
                current_backoff = Some(backoff);
                errored = true;

                let message = format!(
                    "Trying to restart in {}.\nSee log for more details.",
                    utils::compact_duration(&backoff)
                );

                let n = sys::Notification::new(message)
                    .title("Bot crashed!")
                    .icon(sys::NotificationIcon::Error);

                system.notification(n);
                system.error(String::from("Bot crashed, see log for more details."));
                oxidize::log_err!(e, "Bot crashed");
            }
            Ok(()) => {
                error_backoff.reset();
                current_backoff = None;
                log::info!("Bot was shut down cleanly");
            }
        }

        if !system.is_running() {
            break;
        }

        if let Some(current_backoff) = current_backoff.as_ref() {
            log::info!(
                "Restarting in {}...",
                utils::compact_duration(current_backoff)
            );

            let system = system.clone();

            let system_interrupt = async move {
                future::select(
                    system.wait_for_shutdown().boxed(),
                    system.wait_for_restart().boxed(),
                )
                .await;
            };

            let delay = tokio::timer::delay(time::Instant::now() + *current_backoff);

            let _ = runtime.block_on(future::select(system_interrupt.boxed(), delay));
        }

        if !errored {
            let n = sys::Notification::new("Restarted bot").icon(sys::NotificationIcon::Warning);
            system.notification(n);
        }
    }

    log::info!("Exiting...");
    system.join()?;
    Ok(())
}

/// Actual main function, running the application loop.
async fn try_main(
    system: sys::System,
    old_root: Option<PathBuf>,
    root: PathBuf,
) -> Result<(), Error> {
    log::info!("Starting Oxidize Bot Version {}", oxidize::VERSION);

    if !root.is_dir() {
        std::fs::create_dir_all(&root)
            .with_context(|_| format_err!("failed to create root: {}", root.display()))?;
    }

    let injector = injector::Injector::new();

    let mut modules = Vec::<Box<dyn module::Module>>::new();
    let mut futures = futures::stream::FuturesUnordered::new();

    let database_path = {
        let new = root.join("oxidize.sql");

        if let Some(old) = old_root {
            let old = old.join("setmod.sql");

            if old.is_file() && !new.is_file() {
                std::fs::copy(&old, &new).with_context(|_| {
                    format_err!(
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
        .with_context(|_| format_err!("failed to open database at: {}", database_path.display()))?;
    injector.update(db.clone());

    let scopes_schema = auth::Schema::load_static()?;
    let auth = db.auth(scopes_schema)?;

    let settings_schema = settings::Schema::load_static()?;
    let settings = db.settings(settings_schema)?;

    futures.push(
        settings
            .clone()
            .drive()
            .map_err(Into::into)
            .boxed()
            .instrument(trace_span!(target: "futures", "settings-driver",)),
    );

    settings
        .run_migrations()
        .context("failed to run settings migrations")?;

    injector.update(settings.clone());

    let bad_words = db::Words::load(db.clone())?;

    injector.update(db::AfterStreams::load(db.clone())?);
    injector.update(db::Commands::load(db.clone())?);
    injector.update(db::Aliases::load(db.clone())?);
    injector.update(db::Promotions::load(db.clone())?);
    injector.update(db::Themes::load(db.clone())?);

    let message_bus = Arc::new(bus::Bus::new());
    let global_bus = Arc::new(bus::Bus::new());
    let youtube_bus = Arc::new(bus::Bus::new());
    let global_channel = Arc::new(RwLock::new(None));
    let command_bus = Arc::new(bus::Bus::new());

    futures.push(
        injector
            .clone()
            .drive()
            .map_err(Into::into)
            .boxed()
            .instrument(trace_span!(target: "futures", "injector-driver",)),
    );
    futures.push(
        system_loop(settings.scoped("system"), system.clone())
            .boxed()
            .instrument(trace_span!(target: "futures", "system-loop",)),
    );

    let storage = storage::Storage::open(&root.join("storage"))?;
    injector.update(storage.cache()?);

    let (latest, future) = updater::run(&injector);
    futures.push(
        future
            .boxed()
            .instrument(trace_span!(target: "futures", "remote-updates",)),
    );

    let message_log = message_log::MessageLog::builder()
        .bus(message_bus.clone())
        .limit(512)
        .build();

    let (web, future) = web::setup(
        &injector,
        message_log.clone(),
        message_bus.clone(),
        global_bus.clone(),
        youtube_bus.clone(),
        command_bus.clone(),
        db.clone(),
        auth.clone(),
        global_channel.clone(),
        latest.clone(),
    )
    .await?;

    futures.push(
        future
            .map(|_| Err::<(), _>(format_err!("web server exited unexpectedly")))
            .boxed()
            .instrument(trace_span!(target: "futures", "web")),
    );

    if settings.get::<bool>("first-run")?.unwrap_or(true) {
        log::info!("Opening {} for the first time", web::URL);

        if let Err(e) = webbrowser::open(web::URL) {
            log::error!("failed to open browser: {}", e);
        }

        settings.set("first-run", false)?;
    }

    log::info!("Listening on: {}", web::URL);

    let token_settings = settings.scoped("secrets/oauth2");

    let spotify_setup = {
        let s = token_settings.scoped("spotify");
        let key = injector::Key::tagged(oauth2::TokenId::Spotify)?;
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
        let key = injector::Key::tagged(oauth2::TokenId::YouTube)?;
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
        let key = injector::Key::tagged(oauth2::TokenId::NightBot)?;
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
        let key = injector::Key::tagged(oauth2::TokenId::TwitchStreamer)?;
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
        let key = injector::Key::tagged(oauth2::TokenId::TwitchBot)?;
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
        (nightbot_token, nightbot_future),
        (streamer_token, streamer_future),
        (_, bot_future),
    ) = futures::try_join!(
        spotify_setup,
        youtube_setup,
        nightbot_setup,
        streamer_setup,
        bot_setup
    )?;

    futures.push(
        spotify_future
            .boxed()
            .instrument(trace_span!(target: "futures", "spotify-token",)),
    );

    futures.push(
        youtube_future
            .boxed()
            .instrument(trace_span!(target: "futures", "youtube-token",)),
    );

    futures.push(
        nightbot_future
            .boxed()
            .instrument(trace_span!(target: "futures", "nightbot-token",)),
    );

    futures.push(
        streamer_future
            .boxed()
            .instrument(trace_span!(target: "futures", "streamer-token",)),
    );

    futures.push(
        bot_future
            .boxed()
            .instrument(trace_span!(target: "futures", "bot-token",)),
    );

    futures.push(
        api::open_weather_map::setup(settings.clone(), injector.clone())?
            .boxed()
            .instrument(trace_span!(target: "futures", "open-weather-map",)),
    );

    let (shutdown, shutdown_rx) = utils::Shutdown::new();

    let spotify = Arc::new(api::Spotify::new(spotify_token.clone())?);
    let youtube = Arc::new(api::YouTube::new(youtube_token.clone())?);
    injector.update(youtube.clone());

    let nightbot = Arc::new(api::NightBot::new(nightbot_token.clone())?);

    injector.update(nightbot.clone());
    injector.update(api::Speedrun::new()?);

    let (player, future) = player::run(
        &injector,
        db.clone(),
        spotify.clone(),
        youtube.clone(),
        global_bus.clone(),
        youtube_bus.clone(),
        settings.clone(),
    )?;

    futures.push(
        future
            .boxed()
            .instrument(trace_span!(target: "futures", "player",)),
    );

    web.set_player(player.clone());

    // load the song module if we have a player configuration.
    injector.update(player);

    futures.push(
        api::setbac::run(
            &settings,
            &injector,
            streamer_token.clone(),
            global_bus.clone(),
        )?
        .boxed()
        .instrument(trace_span!(target: "futures", "setbac.tv",)),
    );

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

    let (stream_state_tx, stream_state_rx) = mpsc::channel(64);

    let notify_after_streams = notify_after_streams(&injector, stream_state_rx, system.clone());
    futures.push(
        notify_after_streams
            .boxed()
            .instrument(trace_span!(target: "futures", "notify-after-streams",)),
    );

    let irc = irc::Irc {
        bad_words,
        global_bus,
        command_bus,
        modules,
        shutdown,
        settings,
        auth,
        global_channel,
        injector: injector.clone(),
        stream_state_tx,
        message_log,
    };

    futures.push(
        irc.run()
            .boxed()
            .instrument(trace_span!(target: "futures", "irc",)),
    );

    let stuff = async move { futures.select_next_some().await.map_err(Some) };

    let system_shutdown = system.wait_for_shutdown();
    let system_restart = system.wait_for_restart();

    let shutdown_rx = async move {
        let futures = vec![
            system_shutdown.boxed(),
            system_restart.boxed(),
            shutdown_rx.boxed(),
        ];
        let _ = future::select_all(futures).await;
        Err::<(), _>(None::<Error>)
    };

    let result = future::try_join(
        stuff.instrument(trace_span!(target: "futures", "core")),
        shutdown_rx.instrument(trace_span!(target: "futures", "shutdown")),
    )
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(Some(e)) => Err(e),
        // Shutting down cleanly.
        Err(None) => {
            log::info!("Shutting down...");
            Ok(())
        }
    }
}

/// Notify if there are any after streams.
///
/// If this is clicked, open the after-streams page.
async fn notify_after_streams(
    injector: &injector::Injector,
    mut rx: mpsc::Receiver<stream_info::StreamState>,
    system: sys::System,
) -> Result<(), Error> {
    let (mut after_streams_stream, mut after_streams) = injector.stream::<db::AfterStreams>();

    loop {
        futures::select! {
            update = after_streams_stream.select_next_some() => {
                after_streams = update;
            }
            update = rx.select_next_some() => {
                match update {
                    stream_info::StreamState::Started => {
                        log::info!("Stream started");
                    }
                    stream_info::StreamState::Stopped => {
                        let after_streams = match after_streams.as_ref() {
                            Some(after_streams) => after_streams,
                            None => continue,
                        };

                        let list = after_streams.list()?;

                        if list.len() > 0 {
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
async fn system_loop(settings: settings::Settings, system: sys::System) -> Result<(), Error> {
    settings.set("run-on-startup", system.is_installed()?)?;

    let (mut run_on_startup_stream, _) = settings.stream("run-on-startup").or_with(false)?;

    let build = move |run_on_startup: bool| match (run_on_startup, system.is_installed()?) {
        (true, true) | (false, false) => Ok(()),
        (true, false) => system.install(),
        (false, true) => system.uninstall(),
    };

    while let Some(update) = run_on_startup_stream.next().await {
        build(update)?;
    }

    bail!("run-on-startup stream ended");
}
