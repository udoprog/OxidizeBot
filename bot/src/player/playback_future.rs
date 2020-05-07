use super::{
    Command, ConnectError, ConnectPlayer, ConnectStream, Event, IntegrationEvent, Item, Mixer,
    PlaybackMode, PlayerKind, Song, Source, State, Track, YouTubePlayer,
};
use crate::{
    api, bus, injector,
    prelude::*,
    settings,
    song_file::{SongFile, SongFileBuilder},
    spotify_id::SpotifyId,
    template::Template,
    track_id::TrackId,
    utils, Uri,
};
use anyhow::{anyhow, Result};
use std::{sync::Arc, time::Duration};

static DEFAULT_CURRENT_SONG_TEMPLATE: &str = "Song: {{name}}{{#if artists}} by {{artists}}{{/if}}{{#if paused}} (Paused){{/if}} ({{duration}})\n{{#if user~}}Request by: @{{user~}}{{/if}}";
static DEFAULT_CURRENT_SONG_STOPPED_TEMPLATE: &str = "Not Playing";

/// Future associated with driving audio playback.
pub(super) struct PlaybackFuture {
    pub(super) spotify: Arc<api::Spotify>,
    pub(super) connect_stream: ConnectStream,
    pub(super) connect_player: ConnectPlayer,
    pub(super) youtube_player: YouTubePlayer,
    pub(super) commands: mpsc::UnboundedReceiver<Command>,
    pub(super) bus: bus::Bus<Event>,
    pub(super) mixer: Mixer,
    /// The current state of the player.
    pub(super) state: State,
    /// Current player kind.
    pub(super) player: PlayerKind,
    /// The mode of the player.
    ///
    /// The mode determines if the player is enqueueing songs or immediately
    /// playing them.
    pub(super) playback_mode: PlaybackMode,
    /// Updated to the current playback mode.
    pub(super) playback_mode_stream: settings::Stream<PlaybackMode>,
    /// Player is detached.
    pub(super) detached: bool,
    /// Stream of settings if the player is detached.
    pub(super) detached_stream: settings::Stream<bool>,
    /// Song that is currently loaded.
    pub(super) song: injector::Var<Option<Song>>,
    /// Path to write current song.
    pub(super) song_file: Option<SongFile>,
    /// Song config.
    pub(super) song_switch_feedback: settings::Var<bool>,
    /// Optional stream indicating that we want to send a song update on the global bus.
    pub(super) song_update_interval: Option<tokio::time::Interval>,
    /// Stream for when song update interval is updated.
    pub(super) song_update_interval_stream: settings::Stream<utils::Duration>,
    /// Notifier to use when sending song updates.
    pub(super) global_bus: Arc<bus::Bus<bus::Global>>,
    /// Timeout for end of song.
    pub(super) timeout: Option<tokio::time::Delay>,
}

impl PlaybackFuture {
    /// Check if the player is detached.
    fn is_unmanaged(&self) -> bool {
        if self.detached {
            return true;
        }

        self.playback_mode == PlaybackMode::Queue
    }

    /// Run the playback future.
    pub(super) async fn run(mut self, settings: settings::Settings) -> Result<()> {
        let song_file = settings.scoped("song-file");

        let (mut path_stream, path) = song_file.stream("path").optional().await?;

        let (mut template_stream, template) = song_file
            .stream("template")
            .or(Some(Template::compile(DEFAULT_CURRENT_SONG_TEMPLATE)?))
            .optional()
            .await?;

        let (mut stopped_template_stream, stopped_template) = song_file
            .stream("stopped-template")
            .or(Some(Template::compile(
                DEFAULT_CURRENT_SONG_STOPPED_TEMPLATE,
            )?))
            .optional()
            .await?;

        let (mut update_interval_stream, update_interval) = song_file
            .stream("update-interval")
            .or_with(utils::Duration::seconds(1))
            .await?;

        let (mut enabled_stream, enabled) = song_file.stream("enabled").or_default().await?;

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
        self.update_fallback_items(fallback).await;

        let mut song_file = SongFileBuilder::default();
        song_file.enabled = enabled;
        song_file.path = path;
        song_file.template = template;
        song_file.stopped_template = stopped_template;
        song_file.update_interval = update_interval;
        song_file.init(&mut self.song_file);

        loop {
            let mut song_file_update = self.song_file.as_mut().map(|u| &mut u.update_interval);

            futures::select! {
                fallback = fallback_stream.select_next_some() => {
                    self.update_fallback_items(fallback).await;
                }
                /* current song */
                update = enabled_stream.select_next_some() => {
                    song_file.enabled = update;
                    song_file.init(&mut self.song_file);
                }
                update = path_stream.select_next_some() => {
                    song_file.path = update;
                    song_file.init(&mut self.song_file);
                }
                update = template_stream.select_next_some() => {
                    song_file.template = update;
                    song_file.init(&mut self.song_file);
                }
                update = stopped_template_stream.select_next_some() => {
                    song_file.stopped_template = update;
                    song_file.init(&mut self.song_file);
                }
                update = update_interval_stream.select_next_some() => {
                    song_file.update_interval = update;
                    song_file.init(&mut self.song_file);
                }
                _ = song_file_update.select_next_some() => {
                    let song = self.song.read().await;
                    self.update_song_file(song.as_ref());
                }
                /* player */
                _ = self.timeout.current() => {
                    self.end_of_track().await?;
                }
                update = self.detached_stream.select_next_some() => {
                    if update {
                        self.detach().await?;
                    }

                    self.detached = update;
                }
                update = self.playback_mode_stream.select_next_some() => {
                    self.playback_mode = update;

                    match update {
                        PlaybackMode::Queue => {
                            self.detach().await?;
                        }
                        _ => {
                        },
                    }
                }
                value = self.song_update_interval_stream.select_next_some() => {
                    self.song_update_interval = match value.is_empty() {
                        true => None,
                        false => Some(tokio::time::interval(value.as_std())),
                    };
                }
                _ = self.song_update_interval.select_next_some() => {
                    let song = self.song.read().await;

                    if let State::Playing = self.state {
                        self.global_bus
                            .send(bus::Global::song_progress(song.as_ref()));

                        if let Some(song) = song.as_ref() {
                            if let TrackId::YouTube(ref id) = song.item.track_id {
                                self.youtube_player.tick(song.elapsed(), song.duration(), id.to_string());
                            }
                        }
                    }
                }
                event = self.connect_stream.select_next_some() => {
                    self.handle_player_event(event?).await?;
                }
                command = self.commands.select_next_some() => {
                    self.command(command).await?;
                }
            }
        }
    }

    /// Update fallback items based on an URI.
    async fn update_fallback_items(&mut self, uri: Option<Uri>) {
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

    /// Notify a change in the current song.
    fn notify_song_change(&self, song: Option<&Song>) -> Result<()> {
        self.global_bus.send(bus::Global::song(song)?);
        self.global_bus.send(bus::Global::SongModified);
        self.update_song_file(song);
        Ok(())
    }

    /// Write the current song.
    async fn write_song(&self, song: Option<Song>) -> Result<()> {
        *self.song.write().await = song;
        Ok(())
    }

    /// Write current song. Log any errors.
    ///
    /// MUST NOT be called when self.song is locked.
    fn update_song_file(&self, song: Option<&Song>) {
        let current_song = match self.song_file.as_ref() {
            Some(current_song) => current_song,
            None => return,
        };

        let result = match song {
            Some(song) => current_song.write(song, self.state),
            None => current_song.blank(),
        };

        if let Err(e) = result {
            log::warn!(
                "failed to write current song: {}: {}",
                current_song.path.display(),
                e
            );
        }
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
    async fn send_play_command(&mut self, song: Song) {
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

        self.write_song(song).await?;
        Ok(())
    }

    /// Switch current song to the specified song.
    async fn play_song(&mut self, source: Source, mut song: Song) -> Result<()> {
        song.play();

        self.timeout = Some(tokio::time::delay_until(song.deadline().into()));

        self.send_play_command(song.clone()).await;
        self.switch_current_player(song.player()).await;
        self.write_song(Some(song.clone())).await?;
        self.notify_song_change(Some(&song))?;

        if let Source::Manual = source {
            let feedback = self.song_switch_feedback.load().await;
            self.bus
                .send_sync(Event::Playing(feedback, Some(song.item.clone())));
        }

        self.state = State::Playing;
        Ok(())
    }

    /// Resume playing a specific song.
    async fn resume_song(&mut self, source: Source, song: Song) -> Result<()> {
        self.timeout = Some(tokio::time::delay_until(song.deadline().into()));

        self.send_play_command(song.clone()).await;
        self.switch_current_player(song.player()).await;
        self.notify_song_change(Some(&song))?;

        if let Source::Manual = source {
            let feedback = self.song_switch_feedback.load().await;
            self.bus
                .send_sync(Event::Playing(feedback, Some(song.item.clone())));
        }

        self.state = State::Playing;
        Ok(())
    }

    /// Detach the player.
    async fn detach(&mut self) -> Result<()> {
        // store the currently playing song in the sidelined slot.
        if let Some(mut song) = self.song.write().await.take() {
            song.pause();
            self.mixer.push_sidelined(song);
        }

        self.write_song(None).await?;
        self.player = PlayerKind::None;
        self.state = State::None;
        self.timeout = None;
        Ok(())
    }

    /// Handle incoming command.
    async fn command(&mut self, command: Command) -> Result<()> {
        use self::Command::*;

        if self.detached {
            log::trace!(
                "Ignoring: Command = {:?}, State = {:?}, Player = {:?}",
                command,
                self.state,
                self.player,
            );

            if let Source::Manual = command.source() {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        let command = match (command, self.state) {
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

        log::trace!(
            "Processing: Command = {:?}, State = {:?}, Player = {:?}",
            command,
            self.state,
            self.player,
        );

        match (command, self.state) {
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

                self.state = State::Paused;
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

                self.state = State::Playing;
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

                    self.state = State::Playing;
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

        log::trace!(
            "Processing: Command = {:?}, State = {:?}, Player = {:?}",
            command,
            self.state,
            self.player,
        );

        let command = match (command, self.state) {
            (Toggle(source), State::Paused) | (Toggle(source), State::None) => Play(source),
            (Toggle(source), State::Playing) => Pause(source),
            (command, _) => command,
        };

        match (command, self.state) {
            (Skip(source), _) => {
                log::trace!("Skipping song");

                let song = self.mixer.next_song().await?;

                match (song, self.state) {
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
                        self.state = State::Paused;
                    }
                }
            }
            // initial pause
            (Pause(source), State::Playing) => {
                log::trace!("Pausing player");

                self.send_pause_command().await;
                self.timeout = None;
                self.state = State::Paused;

                let mut song = self.song.write().await;

                if let Some(song) = song.as_mut() {
                    song.pause();
                }

                if let Source::Manual = source {
                    self.bus.send_sync(Event::Pausing);
                }

                self.notify_song_change(song.as_ref())?;
            }
            (Play(source), State::Paused) | (Play(source), State::None) => {
                log::trace!("Starting player");

                let song = {
                    match self.song.write().await.as_mut() {
                        Some(song) => {
                            song.play();
                            Some(song.clone())
                        }
                        None => None,
                    }
                };

                // resume an existing song
                if let Some(song) = song {
                    self.resume_song(source, song.clone()).await?;
                    return Ok(());
                }

                // play the next song in queue.
                if let Some(song) = self.mixer.next_song().await? {
                    self.play_song(source, song).await?;
                } else {
                    if let Source::Manual = source {
                        self.bus.send_sync(Event::Empty);
                    }

                    self.write_song(None).await?;
                    self.state = State::Paused;
                }
            }
            (Sync { song }, _) => {
                log::trace!("Synchronize the state of the player with the given song");

                self.switch_current_player(song.player()).await;

                self.state = song.state();

                if let State::Playing = self.state {
                    self.timeout = Some(tokio::time::delay_until(song.deadline().into()));
                } else {
                    self.timeout = None;
                }

                self.notify_song_change(Some(&song))?;
                self.write_song(Some(song)).await?;
            }
            // queue was modified in some way
            (Modified(source), State::Playing) => {
                if self.song.read().await.is_none() {
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
                    if let Some(mut song) = self.song.write().await.take() {
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

    /// We've reached the end of a track.
    async fn end_of_track(&mut self) -> Result<()> {
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

    /// Handle an event from the connect integration.
    async fn handle_player_event(&mut self, e: IntegrationEvent) -> Result<()> {
        use IntegrationEvent::*;

        if self.detached {
            log::trace!(
                "Ignoring (Detached): IntegrationEvent = {:?}, State = {:?}, Player = {:?}",
                e,
                self.state,
                self.player,
            );

            return Ok(());
        }

        log::trace!(
            "Processing: IntegrationEvent = {:?}, State = {:?}, Player = {:?}",
            e,
            self.state,
            self.player,
        );

        match e {
            DeviceChanged => {
                if self.state != State::Playing {
                    return Ok(());
                }

                let (elapsed, duration, track_id) = {
                    let mut song = self.song.write().await;

                    let song = match song.as_mut() {
                        Some(song) => song,
                        None => return Ok(()),
                    };

                    // pause so that it can get unpaused later.
                    song.pause();
                    (song.elapsed(), song.duration(), song.item.track_id.clone())
                };

                // TODO: how do we deal with playback mode on a device transfer?
                match track_id {
                    TrackId::Spotify(id) => {
                        let result = self.connect_player.play(Some(id), Some(elapsed)).await;

                        if let Err(ConnectError::NoDevice) = result {
                            self.bus.send_sync(Event::NotConfigured);
                        }

                        self.switch_current_player(PlayerKind::Spotify).await;
                        self.state = State::Playing;
                    }
                    TrackId::YouTube(id) => {
                        self.youtube_player.play(elapsed, duration, id);
                        self.switch_current_player(PlayerKind::YouTube).await;
                        self.state = State::Playing;
                    }
                }
            }
        }

        Ok(())
    }
}
