use oxidize_web::web;
use std::{fs, path::Path};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

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
async fn main() -> Result<(), failure::Error> {
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
    let config = toml::from_str(&fs::read_to_string(config_path)?)?;

    web::setup(&root, host, port, config)?.await?;
    Ok(())
}
