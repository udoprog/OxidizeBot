#![feature(async_await)]
#![recursion_limit = "128"]
#![windows_subsystem = "windows"]

use backoff::backoff::Backoff as _;
use failure::{bail, format_err, Error, ResultExt};
use parking_lot::RwLock;
use setmod::{
    api, auth, bus, config, db, injector, irc, module, oauth2, obs, player, prelude::*, secrets,
    settings, sys, updater, utils, web,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time,
};

fn opts() -> clap::App<'static, 'static> {
    clap::App::new("SetMod Bot")
        .version(setmod::VERSION)
        .author("John-John Tedro <udoprog@tedro.se>")
        .about("Bot component of SetMod.")
        .arg(
            clap::Arg::with_name("root")
                .short("r")
                .long("root")
                .value_name("root")
                .help("Directory to run from.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("file")
                .help("Configuration files to use.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("trace")
                .long("trace")
                .help("If we should enable tracing in logs."),
        )
        .arg(
            clap::Arg::with_name("web-root")
                .long("web-root")
                .value_name("dir")
                .help("Directory to use as web root.")
                .takes_value(true),
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

/// Setup a default logging configuration if none is specified.
fn default_log_config(log_file: &Path, trace: bool) -> Result<log4rs::config::Config, Error> {
    use log::LevelFilter;
    use log4rs::{
        append::{console::ConsoleAppender, file::FileAppender},
        config::{Appender, Config, Logger, Root},
    };

    let file = FileAppender::builder().build(log_file)?;
    let stdout = ConsoleAppender::builder().build();

    let mut level = LevelFilter::Info;

    if trace {
        level = LevelFilter::Trace;
    }

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .logger(
            Logger::builder()
                .appender("file")
                .additive(false)
                .build("setmod", level),
        )
        .build(Root::builder().appender("stdout").build(LevelFilter::Warn))?;

    Ok(config)
}

/// Configure logging.
fn setup_logs(
    root: &Path,
    log_config: Option<PathBuf>,
    default_log_file: &Path,
    trace: bool,
) -> Result<(), Error> {
    let file = log_config.unwrap_or_else(|| root.join("log4rs.yaml"));

    if !file.is_file() {
        let config = default_log_config(default_log_file, trace)?;
        log4rs::init_config(config)?;
    } else {
        log4rs::init_file(file, Default::default())?;
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    use std::thread;

    let opts = opts();
    let m = opts.get_matches();

    let root = match m.value_of("root") {
        Some(root) => PathBuf::from(root),
        None => dirs::config_dir()
            .ok_or_else(|| format_err!("no standard configuration directory available"))?
            .join("SetMod"),
    };

    let trace = m.is_present("trace");

    let config = m
        .value_of("config")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("config.toml"))
        .to_owned();

    let web_root = m.value_of("web-root").map(PathBuf::from);
    let log_config = m.value_of("log-config").map(PathBuf::from);
    let default_log_file = root.join("setmod.log");

    setup_logs(&root, log_config, &default_log_file, trace).context("failed to setup logs")?;

    if !root.is_dir() {
        log::info!("Creating SetMod directory: {}", root.display());
        std::fs::create_dir_all(&root)?;
    }

    let system = sys::setup(&root, &default_log_file)?;

    let mut error_backoff = backoff::ExponentialBackoff::default();
    error_backoff.current_interval = time::Duration::from_secs(30);
    error_backoff.initial_interval = time::Duration::from_secs(30);

    let mut current_backoff;
    let mut errored = false;

    if !m.is_present("silent") {
        let startup = sys::Notification::new(format!("Started SetMod {}", setmod::VERSION));
        system.notification(startup);
    }

    loop {
        if errored {
            system.clear();
            errored = false;
        }

        let mut runtime = tokio::runtime::Runtime::new()?;

        let result = runtime.block_on(
            try_main(
                system.clone(),
                root.clone(),
                web_root.clone(),
                config.clone(),
            )
            .boxed()
            .compat(),
        );

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
                setmod::log_err!(e, "Bot crashed");
            }
            Ok(()) => {
                error_backoff.reset();
                current_backoff = None;
                log::info!("Bot was shut down cleanly");
            }
        }

        if !system.is_running() {
            log::info!("Exiting...");
            break;
        }

        if let Some(current_backoff) = current_backoff.as_ref() {
            log::info!(
                "Restarting in {}...",
                utils::compact_duration(current_backoff)
            );
            thread::sleep(current_backoff.clone());
        }

        if !errored {
            let n = sys::Notification::new("Restarted bot").icon(sys::NotificationIcon::Warning);
            system.notification(n);
        }
    }

    system.join()?;
    Ok(())
}

async fn try_main(
    system: sys::System,
    root: PathBuf,
    web_root: Option<PathBuf>,
    config: PathBuf,
) -> Result<(), Error> {
    log::info!("Starting SetMod Version {}", setmod::VERSION);

    let thread_pool = Arc::new(tokio_threadpool::ThreadPool::new());

    let config: config::Config = if config.is_file() {
        log::info!("Loaded configuration from: {}", config.display());
        log::warn!("A configuration file is now optional, please consider removing it!");

        fs::read_to_string(&config)
            .map_err(Error::from)
            .and_then(|s| toml::de::from_str(&s).map_err(Error::from))
            .with_context(|_| format_err!("failed to read configuration: {}", config.display()))?
    } else {
        Default::default()
    };

    let config = Arc::new(config);

    let mut modules = Vec::<Box<dyn module::Module>>::new();

    let database_path = config
        .database_url
        .as_ref()
        .map(|url| url.to_path(&root))
        .unwrap_or_else(|| root.join("setmod.sql"));

    let db = db::Database::open(&database_path, Arc::clone(&thread_pool))
        .with_context(|_| format_err!("failed to open database at: {}", database_path.display()))?;

    let scopes_schema = auth::Schema::load_static()?;
    let auth = db.auth(scopes_schema)?;

    let settings_schema = settings::Schema::load_static()?;
    let settings = db.settings(settings_schema)?;

    settings
        .run_migrations()
        .context("failed to run settings migrations")?;

    let injector = injector::Injector::new();

    let bad_words = db::Words::load(db.clone())?;
    let after_streams = db::AfterStreams::load(db.clone())?;
    let commands = db::Commands::load(db.clone())?;
    let aliases = db::Aliases::load(db.clone())?;
    let promotions = db::Promotions::load(db.clone())?;
    let themes = db::Themes::load(db.clone())?;

    if !config.whitelisted_hosts.is_empty() {
        log::warn!("The `whitelisted_hosts` section in the configuration is now deprecated.");
    }

    if let Some(path) = config.bad_words.as_ref() {
        let path = path.to_path(&root);
        bad_words
            .load_from_path(&path)
            .with_context(|_| format_err!("failed to load bad words from: {}", path.display()))?;
    };

    let secrets_path = root.join("secrets.yml");

    let secrets = if secrets_path.is_file() {
        secrets::Secrets::open(&secrets_path)?
    } else {
        secrets::Secrets::empty()
    };

    if let Some(config) = secrets.load::<oauth2::Config>("spotify::oauth2")? {
        if !settings.has("secrets/oauth2/twitch/config")? {
            log::warn!(
                "migrating secret `spotify::oauth2` from {}",
                secrets_path.display()
            );
            settings.set("secrets/oauth2/spotify/config", config)?;
        }
    }

    if let Some(config) = secrets.load::<oauth2::Config>("twitch::oauth2")? {
        if !settings.has("secrets/oauth2/twitch/config")? {
            log::warn!(
                "migrating secret `twitch::oauth2` from {}",
                secrets_path.display()
            );
            settings.set("secrets/oauth2/twitch/config", config)?;
        }
    }

    let global_bus = Arc::new(bus::Bus::new());
    let youtube_bus = Arc::new(bus::Bus::new());
    let global_channel = Arc::new(RwLock::new(None));

    let mut futures = Vec::<future::BoxFuture<'_, Result<(), Error>>>::new();

    futures.push(system_loop(settings.scoped("system"), system.clone()).boxed());

    let cache = db::Cache::load(db.clone())?;
    futures.push(cache.clone().run().boxed());
    injector.update(cache);

    let (latest, future) = updater::run(&injector);
    futures.push(future.boxed());

    let currency = injector.var(&mut futures);

    let (web, future) = web::setup(
        web_root.as_ref().map(|p| p.as_path()),
        global_bus.clone(),
        youtube_bus.clone(),
        after_streams.clone(),
        db.clone(),
        settings.clone(),
        auth.clone(),
        aliases.clone(),
        commands.clone(),
        promotions.clone(),
        themes.clone(),
        global_channel.clone(),
        currency,
        latest.clone(),
    )?;

    futures.push(future.boxed());

    if settings.get::<bool>("first-run")?.unwrap_or(true) {
        log::info!("Opening {} for the first time", web::URL);

        if let Err(e) = webbrowser::open(web::URL) {
            log::error!("failed to open browser: {}", e);
        }

        settings.set("first-run", false)?;
    }

    log::info!("Listening on: {}", web::URL);

    let token_settings = settings.scoped("secrets/oauth2");

    let (spotify_token, future) = {
        let flow = config::new_oauth2_flow::<config::Spotify>(
            web.clone(),
            "spotify",
            "spotify",
            &token_settings,
        )?
        .with_scopes(vec![
            String::from("playlist-read-collaborative"),
            String::from("playlist-read-private"),
            String::from("user-library-read"),
            String::from("user-modify-playback-state"),
            String::from("user-read-playback-state"),
        ])
        .build(String::from("Spotify"))?;

        flow.into_token()?
    };

    futures.push(future.boxed());

    let (youtube_token, future) = {
        let flow = oauth2::youtube(web.clone(), token_settings.scoped("youtube"))?
            .with_scopes(vec![String::from(
                "https://www.googleapis.com/auth/youtube.readonly",
            )])
            .build(String::from("YouTube"))?;

        flow.into_token()?
    };

    futures.push(future.boxed());

    let (nightbot_token, future) = {
        let flow = oauth2::nightbot(web.clone(), token_settings.scoped("nightbot"))?
            .with_scopes(vec![String::from("channel_send")])
            .build(String::from("NightBot"))?;

        flow.into_token()?
    };

    futures.push(future.boxed());

    let (streamer_token, future) = {
        let flow = oauth2::twitch(web.clone(), token_settings.scoped("twitch-streamer"))?
            .with_scopes(vec![
                String::from("channel_editor"),
                String::from("channel_read"),
                String::from("channel:read:subscriptions"),
            ])
            .build(String::from("Twitch Streamer"))?;

        flow.into_token()?
    };

    futures.push(future.boxed());

    let (bot_token, future) = {
        let flow = oauth2::twitch(web.clone(), token_settings.scoped("twitch-bot"))?
            .with_scopes(vec![
                String::from("channel:moderate"),
                String::from("chat:edit"),
                String::from("chat:read"),
                String::from("clips:edit"),
            ])
            .build(String::from("Twitch Bot"))?;

        flow.into_token()?
    };

    futures.push(future.boxed());

    let (shutdown, shutdown_rx) = utils::Shutdown::new();

    let spotify = Arc::new(api::Spotify::new(spotify_token.clone())?);
    let streamer_twitch = api::Twitch::new(streamer_token.clone())?;
    let bot_twitch = api::Twitch::new(bot_token.clone())?;
    let youtube = Arc::new(api::YouTube::new(youtube_token.clone())?);
    let nightbot = Arc::new(api::NightBot::new(nightbot_token.clone())?);
    injector.update(api::Speedrun::new()?);

    let (player, future) = player::run(
        db.clone(),
        spotify.clone(),
        youtube.clone(),
        config.clone(),
        global_bus.clone(),
        youtube_bus.clone(),
        settings.clone(),
        themes.clone(),
    )?;

    futures.push(future.boxed());

    web.set_player(player.clone());

    // load the song module if we have a player configuration.
    injector.update(player);

    futures.push(
        api::setbac::run(
            &config,
            &settings,
            &injector,
            streamer_token.clone(),
            global_bus.clone(),
        )?
        .boxed(),
    );

    modules.push(Box::new(module::time::Module));
    modules.push(Box::new(module::song::Module));
    modules.push(Box::new(module::command_admin::Module));
    modules.push(Box::new(module::admin::Module::load()));
    modules.push(Box::new(module::alias_admin::Module::load()));
    modules.push(Box::new(module::theme_admin::Module::load()));
    modules.push(Box::new(module::promotions::Module::load(&config)));
    modules.push(Box::new(module::swearjar::Module::load(&config)));
    modules.push(Box::new(module::countdown::Module::load(&config)));
    modules.push(Box::new(module::gtav::Module::load(&config)));
    modules.push(Box::new(module::water::Module::load(&config)));
    modules.push(Box::new(module::misc::Module));
    modules.push(Box::new(module::after_stream::Module));
    modules.push(Box::new(module::clip::Module));
    modules.push(Box::new(module::eight_ball::Module));
    modules.push(Box::new(module::speedrun::Module));
    modules.push(Box::new(module::auth::Module));

    if config.obs.is_some() {
        log::warn!("`[obs]` setting has been deprecated from the configuration");
    }

    if config.features.is_some() {
        log::warn!("`features` setting has been deprecated from the configuration");
    }

    let future = obs::setup(&settings, &injector)?;
    futures.push(future.boxed());

    let irc = irc::Irc {
        db: db,
        youtube,
        nightbot,
        streamer_twitch,
        bot_twitch,
        config,
        token: bot_token,
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
        injector: injector.clone(),
    };

    futures.push(irc.run().boxed());

    let stuff = async move { future::try_join_all(futures).await.map_err(Some) };

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

    let result = future::try_join(stuff, shutdown_rx).await;

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
