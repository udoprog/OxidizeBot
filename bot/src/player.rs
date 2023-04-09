use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use parking_lot::Mutex;

use crate::api;
use crate::bus;
use crate::channel::Channel;
use crate::db;
use crate::prelude::*;
use crate::song_file::SongFile;
use crate::spotify_id::SpotifyId;
use crate::track_id::TrackId;
use crate::utils;

use self::connect::{ConnectDevice, ConnectPlayer, ConnectStream};
use self::mixer::Mixer;
use self::playback_future::PlaybackFuture;
use self::player_internal::{PlayerInitialize, PlayerInternal, PlayerState};
use self::youtube::YouTubePlayer;
pub(crate) use self::{item::Item, song::Song, track::Track};

mod connect;
mod item;
mod mixer;
mod playback_future;
mod player_internal;
mod song;
mod track;
mod youtube;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
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
pub(crate) enum IntegrationEvent {
    /// Indicate that the current device changed.
    DeviceChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerKind {
    Spotify,
    YouTube,
    None,
}

/// The source of action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Source {
    /// Event was generated automatically, don't broadcast feedback.
    Automatic,
    /// Event was generated from user input. Broadcast feedback.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
enum PlaybackMode {
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
pub(crate) enum ModifyVolume {
    Increase(u32),
    Decrease(u32),
    Set(u32),
}

impl ModifyVolume {
    /// Apply the given modification.
    fn apply(self, v: u32) -> u32 {
        use self::ModifyVolume::*;

        let v = match self {
            Increase(n) => v.saturating_add(n),
            Decrease(n) => v.saturating_sub(n),
            Set(v) => v,
        };

        u32::min(100, v)
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
    market: Option<&str>,
) -> Result<Option<Item>> {
    let (track, duration) = match track_id {
        TrackId::Spotify(id) => {
            if !spotify.token.is_ready().await {
                return Ok(None);
            }

            let track_id_string = id.to_base62();
            let track = spotify.track(track_id_string, market).await?;
            let duration = Duration::from_millis(track.duration_ms.into());

            (
                Track::Spotify {
                    track: Box::new(track),
                },
                duration,
            )
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
            (
                Track::YouTube {
                    video: Box::new(video),
                },
                duration.into_std(),
            )
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
#[tracing::instrument(skip_all)]
pub(crate) async fn run(
    injector: &Injector,
    db: db::Database,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    global_bus: bus::Bus<bus::Global>,
    youtube_bus: bus::Bus<bus::YouTube>,
    settings: crate::Settings,
) -> Result<impl Future<Output = Result<()>>> {
    let settings = settings.scoped("player");

    let mut futures = utils::Futures::default();

    let (connect_stream, connect_player, device, future) =
        self::connect::setup(spotify.clone(), settings.scoped("spotify")).await?;

    futures.push(Box::pin(future));

    let (youtube_player, future) =
        self::youtube::setup(youtube_bus, settings.scoped("youtube")).await?;

    futures.push(Box::pin(future));

    futures.push(Box::pin(SongFile::run(
        injector.clone(),
        settings.scoped("song-file"),
    )));

    let bus = bus::Bus::new();

    let (song_update_interval_stream, song_update_interval) = settings
        .stream("song-update-interval")
        .or_with(utils::Duration::seconds(1))
        .await?;

    let song_update_interval = if song_update_interval.is_empty() {
        Fuse::empty()
    } else {
        Fuse::new(tokio::time::interval(song_update_interval.as_std()))
    };

    let (detached_stream, detached) = settings.stream("detached").or_default().await?;

    let duplicate_duration = settings
        .var("duplicate-duration", utils::Duration::default())
        .await?;
    let song_switch_feedback = settings.var("song-switch-feedback", true).await?;
    let max_songs_per_user = settings.var("max-songs-per-user", 2).await?;
    let max_queue_length = settings.var("max-queue-length", 30).await?;

    let mixer = Mixer::new(db.clone());

    let (playback_mode_stream, mode) = settings
        .stream("playback-mode")
        .or_with_else(PlaybackMode::default)
        .await?;

    let internal = Arc::new(PlayerInternal {
        state: Mutex::new(PlayerState {
            player: PlayerKind::None,
            detached,
            mode,
        }),
        closed: Mutex::new(None),
        injector: injector.clone(),
        spotify: spotify.clone(),
        youtube: youtube.clone(),
        connect_player: connect_player.clone(),
        youtube_player,
        mixer,
        bus,
        global_bus,
        song_switch_feedback,
        device,
        max_queue_length,
        max_songs_per_user,
        duplicate_duration,
        themes: injector.var().await,
    });

    let playback = PlaybackFuture {
        internal: internal.clone(),
        connect_stream,
        playback_mode_stream,
        detached_stream,
        song_update_interval,
        song_update_interval_stream,
    };

    futures.push(Box::pin(playback.run(injector.clone(), settings)));

    injector
        .update(Player {
            inner: internal.clone(),
        })
        .await;

    // future to initialize the player future.
    // Yeah, I know....
    Ok(async move {
        let mut initialize = PlayerInitialize::default();

        // Note: these tasks might fail sporadically, since we need to perform external
        // API calls to initialize metadata for playback items.
        retry_until_ok! {
            "Initialize Player", {
                internal.initialize(&mut initialize).await
            }
        };

        tracing::info!("Player is up and running!");

        // Drive child futures now that initialization is done.
        if let Some(result) = futures.next().await {
            result?;
        }

        Ok(())
    })
}

/// Events emitted by the player.
#[derive(Debug, Clone)]
pub(crate) enum Event {
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
    /// Player is detached.
    Detached,
}

/// All parts of a Player that can be shared between threads.
#[derive(Clone)]
pub(crate) struct Player {
    /// Player internals. Wrapped to make cloning cheaper since Player is frequently shared.
    inner: Arc<PlayerInternal>,
}

impl Player {
    /// Get a receiver for player events.
    pub(crate) async fn subscribe(&self) -> bus::Reader<Event> {
        self.inner.bus.subscribe()
    }

    /// Get the current device.
    pub(crate) async fn current_device(&self) -> Option<String> {
        self.inner.device.current_device().await
    }

    /// List all available devices.
    pub(crate) async fn list_devices(&self) -> Result<Vec<api::spotify::Device>> {
        self.inner.device.list_devices().await
    }

    /// External call to set device.
    ///
    /// Should always notify the player to change.
    pub(crate) async fn set_device(&self, device: String) -> Result<()> {
        self.inner.device.set_device(Some(device)).await
    }

    /// Get the next N songs in queue.
    pub(crate) async fn list(&self) -> Vec<Arc<Item>> {
        let items = (*self.inner.mixer.queue().await).clone();
        let song = self.inner.injector.get::<Song>().await;

        song.as_ref()
            .map(|c| c.item.clone())
            .into_iter()
            .chain(items)
            .collect()
    }

    /// Promote the given song to the head of the queue.
    pub(crate) async fn promote_song(
        &self,
        user: Option<&str>,
        n: usize,
    ) -> Result<Option<Arc<Item>>> {
        let promoted = self.inner.mixer.promote_song(user, n).await?;

        if promoted.is_some() {
            self.inner.modified(Source::Manual).await?;
        }

        Ok(promoted)
    }

    /// Toggle playback.
    pub(crate) async fn toggle(&self) -> Result<()> {
        self.inner.toggle(Source::Manual).await?;
        Ok(())
    }

    /// Start playback.
    pub(crate) async fn play(&self) -> Result<()> {
        self.inner.play(Source::Manual).await?;
        Ok(())
    }

    /// Pause playback.
    pub(crate) async fn pause(&self) -> Result<()> {
        self.inner.pause(Source::Manual).await?;
        Ok(())
    }

    /// Skip the current song.
    pub(crate) async fn skip(&self) -> Result<()> {
        self.inner.skip(Source::Manual).await?;
        Ok(())
    }

    /// Get the current volume.
    pub(crate) async fn current_volume(&self) -> Option<u32> {
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

    /// Update volume of the player.
    pub(super) async fn volume(&self, modify: ModifyVolume) -> Option<u32> {
        let track_id = match self.inner.injector.get::<Song>().await {
            Some(song) => song.item.track_id.clone(),
            None => {
                return None;
            }
        };

        Some(match track_id {
            TrackId::Spotify(..) => self.inner.connect_player.volume(modify).await,
            TrackId::YouTube(..) => self.inner.youtube_player.volume(modify).await,
        })
    }

    /// Close the player from more requests.
    pub(crate) async fn close(&self, reason: Option<String>) {
        *self.inner.closed.lock() = Some(reason.map(Arc::new));
    }

    /// Open the player.
    pub(crate) async fn open(&self) {
        *self.inner.closed.lock() = None;
    }

    /// Search for a track.
    pub(crate) async fn search_track(&self, q: &str) -> Result<Option<TrackId>> {
        if q.starts_with("youtube:") {
            let q = q.trim_start_matches("youtube:");
            let results = self.inner.youtube.search(q).await?;
            let result = results
                .items
                .into_iter()
                .filter(|r| matches!(r.id.kind, api::youtube::Kind::Video));
            let mut result = result.flat_map(|r| r.id.video_id);
            return Ok(result.next().map(TrackId::YouTube));
        }

        let q = if q.starts_with("spotify:") {
            q.trim_start_matches("spotify:")
        } else {
            q
        };

        let page = self.inner.spotify.search_track(q, 1).await?;

        match page.tracks.items.into_iter().next().and_then(|t| t.id) {
            Some(track_id) => match SpotifyId::from_base62(&track_id) {
                Ok(track_id) => Ok(Some(TrackId::Spotify(track_id))),
                Err(_) => bail!("search result returned malformed id"),
            },
            None => Ok(None),
        }
    }

    /// Play a theme track.
    pub(crate) async fn play_theme(
        &self,
        channel: &Channel,
        name: &str,
    ) -> Result<(), PlayThemeError> {
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
            &self.inner.spotify,
            &self.inner.youtube,
            None,
            &theme.track_id,
            duration,
            None,
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
            .inject(Source::Manual, item, duration)
            .await
            .map_err(PlayThemeError::Error)?;
        Ok(())
    }

    /// Add the given track to the queue.
    ///
    /// Returns the item added.
    pub(crate) async fn add_track(
        &self,
        user: &str,
        track_id: TrackId,
        bypass_constraints: bool,
        max_duration: Option<utils::Duration>,
    ) -> Result<(Option<usize>, Arc<Item>), AddTrackError> {
        self.inner
            .add_track(user, track_id, bypass_constraints, max_duration)
            .await
    }

    pub(crate) async fn purge(&self) -> Result<Vec<Arc<Item>>> {
        let purged = self.inner.mixer.purge().await?;

        if !purged.is_empty() {
            self.inner.modified(Source::Manual).await?;
        }

        Ok(purged)
    }

    /// Remove the item at the given position.
    pub(crate) async fn remove_at(&self, n: usize) -> Result<Option<Arc<Item>>> {
        let removed = self.inner.mixer.remove_at(n).await?;

        if removed.is_some() {
            self.inner.modified(Source::Manual).await?;
        }

        Ok(removed)
    }

    /// Remove the first track in the queue.
    pub(crate) async fn remove_last(&self) -> Result<Option<Arc<Item>>> {
        let removed = self.inner.mixer.remove_last().await?;

        if removed.is_some() {
            self.inner.modified(Source::Manual).await?;
        }

        Ok(removed)
    }

    /// Remove the last track by the given user.
    pub(crate) async fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>> {
        let removed = self.inner.mixer.remove_last_by_user(user).await?;

        if removed.is_some() {
            self.inner.modified(Source::Manual).await?;
        }

        Ok(removed)
    }

    /// Find the next item that matches the given predicate and how long until it plays.
    pub(crate) async fn find(
        &self,
        mut predicate: impl FnMut(&Item) -> bool,
    ) -> Option<(Duration, Arc<Item>)> {
        let mut duration = Duration::default();

        if let Some(c) = self.inner.injector.get::<Song>().await {
            if predicate(&c.item) {
                return Some((Default::default(), c.item));
            }

            duration += c.remaining();
        }

        let items = self.inner.mixer.queue().await;

        for item in items.iter() {
            if predicate(item) {
                return Some((duration, item.clone()));
            }

            duration += item.duration;
        }

        None
    }

    /// Get the length in number of items and total number of seconds in queue.
    pub(crate) async fn length(&self) -> (usize, Duration) {
        let mut count = 0;
        let mut duration = Duration::default();

        if let Some(song) = self.inner.injector.get::<Song>().await.as_ref() {
            duration += song.remaining();
            count += 1;
        }

        let items = self.inner.mixer.queue().await;

        for item in items.iter() {
            duration += item.duration;
            count += 1;
        }

        (count, duration)
    }

    /// Get the current song, if it is set.
    pub(crate) async fn current(&self) -> Option<Song> {
        self.inner.injector.get::<Song>().await
    }
}

/// Error raised when failing to play a theme song.
pub(crate) enum PlayThemeError {
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
pub(crate) enum AddTrackError {
    /// Queue is full.
    QueueFull,
    /// Queue already contains track.
    QueueContainsTrack(usize),
    /// Too many user tracks.
    TooManyUserTracks(u32),
    /// Player has been closed from adding more tracks to the queue with an optional reason.
    PlayerClosed(Option<Arc<String>>),
    /// Duplicate song that was added at the specified time by the specified user.
    Duplicate {
        duplicate_by: DuplicateBy,
        duration_since: Option<Duration>,
        duplicate_duration: Duration,
    },
    /// Authentication missing for adding the given track.
    MissingAuth,
    /// Playback mode is not supported for the given track.
    UnsupportedPlaybackMode,
    /// Song cannot be played in the streamer's region
    NotPlayable,
    /// Other generic error happened.
    Error(anyhow::Error),
}

impl fmt::Display for AddTrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddTrackError::UnsupportedPlaybackMode => {
                write!(
                    f,
                    "Playback mode not supported for the given track type, sorry :("
                )
            }
            AddTrackError::PlayerClosed(reason) => match reason.as_deref() {
                Some(reason) => {
                    write!(f, "{}", reason)
                }
                None => {
                    write!(f, "Player is closed from further requests, sorry :(")
                }
            },
            AddTrackError::QueueContainsTrack(pos) => {
                write!(
                    f,
                    "Player already contains that track (position #{pos}).",
                    pos = pos + 1,
                )
            }
            AddTrackError::TooManyUserTracks(count) => {
                match count {
                    0 => {
                        write!(f, "Unfortunately you are not allowed to add tracks (track limit is zero) :(")
                    }
                    1 => {
                        write!(
                            f,
                            "<3 your enthusiasm, but you already have a track in the queue.",
                        )
                    }
                    count => {
                        write!(
                            f,
                            "<3 your enthusiasm, but you already have {count} tracks in the queue.",
                            count = count,
                        )
                    }
                }
            }
            AddTrackError::QueueFull => {
                write!(f, "Player is full, try again later!")
            }
            AddTrackError::Duplicate {
                duplicate_by,
                duration_since,
                duplicate_duration,
            } => {
                let duration_since = match duration_since {
                    Some(duration) => format!("{} ago", utils::compact_duration(*duration)),
                    None => String::from("not too long ago"),
                };

                let duplicate_duration = utils::compact_duration(*duplicate_duration);

                write!(
                    f,
                    "That song was requested by {who} {duration}, \
                         you have to wait at least {limit} between duplicate requests!",
                    who = duplicate_by,
                    duration = duration_since,
                    limit = duplicate_duration,
                )
            }
            AddTrackError::MissingAuth => {
                write!(
                    f,
                    "Cannot add the given song because the service has not been authenticated by the streamer!",
                )
            }
            AddTrackError::NotPlayable => {
                write!(f, "This song is not available in the streamer's region :(")
            }
            AddTrackError::Error(e) => {
                write!(f, "{}", e)
            }
        }
    }
}

pub(crate) enum DuplicateBy {
    /// By the requester themselves.
    Requester,
    /// By other user.
    Other(String),
    /// By unknown user (requester not recorded in database).
    Unknown,
}

impl fmt::Display for DuplicateBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DuplicateBy::Requester => write!(f, "you"),
            DuplicateBy::Other(other) => write!(f, "{}", other),
            DuplicateBy::Unknown => write!(f, "an unknown user"),
        }
    }
}
