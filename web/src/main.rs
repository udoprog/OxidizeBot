use anyhow::{bail, Error};
use futures::prelude::*;
use oxidize_web::{api, db, web};
use std::{fs, path::Path, sync::Arc, time};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn opts() -> clap::App<'static, 'static> {
    clap::App::new("Oxidize Web")
        .version(VERSION)
        .author("John-John Tedro <udoprog@tedro.se>")
        .about("Web Components of Oxidize")
        .arg(
            clap::Arg::with_name("config")
                .takes_value(true)
                .long("config")
                .help("Configuration file to use."),
        )
        .arg(
            clap::Arg::with_name("port")
                .takes_value(true)
                .long("port")
                .help("Port to bind to."),
        )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let opts = opts();
    let m = opts.get_matches();

    let config_path = match m.value_of("config") {
        Some(config_path) => Path::new(config_path),
        None => Path::new("config.toml"),
    };

    let root = match config_path.parent() {
        Some(root) => root.to_owned(),
        None => std::env::current_dir().expect("process to have a current directory"),
    };

    let host = match m.value_of("host") {
        Some(host) => host.to_string(),
        None => "127.0.0.1".to_string(),
    };

    let port = match m.value_of("port") {
        Some(port) => str::parse(port)?,
        None => 8000,
    };

    log::info!("Loading config: {}", config_path.display());
    let config = toml::from_str::<web::Config>(&fs::read_to_string(config_path)?)?;

    let db = db::Database::load(Arc::new(
        sled::Db::open(config.database.to_path(root))?.open_tree("storage")?,
    ))?;

    let github = api::GitHub::new()?;

    let mut releases_interval = tokio::time::interval(time::Duration::from_secs(60 * 10)).fuse();

    let web = web::setup(db.clone(), host, port, config)?;
    pin_utils::pin_mut!(web);
    let mut web = web.fuse();

    #[allow(clippy::unnecessary_mut_passed)]
    loop {
        futures::select! {
            _ = web => {
                bail!("web future ended unexpectedly");
            }
            _ = releases_interval.select_next_some() => {
                log::info!("Check for new github releases");

                let github = github.clone();
                let db = db.clone();

                // TODO: move repo name into configuration file.
                let future = async move {
                    let releases = github.releases("udoprog", "OxidizeBot").await?;
                    db.write_github_releases("udoprog", "OxidizeBot", releases)?;
                    Ok::<_, Error>(())
                };

                tokio::spawn(async move {
                    if let Err(e) = future.await {
                        log::error!("failed to refresh github release");
                    }
                });
            }
        }
    }
}
