use crate::{
    api::{ffz, BetterTTV, FrankerFaceZ, Twitch},
    irc, template,
};
use failure::Error;
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::{mem, sync::Arc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Url {
    url: String,
    size: Option<Size>,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Urls {
    small: Option<Url>,
    medium: Option<Url>,
    large: Option<Url>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Size {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Emote {
    urls: Urls,
}

type EmoteByCode = HashMap<String, Arc<Emote>>;

struct Inner {
    ffz: FrankerFaceZ,
    bttv: BetterTTV,
    twitch: Twitch,
    /// Globally available emotes.
    globals: RwLock<Option<Arc<EmoteByCode>>>,
    /// Room-specific emotes.
    rooms: RwLock<HashMap<String, Arc<EmoteByCode>>>,
    /// Twitch emotes.
    twitch_emotes: RwLock<HashMap<String, Arc<Emote>>>,
}

#[derive(Clone)]
pub struct Emotes {
    inner: Arc<Inner>,
}

impl Emotes {
    /// Construct a new emoticon handler.
    pub fn new(twitch: Twitch) -> Result<Self, Error> {
        Ok(Self {
            inner: Arc::new(Inner {
                ffz: FrankerFaceZ::new()?,
                bttv: BetterTTV::new()?,
                twitch,
                globals: Default::default(),
                rooms: Default::default(),
                twitch_emotes: Default::default(),
            }),
        })
    }

    /// Extend the given emote set.
    fn extend_ffz_set(emotes: &mut EmoteByCode, s: ffz::Set) {
        for e in s.emoticons {
            let mut urls = Urls::default();

            let options = vec![
                (1, &mut urls.small, e.urls.x1),
                (2, &mut urls.medium, e.urls.x2),
                (4, &mut urls.large, e.urls.x4),
            ];

            for (factor, dest, url) in options {
                if let Some(url) = url {
                    *dest = Some(Url {
                        url,
                        size: Some(Size {
                            width: e.width * factor,
                            height: e.height * factor,
                        }),
                    });
                }
            }

            emotes.insert(e.name, Arc::new(Emote { urls }));
        }
    }

    /// Construct a set of room emotes from ffz.
    async fn room_emotes_from_ffz(&self, target: &str) -> Result<EmoteByCode, Error> {
        let mut emotes = EmoteByCode::default();

        let target = target.trim_start_matches('#');

        let (global, room) =
            futures::future::try_join(self.inner.ffz.set_global(), self.inner.ffz.room(target))
                .await?;

        for (_, s) in global.sets {
            Self::extend_ffz_set(&mut emotes, s);
        }

        if let Some(room) = room {
            for (_, s) in room.sets {
                Self::extend_ffz_set(&mut emotes, s);
            }
        }

        Ok(emotes)
    }

    /// Construct a set of room emotes from bttv.
    async fn room_emotes_from_bttv(&self, target: &str) -> Result<EmoteByCode, Error> {
        let mut emotes = EmoteByCode::default();

        let target = target.trim_start_matches('#');

        let channel = match self.inner.bttv.channels(target).await? {
            Some(channel) => channel,
            None => return Ok(emotes),
        };

        let url_template = template::Template::compile(&channel.url_template)?;

        for e in channel.emotes {
            let mut urls = Urls::default();

            let options = vec![
                (&mut urls.small, "1x"),
                (&mut urls.medium, "2x"),
                (&mut urls.large, "3x"),
            ];

            for (dest, size) in options.into_iter() {
                let url = url_template.render_to_string(Args {
                    id: e.id.as_str(),
                    image: size,
                })?;

                *dest = Some(Url { url, size: None });
            }

            emotes.insert(e.code, Arc::new(Emote { urls }));
        }

        return Ok(emotes);

        #[derive(Debug, serde::Serialize)]
        struct Args<'a> {
            id: &'a str,
            image: &'a str,
        }
    }

    /// Build a twitch emote.
    fn twitch_emote(id: u64) -> Arc<Emote> {
        let mut urls = Urls::default();

        let options = vec![
            (&mut urls.small, "1.0"),
            (&mut urls.medium, "2.0"),
            (&mut urls.large, "3.0"),
        ];

        for (dest, size) in options.into_iter() {
            let url = format!("//static-cdn.jtvnw.net/emoticons/v1/{}/{}", id, size);
            *dest = Some(Url { url, size: None });
        }

        Arc::new(Emote { urls })
    }

    /// Construct a set of room emotes from twitch.
    async fn emote_sets_from_twitch(&self, emote_sets: &str) -> Result<EmoteByCode, Error> {
        let result = self.inner.twitch.chat_emoticon_images(emote_sets).await?;

        let mut emotes = EmoteByCode::default();

        for (_, set) in result.emoticon_sets {
            for e in set {
                let emote = Self::twitch_emote(e.id);
                emotes.insert(e.code, emote);
            }
        }

        Ok(emotes)
    }

    /// Get all room emotes.
    async fn room_emotes(&self, target: &str) -> Result<Arc<EmoteByCode>, Error> {
        if let Some(emotes) = self.inner.rooms.read().get(target) {
            return Ok(emotes.clone());
        }

        let mut emotes = EmoteByCode::default();
        emotes.extend(self.room_emotes_from_ffz(target).await?);
        emotes.extend(self.room_emotes_from_bttv(target).await?);
        let emotes = Arc::new(emotes);
        self.inner
            .rooms
            .write()
            .insert(target.to_string(), emotes.clone());
        Ok(emotes)
    }

    /// Get all user emotes.
    fn message_emotes_twitch(&self, tags: &irc::Tags, message: &str) -> Result<EmoteByCode, Error> {
        let emotes = match tags.emotes.as_ref() {
            Some(emotes) => match emotes.as_str() {
                "" => return Ok(Default::default()),
                emotes => emotes,
            },
            None => return Ok(Default::default()),
        };

        let mut out = EmoteByCode::default();

        // 300354391:8-16/28087:0-6
        for emote in emotes.split('/') {
            let mut p = emote.split(':');

            let id = match p.next() {
                Some(id) => str::parse::<u64>(id)?,
                None => continue,
            };

            let span = match p.next() {
                Some(rest) => first_span(rest),
                None => continue,
            };

            let word = match span {
                Some((s, e)) => &message[s..=e],
                None => continue,
            };

            if let Some(emote) = self.inner.twitch_emotes.read().get(word) {
                out.insert(word.to_string(), emote.clone());
                continue;
            }

            let emote = Self::twitch_emote(id);
            self.inner
                .twitch_emotes
                .write()
                .insert(word.to_string(), emote.clone());
            out.insert(word.to_string(), emote.clone());
        }

        return Ok(out);

        fn first_span(rest: &str) -> Option<(usize, usize)> {
            let mut it = rest.split(',').next()?.split('-');

            let s = it.next()?;
            let s = str::parse::<usize>(&s).ok()?;

            let e = it.next()?;
            let e = str::parse::<usize>(&e).ok()?;

            Some((s, e))
        }
    }

    /// Get all user emotes.
    async fn global_emotes(&self) -> Result<Arc<EmoteByCode>, Error> {
        if let Some(emotes) = self.inner.globals.read().as_ref() {
            return Ok(emotes.clone());
        }

        let mut emotes = EmoteByCode::default();
        emotes.extend(self.emote_sets_from_twitch("0").await?);
        let emotes = Arc::new(emotes);
        *self.inner.globals.write() = Some(emotes.clone());
        Ok(emotes)
    }

    pub async fn render(
        &self,
        tags: &irc::Tags,
        target: &str,
        message: &str,
    ) -> Result<Rendered, Error> {
        use futures::future;

        let (room_emotes, global_emotes) =
            future::try_join(self.room_emotes(target), self.global_emotes()).await?;
        let message_emotes = self.message_emotes_twitch(tags, message)?;

        Ok(Rendered::render(
            message,
            &*room_emotes,
            &message_emotes,
            &*global_emotes,
        ))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
enum Item {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "emote")]
    Emote { emote: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Rendered {
    items: Vec<Item>,
    emotes: HashMap<String, Arc<Emote>>,
}

impl Rendered {
    /// Convert a text into a rendered collection.
    fn render(
        text: &str,
        room_emotes: &EmoteByCode,
        message_emotes: &EmoteByCode,
        global_emotes: &EmoteByCode,
    ) -> Rendered {
        let mut buf = text;

        let mut emotes = HashMap::new();
        let mut items = Vec::new();

        'outer: loop {
            let mut it = Words::new(buf);

            while let Some((idx, word)) = it.next() {
                let emote = match room_emotes
                    .get(word)
                    .or_else(|| message_emotes.get(word))
                    .or_else(|| global_emotes.get(word))
                {
                    Some(emote) => emote,
                    None => continue,
                };

                if !emotes.contains_key(word) {
                    emotes.insert(word.to_string(), emote.clone());
                }

                let text = &buf[..idx];

                if !text.is_empty() {
                    items.push(Item::Text {
                        text: text.to_string(),
                    });
                }

                items.push(Item::Emote {
                    emote: word.to_string(),
                });

                buf = &buf[(idx + word.len())..];
                continue 'outer;
            }

            break;
        }

        if !buf.is_empty() {
            items.push(Item::Text {
                text: buf.to_string(),
            });
        }

        Rendered { items, emotes }
    }
}

#[derive(Debug)]
pub struct Words<'a> {
    string: &'a str,
    n: usize,
}

impl<'a> Words<'a> {
    /// Split a string into words.
    pub fn new(string: &str) -> Words<'_> {
        Words { string, n: 0 }
    }
}

impl<'a> Iterator for Words<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }

        let s = match self.string.find(|c: char| !c.is_whitespace()) {
            Some(n) => n,
            None => {
                let string = mem::replace(&mut self.string, "");
                self.n = self.n + string.len();
                return None;
            }
        };

        let e = match self.string[s..].find(char::is_whitespace) {
            Some(n) => s + n,
            None => {
                let string = mem::replace(&mut self.string, "");
                let n = self.n + s;
                self.n = self.n + string.len();
                return Some((n, &string[s..]));
            }
        };

        let string = &self.string[s..e];
        self.string = &self.string[e..];
        let s = self.n + s;
        self.n = self.n + e;
        Some((s, string))
    }
}

#[cfg(test)]
mod tests {
    use super::Words;

    #[test]
    pub fn test_words() {
        let w = Words::new("");
        assert_eq!(Vec::<(usize, &str)>::new(), w.collect::<Vec<_>>());

        let w = Words::new("Foo Bar");
        assert_eq!(vec![(0, "Foo"), (4, "Bar")], w.collect::<Vec<_>>());

        let w = Words::new(" Foo   ");
        assert_eq!(vec![(1, "Foo")], w.collect::<Vec<_>>());

        let w = Words::new("test test PrideGive");
        assert_eq!(
            vec![(0, "test"), (5, "test"), (10, "PrideGive")],
            w.collect::<Vec<_>>()
        );
    }
}
