use crate::{
    api, injector::Injector, prelude::*, storage::Cache, timer::Interval, utils::Duration,
};
use failure::Error;
use parking_lot::RwLock;
use std::{sync::Arc, time::Instant};

const USER: &'static str = "udoprog";
const REPO: &'static str = "OxidizeBot";

pub fn run(
    injector: &Injector,
) -> (
    Arc<RwLock<Option<api::github::Release>>>,
    impl Future<Output = Result<(), Error>>,
) {
    let latest = Arc::new(RwLock::new(None));
    let returned_latest = latest.clone();
    let injector = injector.clone();

    let future = async move {
        let github = api::GitHub::new()?;
        let mut interval = Interval::new(Instant::now(), Duration::hours(6).as_std());

        let (mut cache_stream, mut cache) = injector.stream::<Cache>();

        loop {
            futures::select! {
                update = cache_stream.select_next_some() => {
                    cache = update;
                }
                _ = interval.select_next_some() => {
                    log::trace!("Looking for new release...");

                    let future = github.releases(String::from(USER), String::from(REPO));

                    let mut releases = match cache.as_ref() {
                        Some(cache) => cache.wrap(String::from("updater/version"), Duration::hours(1), future).await?,
                        None => future.await?,
                    };

                    releases.sort_by(|a, b| b.published_at.cmp(&a.published_at));

                    let release = match releases.into_iter().filter(|r| !r.prerelease).next() {
                        Some(release) => release,
                        None => continue,
                    };

                    *latest.write() = Some(release);
                }
            }
        }
    };

    (returned_latest, future)
}
