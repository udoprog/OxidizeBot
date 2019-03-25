use crate::{spotify, utils::BoxFuture};
use failure::format_err;
use futures::{
    future::{self, Future},
    sync::{mpsc, oneshot},
};
use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::timer;
use tokio_core::reactor::Core;

/// Setup a player.
pub fn setup(
    core: &mut Core,
    config: &super::Config,
    spotify: Arc<spotify::Spotify>,
) -> Result<
    (
        Box<dyn super::PlayerInterface + 'static>,
        super::PlayerEventStream,
    ),
    failure::Error,
> {
    let devices = core.run(spotify.my_player_devices())?;

    for (i, device) in devices.iter().enumerate() {
        log::info!("device #{}: {}", i, device.name)
    }

    let device = match config.device.as_ref() {
        Some(device) => devices.into_iter().find(|d| d.name == *device),
        None => devices.into_iter().next(),
    };

    let device = device.ok_or_else(|| format_err!("No connected devices found"))?;

    let (tx, rx) = mpsc::unbounded();

    let player = ConnectPlayer {
        spotify,
        tx,
        device,
        loaded: Arc::new(RwLock::new(None)),
        delay_cancel: Arc::new(RwLock::new(None)),
    };

    Ok((Box::new(player), Box::new(rx)))
}

struct ConnectPlayer {
    spotify: Arc<spotify::Spotify>,
    #[allow(unused)]
    tx: mpsc::UnboundedSender<super::PlayerEvent>,
    device: spotify::Device,
    /// Oneshot associated with loaded track.
    loaded: Arc<RwLock<Option<oneshot::Sender<()>>>>,
    /// Oneshot associated to cancel the current delay.
    delay_cancel: Arc<RwLock<Option<oneshot::Sender<()>>>>,
}

impl super::PlayerInterface for ConnectPlayer {
    fn stop(&mut self) {
        // cancel current delay if present.
        if let Some(delay_cancel) = self.delay_cancel.write().expect("poisoned").take() {
            let _ = delay_cancel.send(());
        }

        if let Some(tx) = self.loaded.write().expect("poisoned").take() {
            let _ = tx.send(());
        }
    }

    fn play(&mut self) {
        // cancel current delay if present.
        if let Some(delay_cancel) = self.delay_cancel.write().expect("poisoned").take() {
            let _ = delay_cancel.send(());
        }

        let future = self
            .spotify
            .me_player()
            .and_then::<_, BoxFuture<(), failure::Error>>({
                let device_id = self.device.id.clone();
                let spotify = self.spotify.clone();
                let loaded = self.loaded.clone();
                let delay_cancel = self.delay_cancel.clone();

                move |current| {
                    let progress_ms = match current.progress_ms {
                        Some(progress_ms) => progress_ms,
                        None => {
                            return Box::new(future::err(format_err!("no current song progress")));
                        }
                    };

                    let item = match current.item {
                        Some(item) => item,
                        None => {
                            return Box::new(future::err(format_err!("no currently playing song")));
                        }
                    };

                    let duration = match progress_ms.saturating_sub(item.duration_ms) {
                        0 => return Box::new(future::err(format_err!("no song progress"))),
                        duration => duration,
                    };

                    let duration = Duration::from_millis(duration.into());

                    let play = spotify.me_player_play(&device_id, None);
                    let end_track = end_track_in(duration, loaded.clone(), delay_cancel.clone());
                    Box::new(end_track.join(play).map(|((), ())| ()))
                }
            })
            .and_then(|_| Ok(()))
            .map_err({
                let loaded = self.loaded.clone();

                move |e| {
                    log::error!("failed to play song: {}", e);

                    if let Some(loaded) = loaded.write().expect("poisoned").take() {
                        let _ = loaded.send(());
                    }

                    ()
                }
            });

        tokio::spawn(future);
    }

    fn pause(&mut self) {
        // cancel current delay if present.
        if let Some(delay_cancel) = self.delay_cancel.write().expect("poisoned").take() {
            let _ = delay_cancel.send(());
        }

        tokio::spawn(self.spotify.me_player_pause(&self.device.id).map_err(|e| {
            log::error!("failed to pause player: {}", e);
            ()
        }));
    }

    fn load(&mut self, item: &super::Item, _: u32) -> oneshot::Receiver<()> {
        let track_uri = format!("spotify:track:{}", item.track_id.0.to_base62());

        tokio::spawn(
            self.spotify
                .me_player_play(&self.device.id, Some(&track_uri))
                .map_err(|e| {
                    log::error!("failed to play track: {}", e);
                    ()
                }),
        );

        let (tx, rx) = oneshot::channel();

        // write the oneshot.
        {
            let mut loaded = self.loaded.write().expect("poisoned");

            if let Some(tx) = loaded.replace(tx) {
                let _ = tx.send(());
            }
        }

        // set a deadline for the end of the track.
        tokio::spawn(
            end_track_in(
                item.duration.clone(),
                self.loaded.clone(),
                self.delay_cancel.clone(),
            )
            .map_err(|e| {
                log::error!("failed to end track: {}", e);
                ()
            }),
        );

        rx
    }

    fn volume(&mut self, volume: Option<f32>) {
        let volume = volume.unwrap_or(1f32);

        tokio::spawn(
            self.spotify
                .me_player_volume(&self.device.id, volume)
                .map_err(|e| {
                    log::error!("failed to set player volume: {}", e);
                    ()
                }),
        );
    }
}

fn end_track_in(
    duration: Duration,
    loaded: Arc<RwLock<Option<oneshot::Sender<()>>>>,
    delay_cancel: Arc<RwLock<Option<oneshot::Sender<()>>>>,
) -> impl Future<Item = (), Error = failure::Error> {
    let (delay_tx, delay_rx) = oneshot::channel();
    *delay_cancel.write().expect("poisoned") = Some(delay_tx);

    let deadline = Instant::now() + duration;

    let delay_future = timer::Delay::new(deadline)
        .map_err(|_| format_err!("delay cancelled"))
        .and_then(move |_| {
            if let Some(tx) = loaded.write().expect("poisoned").take() {
                let _ = tx.send(());
            }

            Ok(())
        });

    delay_rx
        .map_err(|_| format_err!("delay channel cancelled"))
        .select(delay_future)
        .map_err(|(e, _)| e)
        .map(|_| ())
}
