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

    /// Replace each key of this channel for which `rewrite` returns a new
    /// key. A row whose new key is already taken is left as is.
    pub async fn rewrite_keys<K, F>(&self, rewrite: F) -> Result<usize>
    where
        K: 'static + serde::Serialize + serde::de::DeserializeOwned,
        F: 'static + Send + Fn(K) -> Option<K>,
    {
        use crate::schema::script_keys::dsl;

        let channel = self.channel.clone();

        self.db
            .asyncify(move |c| {
                c.transaction(|c| {
                    let keys = dsl::script_keys
                        .filter(dsl::channel.eq(&channel))
                        .select(dsl::key)
                        .load::<Vec<u8>>(c)?;

                    let mut rewritten = 0;

                    for old in keys {
                        let Ok(key) = serde_cbor::from_slice::<K>(&old) else {
                            continue;
                        };

                        let Some(key) = rewrite(key) else {
                            continue;
                        };

                        let new = serde_cbor::to_vec(&key)?;

                        let taken = dsl::script_keys
                            .filter(dsl::channel.eq(&channel).and(dsl::key.eq(&new)))
                            .count()
                            .get_result::<i64>(c)?
                            > 0;

                        if taken {
                            continue;
                        }

                        diesel::update(
                            dsl::script_keys
                                .filter(dsl::channel.eq(&channel).and(dsl::key.eq(&old))),
                        )
                        .set(dsl::key.eq(&new))
                        .execute(c)?;

                        rewritten += 1;
                    }

                    Ok(rewritten)
                })
            })
            .await
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use common::Channel;

    use super::ScriptStorage;

    #[tokio::test]
    async fn rewrite_keys() {
        let db = crate::Database::open(Path::new(":memory:")).unwrap();
        let storage = ScriptStorage::new(Channel::new("#test"), db);

        storage.set("old", 1).await.unwrap();
        storage.set("taken", 2).await.unwrap();
        storage.set("new-taken", 3).await.unwrap();

        let rewritten = storage
            .rewrite_keys(|key: String| match key.as_str() {
                "old" => Some("new".to_owned()),
                "taken" => Some("new-taken".to_owned()),
                _ => None,
            })
            .await
            .unwrap();

        assert_eq!(rewritten, 1);
        assert_eq!(storage.get::<_, i64>("old").await.unwrap(), None);
        assert_eq!(storage.get::<_, i64>("new").await.unwrap(), Some(1));
        assert_eq!(storage.get::<_, i64>("taken").await.unwrap(), Some(2));
        assert_eq!(storage.get::<_, i64>("new-taken").await.unwrap(), Some(3));
    }
}
