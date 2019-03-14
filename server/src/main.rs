use failure::{format_err, ResultExt};
use futures::{future, Future};
use setmod_server::{
    commands, config::Config, counters, db, irc, secrets, spotify, twitch, web, words,
};
use std::{fs, path::Path, sync::Arc, thread};
use tokio_core::reactor::Core;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn opts() -> clap::App<'static, 'static> {
    clap::App::new("Batch Censor")
        .version(VERSION)
        .author("John-John Tedro <udoprog@tedro.se>")
        .about("Batch censors a bunch of audio files.")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("file")
                .help("Configuration files to use.")
                .takes_value(true),
        )
}

/// Configure logging.
fn setup_logs(root: &Path) -> Result<log4rs::Handle, failure::Error> {
    use log4rs::{
        append::{console::ConsoleAppender, file::FileAppender},
        config::{Appender, Config, Root},
        encode::pattern::PatternEncoder,
    };

    let stdout = ConsoleAppender::builder().build();

    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build(root.join("setmod.log"))?;

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .build(log::LevelFilter::Info),
        )?;

    Ok(log4rs::init_config(config)?)
}

fn main() -> Result<(), failure::Error> {
    let opts = opts();
    let m = opts.get_matches();

    let config = m
        .value_of("config")
        .map(Path::new)
        .unwrap_or(Path::new("config.yml"));

    let root = config
        .parent()
        .ok_or_else(|| format_err!("missing parent"))?;

    let handle = setup_logs(root).context("failed to setup logs")?;

    let thread_pool = Arc::new(tokio_threadpool::ThreadPool::new());

    let config: Config = if config.is_file() {
        fs::File::open(config)
            .map_err(failure::Error::from)
            .and_then(|f| serde_yaml::from_reader(f).map_err(Into::into))
            .with_context(|_| format_err!("failed to read configuration: {}", config.display()))?
    } else {
        Config::default()
    };

    let secrets_path = config.secrets.to_path(root);
    let secrets = secrets::Secrets::open(&secrets_path)
        .with_context(|_| format_err!("failed to load secrets: {}", secrets_path.display()))?;

    let database_url = config
        .database_url
        .as_ref()
        .map(|d| d.as_str())
        .ok_or_else(|| format_err!("require `database_url`"))?;

    let db = db::Database::open(database_url, Arc::clone(&thread_pool))?;

    let mut commands = commands::Commands::new(db.clone());
    commands.load_from_backend()?;

    let mut counters = counters::Counters::new(db.clone());
    counters.load_from_backend()?;

    let mut bad_words = words::Words::new(db.clone());
    bad_words.load_from_backend()?;

    if let Some(path) = config.bad_words.as_ref() {
        let path = path.to_path(root);
        bad_words.load_from_path(&path)?;
    };

    let notifier = Arc::new(setmod_notifier::Notifier::new());

    let mut core = Core::new()?;

    let mut futures = Vec::<Box<dyn Future<Item = (), Error = failure::Error>>>::new();

    let (web, future) = web::setup()?;

    // NB: spawn the web server on a separate thread because it's needed for the synchronous authentication flow below.
    // TODO: Make everything async (oauth2 currently is not).
    // TODO: Get rid of using threads below. Right now that is necessary to ensure all oauth2 requests are initiated in // the web server.
    let mut runtime = tokio::runtime::Runtime::new()?;
    runtime.spawn(future.map_err(|e| {
        log::error!("Error in web server: {}", e);
        ()
    }));

    log::info!("Listening on: {}", web::URL);

    let spotify_auth_thread = thread::spawn({
        let flow = config
            .spotify
            .new_flow_builder(web.clone(), "spotify", &root, &secrets)?
            .build()?;

        let thread_pool = Arc::clone(&thread_pool);
        move || flow.execute("Authorize Spotify", thread_pool)
    });

    let streamer_auth_thread = thread::spawn({
        let flow = config
            .twitch
            .new_flow_builder(web.clone(), "twitch-streamer", &root, &secrets)?
            .build()?;

        let thread_pool = Arc::clone(&thread_pool);
        move || flow.execute("Authorize as Streamer", thread_pool)
    });

    let irc = match config.irc.as_ref() {
        Some(irc_config) => {
            let auth_thread = thread::spawn({
                let flow = config
                    .twitch
                    .new_flow_builder(web.clone(), "twitch-bot", &root, &secrets)?
                    .build()?;

                let thread_pool = Arc::clone(&thread_pool);
                move || flow.execute("Authorize as Bot", thread_pool)
            });

            Some((irc_config, auth_thread))
        }
        None => None,
    };

    let (spotify_token, future) = spotify_auth_thread.join().expect("thread panicked")?;
    futures.push(Box::new(future));

    let (streamer_token, future) = streamer_auth_thread.join().expect("thread panicked")?;
    futures.push(Box::new(future));

    futures.push(Box::new(notifier.clone().listen()?));

    let spotify = Arc::new(spotify::Spotify::new(spotify_token.clone())?);
    let twitch = twitch::Twitch::new(streamer_token.clone())?;

    if let Some((irc_config, auth_thread)) = irc {
        let (bot_token, future) = auth_thread.join().expect("thread panicked")?;
        futures.push(Box::new(future));

        let future = irc::run(
            &mut core,
            db,
            twitch.clone(),
            spotify.clone(),
            config.streamer.as_ref().map(|s| s.as_str()),
            irc_config,
            bot_token,
            &commands,
            &counters,
            &bad_words,
            &*notifier,
            &secrets,
        )?;
        futures.push(Box::new(future));
    }

    let result = core.run(future::join_all(futures)).map(|_| ());
    drop(handle);
    result
}
