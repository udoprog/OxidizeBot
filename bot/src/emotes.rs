use crate::api::{self, bttv, ffz, BetterTTV, FrankerFaceZ, Tduva, Twitch};
use crate::irc;
use crate::storage::Cache;
use crate::template;
use anyhow::Error;
use futures_cache as cache;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Number of badges inlined for performance reasons.
/// Should be a value larger than the typical number of badges you'd see.
const INLINED_BADGES: usize = 8;
const DEFAULT_BADGE_SIZE: u32 = 18;
const BTTV_BOT_BADGE: &str = "https://cdn.betterttv.net/tags/bot.png";

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
        type LocalOption<'a> = [(u32, &'a mut Option<Url>, Option<String>); 3];

        let mut out = Urls::default();

        let options: SmallVec<LocalOption<'_>> = smallvec![
            (1u32, &mut out.small, urls.x1),
            (2u32, &mut out.medium, urls.x2),
            (4u32, &mut out.large, urls.x4),
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
#[serde(tag = "k", content = "c")]
enum Key<'a> {
    /// Twitch badges for the given room.
    TwitchSubscriberBadges { target: &'a str },
    /// GQL Twitch badges for the given chat (channel/name).
    GqlTwitchChatBadges { target: &'a str, name: &'a str },
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
                cache: cache.namespaced(&"emotes")?,
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
    async fn room_emotes_from_ffz(&self, user: &api::User) -> Result<EmoteByCode, Error> {
        let mut emotes = EmoteByCode::default();

        let (global, room) = tokio::try_join!(
            self.inner.ffz.set_global(),
            self.inner.ffz.room(&user.login),
        )?;

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

            let options: SmallVec<[(&mut Option<Url>, &str); 3]> = smallvec![
                (&mut urls.small, "1x"),
                (&mut urls.medium, "2x"),
                (&mut urls.large, "3x"),
            ];

            for (dest, size) in options {
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
    async fn bttv_bot_badge(&self, user: &api::User, name: &str) -> Result<Option<Badge>, Error> {
        let channel = self
            .inner
            .cache
            .wrap(
                Key::BttvChannel {
                    target: &user.login,
                },
                chrono::Duration::hours(72),
                self.inner.bttv.channels(&user.login),
            )
            .await?;

        let channel = match channel {
            Some(channel) => channel,
            None => return Ok(Default::default()),
        };

        if !channel.bots.contains(name) {
            return Ok(None);
        }

        let mut url = Url::from(String::from(BTTV_BOT_BADGE));

        url.size = Some(Size {
            width: DEFAULT_BADGE_SIZE,
            height: DEFAULT_BADGE_SIZE,
        });

        Ok(Some(Badge {
            title: String::from("Bot"),
            badge_url: None,
            urls: url.into(),
            bg_color: None,
        }))
    }

    /// Construct a set of room emotes from bttv.
    async fn room_emotes_from_bttv(&self, user: &api::User) -> Result<EmoteByCode, Error> {
        let channel = self
            .inner
            .cache
            .wrap(
                Key::BttvChannel {
                    target: &user.login,
                },
                chrono::Duration::hours(72),
                self.inner.bttv.channels(&user.login),
            )
            .await?;

        let channel = match channel {
            Some(channel) => channel,
            None => return Ok(Default::default()),
        };

        self.convert_emotes_from_bttv(channel.emotes, channel.url_template)
    }

    /// Construct a twitch emote.
    fn twitch_emote(id: &str) -> Arc<Emote> {
        let mut urls = Urls::default();

        let options: SmallVec<[(&mut Option<Url>, &str); 3]> = smallvec![
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
        let sets = self.inner.twitch.new_emote_sets(emote_sets).await?;

        let mut emotes = EmoteByCode::default();

        for e in sets {
            emotes.insert(e.name, Self::twitch_emote(&e.id));
        }

        Ok(emotes)
    }

    /// Construct a set of room emotes from ffz.
    async fn emote_sets_from_bttv(&self) -> Result<EmoteByCode, Error> {
        let emotes = self.inner.bttv.emotes().await?;
        self.convert_emotes_from_bttv(emotes.emotes, emotes.url_template)
    }

    /// Get all room emotes.
    async fn room_emotes(&self, user: &api::User) -> Result<Arc<EmoteByCode>, Error> {
        self.inner
            .cache
            .wrap(
                Key::RoomEmotes {
                    target: &user.login,
                },
                chrono::Duration::hours(6),
                async move {
                    let mut emotes = EmoteByCode::default();
                    let (a, b) = tokio::try_join!(
                        self.room_emotes_from_ffz(user),
                        self.room_emotes_from_bttv(user),
                    )?;
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

        // scratch buffer for all the characters in the message.
        // is only filled if needed lazily.
        let mut message_chars = None::<Vec<char>>;

        // 300354391:8-16/28087:0-6
        for emote in emotes.split('/') {
            let mut p = emote.split(':');

            let id = match p.next() {
                Some(id) => id,
                None => continue,
            };

            let span = match p.next() {
                Some(rest) => first_span(rest),
                None => continue,
            };

            let message = message_chars.get_or_insert_with(|| message.chars().collect());

            let word: String = match span {
                Some((s, e)) => message[s..=e].iter().collect(),
                None => continue,
            };

            out.insert(word, Self::twitch_emote(id));
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
            .wrap(Key::GlobalEmotes, chrono::Duration::hours(72), async move {
                let (twitch, bttv) = tokio::try_join!(
                    self.emote_sets_from_twitch("0"),
                    self.emote_sets_from_bttv(),
                )?;

                let mut emotes = EmoteByCode::default();
                emotes.extend(twitch);
                emotes.extend(bttv);
                Ok(Arc::new(emotes))
            })
            .await
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
                chrono::Duration::hours(24),
                self.inner.ffz.user(name),
            )
            .await?;

        let mut out = SmallVec::new();

        let user = match user {
            Some(user) => user,
            None => return Ok(out),
        };

        for (_, badge) in user.badges {
            let urls = (DEFAULT_BADGE_SIZE, DEFAULT_BADGE_SIZE, badge.urls).into();

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

        if let Some(d) = &*self.inner.tduva_data.read().await {
            let entry = self.inner.cache.test(Key::TduvaBadges)?;

            if let cache::State::Fresh(..) = entry {
                for b in &d.chatty {
                    if b.usernames.contains(name) {
                        out.push((DEFAULT_BADGE_SIZE, DEFAULT_BADGE_SIZE, b).into());
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
                chrono::Duration::hours(72),
                self.inner.tduva.res_badges(),
            )
            .await?;

        let mut d = TduvaData::default();

        for badge in badges {
            #[allow(clippy::single_match)]
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
                out.push((DEFAULT_BADGE_SIZE, DEFAULT_BADGE_SIZE, b).into());
            }
        }

        *self.inner.tduva_data.write().await = Some(d);
        Ok(out)
    }

    /// Get chat badges through GQL.
    async fn gql_twitch_chat_badges(
        &self,
        user: &api::User,
        name: &str,
    ) -> Result<SmallVec<[Badge; INLINED_BADGES]>, Error> {
        let badges = self
            .inner
            .cache
            .wrap(
                Key::GqlTwitchChatBadges {
                    target: &user.login,
                    name,
                },
                chrono::Duration::hours(72),
                self.inner.twitch.gql_display_badges(&user.login, name),
            )
            .await?;

        let mut out = SmallVec::new();

        let badges = match badges {
            Some(badges) => badges,
            None => return Ok(out),
        };

        let user = match badges.user {
            Some(user) => user,
            None => return Ok(out),
        };

        for badge in user.display_badges {
            let badge = match badge {
                Some(badge) => badge,
                None => continue,
            };

            let mut url = match url::Url::parse(&badge.image_url) {
                Ok(url) => url,
                Err(_) => continue,
            };

            let mut urls = Urls::default();

            let things: SmallVec<[(&mut Option<Url>, &str, u32); 3]> = smallvec![
                (&mut urls.small, "1", 1),
                (&mut urls.medium, "2", 2),
                (&mut urls.large, "3", 3),
            ];

            for (dest, segment, factor) in things {
                {
                    let mut path = match url.path_segments_mut() {
                        Ok(path) => path,
                        Err(()) => continue,
                    };

                    path.pop();
                    path.push(segment);
                }

                *dest = Some(Url {
                    url: url.to_string(),
                    size: Some(Size {
                        width: DEFAULT_BADGE_SIZE * factor,
                        height: DEFAULT_BADGE_SIZE * factor,
                    }),
                });
            }

            out.push(Badge {
                title: badge.description,
                badge_url: badge.click_url,
                urls,
                bg_color: None,
            });
        }

        Ok(out)
    }

    /// Render all room badges.
    async fn room_badges(
        &self,
        user: &api::User,
        name: &str,
    ) -> Result<SmallVec<[Badge; INLINED_BADGES]>, Error> {
        self.inner
            .cache
            .wrap(
                Key::RoomBadges {
                    target: &user.login,
                    name,
                },
                chrono::Duration::hours(1),
                async move {
                    let mut out = SmallVec::new();

                    let twitch = self.gql_twitch_chat_badges(user, name);
                    let ffz = self.ffz_chat_badges(name);
                    let tduva = self.tduva_chat_badges(name);
                    let bttv = self.bttv_bot_badge(user, name);

                    let (twitch, ffz, tduva, bttv) = tokio::join!(twitch, ffz, tduva, bttv);

                    match twitch {
                        Ok(badges) => out.extend(badges),
                        Err(e) => log::warn!(
                            "{}/{}: failed to get twitch chat badges: {}",
                            user.login,
                            name,
                            e
                        ),
                    }

                    match ffz {
                        Ok(badges) => out.extend(badges),
                        Err(e) => log::warn!(
                            "{}/{}: failed to get ffz chat badges: {}",
                            user.login,
                            name,
                            e
                        ),
                    }

                    match tduva {
                        Ok(badges) => out.extend(badges),
                        Err(e) => log::warn!(
                            "{}/{}: failed to get tduva chat badges: {}",
                            user.login,
                            name,
                            e
                        ),
                    }

                    match bttv {
                        Ok(badges) => out.extend(badges),
                        Err(e) => log::warn!(
                            "{}/{}: failed to get bttv chat badges: {}",
                            user.login,
                            name,
                            e
                        ),
                    }

                    Ok(out)
                },
            )
            .await
    }

    pub async fn render(
        &self,
        tags: &irc::Tags,
        user: &api::User,
        name: &str,
        message: &str,
    ) -> Result<Rendered, Error> {
        let (badges, room_emotes, global_emotes) = tokio::try_join!(
            self.room_badges(user, name),
            self.room_emotes(user),
            self.global_emotes(),
        )?;
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
                self.n += string.len();
                return None;
            }
        };

        let e = match self.string[s..].find(char::is_whitespace) {
            Some(n) => s + n,
            None => {
                let string = mem::replace(&mut self.string, "");
                let n = self.n + s;
                self.n += string.len();
                return Some((n, &string[s..]));
            }
        };

        let string = &self.string[s..e];
        self.string = &self.string[e..];
        let s = self.n + s;
        self.n += e;
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
