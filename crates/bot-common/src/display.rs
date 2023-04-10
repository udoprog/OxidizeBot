use std::borrow::Cow;

use crate::models::spotify::artist::SimplifiedArtist;

/// Format the given number of seconds as a compact human time.
pub fn compact_duration(duration: std::time::Duration) -> String {
    let mut parts = Vec::new();

    let p = crate::duration::partition(duration);

    parts.extend(match p.days {
        0 => None,
        n => Some(format!("{}d", n)),
    });

    parts.extend(match p.hours {
        0 => None,
        n => Some(format!("{}h", n)),
    });

    parts.extend(match p.minutes {
        0 => None,
        n => Some(format!("{}m", n)),
    });

    parts.extend(match p.seconds {
        0 => None,
        n => Some(format!("{}s", n)),
    });

    if parts.is_empty() {
        if p.milliseconds > 0 {
            return String::from("<1s");
        }

        return String::from("0s");
    }

    parts.join(" ")
}

/// Format the given number of seconds as a long human time.
pub fn long_duration(duration: std::time::Duration) -> String {
    let mut parts = Vec::new();

    let p = crate::duration::partition(duration);

    parts.extend(match p.hours {
        0 => None,
        1 => Some("one hour".to_string()),
        n => Some(format!("{} hours", english_num(n))),
    });

    parts.extend(match p.minutes {
        0 => None,
        1 => Some("one minute".to_string()),
        n => Some(format!("{} minutes", english_num(n))),
    });

    parts.extend(match p.seconds {
        0 => None,
        1 => Some("one second".to_string()),
        n => Some(format!("{} seconds", english_num(n))),
    });

    if parts.is_empty() {
        return String::from("0 seconds");
    }

    parts.join(", ")
}

/// Format the given number of seconds as a digital duration.
pub fn digital_duration(duration: std::time::Duration) -> String {
    let mut parts = Vec::new();

    let p = crate::duration::partition(duration);

    parts.extend(match p.hours {
        0 => None,
        n => Some(format!("{:02}", n)),
    });

    parts.push(format!("{:02}", p.minutes));
    parts.push(format!("{:02}", p.seconds));

    parts.join(":")
}

/// Format the given number as a string according to english conventions.
pub fn english_num(n: u64) -> Cow<'static, str> {
    let n = match n {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        n => return Cow::from(n.to_string()),
    };

    Cow::Borrowed(n)
}

/// Formats the given list of strings as a comma-separated set of values.
pub fn human_list(list: &[String]) -> Option<String> {
    if list.is_empty() {
        return None;
    }

    let mut it = list.iter();
    let mut list = String::new();

    if let Some(el) = it.next() {
        list.push_str(el);
    }

    let back = it.next_back();

    for el in it {
        list.push_str(", ");
        list.push_str(el);
    }

    if let Some(el) = back {
        list.push_str(", & ");
        list.push_str(el);
    }

    Some(list)
}

/// Render artists in a human readable form INCLUDING an oxford comma.
pub fn human_artists(artists: &[SimplifiedArtist]) -> Option<String> {
    if artists.is_empty() {
        return None;
    }

    let mut it = artists.iter();
    let mut artists = String::new();

    if let Some(artist) = it.next() {
        artists.push_str(artist.name.as_str());
    }

    let back = it.next_back();

    for artist in it {
        artists.push_str(", ");
        artists.push_str(artist.name.as_str());
    }

    if let Some(artist) = back {
        artists.push_str(", and ");
        artists.push_str(artist.name.as_str());
    }

    Some(artists)
}
