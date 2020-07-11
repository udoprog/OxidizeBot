use super::{
    convert_item, AddTrackError, ConnectDevice, ConnectPlayer, Event, IntegrationEvent, Item,
    Mixer, PlaybackMode, PlayerKind, Song, Source, State, Track, YouTubePlayer,
};
use crate::{
    api, bus, db, injector, prelude::*, settings, spotify_id::SpotifyId, track_id::TrackId, utils,
    Uri,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::{sync::Arc, time::Duration};

pub(super) struct PlayerInternal {
    pub(super) injector: injector::Injector,
    /// Current player kind.
    pub(super) player: PlayerKind,
    /// Updated to the current playback mode.
    /// Player is detached.
    pub(super) detached: bool,
    /// API clients and streams.
    pub(super) spotify: Arc<api::Spotify>,
    pub(super) youtube: Arc<api::YouTube>,
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
    pub(super) device: ConnectDevice,
    pub(super) max_queue_length: settings::Var<u32>,
    pub(super) max_songs_per_user: settings::Var<u32>,
    pub(super) duplicate_duration: settings::Var<utils::Duration>,
    /// Theme songs.
    pub(super) themes: injector::Var<Option<db::Themes>>,
    /// Player is closed for more requests.
    pub(super) closed: Option<Option<Arc<String>>>,
}

impl PlayerInternal {
    /// Initialize the queue from the database.
    pub async fn initialize_queue(&mut self) -> Result<()> {
        self.mixer
            .initialize_queue(&*self.spotify, &*self.youtube)
            .await
    }

    /// Try to sync Spotify playback.
    pub async fn sync_spotify_playback(&mut self) -> Result<()> {
        if !self.spotify.token.is_ready().await {
            return Ok(());
        }

        let p = match self.spotify.me_player().await {
            Ok(p) => p,
            Err(e) => {
                log::warn!("Failed to sync playback: {}", e);
                return Ok(());
            }
        };

        if let Some(p) = p {
            log::trace!("Detected playback: {:?}", p);

            match Song::from_playback(&p) {
                Some(song) => {
                    log::trace!("Syncing playback");
                    let volume_percent = p.device.volume_percent;
                    self.device.sync_device(Some(p.device)).await?;
                    self.connect_player
                        .set_scaled_volume(volume_percent)
                        .await?;
                    self.sync(song).await?;
                }
                None => {
                    log::trace!("Pausing playback since item is missing");
                    self.pause(Source::Automatic).await?;
                }
            }
        }

        Ok(())
    }

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
            self.notify_song_change(None).await?;
        }

        Ok(())
    }

    /// Notify a change in the current song.
    async fn notify_song_change(&self, song: Option<&Song>) -> Result<()> {
        self.global_bus.send(bus::Global::song(song)?).await;
        self.global_bus.send(bus::Global::SongModified).await;
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
    async fn switch_current_player(&mut self, player: PlayerKind) -> Result<()> {
        use self::PlayerKind::*;

        match (self.player, player) {
            (Spotify, Spotify) => (),
            (YouTube, YouTube) => (),
            (Spotify, _) | (None, YouTube) => {
                self.connect_player.stop().await?;
            }
            (YouTube, _) | (None, Spotify) => self.youtube_player.stop().await,
            (None, None) => (),
        }

        self.player = player;
        Ok(())
    }

    /// Send a pause command to the appropriate player.
    async fn send_pause_command(&mut self) -> Result<()> {
        match self.player {
            PlayerKind::Spotify => {
                log::trace!("pausing spotify player");
                self.connect_player.pause().await?;
            }
            PlayerKind::YouTube => {
                log::trace!("pausing youtube player");
                self.youtube_player.pause().await;
            }
            _ => (),
        }

        Ok(())
    }

    /// Play the given song.
    async fn send_play_command(&mut self, song: &Song) -> Result<()> {
        match song.item.track_id.clone() {
            TrackId::Spotify(id) => {
                self.connect_player
                    .play(Some(id), Some(song.elapsed()))
                    .await?;
            }
            TrackId::YouTube(id) => {
                self.youtube_player
                    .play(song.elapsed(), song.duration(), id)
                    .await;
            }
        }

        Ok(())
    }

    /// Switch the player to the specified song without changing its state.
    async fn switch_to_song(&mut self, mut song: Option<Song>) -> Result<()> {
        if let Some(song) = song.as_mut() {
            song.pause();
            self.switch_current_player(song.player()).await?;
        } else {
            self.switch_current_player(PlayerKind::None).await?;
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

        self.send_play_command(&song).await?;
        self.switch_current_player(song.player()).await?;
        self.notify_song_change(Some(&song)).await?;

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
        self.send_play_command(&song).await?;
        self.switch_current_player(song.player()).await?;
        self.notify_song_change(Some(&song)).await?;

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
        self.injector.update(State::None).await;

        // store the currently playing song in the sidelined slot.
        if let Some(mut song) = self.injector.clear::<Song>().await {
            song.pause();
            self.mixer.push_sidelined(song);
        }

        Ok(())
    }

    pub(super) async fn toggle(&mut self, source: Source) -> Result<()> {
        let state = self.injector.get::<State>().await.unwrap_or_default();

        match state {
            State::Paused | State::None => self.play(source).await?,
            State::Playing => self.pause(source).await?,
        }

        Ok(())
    }

    pub(super) async fn play(&mut self, source: Source) -> Result<()> {
        if self.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        log::trace!("Starting Player");

        match self.playback_mode {
            PlaybackMode::Default => {
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
            PlaybackMode::Queue => {
                self.connect_player.play(None, None).await?;

                if let Source::Manual = source {
                    let feedback = self.song_switch_feedback.load().await;
                    self.bus.send_sync(Event::Playing(feedback, None));
                }

                self.injector.update(State::Playing).await;
            }
        }

        Ok(())
    }

    /// Pause playback.
    pub(super) async fn pause(&mut self, source: Source) -> Result<()> {
        if self.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        log::trace!("Pausing Player");

        match self.playback_mode {
            PlaybackMode::Default => {
                self.send_pause_command().await?;
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

                self.notify_song_change(song.as_ref()).await?;
            }
            PlaybackMode::Queue => {
                self.connect_player.pause().await?;

                if let Source::Manual = source {
                    self.bus.send_sync(Event::Pausing);
                }

                self.injector.update(State::Paused).await;
            }
        }

        Ok(())
    }

    pub(super) async fn skip(&mut self, source: Source) -> Result<()> {
        if self.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        log::trace!("Skipping Song");

        match self.playback_mode {
            PlaybackMode::Default => {
                let state = self.injector.get::<State>().await.unwrap_or_default();
                let song = self.mixer.next_song().await?;

                match (song, state) {
                    (Some(song), State::Playing) => {
                        self.play_song(source, song).await?;
                    }
                    (Some(song), _) => {
                        self.switch_to_song(Some(song.clone())).await?;
                        self.notify_song_change(Some(&song)).await?;
                    }
                    (None, _) => {
                        if let Source::Manual = source {
                            self.bus.send_sync(Event::Empty);
                        }

                        self.switch_to_song(None).await?;
                        self.notify_song_change(None).await?;
                        self.injector.update(State::Paused).await;
                    }
                }
            }
            PlaybackMode::Queue => {
                self.connect_player.next().await?;

                if let Source::Manual = source {
                    self.bus.send_sync(Event::Skip);
                }
            }
        }

        Ok(())
    }

    /// Start playback on a specific song state.
    pub(super) async fn sync(&mut self, song: Song) -> Result<()> {
        log::trace!("Syncing Song");

        self.switch_current_player(song.player()).await?;

        let state = song.state();
        self.notify_song_change(Some(&song)).await?;
        self.injector.update(song).await;
        self.injector.update(state).await;
        Ok(())
    }

    /// Mark the queue as modified and load and notify resources appropriately.
    pub(super) async fn modified(&mut self, source: Source) -> Result<()> {
        if self.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        log::trace!("Pausing player");

        match self.playback_mode {
            PlaybackMode::Default => {
                if !self.injector.exists::<Song>().await {
                    if let Some(song) = self.mixer.next_song().await? {
                        self.play_song(source, song).await?;
                    }
                }

                self.global_bus.send(bus::Global::SongModified).await;
                self.bus.send_sync(Event::Modified);
            }
            _ => (),
        }

        Ok(())
    }

    /// Inject the given item to start playing _immediately_.
    pub(super) async fn inject(
        &mut self,
        source: Source,
        item: Arc<Item>,
        offset: Duration,
    ) -> Result<()> {
        if self.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        log::trace!("Pausing player");

        match self.playback_mode {
            PlaybackMode::Default => {
                // store the currently playing song in the sidelined slot.
                if let Some(mut song) = self.injector.clear::<Song>().await {
                    song.pause();
                    self.mixer.push_sidelined(song);
                }

                self.play_song(source, Song::new(item, offset)).await?;
            }
            PlaybackMode::Queue => match &item.track_id {
                &TrackId::Spotify(id) => {
                    self.connect_player.play(Some(id), Some(offset)).await?;
                    self.injector.update(State::Playing).await;
                }
                _ => {
                    log::info!("Can't inject playback of a non-spotify song.");
                }
            },
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
                        self.connect_player.play(Some(id), Some(elapsed)).await?;
                        self.switch_current_player(PlayerKind::Spotify).await?;
                        self.injector.update(State::Playing).await;
                    }
                    TrackId::YouTube(id) => {
                        self.youtube_player.play(elapsed, duration, id).await;
                        self.switch_current_player(PlayerKind::YouTube).await?;
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
            self.global_bus.send(bus::Global::song_progress(song)).await;

            if let Some(song) = song {
                if let TrackId::YouTube(ref id) = song.item.track_id {
                    self.youtube_player
                        .tick(song.elapsed(), song.duration(), id.to_string())
                        .await;
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

    /// Add the given track to the queue.
    ///
    /// Returns the item added.
    pub(super) async fn add_track(
        &mut self,
        user: &str,
        track_id: TrackId,
        bypass_constraints: bool,
        max_duration: Option<utils::Duration>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        match self.playback_mode {
            PlaybackMode::Default => {
                self.default_add_track(user, track_id, bypass_constraints, max_duration)
                    .await
            }
            PlaybackMode::Queue => {
                self.queue_add_track(user, track_id, bypass_constraints, max_duration)
                    .await
            }
        }
    }

    /// Default method for adding a track.
    async fn default_add_track(
        &mut self,
        user: &str,
        track_id: TrackId,
        bypass_constraints: bool,
        max_duration: Option<utils::Duration>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        let (user_count, len) = {
            if !bypass_constraints {
                if let Some(reason) = &self.closed {
                    return Err(AddTrackError::PlayerClosed(reason.clone()));
                }

                let max_queue_length = self.max_queue_length.load().await;

                // NB: moderator is allowed to violate max queue length.
                if self.mixer.len() >= max_queue_length as usize {
                    return Err(AddTrackError::QueueFull);
                }

                let duplicate_duration = self.duplicate_duration.load().await;

                if !duplicate_duration.is_empty() {
                    if let Some(last) = self
                        .mixer
                        .last_song_within(&track_id, duplicate_duration.clone())
                        .await
                        .map_err(AddTrackError::Error)?
                    {
                        let added_at = DateTime::from_utc(last.added_at, Utc);

                        return Err(AddTrackError::Duplicate(
                            added_at,
                            last.user,
                            duplicate_duration.as_std(),
                        ));
                    }
                }
            }

            let mut user_count = 0;
            let mut len = 0;

            for (index, i) in self.mixer.list().enumerate() {
                len += 1;

                if i.track_id == track_id {
                    return Err(AddTrackError::QueueContainsTrack(index));
                }

                if i.user.as_ref().map(|u| *u == user).unwrap_or_default() {
                    user_count += 1;
                }
            }

            (user_count, len)
        };

        let max_songs_per_user = self.max_songs_per_user.load().await;

        // NB: moderator is allowed to add more songs.
        if !bypass_constraints && user_count >= max_songs_per_user {
            return Err(AddTrackError::TooManyUserTracks(max_songs_per_user));
        }

        let item = convert_item(&*self.spotify, &*self.youtube, Some(user), &track_id, None)
            .await
            .map_err(AddTrackError::Error)?;

        let mut item = match item {
            Some(item) => item,
            None => return Err(AddTrackError::MissingAuth),
        };

        if !item.spotify_playable() {
            return Err(AddTrackError::NotPlayable)
        }

        if let Some(max_duration) = max_duration {
            let max_duration = max_duration.as_std();

            if item.duration > max_duration {
                item.duration = max_duration;
            }
        }

        let item = Arc::new(item);

        self.mixer
            .push_back(item.clone())
            .await
            .map_err(AddTrackError::Error)?;

        self.modified(Source::Manual)
            .await
            .map_err(AddTrackError::Error)?;

        Ok((Some(len), item))
    }

    /// Try to queue up a track.
    async fn queue_add_track(
        &mut self,
        user: &str,
        track_id: TrackId,
        _bypass_constraints: bool,
        _max_duration: Option<utils::Duration>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        let item = convert_item(&*self.spotify, &*self.youtube, Some(user), &track_id, None)
            .await
            .map_err(AddTrackError::Error)?;

        let item = match item {
            Some(item) => item,
            None => return Err(AddTrackError::MissingAuth),
        };

        match track_id {
            TrackId::Spotify(id) => {
                self.connect_player
                    .queue(id)
                    .await
                    .map_err(|e| AddTrackError::Error(e.into()))?;
            }
            TrackId::YouTube(..) => {
                return Err(AddTrackError::UnsupportedPlaybackMode);
            }
        }

        Ok((None, Arc::new(item)))
    }
}
