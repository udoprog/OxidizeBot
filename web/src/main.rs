use anyhow::{bail, Result};
use oxidize_web::api;
use oxidize_web::db;
use oxidize_web::web;
use std::fs;
use std::path::Path;
use std::time;

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
async fn main() -> Result<()> {
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

    let base = config.database.to_path(root);
    let v28 = base.clone();
    let v31 = base.with_extension("31");
    let v31_db;

    if !v31.is_dir() {
        log::warn!("migrating database {} -> {}", v28.display(), v31.display());

        // migrate 28 to 31
        let v28 = sled28::Db::open(v28)?.open_tree("storage")?;
        v31_db = sled31::open(v31)?.open_tree("storage")?;

        let mut count = 0;

        for result in v28.scan_prefix(&[]) {
            let (k, v) = result?;
            v31_db.insert(k, &*v)?;
            count += 1;
        }

        log::warn!("migrated {} records", count);
    } else {
        v31_db = sled31::open(v31)?.open_tree("storage")?;
    }

    let db = db::Database::load(v31_db)?;

    let github = api::GitHub::new()?;

    let mut releases_interval = tokio::time::interval(time::Duration::from_secs(60 * 10));

    let web = web::setup(db.clone(), host, port, config)?;
    tokio::pin!(web);

    #[allow(clippy::unnecessary_mut_passed)]
    loop {
        tokio::select! {
            _ = web.as_mut() => {
                bail!("web future ended unexpectedly");
            }
            _ = releases_interval.tick() => {
                log::info!("Check for new github releases");

                let github = github.clone();
                let db = db.clone();

                // TODO: move repo name into configuration file.
                let future = async move {
                    let releases = github.releases("udoprog", "OxidizeBot").await?;
                    db.write_github_releases("udoprog", "OxidizeBot", releases)?;
                    Ok::<_, anyhow::Error>(())
                };

                tokio::spawn(async move {
                    if let Err(e) = future.await {
                        log::error!("failed to refresh github release: {}", e);
                    }
                });
            }
        }
    }
}
