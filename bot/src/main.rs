use failure::{format_err, ResultExt};
use futures::{future, Future};
use setmod_bot::{
    bus, config::Config, db, features::Feature, irc, module, player, secrets, setbac, spotify,
    twitch, utils, web,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_core::reactor::Core;

fn opts() -> clap::App<'static, 'static> {
    clap::App::new("SetMod Bot")
        .version(setmod_bot::VERSION)
        .author("John-John Tedro <udoprog@tedro.se>")
        .about("Bot component of SetMod.")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("file")
                .help("Configuration files to use.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("web-root")
                .long("web-root")
                .value_name("dir")
                .help("Directory to use as web root.")
                .takes_value(true),
        )
}

/// Configure logging.
fn setup_logs(root: &Path) -> Result<(), failure::Error> {
    log4rs::init_file(root.join("log4rs.yaml"), Default::default())?;
    Ok(())
}

fn main() -> Result<(), failure::Error> {
    let opts = opts();
    let m = opts.get_matches();

    let config = m
        .value_of("config")
        .map(Path::new)
        .unwrap_or(Path::new("config.toml"));

    let root = config
        .parent()
        .ok_or_else(|| format_err!("missing parent"))?;

    let web_root = m
        .value_of("web-root")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("web"));

    setup_logs(root).context("failed to setup logs")?;

    match try_main(&root, &web_root, &config) {
        Err(e) => utils::log_err("bot crashed", e),
        Ok(()) => log::info!("bot was shut down"),
    }

    Ok(())
}

fn try_main(root: &Path, web_root: &Path, config: &Path) -> Result<(), failure::Error> {
    log::info!("Starting SetMod Version {}", setmod_bot::VERSION);

    let thread_pool = Arc::new(tokio_threadpool::ThreadPool::new());

    let config: Config = if config.is_file() {
        fs::read_to_string(config)
            .map_err(failure::Error::from)
            .and_then(|s| toml::de::from_str(&s).map_err(failure::Error::from))
            .with_context(|_| format_err!("failed to read configuration: {}", config.display()))?
    } else {
        Config::default()
    };

    let secrets_path = match config.secrets.as_ref() {
        Some(secrets) => secrets.to_path(root),
        None => root.join("secrets.yml"),
    };

    let mut modules = Vec::new();

    for module in &config.modules {
        modules.push(module.load(&config)?);
    }

    let secrets = secrets::Secrets::open(&secrets_path)
        .with_context(|_| format_err!("failed to load secrets: {}", secrets_path.display()))?;

    let database_url = config
        .database_url
        .as_ref()
        .map(|d| d.as_str())
        .ok_or_else(|| format_err!("require `database_url`"))?;

    let db = db::Database::open(database_url, Arc::clone(&thread_pool))?;

    let _domain_whitelist = db::PersistedSet::<_, String>::load(db.clone(), "whitelisted-domain")?;

    let commands = db::Commands::load(db.clone())?;
    let bad_words = db::Words::load(db.clone())?;

    if let Some(path) = config.bad_words.as_ref() {
        let path = path.to_path(root);
        bad_words
            .load_from_path(&path)
            .with_context(|_| format_err!("failed to load bad words from: {}", path.display()))?;
    };

    let global_bus = Arc::new(bus::Bus::new());

    let mut core = Core::new()?;

    let mut futures =
        Vec::<Box<dyn Future<Item = (), Error = failure::Error> + Send + 'static>>::new();

    let (web, future) = web::setup(web_root, global_bus.clone())?;

    // NB: spawn the web server on a separate thread because it's needed for the synchronous authentication flow below.
    core.runtime().executor().spawn(future.map_err(|e| {
        log::error!("Error in web server: {}", e);
        ()
    }));

    let settings = db.settings();

    if settings.get::<bool>("first-run")?.unwrap_or(true) {
        log::info!("Opening {} for the first time", web::URL);

        if let Err(e) = webbrowser::open(web::URL) {
            log::error!("failed to open browser: {}", e);
        }

        settings.set("first-run", false)?;
    }

    log::info!("Listening on: {}", web::URL);

    let mut tokens = vec![];

    tokens.push({
        let flow = config
            .spotify
            .new_flow_builder(web.clone(), "spotify", &root, &secrets)?
            .with_scopes(vec![
                String::from("playlist-read-collaborative"),
                String::from("playlist-read-private"),
                String::from("user-library-read"),
                String::from("user-modify-playback-state"),
                String::from("user-read-playback-state"),
            ])
            .build()?;

        flow.execute("Authorize Spotify")
    });

    tokens.push({
        let flow = config
            .twitch
            .new_flow_builder(web.clone(), "twitch-streamer", &root, &secrets)?
            .with_scopes(vec![
                String::from("channel_editor"),
                String::from("channel_read"),
            ])
            .build()?;

        flow.execute("Authorize as Streamer")
    });

    if config.irc.is_some() {
        let flow = config
            .twitch
            .new_flow_builder(web.clone(), "twitch-bot", &root, &secrets)?
            .with_scopes(vec![
                String::from("channel:moderate"),
                String::from("chat:edit"),
                String::from("chat:read"),
                String::from("clips:edit"),
            ])
            .build()?;

        tokens.push(flow.execute("Authorize as Bot"));
    };

    let results = core.run(future::join_all(tokens))?;

    let mut it = results.into_iter();

    let (spotify_token, future) = it
        .next()
        .ok_or_else(|| format_err!("expected spotify token"))?;
    futures.push(Box::new(future));

    let (streamer_token, future) = it
        .next()
        .ok_or_else(|| format_err!("expected streamer token"))?;
    futures.push(Box::new(future));

    futures.push(Box::new(global_bus.clone().listen()));

    let (shutdown, shutdown_rx) = utils::Shutdown::new();

    let spotify = Arc::new(spotify::Spotify::new(spotify_token.clone())?);
    let twitch = twitch::Twitch::new(streamer_token.clone())?;

    match config.player.as_ref() {
        // Only setup if the song feature is enabled.
        Some(player) if config.features.test(Feature::Song) => {
            let (future, player) = player::run(
                &mut core,
                db.clone(),
                spotify.clone(),
                &config,
                player,
                global_bus.clone(),
            )?;

            futures.push(Box::new(future));

            if let Some(api_url) = config.api_url.as_ref() {
                futures.push(Box::new(setbac::run_update(
                    api_url,
                    &player,
                    streamer_token.clone(),
                )?));
            }

            web.set_player(player.client());

            // load the song module if we have a player configuration.
            let module = module::song::Config::default();
            modules.push(Box::new(module::song::Module::load(&module, &player)?));

            Some(player)
        }
        _ => None,
    };

    if let Some(irc_config) = config.irc.as_ref() {
        let (bot_token, future) = it
            .next()
            .ok_or_else(|| format_err!("expected streamer token"))?;

        futures.push(Box::new(future));

        let future = irc::Irc {
            core: &mut core,
            db: db,
            streamer_twitch: twitch.clone(),
            bot_twitch: twitch::Twitch::new(bot_token.clone())?,
            config: &config,
            irc_config,
            token: bot_token,
            commands,
            bad_words,
            global_bus,
            modules: &modules,
            shutdown,
        }
        .run()?;

        futures.push(Box::new(future));
    }

    let stuff = future::join_all(futures).map_err(|e| Some(e));
    let shutdown_rx = shutdown_rx
        .map_err(|_| None)
        .and_then::<_, Result<(), Option<failure::Error>>>(|_| Err(None));

    let result = core.run(stuff.join(shutdown_rx).map(|_| ()));

    match result {
        Ok(()) => Ok(()),
        Err(Some(e)) => Err(e),
        // Shutting down cleanly.
        Err(None) => {
            log::info!("shutting down...");
            Ok(())
        }
    }
}
