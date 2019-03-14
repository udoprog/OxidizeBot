use tokio_core::reactor::Core;

pub use crate::track_id::TrackId;
use crate::{db, irc, secrets, spotify, template, themes::Themes, utils};

use chrono::Utc;
use failure::format_err;
use futures::{
    future,
    sync::{mpsc, oneshot},
    Async, Future, Poll, Stream,
};
use hashbrown::HashMap;
use std::{
    collections::VecDeque,
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio_bus::{Bus, BusReader};
use tokio_threadpool::{SpawnHandle, ThreadPool};

use librespot::core::spotify_id::SpotifyId;

mod connect;
mod native;

pub trait PlayerInterface: Send {
    /// Stop playing.
    fn stop(&mut self);

    /// Start playing.
    fn play(&mut self);

    /// Pause playback.
    fn pause(&mut self);

    /// Load the given track.
    ///
    /// The oneshot is triggered when the track has completed.
    fn load(&mut self, item: &Item, offset_ms: u32) -> oneshot::Receiver<()>;

    /// Adjust the volume of the player.
    fn volume(&mut self, volume: Option<f32>);
}

#[derive(Debug)]
pub enum PlayerEvent {
    Filtered,
}

type PlayerEventStream = Box<dyn Stream<Item = PlayerEvent, Error = ()> + Send + 'static>;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_max_queue_length")]
    max_queue_length: u32,
    #[serde(default = "default_max_songs_per_user")]
    max_songs_per_user: u32,
    /// Device to use with connect player.
    #[serde(default)]
    device: Option<String>,
    /// Speaker to use with native player.
    #[serde(default)]
    speaker: Option<String>,
    /// Playlist to fall back on. Will otherwise use the saved songs of the user.
    #[serde(default)]
    playlist: Option<String>,
    /// Volume of player.
    #[serde(default)]
    volume: Option<u32>,
    /// Whether or not to use the connect player.
    #[serde(default)]
    connect: bool,
    /// Write the current song to the specified path.
    #[serde(default)]
    current_song: Option<Arc<CurrentSong>>,
    /// Theme songs that can be triggered on command.
    #[serde(default)]
    themes: Arc<Themes>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CurrentSong {
    path: PathBuf,
    template: template::Template,
    #[serde(default)]
    not_playing: Option<String>,
}

fn default_max_queue_length() -> u32 {
    30
}

fn default_max_songs_per_user() -> u32 {
    2
}

impl CurrentSong {
    /// Either creates or truncates the current song file.
    fn create_or_truncate(&self) -> Result<File, failure::Error> {
        File::create(&self.path).map_err(Into::into)
    }

    /// Blank the current file.
    pub fn blank(&self) -> Result<(), failure::Error> {
        use std::io::Write as _;
        let mut f = self.create_or_truncate()?;

        if let Some(not_playing) = self.not_playing.as_ref() {
            write!(f, "{}", not_playing)?;
        } else {
            write!(f, "Not Playing")?;
        }

        Ok(())
    }

    /// Write the current song to a path.
    pub fn write(&self, item: &Item, paused: bool) -> Result<(), failure::Error> {
        let mut f = self.create_or_truncate()?;
        let data = item.data(paused)?;
        self.template.render(&mut f, &data)?;
        Ok(())
    }
}

impl Config {
    /// Load the configuration from a path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, failure::Error> {
        let f = std::fs::File::open(path)?;
        Ok(serde_yaml::from_reader(f)?)
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub track_id: TrackId,
    pub artists: Vec<String>,
    pub name: String,
    pub user: Option<String>,
    pub duration: Duration,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ItemData<'a> {
    paused: bool,
    track_id: &'a TrackId,
    name: String,
    artists: Option<String>,
    user: Option<&'a str>,
    duration: String,
}

impl Item {
    /// Human readable version of playback item.
    pub fn what(&self) -> String {
        if let Some(artists) = utils::human_artists(&self.artists) {
            format!("\"{}\" by {}", self.name, artists)
        } else {
            format!("\"{}\"", self.name.to_string())
        }
    }

    /// Display the short form duration of this track.
    ///
    /// e.g. 4m32s
    pub fn duration(&self) -> String {
        utils::compact_time(self.duration.as_secs())
    }

    /// Get serializable data for this item.
    pub fn data(&self, paused: bool) -> Result<ItemData<'_>, failure::Error> {
        let artists = utils::human_artists(&self.artists);

        let name = htmlescape::decode_html(&self.name)
            .map_err(|_| format_err!("failed to decode song name: {}", self.name))?;

        Ok(ItemData {
            paused,
            track_id: &self.track_id,
            name,
            artists,
            user: self.user.as_ref().map(|s| s.as_str()),
            duration: self.duration(),
        })
    }
}

#[derive(Debug)]
pub enum Command {
    // Skip the current song.
    Skip,
    // Toggle playback.
    Toggle,
    // Pause playback.
    Pause,
    // Start playback.
    Play,
    // A song was added to the queue.
    Added,
    // Set the gain of the player.
    Volume(u32),
    // Play the given item as a theme at the given offset.
    Inject(Arc<Item>, Duration),
}

impl std::str::FromStr for TrackId {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SpotifyId::from_base62(s)
            .map(TrackId)
            .map_err(|_| format_err!("failed to parse id"))
    }
}

/// Run the player.
pub fn run(
    core: &mut Core,
    db: db::Database,
    channel: &irc::Channel,
    spotify: Arc<spotify::Spotify>,
    config: &Config,
    secrets: &secrets::Secrets,
) -> Result<(PlaybackFuture, Player), failure::Error> {
    let (commands_tx, commands) = mpsc::unbounded();

    let ((player, events), paused) = if config.connect {
        (connect::setup(core, config, spotify.clone())?, false)
    } else {
        (native::setup(core, config, secrets)?, true)
    };

    let bus = Arc::new(RwLock::new(Bus::new(1024)));

    let thread_pool = Arc::new(ThreadPool::new());
    let queue = Queue::new(db.clone());

    let fallback_items = match config.playlist.as_ref() {
        Some(playlist) => playlist_to_items(core, spotify.clone(), playlist)?,
        None => songs_to_items(core, spotify.clone())?,
    };

    // Add tracks from database.
    for song in db.list()? {
        let item = core.run(convert_item(
            &thread_pool,
            spotify.clone(),
            song.user.clone(),
            song.track_id,
        ))?;

        queue.push_back_queue(channel.name.as_str(), item);
    }

    // Blank current song file if specified.
    if let Some(current_song) = config.current_song.as_ref() {
        if let Err(e) = current_song.blank() {
            log::warn!(
                "failed to blank current songs: {}: {}",
                current_song.path.display(),
                e
            );
        }
    }

    let volume = Arc::new(RwLock::new(u32::min(
        100u32,
        config.volume.unwrap_or(100u32),
    )));

    let current = Arc::new(RwLock::new(None));
    let closed = Arc::new(RwLock::new(None));

    let future = PlaybackFuture {
        player,
        events,
        commands,
        queue: queue.clone(),
        bus: bus.clone(),
        pop_front: None,
        paused,
        loaded: None,
        inject: None,
        sidelined: Default::default(),
        fallback_items,
        volume: Arc::clone(&volume),
        channel: Arc::clone(&channel.name),
        current: current.clone(),
        current_song: config.current_song.clone(),
    };

    let player = Player {
        queue,
        max_queue_length: config.max_queue_length,
        max_songs_per_user: config.max_songs_per_user,
        spotify,
        commands_tx,
        bus,
        volume: Arc::clone(&volume),
        channel: Arc::clone(&channel.name),
        current: current.clone(),
        themes: config.themes.clone(),
        closed: closed.clone(),
    };

    if config.connect {
        let client = player.client();
        client.pause()?;

        if let Some(volume) = config.volume {
            client.volume(volume)?;
        }
    }

    Ok((future, player))
}

/// Convert a playlist into items.
fn playlist_to_items(
    core: &mut Core,
    spotify: Arc<spotify::Spotify>,
    playlist: &str,
) -> Result<Vec<Arc<Item>>, failure::Error> {
    let mut items = Vec::new();

    let playlist = core.run(spotify.playlist(playlist))?;

    for playlist_track in core.run(spotify.page_as_stream(playlist.tracks).concat2())? {
        let track = playlist_track.track;

        let track_id = TrackId(
            SpotifyId::from_base62(&track.id)
                .map_err(|_| format_err!("bad spotify id: {}", track.id))?,
        );

        let artists = track
            .artists
            .into_iter()
            .map(|a| a.name)
            .collect::<Vec<_>>();

        items.push(Arc::new(Item {
            track_id,
            artists,
            name: track.name.to_string(),
            user: None,
            duration: Duration::from_millis(track.duration_ms.into()),
        }));
    }

    Ok(items)
}

/// Convert all songs of a user into items.
fn songs_to_items(
    core: &mut Core,
    spotify: Arc<spotify::Spotify>,
) -> Result<Vec<Arc<Item>>, failure::Error> {
    let mut items = Vec::new();

    for added_song in core.run(spotify.my_tracks_stream().concat2())? {
        let track = added_song.track;

        let track_id = TrackId(
            SpotifyId::from_base62(&track.id)
                .map_err(|_| format_err!("bad spotify id: {}", track.id))?,
        );

        let artists = track
            .artists
            .into_iter()
            .map(|a| a.name)
            .collect::<Vec<_>>();

        items.push(Arc::new(Item {
            track_id,
            artists,
            name: track.name.to_string(),
            user: None,
            duration: Duration::from_millis(track.duration_ms.into()),
        }));
    }

    Ok(items)
}

/// Converts a track into an Item.
fn convert_item(
    thread_pool: &ThreadPool,
    spotify: Arc<spotify::Spotify>,
    user: Option<String>,
    track_id: TrackId,
) -> impl Future<Item = Arc<Item>, Error = failure::Error> {
    let track_id_string = track_id.0.to_base62();

    thread_pool
        .spawn_handle(future::lazy(move || spotify.track(&track_id_string)))
        .map(move |full_track| {
            let artists = full_track
                .artists
                .into_iter()
                .map(|a| a.name)
                .collect::<Vec<_>>();

            Arc::new(Item {
                track_id,
                artists,
                name: full_track.name,
                user,
                duration: Duration::from_millis(full_track.duration_ms.into()),
            })
        })
}

/// The origin of a song being played.
#[derive(Debug, Clone, Copy)]
pub enum Origin {
    Injected,
    Fallback,
    Queue,
}

/// Events emitted by the player.
#[derive(Debug, Clone)]
pub enum Event {
    Empty,
    Playing(Origin, Arc<Item>),
    Pausing,
}

/// A handler for the player.
#[derive(Clone)]
pub struct Player {
    queue: Queue,
    max_queue_length: u32,
    max_songs_per_user: u32,
    spotify: Arc<spotify::Spotify>,
    commands_tx: mpsc::UnboundedSender<Command>,
    bus: Arc<RwLock<Bus<Event>>>,
    volume: Arc<RwLock<u32>>,
    channel: Arc<String>,
    /// Current song that is loaded.
    current: Arc<RwLock<Option<Arc<Item>>>>,
    /// Theme songs.
    themes: Arc<Themes>,
    /// Player is closed for more requests.
    closed: Arc<RwLock<Option<Option<Arc<String>>>>>,
}

impl Player {
    /// The client components of the player.
    pub fn client(&self) -> PlayerClient {
        PlayerClient {
            queue: self.queue.clone(),
            channel: Arc::clone(&self.channel),
            thread_pool: Arc::new(ThreadPool::new()),
            max_queue_length: self.max_queue_length,
            max_songs_per_user: self.max_songs_per_user,
            spotify: self.spotify.clone(),
            commands_tx: self.commands_tx.clone(),
            volume: Arc::clone(&self.volume),
            current: self.current.clone(),
            themes: self.themes.clone(),
            closed: self.closed.clone(),
        }
    }

    /// Get a receiver for player events.
    pub fn add_rx(&self) -> BusReader<Event> {
        self.bus.write().expect("lock poisoned").add_rx()
    }
}

/// All parts of a Player that can be shared between threads.
#[derive(Clone)]
pub struct PlayerClient {
    queue: Queue,
    channel: Arc<String>,
    thread_pool: Arc<ThreadPool>,
    max_queue_length: u32,
    max_songs_per_user: u32,
    spotify: Arc<spotify::Spotify>,
    commands_tx: mpsc::UnboundedSender<Command>,
    /// Current volume.
    volume: Arc<RwLock<u32>>,
    /// Current song that is loaded.
    current: Arc<RwLock<Option<Arc<Item>>>>,
    /// Theme songs.
    themes: Arc<Themes>,
    /// Player is closed for more requests.
    closed: Arc<RwLock<Option<Option<Arc<String>>>>>,
}

impl PlayerClient {
    /// Send the given command.
    fn send(&self, command: Command) -> Result<(), failure::Error> {
        self.commands_tx
            .unbounded_send(command)
            .map_err(|_| format_err!("failed to send command"))
    }

    /// Get the next N songs in queue.
    pub fn list(&self, n: usize) -> Vec<Arc<Item>> {
        let current = self.current.read().expect("poisoned");
        let inner = self.queue.queues.read().expect("lock poisoned");

        let queue = match inner.get(self.channel.as_str()) {
            Some(queue) => queue,
            None => return vec![],
        };

        current
            .iter()
            .cloned()
            .chain(queue.iter().take(n).cloned())
            .collect()
    }

    /// Toggle playback.
    pub fn toggle(&self) -> Result<(), failure::Error> {
        self.send(Command::Toggle)
    }

    /// Start playback.
    pub fn play(&self) -> Result<(), failure::Error> {
        self.send(Command::Play)
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<(), failure::Error> {
        self.send(Command::Pause)
    }

    /// Skip the current song.
    pub fn skip(&self) -> Result<(), failure::Error> {
        self.send(Command::Skip)
    }

    /// Update volume of the player.
    pub fn volume(&self, volume: u32) -> Result<(), failure::Error> {
        self.send(Command::Volume(u32::min(100u32, volume)))
    }

    /// Get the current volume.
    pub fn current_volume(&self) -> u32 {
        *self.volume.read().expect("lock poisoned")
    }

    /// Close the player from more requests.
    pub fn close(&self, reason: Option<String>) {
        *self.closed.write().expect("poisoned") = Some(reason.map(Arc::new));
    }

    /// Open the player.
    pub fn open(&self) {
        *self.closed.write().expect("poisoned") = None;
    }

    /// Search for a track.
    pub fn search_track(
        &self,
        q: &str,
    ) -> impl Future<Item = Option<TrackId>, Error = failure::Error> {
        self.spotify
            .search_track(q)
            .and_then(|page| match page.items.into_iter().next() {
                Some(track) => match SpotifyId::from_base62(&track.id) {
                    Ok(track_id) => Ok(Some(TrackId(track_id))),
                    Err(_) => Err(failure::format_err!("search result returned malformed id")),
                },
                None => Ok(None),
            })
    }

    /// Play a theme track.
    pub fn play_theme(&self, name: &str) -> impl Future<Item = (), Error = PlayThemeError> {
        let fut = future::lazy({
            let themes = self.themes.clone();
            let name = name.to_string();

            move || match themes.lookup(&name) {
                Some(theme) => Ok(theme),
                None => Err(PlayThemeError::NoSuchTheme),
            }
        });

        let fut = fut.and_then({
            let thread_pool = Arc::clone(&self.thread_pool);
            let spotify = Arc::clone(&self.spotify);

            move |theme| {
                convert_item(&thread_pool, spotify, None, theme.track.clone())
                    .map(move |item| (item, theme))
                    .map_err(|e| PlayThemeError::Error(e.into()))
            }
        });

        fut.and_then({
            let commands_tx = self.commands_tx.clone();

            move |(item, theme)| {
                let duration = theme.offset.as_duration();

                commands_tx
                    .unbounded_send(Command::Inject(item, duration))
                    .map_err(|e| PlayThemeError::Error(e.into()))
            }
        })
    }

    /// Add the given track to the queue.
    ///
    /// Returns the item added.
    pub fn add_track(
        &self,
        user: &str,
        track_id: TrackId,
        is_moderator: bool,
    ) -> impl Future<Item = (usize, Arc<Item>), Error = AddTrackError> {
        // invariant checks
        let fut = future::lazy({
            let channel = self.channel.clone();
            let queues = self.queue.queues.clone();
            let max_queue_length = self.max_queue_length;
            let max_songs_per_user = self.max_songs_per_user;
            let closed = self.closed.clone();
            let user = user.to_string();
            let track_id = track_id.clone();

            move || {
                let inner = queues.read().expect("lock poisoned");

                // store queue in case there is no queue for channel yet.
                let mut local_queue = None;

                let q = match inner.get(channel.as_str()) {
                    Some(q) => q,
                    None => local_queue.get_or_insert_with(Default::default),
                };

                let len = q.len();

                if !is_moderator {
                    if let Some(reason) = closed.read().expect("poisoned").as_ref() {
                        return Err(AddTrackError::PlayerClosed(reason.clone()));
                    }
                }

                // NB: moderator is allowed to violate max queue length.
                if !is_moderator && len > max_queue_length as usize {
                    return Err(AddTrackError::QueueFull);
                }

                let mut user_count = 0;

                for (index, i) in q.iter().enumerate() {
                    if i.track_id == track_id {
                        return Err(AddTrackError::QueueContainsTrack(index));
                    }

                    if i.user.as_ref().map(|u| *u == user).unwrap_or_default() {
                        user_count += 1;
                    }
                }

                // NB: moderator is allowed to add more songs.
                if !is_moderator && user_count >= max_songs_per_user {
                    return Err(AddTrackError::TooManyUserTracks(max_songs_per_user));
                }

                Ok(len)
            }
        });

        let fut = fut.and_then({
            let user = user.to_string();
            let thread_pool = Arc::clone(&self.thread_pool);
            let spotify = Arc::clone(&self.spotify);

            move |len| {
                convert_item(&thread_pool, spotify, Some(user), track_id)
                    .map(move |item| (len, item))
                    .map_err(|e| AddTrackError::Error(e.into()))
            }
        });

        let fut = fut.and_then({
            let channel = self.channel.clone();
            let queue = self.queue.clone();

            move |(len, item)| {
                queue
                    .push_back(channel.as_str(), Arc::clone(&item))
                    .map(move |_| (len, item))
                    .map_err(|e| AddTrackError::Error(e.into()))
            }
        });

        fut.and_then({
            let commands_tx = self.commands_tx.clone();

            move |(len, item)| {
                commands_tx
                    .unbounded_send(Command::Added)
                    .map(move |_| (len, item))
                    .map_err(|e| AddTrackError::Error(e.into()))
            }
        })
    }

    /// Remove the first track in the queue.
    pub fn remove_first(&self) -> Result<Option<Arc<Item>>, failure::Error> {
        Ok(None)
    }

    pub fn purge(&self) -> Result<Vec<Arc<Item>>, failure::Error> {
        self.queue.purge(self.channel.as_str())
    }

    /// Remove the first track in the queue.
    pub fn remove_last(&self) -> Result<Option<Arc<Item>>, failure::Error> {
        self.queue.remove_last(self.channel.as_str())
    }

    /// Remove the last track by the given user.
    pub fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>, failure::Error> {
        self.queue.remove_last_by_user(self.channel.as_str(), user)
    }

    /// Get the length in number of items and total number of seconds in queue.
    pub fn length(&self) -> (usize, u64) {
        let mut count = 0;
        let mut duration = Duration::default();

        if let Some(item) = self.current.read().expect("poisoned").as_ref() {
            duration += item.duration;
            count += 1;
        }

        let queues = self.queue.queues.read().expect("poisoned");

        if let Some(queue) = queues.get(self.channel.as_str()) {
            for item in &*queue {
                duration += item.duration;
            }

            count += queue.len();
        }

        (count, duration.as_secs())
    }

    /// Get the current song, if it is set.
    pub fn current(&self) -> Option<Arc<Item>> {
        self.current.read().expect("poisoned").clone()
    }
}

/// Error raised when failing to play a theme song.
pub enum PlayThemeError {
    /// No such theme song.
    NoSuchTheme,
    /// Other generic error happened.
    Error(failure::Error),
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
    /// Other generic error happened.
    Error(failure::Error),
}

/// The backend of a words store.
pub trait Backend: Clone + Send + Sync {
    /// List all counters in backend.
    fn list(&self) -> Result<Vec<db::Song>, failure::Error>;

    /// Insert the given song into the backend.
    fn push_back(&self, song: &db::Song) -> Result<(), failure::Error>;

    /// Remove the song, but only log on issues.
    fn remove_song_log(&self, channel: &str, track_id: &TrackId) {
        match self.remove_song(channel, track_id) {
            Err(e) => log::warn!(
                "{}:{}: failed to remove song from database: {}",
                channel,
                track_id.to_base62(),
                e
            ),
            Ok(false) => log::warn!(
                "{}:{}: no songs removed from database",
                channel,
                track_id.to_base62()
            ),
            Ok(true) => {}
        }
    }

    /// Remove the song with the given ID.
    fn remove_song(&self, channel: &str, track_id: &TrackId) -> Result<bool, failure::Error>;

    /// Purge the songs database, but only log on issues.
    fn song_purge_log(&self, channel: &str) -> Option<usize> {
        match self.song_purge(channel) {
            Err(e) => {
                log::warn!("{}:{}: failed to purge songs from database", channel, e);
                None
            }
            Ok(n) => Some(n),
        }
    }

    /// Purge the songs database and return the number of items removed.
    fn song_purge(&self, channel: &str) -> Result<usize, failure::Error>;
}

/// The playback queue.
#[derive(Clone)]
struct Queue {
    db: db::Database,
    queues: Arc<RwLock<HashMap<String, VecDeque<Arc<Item>>>>>,
    thread_pool: Arc<ThreadPool>,
}

impl Queue {
    /// Construct a new queue.
    pub fn new(db: db::Database) -> Self {
        Self {
            db,
            queues: Arc::new(RwLock::new(Default::default())),
            thread_pool: Arc::new(ThreadPool::new()),
        }
    }

    /// Get the front of the queue.
    pub fn front(&self, channel: &str) -> Option<Arc<Item>> {
        let inner = self.queues.read().expect("lock poisoned");

        if let Some(queue) = inner.get(channel) {
            return queue.front().cloned();
        }

        None
    }

    /// Pop the front of the queue.
    pub fn pop_front(&self, channel: &str) -> PopFrontFuture {
        let channel = channel.to_string();
        let db = self.db.clone();
        let queues = self.queues.clone();

        PopFrontFuture(self.thread_pool.spawn_handle(future::lazy(move || {
            let mut queues = queues.write().expect("lock poisoned");

            if let Some(queue) = queues.get_mut(&channel) {
                if let Some(item) = queue.pop_front() {
                    db.remove_song_log(&channel, &item.track_id);
                }
            }

            Ok(None)
        })))
    }

    /// Push item to back of queue.
    pub fn push_back(&self, channel: &str, item: Arc<Item>) -> PushBackFuture {
        let channel = channel.to_string();
        let db = self.db.clone();
        let queues = self.queues.clone();

        PushBackFuture(self.thread_pool.spawn_handle(future::lazy(move || {
            db.push_back(&db::Song {
                channel: channel.to_string(),
                track_id: item.track_id.clone(),
                added_at: Utc::now().naive_utc(),
                user: item.user.clone(),
            })?;

            let mut inner = queues.write().expect("lock poisoned");
            inner.entry(channel).or_default().push_back(item);
            Ok(())
        })))
    }

    /// Purge the song queue.
    pub fn purge(&self, channel: &str) -> Result<Vec<Arc<Item>>, failure::Error> {
        let mut queues = self.queues.write().expect("lock poisoned");

        let q = match queues.get_mut(channel) {
            Some(q) => q,
            None => return Ok(vec![]),
        };

        if q.is_empty() {
            return Ok(vec![]);
        }

        let purged = std::mem::replace(q, VecDeque::new()).into_iter().collect();
        self.db.song_purge_log(channel);
        Ok(purged)
    }

    /// Remove the last element.
    pub fn remove_last(&self, channel: &str) -> Result<Option<Arc<Item>>, failure::Error> {
        let mut queues = self.queues.write().expect("lock poisoned");

        let q = match queues.get_mut(channel) {
            Some(q) => q,
            None => return Ok(None),
        };

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(item) = q.pop_back() {
            self.db.remove_song_log(channel, &item.track_id);
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element by user.
    pub fn remove_last_by_user(
        &self,
        channel: &str,
        user: &str,
    ) -> Result<Option<Arc<Item>>, failure::Error> {
        let mut queues = self.queues.write().expect("lock poisoned");

        let q = match queues.get_mut(channel) {
            Some(q) => q,
            None => return Ok(None),
        };

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(position) = q
            .iter()
            .rposition(|i| i.user.as_ref().map(|u| u == user).unwrap_or_default())
        {
            if let Some(item) = q.remove(position) {
                self.db.remove_song_log(channel, &item.track_id);
                return Ok(Some(item));
            }
        }

        Ok(None)
    }

    /// Push item to back of queue without going through the database.
    pub fn push_back_queue(&self, channel: &str, item: Arc<Item>) {
        let mut inner = self.queues.write().expect("lock poisoned");

        inner
            .entry(channel.to_string())
            .or_default()
            .push_back(item);
    }
}

struct PopFrontFuture(SpawnHandle<Option<Arc<Item>>, failure::Error>);

impl Future for PopFrontFuture {
    type Item = Option<Arc<Item>>;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

struct PushBackFuture(SpawnHandle<(), failure::Error>);

impl Future for PushBackFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

struct Loaded {
    origin: Origin,
    item: Arc<Item>,
    future: oneshot::Receiver<()>,
    started_at: Instant,
    offset: Duration,
}

impl Loaded {
    /// Create a new loaded entry recording the time at which it was started.
    pub fn new(origin: Origin, item: Arc<Item>, future: oneshot::Receiver<()>) -> Self {
        Self {
            origin,
            item,
            future,
            started_at: Instant::now(),
            offset: Default::default(),
        }
    }

    /// Song was loaded with the specified offset.
    pub fn with_offset(self, offset: Duration) -> Self {
        Self { offset, ..self }
    }
}

/// Future associated with driving audio playback.
pub struct PlaybackFuture {
    player: Box<dyn PlayerInterface + 'static>,
    events: PlayerEventStream,
    commands: mpsc::UnboundedReceiver<Command>,
    queue: Queue,
    bus: Arc<RwLock<Bus<Event>>>,
    /// Future associated with popping the front control.
    pop_front: Option<PopFrontFuture>,
    /// Playback is paused.
    paused: bool,
    /// There is a song loaded into the player.
    loaded: Option<Loaded>,
    /// A song to inject to play _right now_.
    inject: Option<(Arc<Item>, Duration)>,
    /// A song that has been sidelined by another song.
    sidelined: VecDeque<(Loaded, Instant)>,
    /// Items to fall back to when there are no more songs in queue.
    fallback_items: Vec<Arc<Item>>,
    /// Current volume.
    volume: Arc<RwLock<u32>>,
    /// Channel playback is associated with.
    channel: Arc<String>,
    /// Current song that is loaded.
    current: Arc<RwLock<Option<Arc<Item>>>>,
    /// Path to write current song.
    current_song: Option<Arc<CurrentSong>>,
}

impl PlaybackFuture {
    /// Play what is at the front of the queue.
    fn next_song(&mut self) -> Option<Loaded> {
        use rand::Rng;

        if let Some((item, offset)) = self.inject.take() {
            // store the currently playing song in the sidelined slot.
            if let Some(loaded) = self.loaded.take() {
                self.sidelined.push_back((loaded, Instant::now()));
            }

            let future = self.player.load(&*item, offset.as_millis() as u32);
            return Some(Loaded::new(Origin::Injected, item, future).with_offset(offset));
        }

        if let Some((loaded, paused_at)) = self.sidelined.pop_front() {
            let offset = if paused_at > loaded.started_at {
                // calculate offset to start playing at
                (paused_at - loaded.started_at) + loaded.offset
            } else {
                Default::default()
            };

            let future = self.player.load(&*loaded.item, offset.as_millis() as u32);
            return Some(Loaded::new(loaded.origin, loaded.item, future).with_offset(offset));
        }

        if let Some(item) = self.queue.front(self.channel.as_str()) {
            self.pop_front = Some(self.queue.pop_front(self.channel.as_str()));
            let future = self.player.load(&*item, 0);
            return Some(Loaded::new(Origin::Queue, item, future));
        }

        if !self.paused || self.loaded.is_some() {
            let mut rng = rand::thread_rng();

            let n = rng.gen_range(0, self.fallback_items.len());

            // Pick a random item to play.
            if let Some(item) = self.fallback_items.get(n) {
                let future = self.player.load(&*item, 0);
                return Some(Loaded::new(Origin::Fallback, item.clone(), future));
            }
        }

        self.paused = true;
        None
    }

    /// Write current song. Log any errors.
    fn current_song(&self) {
        let current_song = match self.current_song.as_ref() {
            Some(current_song) => current_song,
            None => return,
        };

        let result = match self.loaded.as_ref() {
            Some(loaded) => current_song.write(&loaded.item, self.paused),
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

    /// Load the next song.
    fn load_front(&mut self) {
        if let Some(loaded) = self.next_song() {
            *self.current.write().expect("poisoned") = Some(loaded.item.clone());

            if !self.paused {
                self.player.play();
                self.broadcast(Event::Playing(loaded.origin, loaded.item.clone()));
            } else {
                self.player.pause();
            }

            self.loaded = Some(loaded);
            self.current_song();
            return;
        }

        self.loaded = None;
        *self.current.write().expect("poisoned") = None;
        self.broadcast(Event::Empty);
        self.player.stop();
        self.current_song();
    }

    /// Broadcast an event from the player.
    fn broadcast(&self, event: Event) {
        let mut b = self.bus.write().expect("lock poisoned");

        if let Err(e) = b.try_broadcast(event) {
            log::error!("failed to broadcast player event: {:?}", e);
        }
    }

    /// Handle incoming command.
    fn command(&mut self, command: Command) {
        let command = match command {
            Command::Toggle if self.paused => Command::Play,
            Command::Toggle if !self.paused => Command::Pause,
            command => command,
        };

        match command {
            Command::Skip => {
                log::info!("Skipping song");
                self.load_front();
            }
            Command::Pause if !self.paused => {
                log::info!("pausing player");
                self.paused = true;
                self.player.pause();
                self.broadcast(Event::Pausing);
                self.current_song();
            }
            Command::Play if self.paused => {
                log::info!("starting player");
                self.paused = false;

                match self.loaded.as_ref() {
                    Some(loaded) => {
                        self.player.play();
                        self.broadcast(Event::Playing(loaded.origin, loaded.item.clone()));
                        self.current_song();
                    }
                    None => {
                        self.load_front();
                    }
                }
            }
            Command::Added if !self.paused && self.loaded.is_none() => {
                self.load_front();
            }
            Command::Volume(volume) => {
                *self.volume.write().expect("lock poisoned") = volume;
                self.player.volume(Some((volume as f32) / 100f32));
            }
            Command::Inject(item, offset) => {
                self.inject = Some((item, offset));
                self.load_front();
            }
            _ => {}
        }
    }
}

impl Future for PlaybackFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let mut not_ready = true;

            // pop is in progress, make sure that happens before anything else.
            if let Some(pop_front) = self.pop_front.as_mut() {
                if let Async::NotReady = pop_front.poll()? {
                    return Ok(Async::NotReady);
                }

                self.pop_front = None;
                not_ready = false;
            }

            if let Some(loaded) = self.loaded.as_mut() {
                match loaded.future.poll() {
                    Ok(Async::Ready(())) => {
                        log::info!("Song ended");
                        self.load_front();
                        not_ready = false;
                    }
                    Err(oneshot::Canceled) => {
                        self.loaded = None;
                        *self.current.write().expect("poisoned") = None;
                    }
                    Ok(Async::NotReady) => {}
                }
            }

            if let Async::Ready(event) = self
                .events
                .poll()
                .map_err(|_| format_err!("event stream errored"))?
            {
                let event = event.ok_or_else(|| format_err!("events stream ended"))?;

                match event {
                    other => {
                        log::trace!("player event: {:?}", other);
                    }
                }

                not_ready = false;
            }

            if let Async::Ready(command) = self
                .commands
                .poll()
                .map_err(|_| format_err!("events stream errored"))?
            {
                let command = command.ok_or_else(|| format_err!("command stream ended"))?;
                self.command(command);
                not_ready = false;
            }

            if not_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}
