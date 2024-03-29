use std::pin::pin;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_fuse::Fuse;
use async_injector::Injector;
use common::models::{Song, SpotifyId, State};
use common::{Duration, Uri};
use serde::Serialize;

use crate::{ConnectStream, PlaybackMode, PlayerInternal};

/// Future associated with driving audio playback.
pub(super) struct PlaybackFuture {
    pub(super) internal: Arc<PlayerInternal>,
    pub(super) connect_stream: ConnectStream,
    pub(super) playback_mode_stream: settings::Stream<PlaybackMode>,
    /// Stream of settings if the player is detached.
    pub(super) detached_stream: settings::Stream<bool>,
    /// Optional stream indicating that we want to send a song update on the global bus.
    pub(super) song_update_interval: Fuse<tokio::time::Interval>,
    /// Stream for when song update interval is updated.
    pub(super) song_update_interval_stream: settings::Stream<Duration>,
}

impl PlaybackFuture {
    /// Run the playback future.
    #[tracing::instrument(skip_all)]
    pub(super) async fn run(
        mut self,
        injector: Injector,
        settings: settings::Settings<::auth::Scope>,
    ) -> Result<()> {
        // TODO: Remove fallback-uri migration next major release.
        if let Some(fallback_uri) = settings.get::<String>("fallback-uri").await? {
            if str::parse::<Uri>(&fallback_uri).is_err() {
                if let Ok(id) = SpotifyId::from_base62(&fallback_uri) {
                    settings
                        .set("fallback-uri", Uri::SpotifyPlaylist(id))
                        .await?;
                }
            }
        }

        let cache = injector
            .get::<storage::Cache>()
            .await
            .context("missing cache")?;

        let (mut fallback_stream, fallback) = settings.stream("fallback-uri").optional().await?;

        let mut configure_fallback = pin!(Fuse::new(update_fallback_items_task(
            &self.internal,
            fallback,
            &cache
        )));

        let (mut song_stream, song) = injector.stream::<Song>().await;

        let mut song_timeout = pin!(song
            .and_then(|s| match s.state() {
                State::Playing => Some(Fuse::new(tokio::time::sleep_until(s.deadline().into()))),
                _ => None,
            })
            .unwrap_or_default());

        let mut song_update_interval = self.song_update_interval;

        loop {
            tokio::select! {
                song = song_stream.recv() => {
                    song_timeout.set(song.and_then(|s| match s.state() {
                        State::Playing => Some(Fuse::new(tokio::time::sleep_until(s.deadline().into()))),
                        _ => None,
                    }).unwrap_or_default());
                }
                /* player */
                _ = &mut song_timeout => {
                    self.internal.end_of_track().await?;
                }
                update = self.detached_stream.recv() => {
                    self.internal.update_detached(update).await?;
                }
                update = self.playback_mode_stream.recv() => {
                    self.internal.update_playback_mode(update).await?;
                }
                value = self.song_update_interval_stream.recv() => {
                    song_update_interval = if value.is_empty() {
                        Fuse::empty()
                    } else {
                        Fuse::new(tokio::time::interval(value.as_std()))
                    };
                }
                _ = song_update_interval.as_pin_mut().poll_inner(|mut i, cx| i.poll_tick(cx)) => {
                    self.internal.song_update().await;
                }
                event = self.connect_stream.recv() => {
                    self.internal.handle_player_event(event).await?;
                }
                fallback = fallback_stream.recv() => {
                    configure_fallback.set(Fuse::new(update_fallback_items_task(&self.internal, fallback, &cache)));
                }
                _ = configure_fallback.as_mut() => {
                }
            }
        }

        /// Update fallback item tasks.
        async fn update_fallback_items_task(
            internal: &PlayerInternal,
            fallback: Option<Uri>,
            cache: &storage::Cache,
        ) -> Result<()> {
            #[derive(Clone, Copy, Serialize)]
            #[serde(tag = "source")]
            enum Key<'a> {
                Uri { uri: &'a Uri },
                Library,
            }

            let key = match fallback.as_ref() {
                Some(uri) => Key::Uri { uri },
                None => Key::Library,
            };

            let duration = chrono::Duration::hours(4);
            let cache = cache.namespaced(&"fallback-uri")?;

            common::retry_until_ok! {
                "Loading fallback items", {
                    // NB: I don't know what's up, but for some reason this
                    // future blows up the stack.
                    let (what, items) = common::debug_box_pin(cache.wrap(key, duration, internal.load_fallback_items(fallback.as_ref()))).await?;

                    tracing::info!(
                        "Updated fallback queue with {} items from {}.",
                        items.len(),
                        what
                    );

                    internal.update_fallback_items(items).await;
                    Ok(())
                }
            }

            Ok(())
        }
    }
}
