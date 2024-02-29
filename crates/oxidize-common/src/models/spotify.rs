//! All Spotify API endpoint response object
//!
//! Copied under the MIT license from: <https://github.com/ramsayleung/rspotify>.

use serde::de::{Deserialize, Deserializer, Error};

use serde_json::Number;

#[inline]
pub fn f64_to_u32(x: f64) -> Option<u32> {
    let y = x as u32;

    if y as f64 == x {
        Some(y)
    } else {
        None
    }
}

pub fn deserialize_option_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(number) = <Option<Number>>::deserialize(deserializer)? else {
        return Ok(None);
    };

    if let Some(n) = number.as_u64() {
        let Ok(n) = u32::try_from(n) else {
            return Err(D::Error::custom(format_args!(
                "Number {n} is out of numerical bounds 0-{}",
                u32::MAX
            )));
        };

        return Ok(Some(n));
    }

    if let Some(n) = number.as_i64() {
        let Ok(n) = u32::try_from(n) else {
            return Err(D::Error::custom(format_args!(
                "Number {n} is out of numerical bounds 0-{}",
                u32::MAX
            )));
        };

        return Ok(Some(n));
    }

    if let Some(n) = number.as_f64().and_then(f64_to_u32) {
        return Ok(Some(n));
    }

    Err(D::Error::custom(format_args!(
        "Number {number} is not a valid u32"
    )))
}

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
