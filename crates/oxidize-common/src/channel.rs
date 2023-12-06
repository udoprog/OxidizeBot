use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::{AsExpression, FromSqlRow};

use std::borrow::{Borrow, Cow};
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// An owned variant of [`Channel`].
#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
#[serde(transparent)]
#[repr(transparent)]
pub struct OwnedChannel {
    data: String,
}

impl AsRef<Channel> for OwnedChannel {
    #[inline]
    fn as_ref(&self) -> &Channel {
        self
    }
}

impl fmt::Display for OwnedChannel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl fmt::Debug for OwnedChannel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl Borrow<Channel> for OwnedChannel {
    #[inline]
    fn borrow(&self) -> &Channel {
        self
    }
}

impl Deref for OwnedChannel {
    type Target = Channel;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Channel::new(self.data.as_str())
    }
}

/// A wrapper struct indicating a channel.
///
/// We maintain this wrapper, because channels are for historical reasons
/// prefixed with `#`, and using plain strings are to prone to bugs.
///
/// This way, we can ensure that the incoming value is correct.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Text)]
#[diesel(not_sized)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Channel {
    data: str,
}

impl Channel {
    pub fn new<S>(string: &S) -> &Self
    where
        S: ?Sized + AsRef<str>,
    {
        debug_assert!(
            string.as_ref().starts_with('#'),
            "Channel must start with '#'"
        );
        // SAFETY: this operation is safe, because Channel has the same
        // representation as a `str`.
        unsafe { &*(string.as_ref() as *const _ as *const Channel) }
    }

    /// Convert a string into a channel.
    pub fn from_string<S>(string: &S) -> Cow<'_, Channel>
    where
        S: ?Sized + AsRef<str>,
    {
        let string = string.as_ref();

        if string.starts_with('#') {
            Cow::Borrowed(Channel::new(string))
        } else {
            Cow::Owned(OwnedChannel {
                data: format!("#{string}"),
            })
        }
    }
}

impl AsRef<Channel> for Channel {
    #[inline]
    fn as_ref(&self) -> &Channel {
        self
    }
}

impl ToOwned for Channel {
    type Owned = OwnedChannel;

    fn to_owned(&self) -> Self::Owned {
        OwnedChannel {
            data: self.data.to_owned(),
        }
    }
}

impl fmt::Display for Channel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl fmt::Debug for Channel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl<DB> FromSql<Text, DB> for OwnedChannel
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> deserialize::Result<Self> {
        Ok(OwnedChannel {
            data: <_ as FromSql<Text, DB>>::from_sql(value)?,
        })
    }
}

impl<DB> ToSql<Text, DB> for Channel
where
    DB: Backend,
    str: ToSql<Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        <_ as ToSql<Text, DB>>::to_sql(&self.data, out)
    }
}

impl<D> ToSql<Text, D> for OwnedChannel
where
    D: Backend,
    str: ToSql<Text, D>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, D>) -> serialize::Result {
        <_ as ToSql<Text, D>>::to_sql(self.data.as_str(), out)
    }
}

impl PartialEq<Channel> for OwnedChannel {
    #[inline]
    fn eq(&self, other: &Channel) -> bool {
        self.as_ref() == other
    }
}

impl PartialEq<OwnedChannel> for Channel {
    #[inline]
    fn eq(&self, other: &OwnedChannel) -> bool {
        self == other.as_ref()
    }
}
