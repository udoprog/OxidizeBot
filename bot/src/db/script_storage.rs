use crate::db;
use crate::db::models;
use anyhow::Result;
use diesel::prelude::*;

pub use self::models::AfterStream;

#[derive(Clone)]
pub struct ScriptStorage {
    channel: String,
    db: db::Database,
}

impl ScriptStorage {
    /// Open the script storage database.
    pub async fn load(channel: String, db: db::Database) -> Result<Self> {
        Ok(Self { channel, db })
    }

    /// Set the given key.
    pub async fn set<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: 'static + Send + serde::Serialize,
        V: 'static + Send + serde::Serialize,
    {
        use db::schema::script_keys::dsl;

        let channel = self.channel.clone();

        self.db
            .asyncify(move |c| {
                let key = serde_cbor::to_vec(&key)?;
                let value = serde_cbor::to_vec(&value)?;

                let filter =
                    dsl::script_keys.filter(dsl::channel.eq(&channel).and(dsl::key.eq(&key)));

                let first = filter
                    .clone()
                    .first::<db::models::ScriptKey>(c)
                    .optional()?;

                match first {
                    None => {
                        let script_key = db::models::ScriptKey {
                            channel,
                            key,
                            value,
                        };

                        diesel::insert_into(dsl::script_keys)
                            .values(&script_key)
                            .execute(c)?;

                        Ok(())
                    }
                    Some(..) => {
                        let set = db::models::SetScriptKeyValue { value: &value };
                        diesel::update(filter).set(&set).execute(c)?;
                        Ok(())
                    }
                }
            })
            .await
    }

    /// Get the given key.
    pub async fn get<K, V>(&self, key: K) -> Result<Option<V>>
    where
        K: 'static + Send + serde::Serialize,
        for<'de> V: 'static + Send + serde::Deserialize<'de>,
    {
        use db::schema::script_keys::dsl;

        let channel = self.channel.clone();

        self.db
            .asyncify(move |c| {
                let key = serde_cbor::to_vec(&key)?;

                let filter =
                    dsl::script_keys.filter(dsl::channel.eq(&channel).and(dsl::key.eq(&key)));

                let first = filter
                    .clone()
                    .first::<db::models::ScriptKey>(c)
                    .optional()?;

                match first {
                    None => Ok(None),
                    Some(key) => {
                        let value = serde_cbor::from_slice(&key.value)?;
                        Ok(Some(value))
                    }
                }
            })
            .await
    }
}
