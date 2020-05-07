use crate::{
    api, bus, db, injector, prelude::*, settings, song_file::SongFile, spotify_id::SpotifyId,
    track_id::TrackId, utils,
};

pub(self) use self::{
    connect::{ConnectDevice, ConnectError, ConnectPlayer, ConnectStream},
    mixer::Mixer,
    playback_future::PlaybackFuture,
    player_internal::PlayerInternal,
    queue::Queue,
    youtube::YouTubePlayer,
};
pub use self::{item::Item, song::Song, track::Track};
use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use futures::channel::mpsc;
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::trace_span;
use tracing_futures::Instrument as _;

mod connect;
mod item;
mod mixer;
mod playback_future;
mod player_internal;
mod queue;
mod song;
mod track;
mod youtube;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Playing,
    Paused,
    // initial undefined state.
    None,
}

impl Default for State {
    fn default() -> Self {
        Self::None
    }
}

/// Event used by player integrations.
#[derive(Debug)]
pub enum IntegrationEvent {
    /// Indicate that the current device changed.
    DeviceChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Spotify,
    YouTube,
    None,
}

/// The source of action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// Event was generated automatically, don't broadcast feedback.
    Automatic,
    /// Event was generated from user input. Broadcast feedback.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(self) enum PlaybackMode {
    /// The default playback mode.
    #[serde(rename = "default")]
    Default,
    /// Enqueue the next song instead of playing it.
    ///
    /// Only valid for the Spotify player.
    #[serde(rename = "queue")]
    Queue,
}

impl Default for PlaybackMode {
    fn default() -> Self {
        Self::Default
    }
}

/// A volume modification.
pub enum ModifyVolume {
    Increase(u32),
    Decrease(u32),
    Set(u32),
}

impl ModifyVolume {
    /// Apply the given modification.
    pub(self) fn apply(self, v: u32) -> u32 {
        use self::ModifyVolume::*;

        let v = match self {
            Increase(n) => v.saturating_add(n),
            Decrease(n) => v.saturating_sub(n),
            Set(v) => v,
        };

        u32::min(100, v)
    }
}

#[derive(Debug)]
pub(self) enum Command {
    /// Skip the current song.
    Skip(Source),
    /// Toggle playback.
    Toggle(Source),
    /// Pause playback.
    Pause(Source),
    /// Start playback.
    Play(Source),
    /// Start playback on a specific song state.
    Sync { song: Song },
    /// The queue was modified.
    Modified(Source),
    /// Play the given item as a theme at the given offset.
    Inject(Source, Arc<Item>, Duration),
}

impl Command {
    /// Get the source of a command.
    pub(self) fn source(&self) -> Source {
        use self::Command::*;

        match *self {
            Skip(source)
            | Toggle(source)
            | Pause(source)
            | Play(source)
            | Modified(source)
            | Inject(source, ..) => source,
            Sync { .. } => Source::Automatic,
        }
    }
}

/// Converts a track into an Item.
///
/// Returns `None` if the service required to convert the item is not
/// authenticated.
async fn convert_item(
    spotify: &api::Spotify,
    youtube: &api::YouTube,
    user: Option<&str>,
    track_id: &TrackId,
    duration_override: Option<Duration>,
) -> Result<Option<Item>> {
    let (track, duration) = match track_id {
        TrackId::Spotify(id) => {
            if !spotify.token.is_ready().await {
                return Ok(None);
            }

            let track_id_string = id.to_base62();
            let track = spotify.track(track_id_string).await?;
            let duration = Duration::from_millis(track.duration_ms.into());

            (Track::Spotify { track }, duration)
        }
        TrackId::YouTube(id) => {
            if !youtube.token.is_ready().await {
                return Ok(None);
            }

            let video = youtube.videos_by_id(id, "contentDetails,snippet").await?;

            let video = match video {
                Some(video) => video,
                None => bail!("no video found for id `{}`", id),
            };

            let content_details = video
                .content_details
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("video does not have content details"))?;

            let duration = str::parse::<utils::PtDuration>(&content_details.duration)?;
            (Track::YouTube { video }, duration.into_std())
        }
    };

    let duration = match duration_override {
        Some(duration) => duration,
        None => duration,
    };

    Ok(Some(Item {
        track_id: track_id.clone(),
        track,
        user: user.map(|user| user.to_string()),
        duration,
    }))
}

/// Run the player.
pub async fn run(
    injector: injector::Injector,
    db: db::Database,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    global_bus: Arc<bus::Bus<bus::Global>>,
    youtube_bus: Arc<bus::Bus<bus::YouTube>>,
    settings: settings::Settings,
) -> Result<(Player, impl Future<Output = Result<()>>)> {
    let settings = settings.scoped("player");

    let mut futures = utils::Futures::default();

    let (connect_stream, connect_player, device, future) =
        self::connect::setup(spotify.clone(), settings.scoped("spotify")).await?;

    futures.push(
        future
            .instrument(trace_span!(target: "futures", "spotify"))
            .boxed(),
    );

    let (youtube_player, future) =
        self::youtube::setup(youtube_bus, settings.scoped("youtube")).await?;

    futures.push(
        future
            .instrument(trace_span!(target: "futures", "youtube"))
            .boxed(),
    );

    let bus = bus::Bus::new();
    let queue = Queue::new(db.clone());

    let closed = injector::Var::new(None);

    let (song_update_interval_stream, song_update_interval) = settings
        .stream("song-update-interval")
        .or_with(utils::Duration::seconds(1))
        .await?;

    let playback_mode = settings
        .var("playback-mode", PlaybackMode::default())
        .await?;

    let song_update_interval = if song_update_interval.is_empty() {
        None
    } else {
        Some(tokio::time::interval(song_update_interval.as_std()))
    };

    let (commands_tx, commands) = mpsc::unbounded();

    let (detached_stream, detached) = settings.stream("detached").or_default().await?;

    let duplicate_duration = settings
        .var("duplicate-duration", utils::Duration::default())
        .await?;
    let song_switch_feedback = settings.var("song-switch-feedback", true).await?;
    let max_songs_per_user = settings.var("max-songs-per-user", 2).await?;
    let max_queue_length = settings.var("max-queue-length", 30).await?;

    let parent_player = Player {
        inner: Arc::new(PlayerInner {
            injector: injector.clone(),
            device,
            queue: queue.clone(),
            connect_player: connect_player.clone(),
            youtube_player: youtube_player.clone(),
            max_queue_length,
            max_songs_per_user,
            duplicate_duration,
            spotify: spotify.clone(),
            youtube: youtube.clone(),
            commands_tx,
            bus: bus.clone(),
            themes: injector.var().await?,
            closed,
            playback_mode: playback_mode.clone(),
        }),
    };

    let player = parent_player.clone();

    // future to initialize the player future.
    // Yeah, I know....
    let future = async move {
        {
            // Add tracks from database.
            for song in db.player_list().await? {
                let item = convert_item(
                    &*spotify,
                    &*youtube,
                    song.user.as_deref(),
                    &song.track_id,
                    None,
                )
                .await;

                if let Ok(Some(item)) = item {
                    queue.push_back_queue(Arc::new(item)).await;
                } else {
                    log::warn!("failed to convert db item: {:?}", song);
                }
            }
        }

        let mixer = Mixer::new(queue);

        let (playback_mode_stream, playback_mode) = settings
            .stream("playback-mode")
            .or_with_else(PlaybackMode::default)
            .await?;

        let internal = Arc::new(RwLock::new(PlayerInternal {
            injector: injector.clone(),
            player: PlayerKind::None,
            detached,
            spotify: spotify.clone(),
            connect_player: connect_player.clone(),
            youtube_player,
            playback_mode,
            mixer,
            bus,
            global_bus,
            song_switch_feedback,
            song_timeout_at: None,
        }));

        let playback = PlaybackFuture {
            internal: internal.clone(),
            connect_stream,
            commands,
            playback_mode_stream,
            detached_stream,
            song_update_interval,
            song_update_interval_stream,
        };

        player.sync_spotify_playback().await?;

        futures.push(
            SongFile::run(injector.clone(), settings.scoped("song-file"))
                .instrument(trace_span!(target: "futures", "song-file"))
                .boxed(),
        );

        futures.push(
            playback
                .run(settings)
                .instrument(trace_span!(target: "futures", "playback"))
                .boxed(),
        );

        futures.select_next_some().await?;
        Ok(())
    };

    Ok((parent_player, future.boxed()))
}

/// Events emitted by the player.
#[derive(Debug, Clone)]
pub enum Event {
    /// Player is empty.
    Empty,
    /// Player is playing a song. If the song is known, it's provided.
    Playing(bool, Option<Arc<Item>>),
    /// The current song was skipped, and we don't know which song is playing
    /// next.
    Skip,
    /// Player is pausing.
    Pausing,
    /// queue was modified in some way.
    Modified,
    /// player has not been configured.
    NotConfigured,
    /// Player is detached.
    Detached,
}

/// Internal of the player.
pub struct PlayerInner {
    injector: injector::Injector,
    device: ConnectDevice,
    queue: Queue,
    connect_player: ConnectPlayer,
    youtube_player: YouTubePlayer,
    max_queue_length: settings::Var<u32>,
    max_songs_per_user: settings::Var<u32>,
    duplicate_duration: settings::Var<utils::Duration>,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    commands_tx: mpsc::UnboundedSender<Command>,
    bus: bus::Bus<Event>,
    /// Theme songs.
    themes: injector::Var<Option<db::Themes>>,
    /// Player is closed for more requests.
    closed: injector::Var<Option<Option<Arc<String>>>>,
    /// The current playback mode.
    playback_mode: settings::Var<PlaybackMode>,
}

/// All parts of a Player that can be shared between threads.
#[derive(Clone)]
pub struct Player {
    /// Player internals. Wrapped to make cloning cheaper since Player is frequently shared.
    inner: Arc<PlayerInner>,
}

impl Player {
    /// Get a receiver for player events.
    pub fn add_rx(&self) -> bus::Reader<Event> {
        self.inner.bus.add_rx()
    }

    /// Try to sync Spotify playback.
    pub async fn sync_spotify_playback(&self) -> Result<()> {
        if !self.inner.spotify.token.is_ready().await {
            return Ok(());
        }

        let p = match self.inner.spotify.me_player().await {
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
                    self.inner.device.sync_device(Some(p.device)).await?;
                    self.inner
                        .connect_player
                        .set_scaled_volume(volume_percent)
                        .await?;
                    self.play_sync(song)?;
                }
                None => {
                    log::trace!("Pausing playback since item is missing");
                    self.pause_with_source(Source::Automatic)?;
                }
            }
        }

        Ok(())
    }

    /// Synchronize playback with the given song.
    fn play_sync(&self, song: Song) -> Result<()> {
        self.send(Command::Sync { song })
    }

    /// Get the current device.
    pub async fn current_device(&self) -> Option<String> {
        self.inner.device.current_device().await
    }

    /// List all available devices.
    pub async fn list_devices(&self) -> Result<Vec<api::spotify::Device>> {
        self.inner.device.list_devices().await
    }

    /// External call to set device.
    ///
    /// Should always notify the player to change.
    pub async fn set_device(&self, device: String) -> Result<()> {
        self.inner.device.set_device(Some(device)).await
    }

    /// Clear the current device.
    pub async fn clear_device(&self) -> Result<()> {
        self.inner.device.set_device(None).await
    }

    /// Send the given command.
    fn send(&self, command: Command) -> Result<()> {
        self.inner
            .commands_tx
            .unbounded_send(command)
            .map_err(|_| anyhow!("failed to send command"))
    }

    /// Get the next N songs in queue.
    pub async fn list(&self) -> Vec<Arc<Item>> {
        let queue = self.inner.queue.queue.read().await;
        let song = self.inner.injector.get::<Song>().await;

        song.as_ref()
            .map(|c| c.item.clone())
            .into_iter()
            .chain(queue.iter().cloned())
            .collect()
    }

    /// Promote the given song to the head of the queue.
    pub async fn promote_song(&self, user: Option<&str>, n: usize) -> Result<Option<Arc<Item>>> {
        let promoted = self.inner.queue.promote_song(user, n).await?;

        if promoted.is_some() {
            self.modified();
        }

        Ok(promoted)
    }

    /// Toggle playback.
    pub fn toggle(&self) -> Result<()> {
        self.send(Command::Toggle(Source::Manual))
    }

    /// Start playback.
    pub fn play(&self) -> Result<()> {
        self.send(Command::Play(Source::Manual))
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<()> {
        self.send(Command::Pause(Source::Manual))
    }

    /// Pause playback.
    pub fn pause_with_source(&self, source: Source) -> Result<()> {
        self.send(Command::Pause(source))
    }

    /// Skip the current song.
    pub fn skip(&self) -> Result<()> {
        self.send(Command::Skip(Source::Manual))
    }

    /// Update volume of the player.
    pub async fn volume(&self, modify: ModifyVolume) -> Result<Option<u32>> {
        let track_id = match self.inner.injector.get::<Song>().await {
            Some(song) => song.item.track_id.clone(),
            None => {
                return Ok(None);
            }
        };

        match track_id {
            TrackId::Spotify(..) => match self.inner.connect_player.volume(modify).await {
                Err(ConnectError::NoDevice) => {
                    self.inner.bus.send_sync(Event::NotConfigured);
                    Ok(None)
                }
                Err(e) => Err(e.into()),
                Ok(volume) => Ok(Some(volume)),
            },
            TrackId::YouTube(..) => Ok(Some(self.inner.youtube_player.volume(modify).await?)),
        }
    }

    /// Get the current volume.
    pub async fn current_volume(&self) -> Option<u32> {
        let track_id = self
            .inner
            .injector
            .get::<Song>()
            .await
            .as_ref()?
            .item
            .track_id
            .clone();

        match track_id {
            TrackId::Spotify(..) => Some(self.inner.connect_player.current_volume().await),
            TrackId::YouTube(..) => Some(self.inner.youtube_player.current_volume().await),
        }
    }

    /// Close the player from more requests.
    pub async fn close(&self, reason: Option<String>) {
        *self.inner.closed.write().await = Some(reason.map(Arc::new));
    }

    /// Open the player.
    pub async fn open(&self) {
        *self.inner.closed.write().await = None;
    }

    /// Search for a track.
    pub async fn search_track(&self, q: &str) -> Result<Option<TrackId>> {
        if q.starts_with("youtube:") {
            let q = q.trim_start_matches("youtube:");
            let results = self.inner.youtube.search(q).await?;

            let result = results.items.into_iter().filter(|r| match r.id.kind {
                api::youtube::Kind::Video => true,
                _ => false,
            });

            let mut result = result.flat_map(|r| r.id.video_id);
            return Ok(result.next().map(TrackId::YouTube));
        }

        let q = if q.starts_with("spotify:") {
            q.trim_start_matches("spotify:")
        } else {
            q
        };

        let page = self.inner.spotify.search_track(q).await?;

        match page.items.into_iter().next().and_then(|t| t.id) {
            Some(track_id) => match SpotifyId::from_base62(&track_id) {
                Ok(track_id) => Ok(Some(TrackId::Spotify(track_id))),
                Err(_) => bail!("search result returned malformed id"),
            },
            None => Ok(None),
        }
    }

    /// Play a theme track.
    pub async fn play_theme(&self, channel: &str, name: &str) -> Result<(), PlayThemeError> {
        let themes = match self.inner.themes.load().await {
            Some(themes) => themes,
            None => return Err(PlayThemeError::NotConfigured),
        };

        let theme = match themes.get(channel, name).await {
            Some(theme) => theme,
            None => return Err(PlayThemeError::NoSuchTheme),
        };

        let duration = theme.end.clone().map(|o| o.as_duration());

        let item = convert_item(
            &*self.inner.spotify,
            &*self.inner.youtube,
            None,
            &theme.track_id,
            duration,
        )
        .await
        .map_err(PlayThemeError::Error)?;

        let item = match item {
            Some(item) => item,
            None => return Err(PlayThemeError::MissingAuth),
        };

        let item = Arc::new(item);
        let duration = theme.start.as_duration();

        self.inner
            .commands_tx
            .unbounded_send(Command::Inject(Source::Manual, item, duration))
            .map_err(|e| PlayThemeError::Error(e.into()))?;

        Ok(())
    }

    /// Add the given track to the queue.
    ///
    /// Returns the item added.
    pub async fn add_track(
        &self,
        user: &str,
        track_id: TrackId,
        bypass_constraints: bool,
        max_duration: Option<utils::Duration>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        match self.inner.playback_mode.load().await {
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
        &self,
        user: &str,
        track_id: TrackId,
        bypass_constraints: bool,
        max_duration: Option<utils::Duration>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        let (user_count, len) = {
            let queue_inner = self.inner.queue.queue.read().await;
            let len = queue_inner.len();

            if !bypass_constraints {
                if let Some(reason) = self.inner.closed.read().await.as_ref() {
                    return Err(AddTrackError::PlayerClosed(reason.clone()));
                }

                let max_queue_length = self.inner.max_queue_length.load().await;

                // NB: moderator is allowed to violate max queue length.
                if len >= max_queue_length as usize {
                    return Err(AddTrackError::QueueFull);
                }

                let duplicate_duration = self.inner.duplicate_duration.load().await;

                if !duplicate_duration.is_empty() {
                    if let Some(last) = self
                        .inner
                        .queue
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

            for (index, i) in queue_inner.iter().enumerate() {
                if i.track_id == track_id {
                    return Err(AddTrackError::QueueContainsTrack(index));
                }

                if i.user.as_ref().map(|u| *u == user).unwrap_or_default() {
                    user_count += 1;
                }
            }

            (user_count, len)
        };

        let max_songs_per_user = self.inner.max_songs_per_user.load().await;

        // NB: moderator is allowed to add more songs.
        if !bypass_constraints && user_count >= max_songs_per_user {
            return Err(AddTrackError::TooManyUserTracks(max_songs_per_user));
        }

        let item = convert_item(
            &*self.inner.spotify,
            &*self.inner.youtube,
            Some(user),
            &track_id,
            None,
        )
        .await
        .map_err(AddTrackError::Error)?;

        let mut item = match item {
            Some(item) => item,
            None => return Err(AddTrackError::MissingAuth),
        };

        if let Some(max_duration) = max_duration {
            let max_duration = max_duration.as_std();

            if item.duration > max_duration {
                item.duration = max_duration;
            }
        }

        let item = Arc::new(item);

        self.inner
            .queue
            .push_back(item.clone())
            .await
            .map_err(AddTrackError::Error)?;

        self.inner
            .commands_tx
            .unbounded_send(Command::Modified(Source::Manual))
            .map_err(|e| AddTrackError::Error(e.into()))?;

        Ok((Some(len), item))
    }

    /// Try to queue up a track.
    async fn queue_add_track(
        &self,
        user: &str,
        track_id: TrackId,
        _bypass_constraints: bool,
        _max_duration: Option<utils::Duration>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        let item = convert_item(
            &*self.inner.spotify,
            &*self.inner.youtube,
            Some(user),
            &track_id,
            None,
        )
        .await
        .map_err(AddTrackError::Error)?;

        let item = match item {
            Some(item) => item,
            None => return Err(AddTrackError::MissingAuth),
        };

        match track_id {
            TrackId::Spotify(id) => {
                self.inner
                    .connect_player
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

    /// Remove the first track in the queue.
    pub fn remove_first(&self) -> Result<Option<Arc<Item>>> {
        Ok(None)
    }

    pub async fn purge(&self) -> Result<Vec<Arc<Item>>> {
        let purged = self.inner.queue.purge().await?;

        if !purged.is_empty() {
            self.modified();
        }

        Ok(purged)
    }

    /// Remove the item at the given position.
    pub async fn remove_at(&self, n: usize) -> Result<Option<Arc<Item>>> {
        let removed = self.inner.queue.remove_at(n).await?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Remove the first track in the queue.
    pub async fn remove_last(&self) -> Result<Option<Arc<Item>>> {
        let removed = self.inner.queue.remove_last().await?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Remove the last track by the given user.
    pub async fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>> {
        let removed = self.inner.queue.remove_last_by_user(user).await?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Find the next item that matches the given predicate and how long until it plays.
    pub async fn find(
        &self,
        mut predicate: impl FnMut(&Item) -> bool,
    ) -> Option<(Duration, Arc<Item>)> {
        let mut duration = Duration::default();

        if let Some(c) = self.inner.injector.get::<Song>().await {
            if predicate(&c.item) {
                return Some((Default::default(), c.item.clone()));
            }

            duration += c.remaining();
        }

        let queue = self.inner.queue.queue.read().await;

        for item in &*queue {
            if predicate(item) {
                return Some((duration, item.clone()));
            }

            duration += item.duration;
        }

        None
    }

    /// Get the length in number of items and total number of seconds in queue.
    pub async fn length(&self) -> (usize, Duration) {
        let mut count = 0;
        let mut duration = Duration::default();

        if let Some(song) = self.inner.injector.get::<Song>().await.as_ref() {
            duration += song.remaining();
            count += 1;
        }

        let queue = self.inner.queue.queue.read().await;

        for item in &*queue {
            duration += item.duration;
        }

        count += queue.len();
        (count, duration)
    }

    /// Get the current song, if it is set.
    pub async fn current(&self) -> Option<Song> {
        self.inner.injector.get::<Song>().await
    }

    /// Indicate that the queue has been modified.
    fn modified(&self) {
        if let Err(e) = self
            .inner
            .commands_tx
            .unbounded_send(Command::Modified(Source::Manual))
        {
            log::error!("failed to send queue modified notification: {}", e);
        }
    }
}

/// Error raised when failing to play a theme song.
pub enum PlayThemeError {
    /// No such theme song.
    NoSuchTheme,
    /// Themes system is not configured.
    NotConfigured,
    /// Authentication missing to play the given theme.
    MissingAuth,
    /// Other generic error happened.
    Error(anyhow::Error),
}

/// Error raised when trying to add track.
pub enum AddTrackError {
    /// Queue is full.
    QueueFull,
    /// Queue already contains track.
    QueueContainsTrack(usize),
    /// Too many user tracks.
    TooManyUserTracks(u32),
    /// Player has been closed from adding more tracks to the queue with an optional reason.
    PlayerClosed(Option<Arc<String>>),
    /// Duplicate song that was added at the specified time by the specified user.
    Duplicate(DateTime<Utc>, Option<String>, Duration),
    /// Authentication missing for adding the given track.
    MissingAuth,
    /// Playback mode is not supported for the given track.
    UnsupportedPlaybackMode,
    /// Other generic error happened.
    Error(anyhow::Error),
}
