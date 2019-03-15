table! {
    balances (channel, user) {
        channel -> Text,
        user -> Text,
        amount -> Integer,
    }
}

table! {
    commands (channel, name) {
        channel -> Text,
        name -> Text,
        text -> Text,
    }
}

table! {
    after_streams (channel, added_at, user) {
        channel -> Text,
        added_at -> Timestamp,
        user -> Text,
        text -> Text,
    }
}

table! {
    bad_words (word) {
        word -> Text,
        why -> Nullable<Text>,
    }
}

table! {
    counters (channel, name) {
        channel -> Text,
        name -> Text,
        count -> Integer,
        text -> Text,
    }
}

table! {
    songs (id) {
        id -> Integer,
        deleted -> Bool,
        track_id -> Text,
        added_at -> Timestamp,
        user -> Nullable<Text>,
    }
}
