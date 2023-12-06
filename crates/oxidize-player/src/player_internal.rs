use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_injector::Injector;
use chrono::{DateTime, Utc};
use common::models::spotify::context::FullPlayingContext;
use common::models::spotify::track::FullTrack;
use common::models::spotify::user::PrivateUser;
use common::models::{SpotifyId, State, TrackId};
use common::stream::Stream;
use common::stream::StreamExt;
use common::Uri;
use parking_lot::Mutex;

use crate::{
    convert_item, AddTrackError, ConnectDevice, ConnectPlayer, DuplicateBy, Event,
    IntegrationEvent, Item, Mixer, PlaybackMode, PlayerKind, Song, Source, Track, YouTubePlayer,
};

#[derive(Default)]
pub(super) struct PlayerInitialize {
    /// Queue is initialized.
    pub(super) queue: bool,
    /// Playback is initialized.
    pub(super) playback: bool,
}

pub(super) struct PlayerInternal {
    /// Player state.
    pub(super) state: Mutex<PlayerState>,
    /// Player is closed for more requests.
    pub(super) closed: Mutex<Option<Option<Arc<String>>>>,
    /// Injector.
    pub(super) injector: Injector,
    /// API clients and streams.
    pub(super) spotify: Arc<api::Spotify>,
    pub(super) youtube: Arc<api::YouTube>,
    pub(super) connect_player: ConnectPlayer,
    pub(super) youtube_player: YouTubePlayer,
    /// The internal mixer.
    pub(super) mixer: Mixer,
    /// The player bus.
    pub(super) bus: bus::Bus<Event>,
    /// Notifier to use when sending song updates.
    pub(super) global_bus: bus::Bus<bus::Global>,
    /// Song config.
    pub(super) song_switch_feedback: settings::Var<bool>,
    pub(super) device: ConnectDevice,
    pub(super) max_queue_length: settings::Var<u32>,
    pub(super) max_songs_per_user: settings::Var<u32>,
    pub(super) duplicate_duration: settings::Var<common::Duration>,
    /// Theme songs.
    pub(super) themes: async_injector::Ref<db::Themes>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PlayerState {
    /// Current player kind.
    pub(super) player: PlayerKind,
    /// Updated to the current playback mode.
    /// Player is detached.
    pub(super) detached: bool,
    /// The mode of the player.
    ///
    /// The mode determines if the player is enqueueing songs or immediately
    /// playing them.
    pub(super) mode: PlaybackMode,
}

impl PlayerInternal {
    fn state(&self) -> PlayerState {
        *self.state.lock()
    }

    /// Initialize the internal player if necessary.
    pub(crate) async fn initialize(&self, initialize: &mut PlayerInitialize) -> Result<()> {
        /// Convert a playback information into a Song struct.
        fn from_playback(playback: &FullPlayingContext) -> Option<Song> {
            let progress_ms = playback.progress_ms.unwrap_or_default();

            let track = match playback.item.clone() {
                Some(track) => track,
                _ => {
                    tracing::warn!("No playback item in current playback");
                    return None;
                }
            };

            let track_id = match &track.id {
                Some(track_id) => track_id,
                None => {
                    tracing::warn!("Current playback doesn't have a track id");
                    return None;
                }
            };

            let track_id = match SpotifyId::from_base62(track_id) {
                Ok(spotify_id) => TrackId::Spotify(spotify_id),
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse track id from current playback: {}: {}",
                        track_id,
                        e
                    );
                    return None;
                }
            };

            let elapsed = Duration::from_millis(progress_ms as u64);
            let duration = Duration::from_millis(track.duration_ms.into());

            let item = Arc::new(Item::new(
                track_id,
                Track::Spotify {
                    track: Box::new(track),
                },
                None,
                duration,
            ));

            let mut song = Song::new(item, elapsed);

            if playback.is_playing {
                song.play();
            } else {
                song.pause();
            }

            Some(song)
        }

        if !initialize.playback {
            tracing::trace!("Waiting until token is ready");
            self.spotify.token().wait_until_ready().await;

            let p = self.spotify.me_player().await?;

            if let Some(p) = p {
                tracing::trace!("Detected Spotify playback: {:?}", p);

                match from_playback(&p) {
                    Some(song) => {
                        tracing::trace!("Syncing playback");
                        let volume_percent = p.device.volume_percent;
                        self.device.sync_device(Some(p.device)).await?;
                        self.connect_player.set_scaled_volume(volume_percent).await;
                        self.sync(song).await?;
                    }
                    None => {
                        tracing::trace!("Pausing playback since item is missing");
                        self.pause(Source::Automatic).await?;
                    }
                }
            }

            initialize.playback = true;
        }

        if !initialize.queue {
            self.mixer
                .initialize_queue(&self.spotify, &self.youtube)
                .await?;

            initialize.queue = true;
        }

        Ok(())
    }

    /// Check if the player is unmanaged.
    ///
    /// An unmanaged player doesn't process default commands that deal with the
    /// internal player.
    fn is_unmanaged(&self) -> bool {
        let state = self.state();

        if state.detached {
            return true;
        }

        state.mode == PlaybackMode::Queue
    }

    /// We've reached the end of track, process it.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn end_of_track(&self) -> Result<()> {
        if self.is_unmanaged() {
            tracing::warn!("End of track called even though we are no longer managing the player");
            return Ok(());
        }

        tracing::trace!("Song ended, loading next song...");

        if let Some(song) = self.mixer.next_song().await? {
            self.play_song(Source::Manual, song).await?;
        } else {
            self.bus.send_sync(Event::Empty);
            self.notify_song_change(None).await?;
        }

        Ok(())
    }

    /// Notify a change in the current song.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    async fn notify_song_change(&self, song: Option<&Song>) -> Result<()> {
        tracing::trace!("Notify song change");
        self.global_bus.send(bus::Global::song(song)?).await;
        self.global_bus.send(bus::Global::SongModified).await;
        Ok(())
    }

    /// Switch the current player and send the appropriate play commands.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    async fn switch_current_player(&self, player: PlayerKind) -> Result<()> {
        use common::models::PlayerKind::*;

        tracing::trace!("Switch current player");
        let state = self.state();

        match (state.player, player) {
            (Spotify, Spotify) => (),
            (YouTube, YouTube) => (),
            (Spotify, _) | (None, YouTube) => {
                self.connect_player.stop().await;
            }
            (YouTube, _) | (None, Spotify) => {
                self.youtube_player.stop().await;
            }
            (None, None) => (),
        }

        self.state.lock().player = player;
        Ok(())
    }

    /// Send a pause command to the appropriate player.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    async fn send_pause_command(&self) {
        tracing::trace!("Sending pause command");

        match self.state().player {
            PlayerKind::Spotify => {
                tracing::trace!("Pausing Spotify player");
                self.connect_player.pause().await;
            }
            PlayerKind::YouTube => {
                tracing::trace!("Pausing YouTube player");
                self.youtube_player.pause().await;
            }
            _ => (),
        }
    }

    /// Play the given song.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    async fn send_play_command(&self, song: &Song) {
        tracing::trace!("Sending play command");

        match song.item().track_id().clone() {
            TrackId::Spotify(id) => {
                self.connect_player
                    .play(Some(id), Some(song.elapsed()))
                    .await;
            }
            TrackId::YouTube(id) => {
                self.youtube_player
                    .play(song.elapsed(), song.item().duration(), id)
                    .await;
            }
        }
    }

    /// Switch the player to the specified song without changing its state.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    async fn switch_to_song(&self, mut song: Option<Song>) -> Result<()> {
        tracing::trace!("Switching to song");

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
    #[tracing::instrument(skip(self))]
    async fn play_song(&self, source: Source, mut song: Song) -> Result<()> {
        tracing::trace!("Playing song");

        song.play();

        self.send_play_command(&song).await;
        self.switch_current_player(song.player()).await?;
        self.notify_song_change(Some(&song)).await?;

        if let Source::Manual = source {
            let feedback = self.song_switch_feedback.load().await;
            self.bus
                .send_sync(Event::Playing(feedback, Some(song.item().clone())));
        }

        self.injector.update(State::Playing).await;
        self.injector.update(song).await;
        Ok(())
    }

    /// Resume playing a specific song.
    #[tracing::instrument(skip(self))]
    async fn resume_song(&self, source: Source, song: Song) -> Result<()> {
        tracing::trace!("Resuming song");

        self.send_play_command(&song).await;
        self.switch_current_player(song.player()).await?;
        self.notify_song_change(Some(&song)).await?;

        if let Source::Manual = source {
            let feedback = self.song_switch_feedback.load().await;
            self.bus
                .send_sync(Event::Playing(feedback, Some(song.item().clone())));
        }

        self.injector.update(State::Playing).await;
        self.injector.update(song).await;
        Ok(())
    }

    /// Detach the player.
    #[tracing::instrument(skip(self))]
    async fn detach(&self) -> Result<()> {
        tracing::trace!("Detaching");

        self.state.lock().player = PlayerKind::None;
        self.injector.update(State::None).await;

        // store the currently playing song in the sidelined slot.
        if let Some(mut song) = self.injector.clear::<Song>().await {
            song.pause();
            self.mixer.push_sidelined(song);
        }

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn toggle(&self, source: Source) -> Result<()> {
        tracing::trace!("Toggling");

        let state = self.injector.get::<State>().await.unwrap_or_default();

        match state {
            State::Paused | State::None => self.play(source).await?,
            State::Playing => self.pause(source).await?,
        }

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn play(&self, source: Source) -> Result<()> {
        tracing::trace!("Playing");

        let state = self.state();

        if state.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        match state.mode {
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
                self.connect_player.play(None, None).await;

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
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn pause(&self, source: Source) -> Result<()> {
        tracing::trace!("Pausing Player");

        let state = self.state();

        if state.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        match state.mode {
            PlaybackMode::Default => {
                self.send_pause_command().await;
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
                self.connect_player.pause().await;

                if let Source::Manual = source {
                    self.bus.send_sync(Event::Pausing);
                }

                self.injector.update(State::Paused).await;
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn skip(&self, source: Source) -> Result<()> {
        tracing::trace!("Skipping Song");

        let state = self.state();

        if state.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        match state.mode {
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
                self.connect_player.next().await;

                if let Source::Manual = source {
                    self.bus.send_sync(Event::Skip);
                }
            }
        }

        Ok(())
    }

    /// Start playback on a specific song state.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn sync(&self, song: Song) -> Result<()> {
        tracing::trace!("Syncing Song");

        self.switch_current_player(song.player()).await?;

        let state = song.state();
        self.notify_song_change(Some(&song)).await?;
        self.injector.update(song).await;
        self.injector.update(state).await;
        Ok(())
    }

    /// Mark the queue as modified and load and notify resources appropriately.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn modified(&self, source: Source) -> Result<()> {
        tracing::trace!("Modified player");

        let state = self.state();

        if state.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        if let PlaybackMode::Default = state.mode {
            if !self.injector.exists::<Song>().await {
                if let Some(song) = self.mixer.next_song().await? {
                    self.play_song(source, song).await?;
                }
            }

            self.global_bus.send(bus::Global::SongModified).await;
            self.bus.send_sync(Event::Modified);
        }

        Ok(())
    }

    /// Inject the given item to start playing _immediately_.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn inject(
        &self,
        source: Source,
        item: Arc<Item>,
        offset: Duration,
    ) -> Result<()> {
        tracing::trace!("Injecting song");

        let state = self.state();

        if state.detached {
            if let Source::Manual = source {
                self.bus.send_sync(Event::Detached);
            }

            return Ok(());
        }

        match state.mode {
            PlaybackMode::Default => {
                // store the currently playing song in the sidelined slot.
                if let Some(mut song) = self.injector.clear::<Song>().await {
                    song.pause();
                    self.mixer.push_sidelined(song);
                }

                self.play_song(source, Song::new(item, offset)).await?;
            }
            PlaybackMode::Queue => match item.track_id() {
                &TrackId::Spotify(id) => {
                    self.connect_player.play(Some(id), Some(offset)).await;
                    self.injector.update(State::Playing).await;
                }
                _ => {
                    tracing::info!("Can't inject playback of a non-spotify song.");
                }
            },
        }

        Ok(())
    }

    /// Update fallback items.
    #[tracing::instrument(skip_all, fields(state = ?self.state(), items = items.len()))]
    pub(super) async fn update_fallback_items(&self, items: Vec<Arc<Item>>) {
        self.mixer.update_fallback_items(items).await;
    }

    /// Load fallback items with the given uri.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn load_fallback_items(
        &self,
        uri: Option<&Uri>,
    ) -> Result<(String, Vec<Arc<Item>>)> {
        tracing::trace!("Loading fallback items");

        let (what, items) = match uri {
            Some(uri) => {
                let id = match uri {
                    Uri::SpotifyPlaylist(id) => id,
                    uri => {
                        return Err(anyhow!(
                            "Bad fallback URI `{}`, expected Spotify Playlist",
                            uri
                        ));
                    }
                };

                let (name, items) = download_spotify_playlist(&self.spotify, *id).await?;
                let items = convert(items).await?;
                (Some(name), items)
            }
            None => {
                let items = download_spotify_library(&self.spotify);
                let items = convert(items).await?;
                (None, items)
            }
        };

        let what = what
            .as_ref()
            .map(|u| format!("\"{}\" playlist", u))
            .unwrap_or_else(|| String::from("your library"));

        return Ok((what, items));

        async fn convert(stream: impl Stream<Item = Result<FullTrack>>) -> Result<Vec<Arc<Item>>> {
            let mut stream = pin!(stream);

            let mut items = Vec::new();

            while let Some(track) = stream.next().await.transpose()? {
                let track_id = match &track.id {
                    Some(track_id) => track_id,
                    None => {
                        continue;
                    }
                };

                let track_id = TrackId::Spotify(
                    SpotifyId::from_base62(track_id)
                        .map_err(|_| anyhow!("bad spotify id: {}", track_id))?,
                );

                let duration = Duration::from_millis(track.duration_ms.into());

                let item = Item::new(
                    track_id,
                    Track::Spotify {
                        track: Box::new(track),
                    },
                    None,
                    duration,
                );

                if item.is_playable() {
                    items.push(Arc::new(item));
                }
            }

            Ok(items)
        }

        /// Download a playlist from Spotify.
        async fn download_spotify_playlist(
            spotify: &api::Spotify,
            playlist: SpotifyId,
        ) -> Result<(String, impl Stream<Item = Result<FullTrack>> + '_)> {
            let streamer = spotify.me().await?;

            let playlist = spotify
                .playlist(playlist, streamer.country.as_deref())
                .await?;

            let name = playlist.name.to_string();

            let items = async_stream::try_stream! {
                let mut playlist_tracks = pin!(spotify.page_as_stream(playlist.tracks));

                while let Some(playlist_track) = playlist_tracks.next().await.transpose()? {
                    yield playlist_track.track;
                }
            };

            Ok((name, items))
        }

        /// Download a spotify library.
        fn download_spotify_library(
            spotify: &api::Spotify,
        ) -> impl Stream<Item = Result<FullTrack>> + '_ {
            async_stream::try_stream! {
                let saved_tracks = spotify.my_tracks().await?;
                let mut saved_tracks = pin!(spotify.page_as_stream(saved_tracks));

                while let Some(track) = saved_tracks.next().await.transpose()? {
                    yield track.track;
                }
            }
        }
    }

    /// Handle an event from the connect integration.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    pub(super) async fn handle_player_event(&self, e: IntegrationEvent) -> Result<()> {
        use IntegrationEvent::*;

        tracing::trace!("Handling player event");

        let state = self.injector.get::<State>().await.unwrap_or_default();

        if self.state().detached {
            return Ok(());
        }

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
                            (
                                song.elapsed(),
                                song.item().duration(),
                                song.item().track_id().clone(),
                            )
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
                        self.connect_player.play(Some(id), Some(elapsed)).await;
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
    pub(super) async fn song_update(&self) {
        if let State::Playing = self.injector.get::<State>().await.unwrap_or_default() {
            let song = self.injector.get::<Song>().await;
            let song = song.as_ref();
            self.global_bus.send(bus::Global::song_progress(song)).await;

            if let Some(song) = song {
                if let TrackId::YouTube(id) = song.item().track_id() {
                    self.youtube_player
                        .tick(song.elapsed(), song.item().duration(), id.to_string())
                        .await;
                }
            }
        }
    }

    /// Update the detached state.
    pub(super) async fn update_detached(&self, detached: bool) -> Result<()> {
        let state = self.state();

        if state.detached {
            self.detach().await?;
        }

        self.state.lock().detached = detached;
        Ok(())
    }

    /// Update the current playback mode.
    pub(super) async fn update_playback_mode(&self, mode: PlaybackMode) -> Result<()> {
        self.state.lock().mode = mode;

        if let PlaybackMode::Queue = mode {
            self.detach().await?;
        }

        Ok(())
    }

    /// Add the given track to the queue.
    ///
    /// Returns the item added.
    pub(super) async fn add_track(
        &self,
        user: &str,
        track_id: TrackId,
        bypass_constraints: bool,
        max_duration: Option<common::Duration>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        // TODO: cache this value
        let streamer: PrivateUser = self.spotify.me().await.map_err(AddTrackError::Error)?;
        let market = streamer.country.as_deref();

        match self.state().mode {
            PlaybackMode::Default => {
                self.default_add_track(user, track_id, bypass_constraints, max_duration, market)
                    .await
            }
            PlaybackMode::Queue => {
                self.queue_add_track(user, track_id, bypass_constraints, max_duration, market)
                    .await
            }
        }
    }

    /// Default method for adding a track.
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    async fn default_add_track(
        &self,
        user: &str,
        track_id: TrackId,
        bypass_constraints: bool,
        max_duration: Option<common::Duration>,
        market: Option<&str>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        tracing::trace!("Add track");

        let (user_count, len) = {
            if !bypass_constraints {
                let closed = (*self.closed.lock()).as_ref().cloned();

                if let Some(reason) = closed {
                    return Err(AddTrackError::PlayerClosed(reason));
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
                        .last_song_within(&track_id, duplicate_duration)
                        .await
                        .map_err(AddTrackError::Error)?
                    {
                        let added_at =
                            DateTime::<Utc>::from_naive_utc_and_offset(last.added_at, Utc);
                        let duration_since =
                            Utc::now().signed_duration_since(added_at).to_std().ok();

                        let duplicate_by = match last.user {
                            Some(who) if who == user => DuplicateBy::Requester,
                            Some(who) => DuplicateBy::Other(who),
                            None => DuplicateBy::Unknown,
                        };

                        return Err(AddTrackError::Duplicate {
                            duplicate_by,
                            duration_since,
                            duplicate_duration: duplicate_duration.as_std(),
                        });
                    }
                }
            }

            let mut user_count = 0;
            let mut len = 0;

            let items = self.mixer.queue().await;

            for (index, i) in items.iter().enumerate() {
                len += 1;

                if *i.track_id() == track_id {
                    return Err(AddTrackError::QueueContainsTrack(index));
                }

                if i.user().map(|u| *u == user).unwrap_or_default() {
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

        let item = convert_item(
            &self.spotify,
            &self.youtube,
            Some(user),
            &track_id,
            None,
            market,
        )
        .await
        .map_err(AddTrackError::Error)?;

        let mut item = match item {
            Some(item) => item,
            None => return Err(AddTrackError::MissingAuth),
        };

        if !item.is_playable() {
            return Err(AddTrackError::NotPlayable);
        }

        if let Some(max_duration) = max_duration {
            let max_duration = max_duration.as_std();

            if item.duration() > max_duration {
                item.set_duration(max_duration);
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
    #[tracing::instrument(skip(self), fields(state = ?self.state()))]
    async fn queue_add_track(
        &self,
        user: &str,
        track_id: TrackId,
        _bypass_constraints: bool,
        _max_duration: Option<common::Duration>,
        market: Option<&str>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        tracing::trace!("Add track");

        let item = convert_item(
            &self.spotify,
            &self.youtube,
            Some(user),
            &track_id,
            None,
            market,
        )
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
