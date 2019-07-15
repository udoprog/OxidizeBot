use crate::{
    api::{bttv, ffz, twitch::Channel, BetterTTV, FrankerFaceZ, Tduva, Twitch},
    irc,
    prelude::*,
    storage::{cache, Cache},
    template,
    utils::Duration,
};
use failure::Error;
use hashbrown::{HashMap, HashSet};
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::{mem, sync::Arc};

/// Number of badges inlined for performance reasons.
/// Should be a value larger than the typical number of badges you'd see.
const INLINED_BADGES: usize = 8;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Url {
    url: String,
    size: Option<Size>,
}

impl From<String> for Url {
    fn from(url: String) -> Self {
        Url { url, size: None }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Urls {
    small: Option<Url>,
    medium: Option<Url>,
    large: Option<Url>,
}

impl From<(u32, u32, ffz::Urls)> for Urls {
    fn from((width, height, urls): (u32, u32, ffz::Urls)) -> Self {
        let mut out = Urls::default();

        let options = vec![
            (1, &mut out.small, urls.x1),
            (2, &mut out.medium, urls.x2),
            (4, &mut out.large, urls.x4),
        ];

        for (factor, dest, url) in options {
            if let Some(url) = url {
                *dest = Some(Url {
                    url,
                    size: Some(Size {
                        width: width * factor,
                        height: height * factor,
                    }),
                });
            }
        }

        out
    }
}

impl From<Url> for Urls {
    fn from(value: Url) -> Self {
        Urls {
            small: Some(value),
            medium: None,
            large: None,
        }
    }
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "key")]
enum Key<'a> {
    /// Twitch badges for the given room.
    TwitchSubscriberBadges { target: &'a str },
    /// Twitch badges for the given chat (channel).
    TwitchChatBadges { target: &'a str },
    /// FFZ information for a given user.
    FfzUser { name: &'a str },
    /// All badges for the given room and name combo.
    RoomBadges { target: &'a str, name: &'a str },
    /// Channel information from BTTV.
    BttvChannel { target: &'a str },
    /// Emotes associated with a single room.
    RoomEmotes { target: &'a str },
    /// Global emotes.
    GlobalEmotes,
    /// Badges from tduva.
    TduvaBadges,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TduvaBadge {
    id: String,
    version: String,
    image_url: String,
    color: Option<String>,
    title: String,
    usernames: HashSet<String>,
}

#[derive(Default)]
struct TduvaData {
    chatty: Vec<TduvaBadge>,
}

struct Inner {
    cache: Cache,
    ffz: FrankerFaceZ,
    bttv: BetterTTV,
    tduva: Tduva,
    tduva_data: RwLock<Option<TduvaData>>,
    twitch: Twitch,
}

#[derive(Clone)]
pub struct Emotes {
    inner: Arc<Inner>,
}

impl Emotes {
    /// Construct a new emoticon handler.
    pub fn new(cache: Cache, twitch: Twitch) -> Result<Self, Error> {
        Ok(Self {
            inner: Arc::new(Inner {
                cache: cache.namespaced("emotes"),
                ffz: FrankerFaceZ::new()?,
                bttv: BetterTTV::new()?,
                tduva: Tduva::new()?,
                tduva_data: Default::default(),
                twitch,
            }),
        })
    }

    /// Extend the given emote set.
    fn extend_ffz_set(emotes: &mut EmoteByCode, s: ffz::Set) {
        for e in s.emoticons {
            let urls = (e.width, e.height, e.urls).into();
            emotes.insert(e.name, Arc::new(Emote { urls }));
        }
    }

    /// Construct a set of room emotes from ffz.
    async fn room_emotes_from_ffz(&self, channel: &Channel) -> Result<EmoteByCode, Error> {
        let mut emotes = EmoteByCode::default();

        let (global, room) = future::try_join(
            self.inner.ffz.set_global(),
            self.inner.ffz.room(&channel.name),
        )
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

    /// Convert emotes from BTTV.
    fn convert_emotes_from_bttv(
        &self,
        emotes: Vec<bttv::Emote>,
        url_template: String,
    ) -> Result<EmoteByCode, Error> {
        let mut out = EmoteByCode::default();

        let url_template = template::Template::compile(&url_template)?;

        for e in emotes {
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

            out.insert(e.code, Arc::new(Emote { urls }));
        }

        return Ok(out);

        #[derive(Debug, serde::Serialize)]
        struct Args<'a> {
            id: &'a str,
            image: &'a str,
        }
    }

    /// Construct a set of room emotes from bttv.
    async fn bttv_bot_badge(&self, channel: &Channel, name: &str) -> Result<Option<Badge>, Error> {
        let channel = self
            .inner
            .cache
            .wrap(
                Key::BttvChannel {
                    target: &channel.name,
                },
                Duration::hours(72),
                self.inner.bttv.channels(&channel.name),
            )
            .await?;

        let channel = match channel {
            Some(channel) => channel,
            None => return Ok(Default::default()),
        };

        if !channel.bots.contains(name) {
            return Ok(None);
        }

        let mut url = Url::from(String::from("https://cdn.betterttv.net/tags/bot.png"));

        url.size = Some(Size {
            width: 18,
            height: 18,
        });

        Ok(Some(Badge {
            title: String::from("BetterTTV Bot Badge"),
            badge_url: None,
            urls: url.into(),
            bg_color: None,
        }))
    }

    /// Construct a set of room emotes from bttv.
    async fn room_emotes_from_bttv(&self, channel: &Channel) -> Result<EmoteByCode, Error> {
        let channel = self
            .inner
            .cache
            .wrap(
                Key::BttvChannel {
                    target: &channel.name,
                },
                Duration::hours(72),
                self.inner.bttv.channels(&channel.name),
            )
            .await?;

        let channel = match channel {
            Some(channel) => channel,
            None => return Ok(Default::default()),
        };

        self.convert_emotes_from_bttv(channel.emotes, channel.url_template)
    }

    /// Construct a twitch emote.
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
                emotes.insert(e.code, Self::twitch_emote(e.id));
            }
        }

        Ok(emotes)
    }

    /// Construct a set of room emotes from ffz.
    async fn emote_sets_from_bttv(&self) -> Result<EmoteByCode, Error> {
        let emotes = self.inner.bttv.emotes().await?;
        self.convert_emotes_from_bttv(emotes.emotes, emotes.url_template)
    }

    /// Get all room emotes.
    async fn room_emotes(&self, channel: &Channel) -> Result<Arc<EmoteByCode>, Error> {
        self.inner
            .cache
            .wrap(
                Key::RoomEmotes {
                    target: &channel.name,
                },
                Duration::hours(6),
                async {
                    let mut emotes = EmoteByCode::default();
                    let (a, b) = future::try_join(
                        self.room_emotes_from_ffz(channel),
                        self.room_emotes_from_bttv(channel),
                    )
                    .await?;
                    emotes.extend(a);
                    emotes.extend(b);
                    Ok(Arc::new(emotes))
                },
            )
            .await
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

            out.insert(word.to_string(), Self::twitch_emote(id));
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
        self.inner
            .cache
            .wrap(Key::GlobalEmotes, Duration::hours(72), async {
                let (twitch, bttv) = future::try_join(
                    self.emote_sets_from_twitch("0"),
                    self.emote_sets_from_bttv(),
                )
                .await?;

                let mut emotes = EmoteByCode::default();
                emotes.extend(twitch);
                emotes.extend(bttv);
                Ok(Arc::new(emotes))
            })
            .await
    }

    /// Get twitch subscriber badges.
    async fn twitch_subscriber_badge(
        &self,
        channel: &Channel,
        needle: u32,
    ) -> Result<Option<Badge>, Error> {
        let badges = self
            .inner
            .cache
            .wrap(
                Key::TwitchSubscriberBadges {
                    target: &channel.name,
                },
                Duration::hours(24),
                self.inner.twitch.badges_display(&channel.id),
            )
            .await?;

        let mut badges = match badges {
            Some(badges) => badges,
            None => return Ok(None),
        };

        let subscriber = match badges.badge_sets.remove("subscriber") {
            Some(subscriber) => subscriber,
            None => return Ok(None),
        };

        let mut best = None;

        for (version, badge) in subscriber.versions {
            let version = match str::parse::<u32>(&version).ok() {
                Some(version) => version,
                None => continue,
            };

            best = match best {
                Some((v, _)) if version <= needle && version > v => Some((version, badge)),
                Some(best) => Some(best),
                None => Some((version, badge)),
            };
        }

        if let Some((_, badge)) = best {
            let mut urls = Urls::default();
            urls.small = Some(Url::from(badge.image_url_1x));
            urls.medium = Some(Url::from(badge.image_url_2x));
            urls.large = Some(Url::from(badge.image_url_4x));

            return Ok(Some(Badge {
                title: badge.title,
                badge_url: None,
                urls,
                bg_color: None,
            }));
        }

        Ok(None)
    }

    /// Get ffz chat badges.
    async fn ffz_chat_badges(
        &self,
        name: &str,
    ) -> Result<SmallVec<[Badge; INLINED_BADGES]>, Error> {
        let user = self
            .inner
            .cache
            .wrap(
                Key::FfzUser { name },
                Duration::hours(24),
                self.inner.ffz.user(name),
            )
            .await?;

        let mut out = SmallVec::new();

        let user = match user {
            Some(user) => user,
            None => return Ok(out),
        };

        for (_, badge) in user.badges {
            let urls = (18u32, 18u32, badge.urls).into();

            out.push(Badge {
                title: badge.title,
                badge_url: None,
                urls,
                bg_color: Some(badge.color),
            });
        }

        Ok(out)
    }

    /// Get tduva chat badges.
    async fn tduva_chat_badges(
        &self,
        name: &str,
    ) -> Result<SmallVec<[Badge; INLINED_BADGES]>, Error> {
        let mut out = SmallVec::new();

        if let Some(d) = self.inner.tduva_data.read().as_ref() {
            let entry = self.inner.cache.test(Key::TduvaBadges)?;

            if let cache::State::Fresh(..) = entry.state {
                for b in &d.chatty {
                    if b.usernames.contains(name) {
                        out.push((18, 18, b).into());
                    }
                }

                return Ok(out);
            }
        }

        let badges = self
            .inner
            .cache
            .wrap(
                Key::TduvaBadges,
                Duration::hours(72),
                self.inner.tduva.res_badges(),
            )
            .await?;

        let mut d = TduvaData::default();

        for badge in badges {
            match badge.id.as_str() {
                "chatty" => {
                    d.chatty.push(TduvaBadge {
                        id: badge.id,
                        version: badge.version,
                        image_url: badge.image_url,
                        color: badge.color,
                        title: badge.meta_title,
                        usernames: badge.usernames.into_iter().collect(),
                    });
                }
                _ => (),
            }
        }

        for b in &d.chatty {
            if b.usernames.contains(name) {
                out.push((18, 18, b).into());
            }
        }

        *self.inner.tduva_data.write() = Some(d);
        Ok(out)
    }

    /// Get twitch chat badges.
    async fn twitch_chat_badges(
        &self,
        channel: &Channel,
        chat_badges: impl Iterator<Item = (&str, u32)>,
    ) -> Result<SmallVec<[Badge; INLINED_BADGES]>, Error> {
        let badges = self
            .inner
            .cache
            .wrap(
                Key::TwitchChatBadges {
                    target: &channel.name,
                },
                Duration::hours(72),
                self.inner.twitch.chat_badges(&channel.id),
            )
            .await?;

        let mut out = SmallVec::new();

        let mut badges = match badges {
            Some(badges) => badges,
            None => return Ok(out),
        };

        for (name, version) in chat_badges {
            let name = match name {
                "admin" => "admin",
                "broadcaster" => "broadcaster",
                "global_mod" => "global_mod",
                "moderator" => "mod",
                "staff" => "staff",
                "turbo" => "turbo",
                "subscriber" => {
                    // NB: subscriber badges are handled separately.
                    out.extend(self.twitch_subscriber_badge(channel, version).await?);
                    continue;
                }
                "bits" => {
                    // NB: bits badges are not supported.
                    continue;
                }
                name => {
                    // NB: not supported.
                    log::trace!("Unsupported badge: {}", name);
                    continue;
                }
            };

            let badge = match badges.badges.remove(name) {
                Some(badge) => badge,
                None => continue,
            };

            let image = match badge.image {
                Some(image) => image,
                None => continue,
            };

            let mut urls = Urls::default();
            urls.small = Some(image.into());

            out.push(Badge {
                title: name.to_string(),
                badge_url: None,
                urls,
                bg_color: None,
            });
        }

        Ok(out)
    }

    /// Render all room badges.
    async fn room_badges(
        &self,
        tags: &irc::Tags,
        channel: &Channel,
        name: &str,
    ) -> Result<SmallVec<[Badge; INLINED_BADGES]>, Error> {
        let key = Key::RoomBadges {
            target: &channel.name,
            name,
        };

        return self
            .inner
            .cache
            .wrap(key, Duration::hours(1), async {
                let mut out = SmallVec::new();

                if let Some(badges) = tags.badges.as_ref() {
                    match self.twitch_chat_badges(channel, split_badges(badges)).await {
                        Ok(badges) => out.extend(badges),
                        Err(e) => log::warn!(
                            "{}/{}: failed to get twitch chat badges: {}",
                            channel.name,
                            name,
                            e
                        ),
                    }
                }

                let ffz = self.ffz_chat_badges(name);
                let tduva = self.tduva_chat_badges(name);
                let bttv = self.bttv_bot_badge(channel, name);

                let (ffz, tduva, bttv) = future::join3(ffz, tduva, bttv).await;

                match ffz {
                    Ok(badges) => out.extend(badges),
                    Err(e) => log::warn!(
                        "{}/{}: failed to get ffz chat badges: {}",
                        channel.name,
                        name,
                        e
                    ),
                }

                match tduva {
                    Ok(badges) => out.extend(badges),
                    Err(e) => log::warn!(
                        "{}/{}: failed to get tduva chat badges: {}",
                        channel.name,
                        name,
                        e
                    ),
                }

                match bttv {
                    Ok(badges) => out.extend(badges),
                    Err(e) => log::warn!(
                        "{}/{}: failed to get bttv chat badges: {}",
                        channel.name,
                        name,
                        e
                    ),
                }

                Ok(out)
            })
            .await;

        /// Split all the badges.
        fn split_badges<'a>(badges: &'a str) -> impl Iterator<Item = (&'a str, u32)> {
            badges.split(',').flat_map(|b| {
                let mut it = b.split('/');
                let badge = it.next()?;
                let version = str::parse::<u32>(it.next()?).ok()?;
                Some((badge, version))
            })
        }
    }

    pub async fn render(
        &self,
        tags: &irc::Tags,
        channel: &Channel,
        name: &str,
        message: &str,
    ) -> Result<Rendered, Error> {
        let (badges, room_emotes, global_emotes) = future::try_join3(
            self.room_badges(tags, channel, name),
            self.room_emotes(channel),
            self.global_emotes(),
        )
        .await?;
        let message_emotes = self.message_emotes_twitch(tags, message)?;

        Ok(Rendered::render(
            badges,
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
    #[serde(rename = "url")]
    Url { url: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Badge {
    /// Title for badge.
    title: String,
    /// Add a link to the badge.
    badge_url: Option<String>,
    /// Urls to pick for badge.
    urls: Urls,
    /// Optional background color.
    bg_color: Option<String>,
}

impl<'a> From<(u32, u32, &'a TduvaBadge)> for Badge {
    fn from((width, height, value): (u32, u32, &'a TduvaBadge)) -> Self {
        Badge {
            title: value.title.clone(),
            badge_url: Some(value.image_url.clone()),
            urls: Urls {
                small: Some(Url {
                    url: value.image_url.clone(),
                    size: Some(Size { width, height }),
                }),
                medium: None,
                large: None,
            },
            bg_color: value.color.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Rendered {
    badges: SmallVec<[Badge; INLINED_BADGES]>,
    items: Vec<Item>,
    emotes: HashMap<String, Arc<Emote>>,
}

impl Rendered {
    /// Convert a text into a rendered collection.
    fn render(
        badges: SmallVec<[Badge; INLINED_BADGES]>,
        text: &str,
        room_emotes: &EmoteByCode,
        message_emotes: &EmoteByCode,
        global_emotes: &EmoteByCode,
    ) -> Rendered {
        use url::Url;

        let mut buf = text;

        let mut emotes = HashMap::new();
        let mut items = Vec::new();

        let emote = |word| {
            room_emotes
                .get(word)
                .or_else(|| message_emotes.get(word))
                .or_else(|| global_emotes.get(word))
        };

        'outer: loop {
            let mut it = Words::new(buf);

            while let Some((idx, word)) = it.next() {
                if let Some(emote) = emote(word) {
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

                if let Ok(url) = Url::parse(word) {
                    let text = &buf[..idx];

                    if !text.is_empty() {
                        items.push(Item::Text {
                            text: text.to_string(),
                        });
                    }

                    items.push(Item::Url {
                        url: url.to_string(),
                    });

                    buf = &buf[(idx + word.len())..];
                    continue 'outer;
                }
            }

            break;
        }

        if !buf.is_empty() {
            items.push(Item::Text {
                text: buf.to_string(),
            });
        }

        Rendered {
            badges,
            items,
            emotes,
        }
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
