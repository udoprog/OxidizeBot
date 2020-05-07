use super::{
    Command, ConnectError, ConnectPlayer, Event, IntegrationEvent, Item, Mixer, PlaybackMode,
    PlayerKind, Song, Source, State, Track, YouTubePlayer,
};
use crate::{
    api, bus, injector, prelude::*, settings, spotify_id::SpotifyId, track_id::TrackId, Uri,
};
use anyhow::{anyhow, Result};
use std::{sync::Arc, time::Duration, time::Instant};

pub(super) struct PlayerInternal {
    pub(super) injector: injector::Injector,
    /// Current player kind.
    pub(super) player: PlayerKind,
    /// Updated to the current playback mode.
    /// Player is detached.
    pub(super) detached: bool,
    /// API clients and streams.
    pub(super) spotify: Arc<api::Spotify>,
    pub(super) connect_player: ConnectPlayer,
    pub(super) youtube_player: YouTubePlayer,
    /// The mode of the player.
    ///
    /// The mode determines if the player is enqueueing songs or immediately
    /// playing them.
    pub(super) playback_mode: PlaybackMode,
    /// The internal mixer.
    pub(super) mixer: Mixer,
    /// The player bus.
    pub(super) bus: bus::Bus<Event>,
    /// Notifier to use when sending song updates.
    pub(super) global_bus: Arc<bus::Bus<bus::Global>>,
    /// Song config.
    pub(super) song_switch_feedback: settings::Var<bool>,
    /// The next song timeout.
    pub(super) song_timeout_at: Option<Instant>,
}

impl PlayerInternal {
    /// Check if the player is unmanaged.
    ///
    /// An unmanaged player doesn't process default commands that deal with the
    /// internal player.
    fn is_unmanaged(&self) -> bool {
        if self.detached {
            return true;
        }

        self.playback_mode == PlaybackMode::Queue
    }

    /// We've reached the end of track, process it.
    pub(super) async fn end_of_track(&mut self) -> Result<()> {
        if self.is_unmanaged() {
            log::warn!("End of track called even though we are no longer managing the player");
            return Ok(());
        }

        log::trace!("Song ended, loading next song...");

        if let Some(song) = self.mixer.next_song().await? {
            self.play_song(Source::Manual, song).await?;
        } else {
            self.bus.send_sync(Event::Empty);
            self.notify_song_change(None)?;
        }

        Ok(())
    }

    /// Notify a change in the current song.
    fn notify_song_change(&self, song: Option<&Song>) -> Result<()> {
        self.global_bus.send(bus::Global::song(song)?);
        self.global_bus.send(bus::Global::SongModified);
        Ok(())
    }

    /// Convert all songs of a user into items.
    async fn songs_to_items(spotify: &Arc<api::Spotify>) -> Result<Vec<Arc<Item>>> {
        let mut items = Vec::new();

        for added_song in spotify.my_tracks_stream().try_concat().await? {
            let track = added_song.track;

            let track_id = match &track.id {
                Some(track_id) => track_id,
                None => {
                    continue;
                }
            };

            let track_id = TrackId::Spotify(
                SpotifyId::from_base62(&track_id)
                    .map_err(|_| anyhow!("bad spotify id: {}", track_id))?,
            );

            let duration = Duration::from_millis(track.duration_ms.into());

            items.push(Arc::new(Item {
                track_id,
                track: Track::Spotify { track },
                user: None,
                duration,
            }));
        }

        Ok(items)
    }

    /// Switch the current player and send the appropriate play commands.
    async fn switch_current_player(&mut self, player: PlayerKind) {
        use self::PlayerKind::*;

        match (self.player, player) {
            (Spotify, Spotify) => (),
            (YouTube, YouTube) => (),
            (Spotify, _) | (None, YouTube) => {
                let result = self.connect_player.stop().await;

                if let Err(ConnectError::NoDevice) = result {
                    self.bus.send_sync(Event::NotConfigured);
                }
            }
            (YouTube, _) | (None, Spotify) => self.youtube_player.stop(),
            (None, None) => (),
        }

        self.player = player;
    }

    /// Send a pause command to the appropriate player.
    async fn send_pause_command(&mut self) {
        match self.player {
            PlayerKind::Spotify => {
                log::trace!("pausing spotify player");

                let result = self.connect_player.pause().await;

                if let Err(ConnectError::NoDevice) = result {
                    self.bus.send_sync(Event::NotConfigured);
                }
            }
            PlayerKind::YouTube => {
                log::trace!("pausing youtube player");
                self.youtube_player.pause();
            }
            _ => (),
        }
    }

    /// Play the given song.
    async fn send_play_command(&mut self, song: &Song) {
        match song.item.track_id.clone() {
            TrackId::Spotify(id) => {
                let result = self
                    .connect_player
                    .play(Some(id), Some(song.elapsed()))
                    .await;

                if let Err(ConnectError::NoDevice) = result {
                    self.bus.send_sync(Event::NotConfigured);
                }
            }
            TrackId::YouTube(id) => {
                self.youtube_player
                    .play(song.elapsed(), song.duration(), id);
            }
        }
    }

    /// Switch the player to the specified song without changing its state.
    async fn switch_to_song(&mut self, mut song: Option<Song>) -> Result<()> {
        if let Some(song) = song.as_mut() {
            song.pause();
            self.switch_current_player(song.player()).await;
        } else {
            self.switch_current_player(PlayerKind::None).await;
        }

        if let Some(song) = song {
            self.injector.update(song).await;
        } else {
            self.injector.clear::<Song>().await;
        }

        Ok(())
    }

    /// Switch current song to the specified song.
    async fn play_song(&mut self, source: Source, mut song: Song) -> Result<()> {
        song.play();

        self.song_timeout_at = Some(song.deadline());

        self.send_play_command(&song).await;
        self.switch_current_player(song.player()).await;
        self.notify_song_change(Some(&song))?;

        if let Source::Manual = source {
            let feedback = self.song_switch_feedback.load().await;
            self.bus
                .send_sync(Event::Playing(feedback, Some(song.item.clone())));
        }

        self.injector.update(State::Playing).await;
        self.injector.update(song).await;
        Ok(())
    }

    /// Resume playing a specific song.
    async fn resume_song(&mut self, source: Source, song: Song) -> Result<()> {
        self.song_timeout_at = Some(song.deadline().into());

        self.send_play_command(&song).await;
        self.switch_current_player(song.player()).await;
        self.notify_song_change(Some(&song))?;

        if let Source::Manual = source {
            let feedback = self.song_switch_feedback.load().await;
            self.bus
                .send_sync(Event::Playing(feedback, Some(song.item.clone())));
        }

        self.injector.update(State::Playing).await;
        self.injector.update(song).await;
        Ok(())
    }

    /// Detach the player.
    async fn detach(&mut self) -> Result<()> {
        self.player = PlayerKind::None;
        self.song_timeout_at = None;
        self.injector.update(State::None).await;

        // store the currently playing song in the sidelined slot.
        if let Some(mut song) = self.injector.clear::<Song>().await {
            song.pause();
            self.mixer.push_sidelined(song);
        }

        Ok(())
    }

    /// Handle incoming command.
    pub(super) async fn command(&mut self, command: Command) -> Result<()> {
        use self::Command::*;

        let state = self.injector.get::<State>().await.unwrap_or_default();

        if self.detached {
            log::trace!(
                "Ignoring: Command = {:?}, State = {:?}, Player = {:?}",
                command,
                state,
                self.player,
            );

            if let Source::Manual = command.source() {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        let command = match (command, state) {
            (Toggle(source), State::Paused) | (Toggle(source), State::None) => Play(source),
            (Toggle(source), State::Playing) => Pause(source),
            (command, _) => command,
        };

        match self.playback_mode {
            PlaybackMode::Default => {
                self.default_playback_command(command).await?;
            }
            PlaybackMode::Queue => {
                self.queue_playback_command(command).await?;
            }
        }

        Ok(())
    }

    /// Handle the default playback command.
    async fn queue_playback_command(&mut self, command: Command) -> Result<()> {
        use self::Command::*;

        let state = self.injector.get::<State>().await.unwrap_or_default();

        log::trace!(
            "Processing: Command = {:?}, State = {:?}, Player = {:?}",
            command,
            state,
            self.player,
        );

        match (command, state) {
            (Skip(source), _) => {
                log::trace!("Skipping song");

                let result = self.connect_player.next().await;

                if let Err(ConnectError::NoDevice) = result {
                    self.bus.send_sync(Event::NotConfigured);
                }

                if let Source::Manual = source {
                    self.bus.send_sync(Event::Skip);
                }
            }
            // initial pause
            (Pause(source), State::Playing) => {
                log::trace!("Pausing player");

                let result = self.connect_player.pause().await;

                if let Err(ConnectError::NoDevice) = result {
                    self.bus.send_sync(Event::NotConfigured);
                }

                if let Source::Manual = source {
                    self.bus.send_sync(Event::Pausing);
                }

                self.injector.update(State::Paused).await;
            }
            (Play(source), State::Paused) | (Play(source), State::None) => {
                log::trace!("Starting player");

                let result = self.connect_player.play(None, None).await;

                if let Err(ConnectError::NoDevice) = result {
                    self.bus.send_sync(Event::NotConfigured);
                }

                if let Source::Manual = source {
                    let feedback = self.song_switch_feedback.load().await;
                    self.bus.send_sync(Event::Playing(feedback, None));
                }

                self.injector.update(State::Playing).await;
            }
            (Sync { .. }, _) => {
                log::info!("Synchronization not supported with the current playback mode");
            }
            // queue was modified in some way
            (Modified(..), State::Playing) => {
                log::info!("Song modifications are not supported with the current playback mode");
            }
            (Inject(_, item, offset), State::Playing) => match &item.track_id {
                &TrackId::Spotify(id) => {
                    let result = self.connect_player.play(Some(id), Some(offset)).await;

                    if let Err(ConnectError::NoDevice) = result {
                        self.bus.send_sync(Event::NotConfigured);
                    }

                    self.injector.update(State::Playing).await;
                }
                _ => {
                    log::info!("Can't inject playback of a non-spotify song.");
                }
            },
            _ => (),
        }

        Ok(())
    }

    /// Handle the default playback command.
    async fn default_playback_command(&mut self, command: Command) -> Result<()> {
        use self::Command::*;

        let state = self.injector.get::<State>().await.unwrap_or_default();

        log::trace!(
            "Processing: Command = {:?}, State = {:?}, Player = {:?}",
            command,
            state,
            self.player,
        );

        let command = match (command, state) {
            (Toggle(source), State::Paused) | (Toggle(source), State::None) => Play(source),
            (Toggle(source), State::Playing) => Pause(source),
            (command, _) => command,
        };

        match (command, state) {
            (Skip(source), _) => {
                log::trace!("Skipping song");

                let song = self.mixer.next_song().await?;

                match (song, state) {
                    (Some(song), State::Playing) => {
                        self.play_song(source, song).await?;
                    }
                    (Some(song), _) => {
                        self.switch_to_song(Some(song.clone())).await?;
                        self.notify_song_change(Some(&song))?;
                    }
                    (None, _) => {
                        if let Source::Manual = source {
                            self.bus.send_sync(Event::Empty);
                        }

                        self.switch_to_song(None).await?;
                        self.notify_song_change(None)?;
                        self.injector.update(State::Paused).await;
                    }
                }
            }
            // initial pause
            (Pause(source), State::Playing) => {
                log::trace!("Pausing player");

                self.send_pause_command().await;
                self.song_timeout_at = None;
                self.injector.update(State::Paused).await;

                let song = self
                    .injector
                    .mutate(|song: &mut Song| {
                        song.pause();
                        song.clone()
                    })
                    .await;

                if let Source::Manual = source {
                    self.bus.send_sync(Event::Pausing);
                }

                self.notify_song_change(song.as_ref())?;
            }
            (Play(source), State::Paused) | (Play(source), State::None) => {
                log::trace!("Starting player");

                let song = {
                    match self.injector.get::<Song>().await {
                        Some(mut song) => {
                            song.play();
                            Some(song)
                        }
                        None => None,
                    }
                };

                // resume an existing song
                if let Some(song) = song {
                    self.resume_song(source, song).await?;
                    return Ok(());
                }

                // play the next song in queue.
                if let Some(song) = self.mixer.next_song().await? {
                    self.play_song(source, song).await?;
                } else {
                    if let Source::Manual = source {
                        self.bus.send_sync(Event::Empty);
                    }

                    self.injector.clear::<Song>().await;
                    self.injector.update(State::Paused).await;
                }
            }
            (Sync { song }, _) => {
                log::trace!("Synchronize the state of the player with the given song");

                self.switch_current_player(song.player()).await;

                let state = song.state();

                if let State::Playing = state {
                    self.song_timeout_at = Some(song.deadline());
                } else {
                    self.song_timeout_at = None;
                }

                self.notify_song_change(Some(&song))?;
                self.injector.update(song).await;
                self.injector.update(state).await;
            }
            // queue was modified in some way
            (Modified(source), State::Playing) => {
                if !self.injector.exists::<Song>().await {
                    if let Some(song) = self.mixer.next_song().await? {
                        self.play_song(source, song).await?;
                    }
                }

                self.global_bus.send(bus::Global::SongModified);
                self.bus.send_sync(Event::Modified);
            }
            (Inject(source, item, offset), State::Playing) => {
                {
                    // store the currently playing song in the sidelined slot.
                    if let Some(mut song) = self.injector.clear::<Song>().await {
                        song.pause();
                        self.mixer.push_sidelined(song);
                    }
                }

                self.play_song(source, Song::new(item, offset)).await?;
            }
            _ => (),
        }

        Ok(())
    }

    /// Update fallback items based on an URI.
    pub(super) async fn update_fallback_items(&mut self, uri: Option<Uri>) {
        let result = match uri.as_ref() {
            Some(uri) => {
                let id = match uri {
                    Uri::SpotifyPlaylist(id) => id,
                    uri => {
                        log::warn!("Bad fallback URI `{}`, expected Spotify Playlist", uri);
                        return;
                    }
                };

                let result = Self::playlist_to_items(&self.spotify, id.to_string()).await;

                match result {
                    Ok((name, items)) => Ok((Some(name), items)),
                    Err(e) => {
                        log::warn!(
                            "Failed to load playlist `{}`, \
                             falling back to library: {}",
                            uri,
                            e
                        );
                        Self::songs_to_items(&self.spotify)
                            .await
                            .map(|items| (None, items))
                    }
                }
            }
            None => Self::songs_to_items(&self.spotify)
                .await
                .map(|items| (None, items)),
        };

        let (what, items) = match result {
            Ok(result) => result,
            Err(e) => {
                log_error!(e, "Failed to configure fallback items");
                return;
            }
        };

        let what = what
            .as_ref()
            .map(|u| format!("\"{}\" playlist", u))
            .unwrap_or_else(|| String::from("your library"));

        log::info!(
            "Updated fallback queue with {} items from {}.",
            items.len(),
            what
        );

        self.mixer.update_fallback_items(items);
    }

    /// Convert a playlist into items.
    async fn playlist_to_items(
        spotify: &Arc<api::Spotify>,
        playlist: String,
    ) -> Result<(String, Vec<Arc<Item>>)> {
        let mut items = Vec::new();

        let playlist = spotify.playlist(playlist).await?;
        let name = playlist.name.to_string();

        for playlist_track in spotify.page_as_stream(playlist.tracks).try_concat().await? {
            let track = playlist_track.track;

            let track_id = match &track.id {
                Some(track_id) => track_id,
                None => {
                    continue;
                }
            };

            let track_id = TrackId::Spotify(
                SpotifyId::from_base62(&track_id)
                    .map_err(|_| anyhow!("bad spotify id: {}", track_id))?,
            );

            let duration = Duration::from_millis(track.duration_ms.into());

            items.push(Arc::new(Item {
                track_id,
                track: Track::Spotify { track },
                user: None,
                duration,
            }));
        }

        Ok((name, items))
    }

    /// Handle an event from the connect integration.
    pub(super) async fn handle_player_event(&mut self, e: IntegrationEvent) -> Result<()> {
        use IntegrationEvent::*;

        let state = self.injector.get::<State>().await.unwrap_or_default();

        if self.detached {
            log::trace!(
                "Ignoring (Detached): IntegrationEvent = {:?}, State = {:?}, Player = {:?}",
                e,
                state,
                self.player,
            );

            return Ok(());
        }

        log::trace!(
            "Processing: IntegrationEvent = {:?}, State = {:?}, Player = {:?}",
            e,
            state,
            self.player,
        );

        match e {
            DeviceChanged => {
                if state != State::Playing {
                    return Ok(());
                }

                let (elapsed, duration, track_id) = {
                    let result = self
                        .injector
                        .mutate(|song: &mut Song| {
                            // pause so that it can get unpaused later.
                            song.pause();
                            (song.elapsed(), song.duration(), song.item.track_id.clone())
                        })
                        .await;

                    match result {
                        Some(result) => result,
                        None => return Ok(()),
                    }
                };

                // TODO: how do we deal with playback mode on a device transfer?
                match track_id {
                    TrackId::Spotify(id) => {
                        let result = self.connect_player.play(Some(id), Some(elapsed)).await;

                        if let Err(ConnectError::NoDevice) = result {
                            self.bus.send_sync(Event::NotConfigured);
                        }

                        self.switch_current_player(PlayerKind::Spotify).await;
                        self.injector.update(State::Playing).await;
                    }
                    TrackId::YouTube(id) => {
                        self.youtube_player.play(elapsed, duration, id);
                        self.switch_current_player(PlayerKind::YouTube).await;
                        self.injector.update(State::Playing).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a song file update.
    pub(super) async fn song_update(&mut self) {
        if let State::Playing = self.injector.get::<State>().await.unwrap_or_default() {
            let song = self.injector.get::<Song>().await;
            let song = song.as_ref();
            self.global_bus.send(bus::Global::song_progress(song));

            if let Some(song) = song {
                if let TrackId::YouTube(ref id) = song.item.track_id {
                    self.youtube_player
                        .tick(song.elapsed(), song.duration(), id.to_string());
                }
            }
        }
    }

    /// Update the detached state.
    pub(super) async fn update_detached(&mut self, detached: bool) -> Result<()> {
        if detached {
            self.detach().await?;
        }

        self.detached = detached;
        Ok(())
    }

    /// Update the current playback mode.
    pub(super) async fn update_playback_mode(&mut self, mode: PlaybackMode) -> Result<()> {
        self.playback_mode = mode;

        match mode {
            PlaybackMode::Queue => {
                self.detach().await?;
            }
            _ => {}
        }

        Ok(())
    }
}
