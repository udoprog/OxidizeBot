use anyhow::Result;
use diesel::prelude::*;

use common::{Channel, OwnedChannel};

#[derive(Clone)]
pub struct ScriptStorage {
    channel: OwnedChannel,
    db: crate::Database,
}

impl ScriptStorage {
    /// Open the script storage database.
    pub fn new(channel: &Channel, db: crate::Database) -> Self {
        Self {
            channel: channel.to_owned(),
            db,
        }
    }

    /// Set the given key.
    pub async fn set<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: 'static + Send + serde::Serialize,
        V: 'static + Send + serde::Serialize,
    {
        use crate::schema::script_keys::dsl;

        let channel = self.channel.clone();

        self.db
            .asyncify(move |c| {
                let key = serde_cbor::to_vec(&key)?;
                let value = serde_cbor::to_vec(&value)?;

                let filter =
                    dsl::script_keys.filter(dsl::channel.eq(&channel).and(dsl::key.eq(&key)));

                let first = filter.first::<crate::models::ScriptKey>(c).optional()?;

                match first {
                    None => {
                        let script_key = crate::models::ScriptKey {
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
                        let set = crate::models::SetScriptKeyValue { value: &value };
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
        use crate::schema::script_keys::dsl;

        let channel = self.channel.clone();

        self.db
            .asyncify(move |c| {
                let key = serde_cbor::to_vec(&key)?;

                let filter =
                    dsl::script_keys.filter(dsl::channel.eq(&channel).and(dsl::key.eq(&key)));

                let first = filter.first::<crate::models::ScriptKey>(c).optional()?;

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
