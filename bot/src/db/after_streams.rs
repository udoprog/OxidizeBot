use crate::db::{self, models, schema};
use diesel::prelude::*;

pub use self::models::AfterStream;

#[derive(Clone)]
pub struct AfterStreams {
    db: db::Database,
}

impl AfterStreams {
    /// Open the after streams database.
    pub fn load(db: db::Database) -> Result<Self, anyhow::Error> {
        Ok(AfterStreams { db })
    }

    /// Push the given afterstream message.
    pub fn push(&self, channel: &str, user: &str, text: &str) -> Result<(), anyhow::Error> {
        use self::schema::after_streams::dsl;
        let c = self.db.pool.lock();

        let after_stream = models::InsertAfterStream {
            channel: Some(String::from(channel)),
            user: String::from(user),
            text: String::from(text),
        };

        diesel::insert_into(dsl::after_streams)
            .values(&after_stream)
            .execute(&*c)?;

        Ok(())
    }

    /// Delete the after stream with the given id.
    pub fn delete(&self, id: i32) -> Result<bool, anyhow::Error> {
        use self::schema::after_streams::dsl;
        let c = self.db.pool.lock();
        let count = diesel::delete(dsl::after_streams.filter(dsl::id.eq(id))).execute(&*c)?;
        Ok(count == 1)
    }

    /// List all available after streams.
    pub fn list(&self) -> Result<Vec<AfterStream>, anyhow::Error> {
        use self::schema::after_streams::dsl;
        let c = self.db.pool.lock();
        Ok(dsl::after_streams
            .order(dsl::added_at.asc())
            .load::<models::AfterStream>(&*c)?)
    }
}
