#![recursion_limit = "256"]
#![type_length_limit="4194304"]
#![cfg_attr(feature = "windows", windows_subsystem = "windows")]
#![cfg_attr(backtrace, feature(backtrace))]

use anyhow::{anyhow, bail, Context, Result};
use backoff::backoff::Backoff as _;
use oxidize::api;
use oxidize::auth;
use oxidize::bus;
use oxidize::db;
use oxidize::injector;
use oxidize::irc;
use oxidize::message_log;
use oxidize::module;
use oxidize::oauth2;
use oxidize::player;
use oxidize::prelude::*;
use oxidize::settings;
use oxidize::storage;
use oxidize::stream_info;
use oxidize::sys;
use oxidize::tracing_utils;
use oxidize::updater;
use oxidize::utils;
use oxidize::web;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time;
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
fn tracing_config() -> Result<()> {
    tracing::subscriber::set_global_default(tracing_utils::Subscriber::new())?;
    Ok(())
}

/// Setup a default logging configuration if none is specified.
fn default_log_config(
    log_path: &Path,
    trace: bool,
    modules: &[&str],
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
) -> Result<()> {
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

#[derive(Debug, Clone, Copy)]
enum Intent {
    Shutdown,
    Restart,
}

fn main() -> Result<()> {
    let opts = opts();
    let m = opts.get_matches();

    let (old_root, root) = match m.value_of("root") {
        Some(root) => (None, PathBuf::from(root)),
        None => {
            let base = dirs::config_dir()
                .ok_or_else(|| anyhow!("no standard configuration directory available"))?;
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
    error_backoff.current_interval = time::Duration::from_secs(5);
    error_backoff.initial_interval = time::Duration::from_secs(5);
    error_backoff.max_elapsed_time = None;

    let is_silent = m.is_present("silent");

    if !is_silent {
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

    loop {
        let mut runtime = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()?;

        let future = {
            try_main(&system, &root, &db, &storage)
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
            if !is_silent {
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
                tokio::select! {
                    _ = system.wait_for_shutdown() => Intent::Shutdown,
                    _ = system.wait_for_restart() => Intent::Restart,
                    _ = tokio::signal::ctrl_c() => Intent::Shutdown,
                    _ = tokio::time::delay_for(backoff) => Intent::Restart,
                }
            });

            if let Intent::Shutdown = intent {
                break;
            }
        }

        if !is_silent {
            let n =
                sys::Notification::new("Restarted OxidizeBot").icon(sys::NotificationIcon::Warning);
            system.notification(n);
        }
    }

    if !is_silent {
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
    db: &db::Database,
    storage: &storage::Storage,
) -> Result<Intent> {
    log::info!("Starting Oxidize Bot Version {}", oxidize::VERSION);

    if !root.is_dir() {
        std::fs::create_dir_all(&root)
            .with_context(|| anyhow!("failed to create root: {}", root.display()))?;
    }

    let injector = injector::Injector::new();

    let mut modules = Vec::<Box<dyn module::Module>>::new();
    let mut futures = futures::stream::FuturesUnordered::new();

    injector.update(db.clone()).await;

    let scopes_schema = auth::Schema::load_static()?;
    let auth = db.auth(scopes_schema).await?;

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
        .await
        .context("failed to run settings migrations")?;

    injector.update(settings.clone()).await;

    let bad_words = db::Words::load(db.clone()).await?;

    injector
        .update(db::AfterStreams::load(db.clone()).await?)
        .await;
    injector.update(db::Commands::load(db.clone()).await?).await;
    injector.update(db::Aliases::load(db.clone()).await?).await;
    injector
        .update(db::Promotions::load(db.clone()).await?)
        .await;
    injector.update(db::Themes::load(db.clone()).await?).await;

    let message_bus = Arc::new(bus::Bus::new());
    let global_bus = Arc::new(bus::Bus::new());
    let youtube_bus = Arc::new(bus::Bus::new());
    let global_channel = injector::Var::new(None);
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

    injector.update(storage.cache()?).await;

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
        auth.clone(),
        global_channel.clone(),
        latest.clone(),
    )
    .await?;

    futures.push(
        future
            .map(|_| Err::<(), _>(anyhow!("web server exited unexpectedly")))
            .boxed()
            .instrument(trace_span!(target: "futures", "web")),
    );

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
        (_, nightbot_future),
        (_, streamer_future),
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
        api::open_weather_map::setup(settings.clone(), injector.clone())
            .await?
            .boxed()
            .instrument(trace_span!(target: "futures", "open-weather-map",)),
    );

    let (restart, internal_restart) = utils::Restart::new();

    let spotify = Arc::new(api::Spotify::new(spotify_token.clone())?);
    let youtube = Arc::new(api::YouTube::new(youtube_token.clone())?);
    injector.update(youtube.clone()).await;

    futures.push(
        api::NightBot::run(injector.clone())
            .boxed()
            .instrument(trace_span!(target: "futures", "nightbot-client")),
    );

    injector.update(api::Speedrun::new()?).await;

    let (player, future) = player::run(
        injector.clone(),
        db.clone(),
        spotify.clone(),
        youtube.clone(),
        global_bus.clone(),
        youtube_bus.clone(),
        settings.clone(),
    )
    .await?;

    futures.push(
        future
            .boxed()
            .instrument(trace_span!(target: "futures", "player",)),
    );

    web.set_player(player.clone()).await;

    // load the song module if we have a player configuration.
    injector.update(player).await;

    futures.push(
        api::setbac::run(&settings, &injector, global_bus.clone())
            .await?
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
    modules.push(Box::new(module::help::Module));

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
        restart,
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

    tokio::select! {
        result = futures.select_next_some() => {
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
        _ = internal_restart => {
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
    injector: &injector::Injector,
    mut rx: mpsc::Receiver<stream_info::StreamState>,
    system: sys::System,
) -> Result<()> {
    let (mut after_streams_stream, mut after_streams) = injector.stream::<db::AfterStreams>().await;

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

                        let list = after_streams.list().await?;

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
async fn system_loop(settings: settings::Settings, system: sys::System) -> Result<()> {
    settings
        .set("run-on-startup", system.is_installed()?)
        .await?;

    let (mut run_on_startup_stream, _) = settings.stream("run-on-startup").or_with(false).await?;

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
