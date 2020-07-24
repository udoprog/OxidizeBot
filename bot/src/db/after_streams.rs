use crate::db;
use crate::db::models;
use crate::db::schema;
use anyhow::Result;
use diesel::prelude::*;

pub use self::models::AfterStream;

#[derive(Clone)]
pub struct AfterStreams {
    db: db::Database,
}

impl AfterStreams {
    /// Open the after streams database.
    pub async fn load(db: db::Database) -> Result<Self> {
        Ok(Self { db })
    }

    /// Push the given afterstream message.
    pub async fn push(&self, channel: &str, user: &str, text: &str) -> Result<()> {
        use self::schema::after_streams::dsl;

        let channel = channel.to_string();
        let user = user.to_string();
        let text = text.to_string();

        self.db
            .asyncify(move |c| {
                let after_stream = models::InsertAfterStream {
                    channel: Some(String::from(channel)),
                    user: String::from(user),
                    text: String::from(text),
                };

                diesel::insert_into(dsl::after_streams)
                    .values(&after_stream)
                    .execute(c)?;

                Ok(())
            })
            .await
    }

    /// Delete the after stream with the given id.
    pub async fn delete(&self, id: i32) -> Result<bool> {
        use self::schema::after_streams::dsl;

        self.db
            .asyncify(move |c| {
                let count = diesel::delete(dsl::after_streams.filter(dsl::id.eq(id))).execute(c)?;
                Ok(count == 1)
            })
            .await
    }

    /// List all available after streams.
    pub async fn list(&self) -> Result<Vec<AfterStream>> {
        use self::schema::after_streams::dsl;

        self.db
            .asyncify(move |c| {
                Ok(dsl::after_streams
                    .order(dsl::added_at.asc())
                    .load::<models::AfterStream>(c)?)
            })
            .await
    }
}
