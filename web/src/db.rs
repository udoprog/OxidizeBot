use crate::oauth2;
use failure::Error;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt, sync::Arc};

#[derive(Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub token: oauth2::SavedToken,
}

/// Internal key serialization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Connection { user: String, id: String },
    ConnectionsByUser { user: String },
    UserToKey { user: String },
    KeyToUser { key: String },
}

impl Key {
    /// Serialize the current key.
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        Ok(serde_cbor::to_vec(self)?)
    }

    /// Deserialize a key.
    pub fn deserialize(bytes: &[u8]) -> Result<Key, Error> {
        Ok(serde_cbor::from_slice(bytes)?)
    }
}

impl<'de> serde::Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::Error;

        return deserializer.deserialize_seq(KeyVisitor);

        struct KeyVisitor;

        impl<'de> serde::de::Visitor<'de> for KeyVisitor {
            type Value = Key;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid key")
            }

            #[inline]
            fn visit_seq<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let ns = visitor.next_element::<String>()?;

                let ns = match ns {
                    Some(ns) => ns,
                    None => return Err(Error::custom("expected namespace element")),
                };

                let key = match ns.as_str() {
                    "connections" => {
                        let user = visitor
                            .next_element::<String>()?
                            .ok_or_else(|| Error::custom("expected: name"))?;

                        match visitor.next_element::<String>()? {
                            Some(id) => Key::Connection { user, id },
                            None => Key::ConnectionsByUser { user },
                        }
                    }
                    "key-to-user" => {
                        let key = visitor
                            .next_element::<String>()?
                            .ok_or_else(|| Error::custom("expected: key"))?;

                        Key::KeyToUser { key }
                    }
                    "user-to-key" => {
                        let user = visitor
                            .next_element::<String>()?
                            .ok_or_else(|| Error::custom("expected: user"))?;

                        Key::UserToKey { user }
                    }
                    other => return Err(Error::custom(format!("unexpected namespace: {}", other))),
                };

                Ok(key)
            }
        }
    }
}

impl serde::Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq as _;

        let mut seq = serializer.serialize_seq(None)?;

        match self {
            Self::Connection { ref user, ref id } => {
                seq.serialize_element("connections")?;
                seq.serialize_element(user)?;
                seq.serialize_element(id)?;
            }
            Self::ConnectionsByUser { ref user } => {
                seq.serialize_element("connections")?;
                seq.serialize_element(user)?;
            }
            Self::UserToKey { ref user } => {
                seq.serialize_element("user-to-key")?;
                seq.serialize_element(user)?;
            }
            Self::KeyToUser { ref key } => {
                seq.serialize_element("key-to-user")?;
                seq.serialize_element(key)?;
            }
        }

        seq.end()
    }
}

#[derive(Clone)]
pub struct Database {
    tree: Arc<sled::Tree>,
}

impl Database {
    /// Open a new database instance.
    pub fn load(tree: Arc<sled::Tree>) -> Result<Database, Error> {
        Ok(Self { tree })
    }

    /// Get the current key by the specified user.
    pub fn get_key(&self, user: &str) -> Result<Option<String>, Error> {
        let key = Key::UserToKey {
            user: user.to_string(),
        };

        self.get::<String>(&key)
    }

    /// Get the user that corresponds to the given key.
    pub fn get_user_by_key(&self, key: &str) -> Result<Option<String>, Error> {
        let key = Key::KeyToUser {
            key: key.to_string(),
        };

        self.get::<String>(&key)
    }

    /// Store the given key.
    pub fn insert_key(&self, user: &str, key: &str) -> Result<(), Error> {
        let user_to_key = Key::UserToKey {
            user: user.to_string(),
        };

        let key_to_user = Key::KeyToUser {
            key: key.to_string(),
        };

        let mut tx = self.transaction();
        tx.insert(&user_to_key, &key)?;
        tx.insert(&key_to_user, &user)?;
        tx.commit()?;
        Ok(())
    }

    /// Delete the key associated with the specified user.
    pub fn delete_key(&self, user: &str) -> Result<(), Error> {
        let user_to_key = Key::UserToKey {
            user: user.to_string(),
        };

        if let Some(key) = self.get::<String>(&user_to_key)? {
            let key_to_user = Key::KeyToUser {
                key: key.to_string(),
            };

            let mut tx = self.transaction();
            tx.remove(&user_to_key)?;
            tx.remove(&key_to_user)?;
            tx.commit()?;
        }

        Ok(())
    }

    /// Get the connection with the specified ID.
    pub fn get_connection(&self, user: &str, id: &str) -> Result<Option<Connection>, Error> {
        let key = Key::Connection {
            user: user.to_string(),
            id: id.to_string(),
        };

        self.get(&key)
    }

    /// Add the specified connection.
    pub fn add_connection(&self, user: &str, connection: &Connection) -> Result<(), Error> {
        let key = Key::Connection {
            user: user.to_string(),
            id: connection.id.clone(),
        };

        self.insert(&key, connection)
    }

    /// Delete the specified connection.
    pub fn delete_connection(&self, user: &str, id: &str) -> Result<(), Error> {
        let key = Key::Connection {
            user: user.to_string(),
            id: id.to_string(),
        };

        self.remove(&key)
    }

    /// Get all connections for the specified user.
    pub fn connections_by_user(&self, queried_user: &str) -> Result<Vec<Connection>, Error> {
        let key = Key::ConnectionsByUser {
            user: queried_user.to_string(),
        };

        let key = key.serialize()?;
        let prefix = &key[..(key.len() - 1)];

        let mut out = Vec::new();

        for result in self.tree.range(prefix..) {
            let (key, value) = result?;

            // TODO: do something with the id?
            let _id = match Key::deserialize(key.as_ref())? {
                Key::Connection { ref user, ref id } if user == queried_user => id.to_string(),
                Key::ConnectionsByUser { ref user } if user == queried_user => {
                    continue;
                }
                _ => break,
            };

            let connection = serde_cbor::from_slice(value.as_ref())?;
            out.push(connection);
        }

        Ok(out)
    }

    /// Run the given set of operations in a transaction.
    fn transaction(&self) -> Transaction<'_> {
        Transaction {
            tree: &*self.tree,
            ops: Vec::new(),
        }
    }

    /// Insert the given key and value.
    fn insert<T>(&self, key: &Key, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let key = key.serialize()?;
        let value = serde_cbor::to_vec(value)?;
        self.tree.insert(key, value)?;
        Ok(())
    }

    /// Delete the given key.
    fn remove(&self, key: &Key) -> Result<(), Error> {
        let key = key.serialize()?;
        self.tree.remove(key)?;
        Ok(())
    }

    /// Get the value for the given key.
    fn get<T>(&self, key: &Key) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned,
    {
        let key = key.serialize()?;

        let value = match self.tree.get(&key)? {
            Some(value) => value,
            None => return Ok(None),
        };

        let value = serde_cbor::from_slice(value.as_ref())?;
        Ok(Some(value))
    }
}

pub enum Operation {
    Remove(Vec<u8>),
    Insert(Vec<u8>, Vec<u8>),
}

struct Transaction<'a> {
    tree: &'a sled::Tree,
    ops: Vec<Operation>,
}

impl Transaction<'_> {
    /// Insert the given key and value.
    pub fn insert<T>(&mut self, key: &Key, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let key = key.serialize()?;
        let value = serde_cbor::to_vec(value)?;
        self.ops.push(Operation::Insert(key, value));
        Ok(())
    }

    /// Delete the given key.
    pub fn remove(&mut self, key: &Key) -> Result<(), Error> {
        let key = key.serialize()?;
        self.ops.push(Operation::Remove(key));
        Ok(())
    }

    /// Commit the current transaction.
    pub fn commit(self) -> sled::TransactionResult<()> {
        let Transaction { tree, ops } = self;

        tree.transaction(move |tree| {
            for op in &ops {
                match op {
                    Operation::Insert(key, value) => {
                        tree.insert(key.clone(), value.clone())?;
                    }
                    Operation::Remove(key) => {
                        tree.remove(key.clone())?;
                    }
                }
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Key;
    use failure::Error;

    #[test]
    fn test_subset() -> Result<(), Error> {
        let a = Key::Connection {
            user: "setbac".to_string(),
            id: "twitch".to_string(),
        };

        let a_bytes = a.serialize()?;

        let b = Key::ConnectionsByUser {
            user: "setbac".to_string(),
        };

        let b_bytes = b.serialize()?;

        // everything is a subset *except* the last byte.
        assert!(a_bytes.starts_with(&b_bytes[..(b_bytes.len() - 1)]));

        assert_eq!(a, Key::deserialize(&a_bytes)?);
        assert_eq!(b, Key::deserialize(&b_bytes)?);
        Ok(())
    }
}
