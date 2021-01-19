use crate::auth::Scope;
use crate::currency::Currency;
use crate::irc::RealUser;
use crate::module::song::Constraint;
use crate::player::{AddTrackError, Player};
use crate::settings;
use crate::track_id::{self, TrackId};
use anyhow::Result;
use std::fmt;
use std::sync::Arc;

pub(crate) enum RequestCurrency<'a> {
    /// Use bot currency.
    BotCurrency(Option<&'a Currency>),
    /// Redemption doesn't use currency.
    Redemption,
}

#[derive(Clone)]
pub(crate) struct SongRequester {
    request_reward: settings::Var<u32>,
    spotify: Constraint,
    youtube: Constraint,
}

impl SongRequester {
    /// Construct a new requester module.
    pub(crate) fn new(
        request_reward: settings::Var<u32>,
        spotify: Constraint,
        youtube: Constraint,
    ) -> Self {
        Self {
            request_reward,
            spotify,
            youtube,
        }
    }

    /// Perform the given song request.
    pub(crate) async fn request(
        &self,
        q: &str,
        channel: &str,
        user: &str,
        real_user: Option<&RealUser<'_>>,
        currency: RequestCurrency<'_>,
        player: &Player,
    ) -> Result<RequestOutcome, RequestError> {
        if q.is_empty() {
            return Err(RequestError::BadRequest(None));
        }

        let request_reward = self.request_reward.load().await;
        let spotify = self.spotify.clone();
        let youtube = self.youtube.clone();

        let track_id = match TrackId::parse_with_urls(&q) {
            Ok(track_id) => Some(track_id),
            Err(e) => {
                match e {
                    // NB: fall back to searching.
                    track_id::ParseTrackIdError::MissingUriPrefix => (),
                    // show other errors.
                    e => {
                        log::warn!("bad song request: {}", e);
                        let e = format!("{} :(", e);
                        return Err(RequestError::BadRequest(Some(e)));
                    }
                }

                log::trace!("Failed to parse as URL/URI: {}: {}", q, e);
                None
            }
        };

        let track_id = match track_id {
            Some(track_id) => Some(track_id),
            None => player.search_track(q).await.map_err(RequestError::Error)?,
        };

        let track_id = match track_id {
            Some(track_id) => track_id,
            None => {
                return Err(RequestError::NoMatchingSong);
            }
        };

        let (what, scope, enabled) = match track_id {
            TrackId::Spotify(..) => {
                let enabled = spotify.enabled.load().await;
                ("Spotify", Scope::SongSpotify, enabled)
            }
            TrackId::YouTube(..) => {
                let enabled = youtube.enabled.load().await;
                ("YouTube", Scope::SongYouTube, enabled)
            }
        };

        if !enabled {
            return Err(RequestError::NotEnabled(what));
        }

        let has_bypass_constraints = if let Some(user) = real_user {
            if !user.has_scope(scope).await {
                return Err(RequestError::NotAllowed(what));
            }

            user.has_scope(Scope::SongBypassConstraints).await
        } else {
            false
        };

        let max_duration = match track_id {
            TrackId::Spotify(_) => spotify.max_duration.load().await,
            TrackId::YouTube(_) => youtube.max_duration.load().await,
        };

        let min_currency = match track_id {
            TrackId::Spotify(_) => spotify.min_currency.load().await,
            TrackId::YouTube(_) => youtube.min_currency.load().await,
        };

        if !has_bypass_constraints {
            match min_currency {
                // don't test if min_currency is not defined.
                0 => (),
                min_currency => {
                    match currency {
                        RequestCurrency::BotCurrency(currency) => {
                            let currency = match currency {
                                Some(currency) => currency,
                                None => {
                                    return Err(RequestError::NoCurrency);
                                }
                            };

                            let balance = currency
                                .balance_of(channel, user)
                                .await
                                .map_err(RequestError::Error)?
                                .unwrap_or_default();

                            if balance.balance < min_currency {
                                return Err(RequestError::NoBalance {
                                    currency: currency.name.clone(),
                                    required: min_currency,
                                    balance: balance.balance,
                                });
                            }
                        }
                        // Redemption uses own mechanism for paying.
                        RequestCurrency::Redemption => (),
                    }
                }
            }
        }

        let result = player
            .add_track(user, track_id, has_bypass_constraints, max_duration)
            .await;

        let (pos, item) = match result {
            Ok((pos, item)) => (pos, item),
            Err(e) => return Err(RequestError::AddTrackError(e)),
        };

        let currency = match currency {
            RequestCurrency::BotCurrency(Some(currency)) if request_reward > 0 => currency,
            _ => {
                return Ok(if let Some(pos) = pos {
                    RequestOutcome::AddedAt {
                        what: item.what(),
                        pos: pos + 1,
                    }
                } else {
                    RequestOutcome::Added { what: item.what() }
                });
            }
        };

        currency
            .balance_add(channel, user, request_reward as i64)
            .await
            .map_err(RequestError::Error)?;

        Ok(if let Some(pos) = pos {
            RequestOutcome::RewardedAt {
                currency: currency.name.clone(),
                amount: request_reward,
                what: item.what(),
                pos: pos + 1,
            }
        } else {
            RequestOutcome::Rewarded {
                currency: currency.name.clone(),
                amount: request_reward,
                what: item.what(),
            }
        })
    }
}

pub(crate) enum RequestOutcome {
    /// The given track was added at the given position.
    AddedAt { what: String, pos: usize },
    /// The given track was added.
    Added { what: String },
    /// Added the given track and gave the specified reward at the given position.
    RewardedAt {
        currency: Arc<String>,
        amount: u32,
        what: String,
        pos: usize,
    },
    /// Added the given track and gave the specified reward.
    Rewarded {
        currency: Arc<String>,
        amount: u32,
        what: String,
    },
}

impl fmt::Display for RequestOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestOutcome::AddedAt { what, pos } => {
                write!(
                    f,
                    "Added {what} at position #{pos}!",
                    what = what,
                    pos = pos,
                )
            }
            RequestOutcome::Added { what } => {
                write!(f, "Added {what}!", what = what)
            }
            RequestOutcome::RewardedAt {
                currency,
                amount,
                what,
                pos,
            } => {
                write!(
                    f,
                    "Added {what} at position #{pos}, here's your {amount} {currency}!",
                    what = what,
                    pos = pos,
                    amount = amount,
                    currency = currency,
                )
            }
            RequestOutcome::Rewarded {
                currency,
                amount,
                what,
            } => {
                write!(
                    f,
                    "Added {what}, here's your {amount} {currency}!",
                    what = what,
                    amount = amount,
                    currency = currency,
                )
            }
        }
    }
}

pub(crate) enum RequestError {
    /// Bad request with an optional textual reason.
    BadRequest(Option<String>),
    /// No song matches the request.
    NoMatchingSong,
    /// Song request of the specified kind are not enabled.
    NotEnabled(&'static str),
    /// User is not allowed to perform that kind of song request.
    NotAllowed(&'static str),
    /// No currency configured for stream.
    NoCurrency,
    /// Not enough stream currency balance.
    NoBalance {
        currency: Arc<String>,
        required: i64,
        balance: i64,
    },
    /// Error raised when adding track.
    AddTrackError(AddTrackError),
    /// A generic error.
    Error(anyhow::Error),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::BadRequest(reason) => {
                if let Some(reason) = reason {
                    write!(f, "bad request, {}", reason)
                } else {
                    write!(f, "bad request")
                }
            }
            RequestError::NoMatchingSong => {
                write!(f, "Could not find a track matching your request, sorry :(")
            }
            RequestError::NotEnabled(what) => {
                write!(
                    f,
                    "{} song requests are currently not enabled, sorry :(",
                    what
                )
            }
            RequestError::NotAllowed(what) => {
                write!(
                    f,
                    "You are not allowed to do {what} requests, sorry :(",
                    what = what
                )
            }
            RequestError::NoCurrency => {
                write!(f, "No currency configured for stream, but it is required.")
            }
            RequestError::NoBalance {
                currency,
                required,
                balance,
            } => {
                write! {
                    f,
                    "You don't have enough {currency} to request songs. Need {required}, but you have {balance}, sorry :(",
                    currency = currency,
                    required = required,
                    balance = balance,
                }
            }
            RequestError::AddTrackError(e) => {
                write!(f, "{}", e)
            }
            RequestError::Error(e) => {
                write!(f, "{}", e)
            }
        }
    }
}
