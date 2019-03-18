use futures::Future as _;
use setmod_web::web;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn opts() -> clap::App<'static, 'static> {
    clap::App::new("SetMod Web")
        .version(VERSION)
        .author("John-John Tedro <udoprog@tedro.se>")
        .about("Web Components of SetMod")
        .arg(
            clap::Arg::with_name("no-auth")
                .long("no-auth")
                .help("Do not require authentication."),
        )
}

fn main() -> Result<(), failure::Error> {
    pretty_env_logger::init();

    let opts = opts();
    let m = opts.get_matches();

    let no_auth = m.is_present("no-auth");

    tokio::run(web::setup(no_auth)?.map_err(|e| {
        log::error!("web server failed: {}", e);
        ()
    }));

    Ok(())
}
