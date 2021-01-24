use crate::player;
use crate::prelude::*;
use crate::template::Template;
use crate::utils;
use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;

static DEFAULT_CURRENT_SONG_TEMPLATE: &str = "Song: {{name}}{{#if artists}} by {{artists}}{{/if}}{{#if paused}} (Paused){{/if}} ({{duration}})\n{{#if user~}}Request by: @{{user~}}{{/if}}";
static DEFAULT_CURRENT_SONG_STOPPED_TEMPLATE: &str = "Not Playing";

#[derive(Debug, Clone, Default)]
pub struct SongFileBuilder {
    pub enabled: bool,
    pub path: Option<PathBuf>,
    pub template: Option<Template>,
    pub stopped_template: Option<Template>,
    pub update_interval: utils::Duration,
}

impl SongFileBuilder {
    /// Construct a new SongFile handler if all the necessary options are available.
    pub fn build(&self) -> Option<SongFile> {
        if !self.enabled {
            log::trace!("not enabled");
            return None;
        }

        let path = self.path.as_ref()?;
        let template = self.template.as_ref()?;
        let update_interval = if !self.update_interval.is_empty() {
            self.update_interval
        } else {
            log::trace!("no update interval configured");
            return None;
        };

        let update_interval = tokio::time::interval(update_interval.as_std());

        Some(SongFile {
            path: path.clone(),
            template: template.clone(),
            stopped_template: self.stopped_template.clone(),
            update_interval,
        })
    }

    /// Initialize the given current song.
    pub fn init(&self, value: &mut Fuse<SongFile>) {
        let update = match self.build() {
            Some(update) => Fuse::new(update),
            None => Fuse::empty(),
        };

        if let Some(old) = std::mem::replace(value, update).as_inner_ref() {
            old.blank_log();
        }
    }
}

pub struct SongFile {
    /// Path to render current song at.
    path: PathBuf,
    /// Message to render when a song is playing.
    template: Template,
    /// Message to show when no song is playing.
    stopped_template: Option<Template>,
    /// Update frequency.
    update_interval: tokio::time::Interval,
}

impl SongFile {
    pub(crate) async fn run(injector: Injector, settings: crate::Settings) -> Result<()> {
        let (mut song_stream, mut song) = injector.stream::<player::Song>().await;
        let (mut state_stream, mut state) = injector.stream::<player::State>().await;
        let (mut path_stream, path) = settings.stream("path").optional().await?;

        let (mut template_stream, template) = settings
            .stream("template")
            .or(Some(Template::compile(DEFAULT_CURRENT_SONG_TEMPLATE)?))
            .optional()
            .await?;

        let (mut stopped_template_stream, stopped_template) = settings
            .stream("stopped-template")
            .or(Some(Template::compile(
                DEFAULT_CURRENT_SONG_STOPPED_TEMPLATE,
            )?))
            .optional()
            .await?;

        let (mut update_interval_stream, update_interval) = settings
            .stream("update-interval")
            .or_with(utils::Duration::seconds(1))
            .await?;

        let (mut enabled_stream, enabled) = settings.stream("enabled").or_default().await?;

        let mut song_file = Fuse::empty();

        let mut builder = SongFileBuilder::default();
        builder.enabled = enabled;
        builder.path = path;
        builder.template = template;
        builder.stopped_template = stopped_template;
        builder.update_interval = update_interval;
        builder.init(&mut song_file);

        loop {
            tokio::select! {
                /* current song */
                update = enabled_stream.recv() => {
                    builder.enabled = update;
                    builder.init(&mut song_file);
                }
                update = path_stream.recv() => {
                    builder.path = update;
                    builder.init(&mut song_file);
                }
                update = template_stream.recv() => {
                    builder.template = update;
                    builder.init(&mut song_file);
                }
                update = stopped_template_stream.recv() => {
                    builder.stopped_template = update;
                    builder.init(&mut song_file);
                }
                update = update_interval_stream.recv() => {
                    builder.update_interval = update;
                    builder.init(&mut song_file);
                }
                _ = song_file.as_pin_mut().poll_inner(|mut f, cx| f.update_interval.poll_tick(cx)) => {
                }
                update = song_stream.recv() => {
                    song = update;
                }
                update = state_stream.recv() => {
                    state = update;
                }
            }

            if let Some(song_file) = song_file.as_inner_mut() {
                song_file.update_song(song.as_ref(), state).await;
            }
        }
    }

    /// Write current song. Log any errors.
    async fn update_song(&self, song: Option<&player::Song>, state: Option<player::State>) {
        log::trace!("updating song: {:?} {:?}", song, state);

        let state = state.unwrap_or_default();

        let result = match song {
            Some(song) => self.write(song, state),
            None => self.blank(),
        };

        if let Err(e) = result {
            log::warn!(
                "failed to write current song: {}: {}",
                self.path.display(),
                e
            );
        }
    }

    /// Either creates or truncates the current song file.
    fn create_or_truncate(&self) -> Result<File, anyhow::Error> {
        File::create(&self.path).map_err(Into::into)
    }

    /// Blank the current file.
    pub fn blank(&self) -> Result<(), anyhow::Error> {
        use std::io::Write as _;
        let mut f = self.create_or_truncate()?;

        if let Some(stopped_template) = self.stopped_template.as_ref() {
            write!(f, "{}", stopped_template)?;
        } else {
            write!(f, "Not Playing")?;
        }

        Ok(())
    }

    /// Write the current song to a path.
    pub fn write(&self, song: &player::Song, state: player::State) -> Result<(), anyhow::Error> {
        let mut f = self.create_or_truncate()?;
        let data = song.data(state)?;
        self.template.render(&mut f, &data)?;
        Ok(())
    }

    /// Clear the old log.
    pub fn blank_log(&self) {
        if let Err(e) = self.blank() {
            log::error!("Failed to blank file: {}: {}", self.path.display(), e);
        }
    }
}
