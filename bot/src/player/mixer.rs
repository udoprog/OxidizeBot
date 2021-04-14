use crate::api;
use crate::db;
use crate::player::{convert_item, Item, Song};
use crate::track_id::TrackId;
use crate::utils;
use anyhow::Result;
use chrono::Utc;
use std::collections::VecDeque;
use std::sync::Arc;

/// Mixer decides what song to play next.
pub(super) struct Mixer {
    /// Database access.
    db: db::Database,
    /// In-memory queue.
    queue: VecDeque<Arc<Item>>,
    /// A song that has been sidelined by another song.
    sidelined: VecDeque<Song>,
    /// Currently loaded fallback items.
    fallback_items: Vec<Arc<Item>>,
    /// Items ordered in the reverse way they are meant to be played.
    fallback_queue: VecDeque<Arc<Item>>,
}

impl Mixer {
    /// The minimum size of the fallback queue.
    const FALLBACK_QUEUE_SIZE: usize = 10;

    /// Construct a new mixer around the given queue.
    pub(super) fn new(db: db::Database) -> Self {
        Self {
            db,
            queue: Default::default(),
            sidelined: Default::default(),
            fallback_items: Default::default(),
            fallback_queue: Default::default(),
        }
    }

    /// Initialize the queue from the database.
    pub(super) async fn initialize_queue(
        &mut self,
        spotify: &api::Spotify,
        youtube: &api::YouTube,
    ) -> Result<()> {
        // TODO: cache this value
        let streamer = spotify.me().await?;
        let market = streamer.country.as_deref();

        // Add tracks from database.
        for song in self.db.player_list().await? {
            let item = convert_item(
                spotify,
                youtube,
                song.user.as_deref(),
                &song.track_id,
                None,
                market,
            )
            .await;

            if let Ok(Some(item)) = item {
                self.queue.push_back(Arc::new(item));
            } else {
                log::warn!("failed to convert db item: {:?}", song);
            }
        }

        Ok(())
    }

    /// List items in the queue.
    pub(super) fn list(&self) -> impl Iterator<Item = &Arc<Item>> {
        self.queue.iter()
    }

    /// Get the length of the queue in the mixer.
    pub(super) fn len(&self) -> usize {
        self.queue.len()
    }

    /// Push item to back of queue.
    pub(super) async fn push_back(&mut self, item: Arc<Item>) -> Result<()> {
        self.db
            .player_push_back(&db::models::AddSong {
                track_id: item.track_id.clone(),
                added_at: Utc::now().naive_utc(),
                user: item.user.clone(),
            })
            .await?;

        self.queue.push_back(item);
        Ok(())
    }

    /// Purge the song queue.
    pub(super) async fn purge(&mut self) -> Result<Vec<Arc<Item>>> {
        if self.queue.is_empty() {
            return Ok(vec![]);
        }

        let purged = std::mem::replace(&mut self.queue, VecDeque::new())
            .into_iter()
            .collect();

        self.db.player_song_purge().await?;
        Ok(purged)
    }

    /// Remove the item at the given position.
    pub(super) async fn remove_at(&mut self, n: usize) -> Result<Option<Arc<Item>>> {
        if self.queue.is_empty() {
            return Ok(None);
        }

        if let Some(item) = self.queue.remove(n) {
            self.db.player_remove_song(&item.track_id, false).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element.
    pub(super) async fn remove_last(&mut self) -> Result<Option<Arc<Item>>> {
        if self.queue.is_empty() {
            return Ok(None);
        }

        if let Some(item) = self.queue.pop_back() {
            self.db.player_remove_song(&item.track_id, false).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last requested song matching the given user.
    pub(super) async fn remove_last_by_user(&mut self, user: &str) -> Result<Option<Arc<Item>>> {
        if self.queue.is_empty() {
            return Ok(None);
        }

        if let Some(position) = self
            .queue
            .iter()
            .rposition(|i| i.user.as_ref().map(|u| u == user).unwrap_or_default())
        {
            if let Some(item) = self.queue.remove(position) {
                self.db.player_remove_song(&item.track_id, false).await?;
                return Ok(Some(item));
            }
        }

        Ok(None)
    }

    /// Promote the given song.
    pub(super) async fn promote_song(
        &mut self,
        user: Option<&str>,
        n: usize,
    ) -> Result<Option<Arc<Item>>> {
        // OK, but song doesn't exist or index is out of bound.
        if self.queue.is_empty() || n >= self.queue.len() {
            return Ok(None);
        }

        if let Some(removed) = self.queue.remove(n) {
            self.queue.push_front(removed);
        }

        if let Some(item) = self.queue.get(0).cloned() {
            self.db.player_promote_song(user, &item.track_id).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Check if a song has been queued within the specified period of time.
    pub(super) async fn last_song_within(
        &self,
        track_id: &TrackId,
        duration: utils::Duration,
    ) -> Result<Option<db::models::Song>> {
        self.db.player_last_song_within(track_id, duration).await
    }

    /// Get next song to play.
    ///
    /// Will shuffle all fallback items and add them to a queue to avoid playing the same song twice.
    pub(super) fn next_fallback_item(&mut self) -> Option<Song> {
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
    pub(super) async fn next_song(&mut self) -> Result<Option<Song>> {
        if let Some(song) = self.sidelined.pop_front() {
            return Ok(Some(song));
        }

        // Take next from queue.
        if let Some(item) = self.pop_front().await? {
            return Ok(Some(Song::new(item.clone(), Default::default())));
        }

        if self.fallback_items.is_empty() {
            log::warn!("there are no fallback songs available");
            return Ok(None);
        }

        Ok(self.next_fallback_item())
    }

    /// Pop the front of the queue.
    async fn pop_front(&mut self) -> Result<Option<Arc<Item>>> {
        if let Some(item) = self.queue.pop_front() {
            self.db.player_remove_song(&item.track_id, true).await?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    /// Push a song to the sidelined queue.
    pub(super) fn push_sidelined(&mut self, song: Song) {
        self.sidelined.push_back(song);
    }

    /// Update available fallback items and clear the current fallback queue.
    pub(super) fn update_fallback_items(&mut self, items: Vec<Arc<Item>>) {
        self.fallback_items = items;
        self.fallback_queue.clear();
    }
}
