use tokio_core::reactor::Core;

use crate::{api, bus, config, current_song, db, settings, utils};
pub use crate::{spotify_id::SpotifyId, track_id::TrackId};

use chrono::{DateTime, Utc};
use failure::format_err;
use futures::{future, sync::mpsc, Future, Poll, Stream};
use parking_lot::RwLock;
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_bus::{Bus, BusReader};
use tokio_threadpool::{SpawnHandle, ThreadPool};

mod connect;
mod youtube;

/// Event used by player integrations.
#[derive(Debug)]
pub enum IntegrationEvent {
    /// We've reached the end of a track.
    EndOfTrack,
    /// Indicate that the current device changed.
    DeviceChanged,
    /// We've filtered a player event.
    Filtered,
    /// Indicate that we have successfully issued a playing command to the player.
    Playing(Source),
    /// Indicate that we have successfully issued a pause command to the player.
    Pausing(Source),
    /// Indicate that a player has been stoped because of a switch.
    Stopping,
    /// Indicate that we have successfully issued a volume command to the player.
    Volume(Source, u32),
}

/// The source of action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// Event was generated automatically, don't broadcast feedback.
    Automatic,
    /// Event was generated from user input. Broadcast feedback.
    Manual,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// The max queue length of the player.
    #[serde(default = "default_max_queue_length")]
    max_queue_length: u32,
    /// The max number of songs per user.
    #[serde(default = "default_max_songs_per_user")]
    max_songs_per_user: u32,
    /// Playlist to fall back on. Will otherwise use the saved songs of the user.
    #[serde(default)]
    playlist: Option<String>,
    /// Volume of player.
    #[serde(default)]
    volume: Option<u32>,
    /// Whether or not to echo current song.
    #[serde(default = "default_true")]
    echo_current_song: bool,
    /// Device to use with connect player.
    #[serde(default)]
    device: Option<String>,
    /// Interval at which to try to sync the remote player with the local state.
    #[serde(default)]
    sync_player_interval: utils::Duration,
}

fn default_true() -> bool {
    true
}

fn default_max_queue_length() -> u32 {
    30
}

fn default_max_songs_per_user() -> u32 {
    2
}

/// Information on a single track.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum Track {
    #[serde(rename = "spotify")]
    Spotify { track: api::spotify::FullTrack },
    #[serde(rename = "youtube")]
    YouTube { video: api::youtube::Video },
}

impl Track {
    /// Get artists involved as a string.
    pub fn artists(&self) -> Option<String> {
        match *self {
            Track::Spotify { ref track } => utils::human_artists(&track.artists),
            Track::YouTube { ref video } => {
                video.snippet.as_ref().and_then(|s| s.channel_title.clone())
            }
        }
    }

    /// Get name of the track.
    pub fn name(&self) -> String {
        match *self {
            Track::Spotify { ref track } => track.name.to_string(),
            Track::YouTube { ref video } => video
                .snippet
                .as_ref()
                .map(|s| s.title.as_str())
                .unwrap_or("no name")
                .to_string(),
        }
    }

    /// Convert into JSON.
    /// TODO: this is a hack to avoid breaking web API.
    pub fn to_json(&self) -> Result<serde_json::Value, failure::Error> {
        let json = match *self {
            Track::Spotify { ref track } => serde_json::to_value(&track)?,
            Track::YouTube { ref video } => serde_json::to_value(&video)?,
        };

        Ok(json)
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub track_id: TrackId,
    pub track: Track,
    pub user: Option<String>,
    pub duration: Duration,
}

impl Item {
    /// Human readable version of playback item.
    pub fn what(&self) -> String {
        match self.track {
            Track::Spotify { ref track } => {
                if let Some(artists) = utils::human_artists(&track.artists) {
                    format!("\"{}\" by {}", track.name, artists)
                } else {
                    format!("\"{}\"", track.name)
                }
            }
            Track::YouTube { ref video } => match video.snippet.as_ref() {
                Some(snippet) => match snippet.channel_title.as_ref() {
                    Some(channel_title) => {
                        format!("\"{}\" from \"{}\"", snippet.title, channel_title)
                    }
                    None => format!("\"{}\"", snippet.title),
                },
                None => String::from("*Some YouTube Video*"),
            },
        }
    }
}

#[derive(Debug)]
pub enum Command {
    /// Skip the current song.
    Skip(Source),
    /// Toggle playback.
    Toggle(Source),
    /// Pause playback.
    Pause(Source),
    /// Start playback.
    Play(Source),
    /// Start playback on a specific song state.
    PlaySync { song: Option<Song> },
    /// The queue was modified.
    Modified(Source),
    /// Set the gain of the player.
    Volume(Source, u32),
    /// Play the given item as a theme at the given offset.
    Inject(Source, Arc<Item>, Duration),
}

impl Command {
    /// Get the source of a command.
    pub fn source(&self) -> Source {
        use self::Command::*;

        match *self {
            Skip(source) | Toggle(source) | Pause(source) | Play(source) | Modified(source)
            | Volume(source, ..) | Inject(source, ..) => source,
            PlaySync { .. } => Source::Automatic,
        }
    }
}

/// Run the player.
pub fn run(
    core: &mut Core,
    db: db::Database,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    parent_config: &config::Config,
    config: &Config,
    global_bus: Arc<bus::Bus<bus::Global>>,
    youtube_bus: Arc<bus::Bus<bus::YouTube>>,
    settings: settings::Settings,
    themes: db::Themes,
) -> Result<(PlaybackFuture, Player), failure::Error> {
    let settings = settings.scoped(&["player"]);

    let (connect_player, device) =
        connect::setup(core, spotify.clone(), settings.scoped(&["spotify"]))?;
    let youtube_player = youtube::setup(core, youtube_bus.clone(), settings.scoped(&["youtube"]))?;

    let bus = Arc::new(RwLock::new(Bus::new(1024)));

    let thread_pool = Arc::new(ThreadPool::new());
    let queue = Queue::new(db.clone());

    let fallback_items = match config.playlist.as_ref() {
        Some(playlist) => playlist_to_items(core, spotify.clone(), playlist)?,
        None => songs_to_items(core, spotify.clone())?,
    };

    log::info!("Added {} fallback songs", fallback_items.len());

    // Add tracks from database.
    for song in db.list()? {
        queue.push_back_queue(Arc::new(core.run(convert_item(
            &thread_pool,
            spotify.clone(),
            youtube.clone(),
            song.user.clone(),
            song.track_id,
            None,
        ))?));
    }

    // Blank current song file if specified.
    if let Some(current_song) = parent_config.current_song.as_ref() {
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
        config.volume.unwrap_or(50u32),
    )));

    let song = Arc::new(RwLock::new(None));
    let closed = Arc::new(RwLock::new(None));

    let current_song_update = match parent_config
        .current_song
        .as_ref()
        .and_then(|c| c.update_interval())
    {
        Some(update_interval) => Some(tokio_timer::Interval::new_interval(update_interval.clone())),
        None => None,
    };

    let (song_update_interval_stream, song_update_interval) =
        settings.init_and_stream("song-update-interval", utils::Duration::seconds(1))?;

    let song_update_interval = match song_update_interval.is_empty() {
        true => None,
        false => Some(tokio_timer::Interval::new_interval(
            song_update_interval.as_std(),
        )),
    };

    if !config.sync_player_interval.is_empty() {
        log::warn!("### DEPRECATION WARNING");
        log::warn!("[player] sync_player_interval - configuration has been deprecated since it was too unreliable.");
    }

    let mixer = Mixer {
        queue: queue.clone(),
        sidelined: Default::default(),
        fallback_items,
        fallback_queue: Default::default(),
        pop_front: None,
    };

    let (commands_tx, commands) = mpsc::unbounded();

    let (detached_stream, detached) = settings.init_and_stream("detached", false)?;

    let future = PlaybackFuture {
        connect_player,
        youtube_player,
        commands,
        bus: EventBus { bus: bus.clone() },
        mixer,
        // NB: it is not considered paused _yet_.
        // When we issue the pause command below, we only queue up the command.
        state: State::None,
        player: PlayerKind::None,
        detached,
        detached_stream,
        volume: Arc::clone(&volume),
        song: song.clone(),
        current_song: parent_config.current_song.clone(),
        echo_current_song: config.echo_current_song,
        current_song_update,
        song_update_interval,
        song_update_interval_stream,
        global_bus,
    };

    let duplicate_duration =
        settings.sync_var(core, "duplicate-duration", utils::Duration::default())?;

    let max_songs_per_user =
        settings.sync_var(core, "max-songs-per-user", config.max_songs_per_user)?;

    let max_queue_length = settings.sync_var(core, "max-queue-length", config.max_queue_length)?;

    let player = Player {
        device: device.clone(),
        queue,
        db: db.clone(),
        max_queue_length,
        max_songs_per_user,
        duplicate_duration,
        spotify: spotify.clone(),
        youtube: youtube.clone(),
        commands_tx,
        bus,
        volume: volume.clone(),
        song: song.clone(),
        themes: themes.clone(),
        closed: closed.clone(),
    };

    match core
        .run(spotify.me_player())?
        .and_then(|p| Song::from_playback(&p).map(move |s| (s, p.device)))
    {
        // make use of the information on the current playback to get the local player into a good state.
        Some((song, new_device)) => {
            player.play_sync(Some(song))?;
            *volume.write() = new_device.volume_percent;
            *device.device.write() = Some(new_device);
        }
        None => {
            let devices = core.run(spotify.my_player_devices())?;

            for (i, d) in devices.iter().enumerate() {
                log::info!("device #{}: {}", i, d.name)
            }

            *device.device.write() = match config.device.as_ref() {
                Some(device) => devices.into_iter().find(|d| d.name == *device),
                None => devices.into_iter().next(),
            };

            player.pause(Source::Automatic)?;

            if let Some(volume) = config.volume {
                player.volume(Source::Automatic, volume)?;
            }
        }
    }

    Ok((future, player))
}

/// Error value returned if a device has not been configured.
pub struct NotConfigured;

/// Events emitted by the player.
#[derive(Debug, Clone)]
pub enum Event {
    /// Player is empty.
    Empty,
    /// Player is playing the given song.
    Playing(bool, Arc<Item>),
    /// Player is pausing.
    Pausing,
    /// queue was modified in some way.
    Modified,
    /// player has not been configured.
    NotConfigured,
    /// Player is detached.
    Detached,
}

/// Information on current song.
#[derive(Debug, Clone)]
pub struct Song {
    pub item: Arc<Item>,
    /// Since the last time it was unpaused, what was the initial elapsed duration.
    elapsed: Duration,
    /// When the current song started playing.
    started_at: Option<Instant>,
}

impl Song {
    /// Create a new current song.
    pub fn new(item: Arc<Item>, elapsed: Duration) -> Self {
        Song {
            item,
            elapsed,
            started_at: None,
        }
    }

    /// Test if the two songs reference roughly the same song.
    pub fn is_same(&self, song: &Song) -> bool {
        if self.item.track_id != song.item.track_id {
            return false;
        }

        let a = self.elapsed();
        let b = song.elapsed();
        let diff = if a > b { a - b } else { b - a };

        if diff.as_secs() > 5 {
            return false;
        }

        true
    }

    /// Convert a playback information into a Song struct.
    pub fn from_playback(playback: &api::spotify::FullPlayingContext) -> Option<Self> {
        let progress_ms = playback.progress_ms.unwrap_or_default();

        let track = match playback.item.clone() {
            Some(track) => track,
            _ => return None,
        };

        let track_id: TrackId = match str::parse(&track.id) {
            Ok(track_id) => track_id,
            Err(_) => return None,
        };

        let elapsed = Duration::from_millis(progress_ms as u64);
        let duration = Duration::from_millis(track.duration_ms.into());

        let item = Arc::new(Item {
            track_id,
            track: Track::Spotify { track },
            user: None,
            duration,
        });

        let mut song = Song::new(item, elapsed);

        if playback.is_playing {
            song.play();
        } else {
            song.pause();
        }

        Some(song)
    }

    /// Get the deadline for when this song will end, assuming it is currently playing.
    pub fn deadline(&self) -> Instant {
        Instant::now() + self.remaining()
    }

    /// Duration of the current song.
    pub fn duration(&self) -> Duration {
        self.item.duration.clone()
    }

    /// Elapsed time on current song.
    ///
    /// Elapsed need to take started at into account.
    pub fn elapsed(&self) -> Duration {
        let when = self
            .started_at
            .as_ref()
            .and_then(|started_at| {
                let now = Instant::now();

                if now > *started_at {
                    Some(now - *started_at)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        when.checked_add(self.elapsed.clone()).unwrap_or_default()
    }

    /// Remaining time of the current song.
    pub fn remaining(&self) -> Duration {
        self.item
            .duration
            .checked_sub(self.elapsed())
            .unwrap_or_default()
    }

    /// Get serializable data for this item.
    pub fn data(&self, state: State) -> Result<CurrentData<'_>, failure::Error> {
        let artists = self.item.track.artists();

        Ok(CurrentData {
            paused: !state.is_playing(),
            track_id: &self.item.track_id,
            name: self.item.track.name(),
            artists,
            user: self.item.user.as_ref().map(|s| s.as_str()),
            duration: utils::digital_duration(&self.item.duration),
            elapsed: utils::digital_duration(&self.elapsed()),
        })
    }

    /// Check if the song is currently playing.
    pub fn state(&self) -> State {
        match self.started_at.is_some() {
            true => State::Playing,
            false => State::Paused,
        }
    }

    /// Get the player kind for the current song.
    pub fn player(&self) -> PlayerKind {
        match self.item.track_id {
            TrackId::Spotify(..) => PlayerKind::Spotify,
            TrackId::YouTube(..) => PlayerKind::YouTube,
        }
    }

    /// Set the started_at time to now.
    /// For safety, update the current `elapsed` time based on any prior `started_at`.
    pub fn play(&mut self) {
        let duration = self.take_started_at();
        self.elapsed += duration;
        self.started_at = Some(Instant::now());
    }

    /// Update the elapsed time based on when this song was started.
    pub fn pause(&mut self) {
        let duration = self.take_started_at();
        self.elapsed += duration;
    }

    /// Take the current started_at as a duration and leave it as None.
    fn take_started_at(&mut self) -> Duration {
        let started_at = match self.started_at.take() {
            Some(started_at) => started_at,
            None => return Default::default(),
        };

        let now = Instant::now();

        if now < started_at {
            return Default::default();
        }

        now - started_at
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrentData<'a> {
    paused: bool,
    track_id: &'a TrackId,
    name: String,
    artists: Option<String>,
    user: Option<&'a str>,
    duration: String,
    elapsed: String,
}

/// A handler for the player.
#[derive(Clone)]
pub struct Player {
    device: self::connect::ConnectDevice,
    queue: Queue,
    db: db::Database,
    max_queue_length: Arc<RwLock<u32>>,
    max_songs_per_user: Arc<RwLock<u32>>,
    duplicate_duration: Arc<RwLock<utils::Duration>>,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    commands_tx: mpsc::UnboundedSender<Command>,
    bus: Arc<RwLock<Bus<Event>>>,
    volume: Arc<RwLock<u32>>,
    /// The current song that is loaded.
    song: Arc<RwLock<Option<Song>>>,
    /// Theme songs.
    themes: db::Themes,
    /// Player is closed for more requests.
    closed: Arc<RwLock<Option<Option<Arc<String>>>>>,
}

impl Player {
    /// The client components of the player.
    pub fn client(&self) -> PlayerClient {
        PlayerClient {
            device: self.device.clone(),
            db: self.db.clone(),
            queue: self.queue.clone(),
            thread_pool: Arc::new(ThreadPool::new()),
            max_queue_length: self.max_queue_length.clone(),
            max_songs_per_user: self.max_songs_per_user.clone(),
            duplicate_duration: self.duplicate_duration.clone(),
            spotify: self.spotify.clone(),
            youtube: self.youtube.clone(),
            commands_tx: self.commands_tx.clone(),
            bus: self.bus.clone(),
            volume: Arc::clone(&self.volume),
            song: self.song.clone(),
            themes: self.themes.clone(),
            closed: self.closed.clone(),
        }
    }

    /// Get a receiver for player events.
    pub fn add_rx(&self) -> BusReader<Event> {
        self.bus.write().add_rx()
    }

    /// Pause playback.
    pub fn pause(&self, source: Source) -> Result<(), failure::Error> {
        self.send(Command::Pause(source))
    }

    /// Synchronize playback with the given song.
    pub fn play_sync(&self, song: Option<Song>) -> Result<(), failure::Error> {
        self.send(Command::PlaySync { song })
    }

    /// Update volume of the player.
    pub fn volume(&self, source: Source, volume: u32) -> Result<(), failure::Error> {
        self.send(Command::Volume(source, u32::min(100u32, volume)))
    }

    /// Send the given command.
    fn send(&self, command: Command) -> Result<(), failure::Error> {
        self.commands_tx
            .unbounded_send(command)
            .map_err(|_| format_err!("failed to send command"))
    }
}

/// All parts of a Player that can be shared between threads.
#[derive(Clone)]
pub struct PlayerClient {
    device: self::connect::ConnectDevice,
    db: db::Database,
    queue: Queue,
    thread_pool: Arc<ThreadPool>,
    max_queue_length: Arc<RwLock<u32>>,
    max_songs_per_user: Arc<RwLock<u32>>,
    duplicate_duration: Arc<RwLock<utils::Duration>>,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    commands_tx: mpsc::UnboundedSender<Command>,
    bus: Arc<RwLock<Bus<Event>>>,
    /// Song volume.
    volume: Arc<RwLock<u32>>,
    /// Song song that is loaded.
    song: Arc<RwLock<Option<Song>>>,
    /// Theme songs.
    themes: db::Themes,
    /// Player is closed for more requests.
    closed: Arc<RwLock<Option<Option<Arc<String>>>>>,
}

impl PlayerClient {
    /// Get a receiver for player events.
    pub fn add_rx(&self) -> BusReader<Event> {
        self.bus.write().add_rx()
    }

    /// Get the current device.
    pub fn current_device(&self) -> Option<api::spotify::Device> {
        self.device.current_device()
    }

    /// List all available devices.
    pub fn list_devices(
        &self,
    ) -> impl Future<Item = Vec<api::spotify::Device>, Error = failure::Error> {
        self.device.list_devices()
    }

    /// External call to set device.
    ///
    /// Should always notify the player to change.
    pub fn set_device(&self, device: api::spotify::Device) -> Option<api::spotify::Device> {
        self.device.set_device(Some(device))
    }

    /// Clear the current device.
    pub fn clear_device(&self) -> Option<api::spotify::Device> {
        self.device.set_device(None)
    }

    /// Send the given command.
    fn send(&self, command: Command) -> Result<(), failure::Error> {
        self.commands_tx
            .unbounded_send(command)
            .map_err(|_| format_err!("failed to send command"))
    }

    /// Get the next N songs in queue.
    pub fn list(&self) -> Vec<Arc<Item>> {
        let song = self.song.read();
        let queue = self.queue.queue.read();

        song.as_ref()
            .map(|c| c.item.clone())
            .into_iter()
            .chain(queue.iter().cloned())
            .collect()
    }

    /// Promote the given song to the head of the queue.
    pub fn promote_song(&self, user: &str, n: usize) -> Option<Arc<Item>> {
        let promoted = self.queue.promote_song(user, n);

        if promoted.is_some() {
            self.modified();
        }

        promoted
    }

    /// Toggle playback.
    pub fn toggle(&self) -> Result<(), failure::Error> {
        self.send(Command::Toggle(Source::Manual))
    }

    /// Start playback.
    pub fn play(&self) -> Result<(), failure::Error> {
        self.send(Command::Play(Source::Manual))
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<(), failure::Error> {
        self.send(Command::Pause(Source::Manual))
    }

    /// Skip the current song.
    pub fn skip(&self) -> Result<(), failure::Error> {
        self.send(Command::Skip(Source::Manual))
    }

    /// Update volume of the player.
    pub fn volume(&self, volume: u32) -> Result<(), failure::Error> {
        self.send(Command::Volume(Source::Manual, u32::min(100u32, volume)))
    }

    /// Get the current volume.
    pub fn current_volume(&self) -> u32 {
        *self.volume.read()
    }

    /// Close the player from more requests.
    pub fn close(&self, reason: Option<String>) {
        *self.closed.write() = Some(reason.map(Arc::new));
    }

    /// Open the player.
    pub fn open(&self) {
        *self.closed.write() = None;
    }

    /// Search for a track.
    pub fn search_track(&self, q: &str) -> utils::BoxFuture<Option<TrackId>, failure::Error> {
        if q.starts_with("youtube:") {
            let q = q.trim_start_matches("youtube:");

            return Box::new(self.youtube.search(q).and_then(|results| {
                let result = results.items.into_iter().filter(|r| match r.id.kind {
                    api::youtube::Kind::Video => true,
                    _ => false,
                });

                let mut result = result.flat_map(|r| r.id.video_id);

                match result.next() {
                    Some(id) => Ok(Some(TrackId::YouTube(id))),
                    None => Ok(None),
                }
            }));
        }

        let q = if q.starts_with("spotify:") {
            q.trim_start_matches("spotify:")
        } else {
            q
        };

        Box::new(self.spotify.search_track(q).and_then(
            |page| match page.items.into_iter().next() {
                Some(track) => match SpotifyId::from_base62(&track.id) {
                    Ok(track_id) => Ok(Some(TrackId::Spotify(track_id))),
                    Err(_) => Err(failure::format_err!("search result returned malformed id")),
                },
                None => Ok(None),
            },
        ))
    }

    /// Play a theme track.
    pub fn play_theme(
        &self,
        channel: &str,
        name: &str,
    ) -> impl Future<Item = (), Error = PlayThemeError> {
        let fut = future::lazy({
            let themes = self.themes.clone();
            let channel = channel.to_string();
            let name = name.to_string();

            move || match themes.get(&channel, &name) {
                Some(theme) => Ok(theme),
                None => Err(PlayThemeError::NoSuchTheme),
            }
        });

        let fut = fut.and_then({
            let thread_pool = Arc::clone(&self.thread_pool);
            let spotify = Arc::clone(&self.spotify);
            let youtube = Arc::clone(&self.youtube);

            move |theme| {
                let duration = theme.end.clone().map(|o| o.as_duration());

                convert_item(
                    &thread_pool,
                    spotify,
                    youtube,
                    None,
                    theme.track_id.clone(),
                    duration,
                )
                .map(move |item| (item, theme))
                .map_err(|e| PlayThemeError::Error(e.into()))
            }
        });

        fut.and_then({
            let commands_tx = self.commands_tx.clone();

            move |(item, theme)| {
                let item = Arc::new(item);
                let duration = theme.start.as_duration();

                commands_tx
                    .unbounded_send(Command::Inject(Source::Manual, item, duration))
                    .map_err(|e| PlayThemeError::Error(e.into()))
            }
        })
    }

    /// Add the given track to the queue.
    ///
    /// Returns the item added.
    pub fn add_track(
        &self,
        channel: &str,
        user: &str,
        track_id: TrackId,
        is_moderator: bool,
        max_duration: Option<utils::Duration>,
        min_currency: Option<i64>,
    ) -> impl Future<Item = (usize, Arc<Item>), Error = AddTrackError> {
        // invariant checks
        let fut = future::lazy({
            let db = self.db.clone();
            let queue = self.queue.clone();
            let max_queue_length = *self.max_queue_length.read();
            let max_songs_per_user = *self.max_songs_per_user.read();
            let duplicate_duration = self.duplicate_duration.read().clone();
            let closed = self.closed.clone();
            let channel = channel.to_string();
            let user = user.to_string();
            let track_id = track_id.clone();

            move || {
                // NB: immediate access to the queue.
                let queue_inner = queue.queue.read();

                let len = queue_inner.len();

                if !is_moderator {
                    if let Some(reason) = closed.read().as_ref() {
                        return Err(AddTrackError::PlayerClosed(reason.clone()));
                    }

                    // NB: moderator is allowed to violate max queue length.
                    if len >= max_queue_length as usize {
                        return Err(AddTrackError::QueueFull);
                    }

                    if !duplicate_duration.is_empty() {
                        if let Some(last) = queue
                            .last_song_within(&track_id, duplicate_duration.clone())
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

                    if let Some(min_currency) = min_currency {
                        let balance = db
                            .balance_of(channel.as_str(), user.as_str())
                            .map_err(AddTrackError::Error)?
                            .unwrap_or_default();

                        if balance < min_currency {
                            return Err(AddTrackError::NotEnoughCurrency {
                                balance,
                                required: min_currency,
                            });
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
            let youtube = Arc::clone(&self.youtube);

            move |len| {
                convert_item(&thread_pool, spotify, youtube, Some(user), track_id, None)
                    .map(move |item| (len, item))
                    .map_err(|e| AddTrackError::Error(e.into()))
            }
        });

        let fut = fut.map(move |(len, mut item)| {
            if let Some(max_duration) = max_duration {
                let max_duration = max_duration.as_std();

                if item.duration > max_duration {
                    item.duration = max_duration;
                }
            }

            (len, item)
        });

        let fut = fut.and_then({
            let queue = self.queue.clone();

            move |(len, item)| {
                let item = Arc::new(item);

                queue
                    .push_back(item.clone())
                    .map(move |_| (len, item))
                    .map_err(|e| AddTrackError::Error(e.into()))
            }
        });

        fut.and_then({
            let commands_tx = self.commands_tx.clone();

            move |(len, item)| {
                commands_tx
                    .unbounded_send(Command::Modified(Source::Manual))
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
        let purged = self.queue.purge()?;

        if !purged.is_empty() {
            self.modified();
        }

        Ok(purged)
    }

    /// Remove the item at the given position.
    pub fn remove_at(&self, n: usize) -> Result<Option<Arc<Item>>, failure::Error> {
        let removed = self.queue.remove_at(n)?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Remove the first track in the queue.
    pub fn remove_last(&self) -> Result<Option<Arc<Item>>, failure::Error> {
        let removed = self.queue.remove_last()?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Remove the last track by the given user.
    pub fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>, failure::Error> {
        let removed = self.queue.remove_last_by_user(user)?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Find the next item that matches the given predicate and how long until it plays.
    pub fn find(&self, mut predicate: impl FnMut(&Item) -> bool) -> Option<(Duration, Arc<Item>)> {
        let mut duration = Duration::default();

        if let Some(c) = self.song.read().as_ref() {
            if predicate(&c.item) {
                return Some((Default::default(), c.item.clone()));
            }

            duration += c.remaining();
        }

        let queue = self.queue.queue.read();

        for item in &*queue {
            if predicate(item) {
                return Some((duration, item.clone()));
            }

            duration += item.duration;
        }

        None
    }

    /// Get the length in number of items and total number of seconds in queue.
    pub fn length(&self) -> (usize, Duration) {
        let mut count = 0;
        let mut duration = Duration::default();

        if let Some(item) = self.song.read().as_ref() {
            duration += item.remaining();
            count += 1;
        }

        let queue = self.queue.queue.read();

        for item in &*queue {
            duration += item.duration;
        }

        count += queue.len();
        (count, duration)
    }

    /// Get the current song, if it is set.
    pub fn current(&self) -> Option<Song> {
        self.song.read().clone()
    }

    /// Indicate that the queue has been modified.
    fn modified(&self) {
        if let Err(e) = self
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
    /// Duplicate song that was added at the specified time by the specified user.
    Duplicate(DateTime<Utc>, Option<String>, Duration),
    /// Not enough currency to request songs.
    NotEnoughCurrency { required: i64, balance: i64 },
    /// Other generic error happened.
    Error(failure::Error),
}

/// The backend of a words store.
pub trait Backend: Clone + Send + Sync {
    /// List all counters in backend.
    fn list(&self) -> Result<Vec<db::models::Song>, failure::Error>;

    /// Insert the given song into the backend.
    fn push_back(&self, song: &db::models::AddSong) -> Result<(), failure::Error>;

    /// Remove the song, but only log on issues.
    fn remove_song_log(&self, track_id: &TrackId) {
        match self.remove_song(track_id) {
            Err(e) => log::warn!("{}: failed to remove song from database: {}", track_id, e),
            Ok(false) => log::warn!("{}: no songs removed from database", track_id),
            Ok(true) => {}
        }
    }

    /// Remove the song with the given ID.
    fn remove_song(&self, track_id: &TrackId) -> Result<bool, failure::Error>;

    /// Purge the songs database, but only log on issues.
    fn song_purge_log(&self) -> Option<usize> {
        match self.song_purge() {
            Err(e) => {
                log::warn!("failed to purge songs from database: {}", e);
                None
            }
            Ok(n) => Some(n),
        }
    }

    /// Purge the songs database and return the number of items removed.
    fn song_purge(&self) -> Result<usize, failure::Error>;

    /// Purge the songs database, but only log on issues.
    fn promote_song_log(&self, user: &str, track_id: &TrackId) -> Option<bool> {
        match self.promote_song(user, track_id) {
            Err(e) => {
                log::warn!("failed to promote song `{}` in database: {}", track_id, e);
                None
            }
            Ok(n) => Some(n),
        }
    }

    /// Promote the track with the given ID.
    fn promote_song(&self, user: &str, track_id: &TrackId) -> Result<bool, failure::Error>;

    /// Test if the song has been played within a given duration.
    fn last_song_within(
        &self,
        track_id: &TrackId,
        duration: utils::Duration,
    ) -> Result<Option<db::models::Song>, failure::Error>;
}

/// The playback queue.
#[derive(Clone)]
struct Queue {
    db: db::Database,
    queue: Arc<RwLock<VecDeque<Arc<Item>>>>,
    thread_pool: Arc<ThreadPool>,
}

impl Queue {
    /// Construct a new queue.
    pub fn new(db: db::Database) -> Self {
        Self {
            db,
            queue: Arc::new(RwLock::new(Default::default())),
            thread_pool: Arc::new(ThreadPool::new()),
        }
    }

    /// Check ifa song has been queued within the specified period of time.
    pub fn last_song_within(
        &self,
        track_id: &TrackId,
        duration: utils::Duration,
    ) -> Result<Option<db::models::Song>, failure::Error> {
        self.db.last_song_within(track_id, duration)
    }

    /// Get the front of the queue.
    pub fn front(&self) -> Option<Arc<Item>> {
        self.queue.read().front().cloned()
    }

    /// Pop the front of the queue.
    pub fn pop_front(&self) -> PopFrontFuture {
        let db = self.db.clone();
        let queue = self.queue.clone();

        PopFrontFuture(self.thread_pool.spawn_handle(future::lazy(move || {
            if let Some(item) = queue.write().pop_front() {
                db.remove_song_log(&item.track_id);
            }

            Ok(None)
        })))
    }

    /// Push item to back of queue.
    pub fn push_back(&self, item: Arc<Item>) -> PushBackFuture {
        let db = self.db.clone();
        let queue = self.queue.clone();

        PushBackFuture(self.thread_pool.spawn_handle(future::lazy(move || {
            db.push_back(&db::models::AddSong {
                track_id: item.track_id.clone(),
                added_at: Utc::now().naive_utc(),
                user: item.user.clone(),
            })?;

            queue.write().push_back(item);
            Ok(())
        })))
    }

    /// Purge the song queue.
    pub fn purge(&self) -> Result<Vec<Arc<Item>>, failure::Error> {
        let mut q = self.queue.write();

        if q.is_empty() {
            return Ok(vec![]);
        }

        let purged = std::mem::replace(&mut *q, VecDeque::new())
            .into_iter()
            .collect();
        self.db.song_purge_log();
        Ok(purged)
    }

    /// Remove the item at the given position.
    pub fn remove_at(&self, n: usize) -> Result<Option<Arc<Item>>, failure::Error> {
        let mut q = self.queue.write();

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(item) = q.remove(n) {
            self.db.remove_song_log(&item.track_id);
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element.
    pub fn remove_last(&self) -> Result<Option<Arc<Item>>, failure::Error> {
        let mut q = self.queue.write();

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(item) = q.pop_back() {
            self.db.remove_song_log(&item.track_id);
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element by user.
    pub fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>, failure::Error> {
        let mut q = self.queue.write();

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(position) = q
            .iter()
            .rposition(|i| i.user.as_ref().map(|u| u == user).unwrap_or_default())
        {
            if let Some(item) = q.remove(position) {
                self.db.remove_song_log(&item.track_id);
                return Ok(Some(item));
            }
        }

        Ok(None)
    }

    /// Promote the given song.
    pub fn promote_song(&self, user: &str, n: usize) -> Option<Arc<Item>> {
        let mut q = self.queue.write();

        // OK, but song doesn't exist or index is out of bound.
        if q.is_empty() || n >= q.len() {
            return None;
        }

        if let Some(removed) = q.remove(n) {
            q.push_front(removed);
        }

        if let Some(item) = q.get(0).cloned() {
            self.db.promote_song_log(user, &item.track_id);
            return Some(item);
        }

        None
    }

    /// Push item to back of queue without going through the database.
    fn push_back_queue(&self, item: Arc<Item>) {
        self.queue.write().push_back(item);
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

/// Mixer decides what song to play next.
pub struct Mixer {
    /// Persistent queue to take songs from.
    queue: Queue,
    /// A song that has been sidelined by another song.
    sidelined: VecDeque<Song>,
    /// Items to fall back to when there are no more songs in queue.
    fallback_items: Vec<Arc<Item>>,
    /// Items ordered in the reverse way they are meant to be played.
    fallback_queue: VecDeque<Arc<Item>>,
    /// Future associated with popping the front control.
    pop_front: Option<PopFrontFuture>,
}

impl Mixer {
    /// The minimum size of the fallback queue.
    const FALLBACK_QUEUE_SIZE: usize = 10;

    /// Get next song to play.
    ///
    /// Will shuffle all fallback items and add them to a queue to avoid playing the same song twice.
    fn next_fallback_item(&mut self) -> Option<Song> {
        use rand::seq::SliceRandom;

        if self.fallback_items.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();

        while self.fallback_queue.len() < Self::FALLBACK_QUEUE_SIZE {
            let mut extension = self.fallback_items.clone();
            extension.shuffle(&mut rng);
            self.fallback_queue.extend(extension);
        }

        let item = self.fallback_queue.pop_front()?;
        Some(Song::new(item, Default::default()))
    }

    /// Get the next song that should be played.
    ///
    /// This takes into account:
    /// If there are any songs to be injected (e.g. theme songs).
    /// If there are any songs that have been sidelines by injected songs.
    /// If there are any songs in the queue.
    ///
    /// Finally, if there are any songs to fall back to.
    fn next_song(&mut self) -> Option<Song> {
        if let Some(song) = self.sidelined.pop_front() {
            return Some(song);
        }

        // Take next from queue.
        if let Some(item) = self.queue.front() {
            self.pop_front = Some(self.queue.pop_front());
            return Some(Song::new(item, Default::default()));
        }

        if self.fallback_items.is_empty() {
            log::warn!("there are no fallback songs available");
            return None;
        }

        self.next_fallback_item()
    }
}

pub struct EventBus {
    bus: Arc<RwLock<Bus<Event>>>,
}

impl EventBus {
    /// Broadcast an event from the player.
    fn broadcast(&self, event: Event) {
        let mut b = self.bus.write();

        if let Err(e) = b.try_broadcast(event) {
            log::error!("failed to broadcast player event: {:?}", e);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Spotify,
    YouTube,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Playing,
    Paused,
    // initial undefined state.
    None,
}

impl State {
    /// Check if the state is playing.
    pub fn is_playing(self) -> bool {
        match self {
            State::Playing => true,
            _ => false,
        }
    }
}

/// Future associated with driving audio playback.
pub struct PlaybackFuture {
    connect_player: self::connect::ConnectPlayer,
    youtube_player: self::youtube::YouTubePlayer,
    commands: mpsc::UnboundedReceiver<Command>,
    bus: EventBus,
    mixer: Mixer,
    /// We are currently playing.
    state: State,
    /// Current player kind.
    player: PlayerKind,
    /// Player is detached.
    detached: bool,
    /// Stream of settings if the player is detached.
    detached_stream: settings::Stream<bool>,
    /// Song volume.
    volume: Arc<RwLock<u32>>,
    /// Song that is currently loaded.
    song: Arc<RwLock<Option<Song>>>,
    /// Path to write current song.
    current_song: Option<Arc<current_song::CurrentSong>>,
    /// Song config.
    echo_current_song: bool,
    /// Optional stream indicating when current song should update.
    current_song_update: Option<tokio_timer::Interval>,
    /// Optional stream indicating that we want to send a song update on the global bus.
    song_update_interval: Option<tokio_timer::Interval>,
    /// Stream for when song update interval is updated.
    song_update_interval_stream: settings::Stream<utils::Duration>,
    /// Notifier to use when sending song updates.
    global_bus: Arc<bus::Bus<bus::Global>>,
}

impl PlaybackFuture {
    /// Set the current song.
    fn write_song(&self, song: Option<Song>) -> Result<(), failure::Error> {
        self.global_bus.send(bus::Global::song(song.as_ref())?);
        self.current_song(song.as_ref());
        *self.song.write() = song;
        Ok(())
    }

    /// Write current song. Log any errors.
    ///
    /// MUST NOT be called when self.song is locked.
    fn current_song(&self, song: Option<&Song>) {
        let current_song = match self.current_song.as_ref() {
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

    /// Update current player.
    fn set_current_player(&mut self, player: PlayerKind) {
        use self::PlayerKind::*;

        match (self.player, player) {
            (Spotify, Spotify) => (),
            (Spotify, _) => self.connect_player.stop(),
            (YouTube, YouTube) => (),
            (YouTube, _) => self.youtube_player.stop(),
            (None, YouTube) => {
                self.connect_player.stop();
            }
            (None, Spotify) => {
                self.youtube_player.stop();
            }
            _ => (),
        }

        self.player = player;
    }

    /// Update state if it is different from the expected kind and pause the corresponding player.
    fn set_playing(&mut self, player: PlayerKind) {
        self.set_current_player(player);
        self.state = State::Playing;
    }

    /// Play the given song.
    fn play_song(&mut self, source: Source, song: &Song) {
        match song.item.track_id {
            TrackId::Spotify(ref id) => {
                self.connect_player.play(source, &song, id);
                self.set_playing(PlayerKind::Spotify);
            }
            TrackId::YouTube(ref id) => {
                self.youtube_player.play(source, &song, id);
                self.set_playing(PlayerKind::YouTube);
            }
        }
    }

    /// Switch current song to the specified song.
    fn switch_to_song(&mut self, source: Source, song: Song) {
        self.play_song(source, &song);
        *self.song.write() = Some(song);
    }

    /// Detach the player.
    fn detach(&mut self) -> Result<(), failure::Error> {
        self.connect_player.detach();
        self.youtube_player.detach();

        // store the currently playing song in the sidelined slot.
        if let Some(mut song) = self.song.write().take() {
            song.pause();
            self.mixer.sidelined.push_back(song);
        }

        self.write_song(None)?;
        self.player = PlayerKind::None;
        self.state = State::None;
        Ok(())
    }

    /// Handle incoming command.
    fn command(&mut self, command: Command) -> Result<(), failure::Error> {
        if self.detached {
            log::trace!(
                "Ignoring: Command = {:?}, State = {:?}, Player = {:?}",
                command,
                self.state,
                self.player,
            );

            if let Source::Manual = command.source() {
                self.bus.broadcast(Event::Detached);
            }

            return Ok(());
        }

        log::trace!(
            "Processing: Command = {:?}, State = {:?}, Player = {:?}",
            command,
            self.state,
            self.player,
        );

        let command = match command {
            Command::Toggle(source) if !self.state.is_playing() => Command::Play(source),
            Command::Toggle(source) if self.state.is_playing() => Command::Pause(source),
            command => command,
        };

        match (command, self.state) {
            (Command::Skip(source), _) => {
                log::trace!("skipping song");

                if let Some(song) = self.mixer.next_song() {
                    if self.state.is_playing() {
                        self.switch_to_song(source, song);
                    } else {
                        self.set_current_player(song.player());
                        *self.song.write() = Some(song);
                    }
                } else {
                    if let Source::Manual = source {
                        self.bus.broadcast(Event::Empty);
                    }

                    self.write_song(None)?;
                    self.set_current_player(PlayerKind::None);
                    self.state = State::Paused;
                }
            }
            // initial pause
            (Command::Pause(source), State::None) => {
                self.connect_player.pause(source);
                self.youtube_player.pause(source);
                self.state = State::Paused;
            }
            (Command::Pause(source), State::Playing) => match self.player {
                PlayerKind::Spotify => {
                    log::trace!("pausing spotify player");
                    self.connect_player.pause(source);
                    self.state = State::Paused;
                }
                PlayerKind::YouTube => {
                    log::trace!("pausing youtube player");
                    self.youtube_player.pause(source);
                    self.state = State::Paused;
                }
                _ => (),
            },
            (Command::Play(source), State::Paused) | (Command::Play(source), State::None) => {
                log::trace!("starting player");

                let song = self.song.clone();

                // resume an existing song
                if let Some(song) = song.read().as_ref() {
                    self.play_song(source, song);
                    return Ok(());
                }

                // play the next song in queue.
                if let Some(song) = self.mixer.next_song() {
                    self.switch_to_song(source, song);
                } else {
                    if let Source::Manual = source {
                        self.bus.broadcast(Event::Empty);
                    }

                    self.state = State::Paused;
                    self.write_song(None)?;
                }
            }
            (Command::PlaySync { song }, _) => {
                log::trace!("synchronize the state of the player with the current song");

                if let Some(s) = song.as_ref() {
                    match s.item.track_id {
                        TrackId::Spotify(..) => {
                            self.connect_player.play_sync(song.as_ref());
                            self.set_playing(PlayerKind::Spotify);
                        }
                        // NB: doesn't make sense, there is no way to poll the youtube player.
                        TrackId::YouTube(..) => {
                            self.set_playing(PlayerKind::YouTube);
                            log::warn!("youtube player doesn't support syncing");
                            return Ok(());
                        }
                    }
                } else {
                    self.state = State::Paused;
                    self.set_current_player(PlayerKind::None);
                }

                *self.song.write() = song;
            }
            // queue was modified in some way
            (Command::Modified(source), _) => {
                if self.state.is_playing() && self.song.read().is_none() {
                    if let Some(song) = self.mixer.next_song() {
                        self.switch_to_song(source, song);
                    }
                }

                self.bus.broadcast(Event::Modified);
            }
            (Command::Volume(source, volume), _) => {
                self.connect_player.volume(source, volume);
                self.youtube_player.volume(source, volume);
                *self.volume.write() = volume;
            }
            (Command::Inject(source, item, offset), State::Playing) => {
                // store the currently playing song in the sidelined slot.
                if let Some(mut song) = self.song.write().take() {
                    song.pause();
                    self.mixer.sidelined.push_back(song);
                }

                self.switch_to_song(source, Song::new(item, offset));
            }
            _ => (),
        }

        Ok(())
    }

    /// Handle an event from the connect integration.
    fn handle_player_event(&mut self, e: IntegrationEvent) -> Result<(), failure::Error> {
        use self::IntegrationEvent::*;

        if self.detached {
            log::trace!(
                "Ignoring: IntegrationEvent = {:?}, State = {:?}, Player = {:?}",
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
            EndOfTrack => {
                log::trace!("Song ended, loading next song...");

                if let Some(song) = self.mixer.next_song() {
                    self.switch_to_song(Source::Manual, song);
                } else {
                    self.bus.broadcast(Event::Empty);
                    self.write_song(None)?;
                }
            }
            DeviceChanged => {
                if !self.state.is_playing() {
                    return Ok(());
                }

                let volume = *self.volume.read();

                let song = self.song.clone();
                let mut song = song.write();

                let song = match song.as_mut() {
                    Some(song) => song,
                    None => return Ok(()),
                };

                // pause so that it can get unpaused later.
                song.pause();

                match song.item.track_id {
                    TrackId::Spotify(ref id) => {
                        self.connect_player.play(Source::Automatic, &song, id);
                        self.connect_player.volume(Source::Automatic, volume);
                        self.set_playing(PlayerKind::Spotify);
                    }
                    TrackId::YouTube(ref id) => {
                        self.youtube_player.play(Source::Automatic, &song, id);
                        self.youtube_player.volume(Source::Automatic, volume);
                        self.set_playing(PlayerKind::YouTube);
                    }
                }
            }
            Playing(source) => {
                let mut song = self.song.write();

                if let Some(song) = song.as_mut() {
                    log::trace!("marking track as playing: {}", song.item.track_id);
                    song.play();

                    if let Source::Manual = source {
                        self.bus
                            .broadcast(Event::Playing(self.echo_current_song, song.item.clone()));
                    }
                }

                self.global_bus.send(bus::Global::song(song.as_ref())?);
                self.current_song(song.as_ref());
            }
            // A player stopped due to a switch.
            Stopping => (),
            Pausing(source) => {
                let mut song = self.song.write();

                if let Some(song) = song.as_mut() {
                    song.pause();
                }

                self.current_song(song.as_ref());
                self.global_bus.send(bus::Global::song(song.as_ref())?);

                if let Source::Manual = source {
                    self.bus.broadcast(Event::Pausing);
                }
            }
            Volume(..) => {}
            other => {
                log::trace!("player event: {:?}", other);
            }
        }

        Ok(())
    }

    /// Handle global song updates.
    fn handle_global_song_updates(&mut self) -> Result<bool, failure::Error> {
        let mut not_ready = true;

        if let Some(value) = try_infinite!(self.song_update_interval_stream.poll()) {
            self.song_update_interval = match value.is_empty() {
                true => None,
                false => Some(tokio_timer::Interval::new_interval(value.as_std())),
            };

            not_ready = false;
        }

        if let Some(song_update_interval) = self.song_update_interval.as_mut() {
            if let Some(_) = try_infinite!(song_update_interval.poll()) {
                let song = self.song.read();

                if self.state.is_playing() {
                    self.global_bus
                        .send(bus::Global::song_progress(song.as_ref()));

                    if let Some(song) = song.as_ref() {
                        if let TrackId::YouTube(ref id) = song.item.track_id {
                            self.youtube_player.tick(song, id);
                        }
                    }
                }

                not_ready = false;
            }
        }

        Ok(not_ready)
    }
}

impl Future for PlaybackFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use futures::Async;

        loop {
            let mut not_ready = true;

            if let Some(update) = try_infinite!(self.detached_stream.poll()) {
                if update {
                    self.detach()?;
                }

                self.detached = update;
                not_ready = false;
            }

            if let Some(current_song_update) = self.current_song_update.as_mut() {
                if let Some(_) = try_infinite!(current_song_update.poll()) {
                    let song = self.song.read();
                    self.current_song(song.as_ref());
                    not_ready = false;
                }
            }

            if !self.handle_global_song_updates()? {
                not_ready = false;
            }

            // pop is in progress, make sure that happens before anything else.
            if let Some(mut pop_front) = self.mixer.pop_front.take() {
                match pop_front.poll()? {
                    Async::NotReady => self.mixer.pop_front = Some(pop_front),
                    Async::Ready(_) => not_ready = false,
                }
            }

            if let Some(event) = try_infinite!(self.connect_player.poll()) {
                self.handle_player_event(event)?;
                not_ready = false;
            }

            if let Some(event) = try_infinite!(self.youtube_player.poll()) {
                self.handle_player_event(event)?;
                not_ready = false;
            }

            if let Some(command) = try_infinite_empty!(self.commands.poll()) {
                self.command(command)?;
                not_ready = false;
            }

            if not_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}

/// Convert a playlist into items.
fn playlist_to_items(
    core: &mut Core,
    spotify: Arc<api::Spotify>,
    playlist: &str,
) -> Result<Vec<Arc<Item>>, failure::Error> {
    let mut items = Vec::new();

    let playlist = core.run(spotify.playlist(playlist))?;

    for playlist_track in core.run(spotify.page_as_stream(playlist.tracks).concat2())? {
        let track = playlist_track.track;

        let track_id = TrackId::Spotify(
            SpotifyId::from_base62(&track.id)
                .map_err(|_| format_err!("bad spotify id: {}", track.id))?,
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

/// Convert all songs of a user into items.
fn songs_to_items(
    core: &mut Core,
    spotify: Arc<api::Spotify>,
) -> Result<Vec<Arc<Item>>, failure::Error> {
    let mut items = Vec::new();

    for added_song in core.run(spotify.my_tracks_stream().concat2())? {
        let track = added_song.track;

        let track_id = TrackId::Spotify(
            SpotifyId::from_base62(&track.id)
                .map_err(|_| format_err!("bad spotify id: {}", track.id))?,
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

/// Converts a track into an Item.
fn convert_item(
    thread_pool: &ThreadPool,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    user: Option<String>,
    track_id: TrackId,
    duration_override: Option<Duration>,
) -> impl Future<Item = Item, Error = failure::Error> {
    let future: utils::BoxFuture<Item, failure::Error> = match track_id {
        TrackId::Spotify(ref id) => {
            let track_id_string = id.to_base62();

            Box::new(spotify.track(&track_id_string).map(move |track| {
                let duration = match duration_override {
                    Some(duration) => duration,
                    None => Duration::from_millis(track.duration_ms.into()),
                };

                Item {
                    track_id,
                    track: Track::Spotify { track },
                    user,
                    duration,
                }
            }))
        }
        TrackId::YouTube(ref id) => {
            let video_info = youtube.get_video_info(id);
            let by_id = youtube.videos_by_id(id, "contentDetails,snippet");

            Box::new(
                video_info
                    .join(by_id)
                    .and_then::<_, Result<Item, failure::Error>>({
                        let id = id.to_string();

                        move |(video_info, video)| {
                            log::trace!("info = {:?}", video_info);

                            let video = match video {
                                Some(video) => video,
                                None => failure::bail!("no video found for id `{}`", id),
                            };

                            let content_details =
                                video.content_details.as_ref().ok_or_else(|| {
                                    failure::format_err!("video does not have content details")
                                })?;

                            let duration = parse_youtube_duration(&content_details.duration)?;

                            Ok(Item {
                                track_id,
                                track: Track::YouTube { video },
                                user,
                                duration,
                            })
                        }
                    }),
            )
        }
    };

    return thread_pool.spawn_handle(future);

    fn parse_youtube_duration(duration: &str) -> Result<Duration, failure::Error> {
        let duration = duration.trim_start_matches("PT");

        let (duration, hours) = match duration.find('H') {
            Some(index) => {
                let hours = str::parse::<u64>(&duration[..index])?;
                (&duration[(index + 1)..], hours)
            }
            None => (duration, 0u64),
        };

        let (duration, minutes) = match duration.find('M') {
            Some(index) => {
                let minutes = str::parse::<u64>(&duration[..index])?;
                (&duration[(index + 1)..], minutes)
            }
            None => (duration, 0u64),
        };

        let (_, mut seconds) = match duration.find('S') {
            Some(index) => {
                let seconds = str::parse::<u64>(&duration[..index])?;
                (&duration[(index + 1)..], seconds)
            }
            None => (duration, 0u64),
        };

        seconds += minutes * 60;
        seconds += hours * 3600;

        Ok(Duration::from_secs(seconds))
    }
}
