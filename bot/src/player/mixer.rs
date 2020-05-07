use super::{Item, Queue, Song};
use anyhow::Result;
use std::{collections::VecDeque, sync::Arc};

/// Mixer decides what song to play next.
pub(super) struct Mixer {
    /// Persistent queue to take songs from.
    queue: Queue,
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
    pub(super) fn new(queue: Queue) -> Self {
        Self {
            queue,
            sidelined: Default::default(),
            fallback_items: Default::default(),
            fallback_queue: Default::default(),
        }
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
        if let Some(item) = self.queue.front().await {
            let _ = self.queue.pop_front().await?;
            return Ok(Some(Song::new(item, Default::default())));
        }

        if self.fallback_items.is_empty() {
            log::warn!("there are no fallback songs available");
            return Ok(None);
        }

        Ok(self.next_fallback_item())
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
