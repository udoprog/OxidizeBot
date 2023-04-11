/// Struct of tags.
#[derive(Debug, Clone, Default)]
pub struct Tags {
    /// Contents of the id tag if present.
    pub id: Option<String>,
    /// Contents of the msg-id tag if present.
    pub msg_id: Option<String>,
    /// The display name of the user.
    pub display_name: Option<String>,
    /// The ID of the user.
    pub user_id: Option<String>,
    /// Color of the user.
    pub color: Option<String>,
    /// Emotes part of the message.
    pub emotes: Option<String>,
}

impl Tags {
    /// Extract tags from message.
    pub fn from_tags<I, K, V>(tags: I) -> Tags
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut id = None;
        let mut msg_id = None;
        let mut display_name = None;
        let mut user_id = None;
        let mut color = None;
        let mut emotes = None;

        for (key, value) in tags {
            match key.as_ref() {
                "id" => id = Some(value.as_ref().to_owned()),
                "msg-id" => msg_id = Some(value.as_ref().to_owned()),
                "display-name" => display_name = Some(value.as_ref().to_owned()),
                "user-id" => user_id = Some(value.as_ref().to_owned()),
                "color" => color = Some(value.as_ref().to_owned()),
                "emotes" => emotes = Some(value.as_ref().to_owned()),
                key => {
                    tracing::trace!(key, value = value.as_ref(), "unsupported tag");
                }
            }
        }

        Tags {
            id,
            msg_id,
            display_name,
            user_id,
            color,
            emotes,
        }
    }
}
