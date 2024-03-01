//! All Spotify API endpoint response object

// Copied under the MIT license from: <https://github.com/ramsayleung/rspotify>.
//
// Copyright (c) 2015 Vincent Prouillet

pub mod album;
pub mod artist;
pub mod audio;
pub mod category;
pub mod context;
pub mod cud_result;
pub mod device;
pub mod image;
pub mod offset;
pub mod page;
pub mod playing;
pub mod playlist;
pub mod recommend;
pub mod search;
pub mod senum;
pub mod track;
pub mod user;

use std::fmt;

use serde::de::{Deserialize, Deserializer, Error};

use serde_json::Number;

trait NumberTrait: Copy + fmt::Display + TryFrom<u64> + TryFrom<i64> {
    const MAX: Self;

    fn from_f64(x: f64) -> Self;

    fn as_f64(self) -> f64;
}

macro_rules! number {
    ($($t:ty),*) => {
        $(
            impl NumberTrait for $t {
                const MAX: Self = <$t>::MAX;

                fn from_f64(x: f64) -> Self {
                    x as Self
                }

                fn as_f64(self) -> f64 {
                    self as f64
                }
            }
        )*
    };
}

number!(u8, u16, u32, u64, i8, i16, i32, i64);

#[inline]
fn f64_to_number<T>(x: f64) -> Option<T>
where
    T: NumberTrait,
{
    let y = T::from_f64(x);

    if T::as_f64(y) == x {
        Some(y)
    } else {
        None
    }
}

fn deserialize_option_number<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: NumberTrait,
{
    let Some(number) = <Option<Number>>::deserialize(deserializer)? else {
        return Ok(None);
    };

    Ok(Some(convert_number(number)?))
}

fn deserialize_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: NumberTrait,
{
    let number = Number::deserialize(deserializer)?;
    convert_number(number)
}

fn convert_number<T, E>(number: Number) -> Result<T, E>
where
    T: NumberTrait,
    E: Error,
{
    if let Some(n) = number.as_u64() {
        let Ok(n) = T::try_from(n) else {
            return Err(E::custom(format_args!(
                "Number {n} is out of numerical bounds 0-{}",
                T::MAX
            )));
        };

        return Ok(n);
    }

    if let Some(n) = number.as_i64() {
        let Ok(n) = T::try_from(n) else {
            return Err(E::custom(format_args!(
                "Number {n} is out of numerical bounds 0-{}",
                T::MAX
            )));
        };

        return Ok(n);
    }

    if let Some(n) = number.as_f64().and_then(f64_to_number) {
        return Ok(n);
    }

    Err(E::custom(format_args!(
        "Number {number} is not a valid u32"
    )))
}
