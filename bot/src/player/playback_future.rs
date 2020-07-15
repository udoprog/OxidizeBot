use crate::player::{ConnectStream, PlaybackMode, PlayerInternal, Song};
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
    pub(super) song_update_interval: Option<tokio::time::Interval>,
    /// Stream for when song update interval is updated.
    pub(super) song_update_interval_stream: settings::Stream<utils::Duration>,
}

impl PlaybackFuture {
    /// Run the playback future.
    pub(super) async fn run(
        mut self,
        injector: injector::Injector,
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
        let mut song_timeout = song.map(|s| tokio::time::delay_until(s.deadline().into()));

        loop {
            futures::select! {
                song = song_stream.select_next_some() => {
                    song_timeout = song.map(|s| tokio::time::delay_until(s.deadline().into()));
                }
                fallback = fallback_stream.select_next_some() => {
                    self.internal.write().await.update_fallback_items(fallback).await;
                }
                /* player */
                _ = song_timeout.current() => {
                    let mut internal = self.internal.write().await;
                    internal.end_of_track().await?;
                }
                update = self.detached_stream.select_next_some() => {
                    self.internal.write().await.update_detached(update).await?;
                }
                update = self.playback_mode_stream.select_next_some() => {
                    self.internal.write().await.update_playback_mode(update).await?;
                }
                value = self.song_update_interval_stream.select_next_some() => {
                    self.song_update_interval = match value.is_empty() {
                        true => None,
                        false => Some(tokio::time::interval(value.as_std())),
                    };
                }
                _ = self.song_update_interval.select_next_some() => {
                    self.internal.write().await.song_update().await;
                }
                event = self.connect_stream.select_next_some() => {
                    self.internal.write().await.handle_player_event(event?).await?;
                }
            }
        }
    }
}
