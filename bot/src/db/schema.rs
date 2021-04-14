table! {
    balances (channel, user) {
        channel -> Text,
        user -> Text,
        amount -> BigInt,
        watch_time -> BigInt,
    }
}

table! {
    commands (channel, name) {
        channel -> Text,
        name -> Text,
        pattern -> Nullable<Text>,
        count -> Integer,
        text -> Text,
        group -> Nullable<Text>,
        disabled -> Bool,
    }
}

table! {
    after_streams (id) {
        id -> Integer,
        channel -> Nullable<Text>,
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
    songs (id) {
        id -> Integer,
        deleted -> Bool,
        played -> Bool,
        track_id -> Text,
        added_at -> Timestamp,
        promoted_at -> Nullable<Timestamp>,
        promoted_by -> Nullable<Text>,
        user -> Nullable<Text>,
    }
}

table! {
    settings (key) {
        key -> Text,
        value -> Text,
    }
}

table! {
    aliases (channel, name) {
        channel -> Text,
        name -> Text,
        pattern -> Nullable<Text>,
        text -> Text,
        group -> Nullable<Text>,
        disabled -> Bool,
    }
}

table! {
    promotions (channel, name) {
        channel -> Text,
        name -> Text,
        frequency -> Integer,
        promoted_at -> Nullable<Timestamp>,
        text -> Text,
        group -> Nullable<Text>,
        disabled -> Bool,
    }
}

table! {
    themes (channel, name) {
        channel -> Text,
        name -> Text,
        track_id -> Text,
        start -> Integer,
        end -> Nullable<Integer>,
        group -> Nullable<Text>,
        disabled -> Bool,
    }
}

// Grants that have been initialized from their default configuration.
table! {
    initialized_grants (scope) {
        scope -> Text,
        version -> Text,
    }
}

// Grants that are active.
table! {
    grants (scope, role) {
        scope -> Text,
        role -> Text,
    }
}

table! {
    script_keys (channel, key) {
        channel -> Text,
        key -> Binary,
        value -> Binary,
    }
}
