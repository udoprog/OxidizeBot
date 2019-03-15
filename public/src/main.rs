use futures::Future as _;
use setmod_public::web;

fn main() -> Result<(), failure::Error> {
    pretty_env_logger::init();

    tokio::run(web::setup()?.map_err(|e| {
        log::error!("web server failed: {}", e);
        ()
    }));

    Ok(())
}
