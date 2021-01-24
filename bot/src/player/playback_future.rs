use crate::player::{ConnectStream, PlaybackMode, PlayerInternal, Song, State};
use crate::prelude::*;
use crate::settings;
use crate::spotify_id::SpotifyId;
use crate::utils;
use crate::Uri;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Future associated with driving audio playback.
pub(super) struct PlaybackFuture {
    pub(super) internal: Arc<RwLock<PlayerInternal>>,
    pub(super) connect_stream: ConnectStream,
    pub(super) playback_mode_stream: settings::Stream<PlaybackMode>,
    /// Stream of settings if the player is detached.
    pub(super) detached_stream: settings::Stream<bool>,
    /// Optional stream indicating that we want to send a song update on the global bus.
    pub(super) song_update_interval: Fuse<tokio::time::Interval>,
    /// Stream for when song update interval is updated.
    pub(super) song_update_interval_stream: settings::Stream<utils::Duration>,
}

impl PlaybackFuture {
    /// Run the playback future.
    pub(super) async fn run(
        mut self,
        injector: Injector,
        settings: settings::Settings,
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

        let (mut fallback_stream, fallback) = settings.stream("fallback-uri").optional().await?;
        self.internal
            .write()
            .await
            .update_fallback_items(fallback)
            .await;

        let (mut song_stream, song) = injector.stream::<Song>().await;

        let song_timeout = song
            .and_then(|s| match s.state() {
                State::Playing => Some(Fuse::new(tokio::time::sleep_until(s.deadline().into()))),
                _ => None,
            })
            .unwrap_or_default();

        tokio::pin!(song_timeout);

        let mut song_update_interval = self.song_update_interval;

        loop {
            tokio::select! {
                Some(song) = song_stream.next() => {
                    song_timeout.set(song.and_then(|s| match s.state() {
                        State::Playing => Some(Fuse::new(tokio::time::sleep_until(s.deadline().into()))),
                        _ => None,
                    }).unwrap_or_default());
                }
                Some(fallback) = fallback_stream.next() => {
                    self.internal.write().await.update_fallback_items(fallback).await;
                }
                /* player */
                _ = &mut song_timeout => {
                    let mut internal = self.internal.write().await;
                    internal.end_of_track().await?;
                }
                Some(update) = self.detached_stream.next() => {
                    self.internal.write().await.update_detached(update).await?;
                }
                Some(update) = self.playback_mode_stream.next() => {
                    self.internal.write().await.update_playback_mode(update).await?;
                }
                Some(value) = self.song_update_interval_stream.next() => {
                    song_update_interval = if value.is_empty() {
                        Fuse::empty()
                    } else {
                        Fuse::new(tokio::time::interval(value.as_std()))
                    };
                }
                _ = song_update_interval.as_pin_mut().poll_inner(|mut i, cx| i.poll_tick(cx)) => {
                    self.internal.write().await.song_update().await;
                }
                Some(event) = self.connect_stream.next() => {
                    self.internal.write().await.handle_player_event(event?).await?;
                }
            }
        }
    }
}
