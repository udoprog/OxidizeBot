//! Values scripts store in the database.
//!
//! The serialized form must stay identical to rune 0.12's `ConstValue`, which
//! earlier versions of the bot stored, so that existing script data keeps
//! loading.

use std::collections::HashMap;
use std::fmt;

use rune::alloc;
use rune::compile::meta::DocType;
use rune::runtime::{
    Bytes, MaybeTypeOf, Object, OwnedTuple, RuntimeError, TypeHash, Vec as RuneVec,
};
use rune::{FromValue, ToValue, Value};
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum StoredValue {
    Unit,
    Byte(u8),
    Char(char),
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    /// Only produced by rune 0.12 for string literals; read back as a string.
    StaticString(LegacyStaticString),
    Bytes(StoredBytes),
    Vec(Vec<StoredValue>),
    Tuple(Box<[StoredValue]>),
    Object(HashMap<String, StoredValue>),
    Option(Option<Box<StoredValue>>),
    /// Unsigned integers which did not exist in rune 0.12.
    Unsigned(u64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LegacyStaticString {
    inner: String,
    hash: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredBytes(Vec<u8>);

impl Serialize for StoredBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for StoredBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = StoredBytes;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StoredBytes(v.to_vec()))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StoredBytes(v))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut out = Vec::new();

                while let Some(b) = seq.next_element()? {
                    out.push(b);
                }

                Ok(StoredBytes(out))
            }
        }

        deserializer.deserialize_byte_buf(BytesVisitor)
    }
}

impl StoredValue {
    fn from_value_ref(value: &Value) -> Result<Self, RuntimeError> {
        let hash = value.type_hash();

        let out = if value.into_unit().is_ok() {
            Self::Unit
        } else if hash == bool::HASH {
            Self::Bool(rune::from_value(value.clone())?)
        } else if hash == char::HASH {
            Self::Char(rune::from_value(value.clone())?)
        } else if hash == i64::HASH {
            Self::Integer(rune::from_value(value.clone())?)
        } else if hash == u64::HASH {
            Self::Unsigned(rune::from_value(value.clone())?)
        } else if hash == f64::HASH {
            Self::Float(rune::from_value(value.clone())?)
        } else if hash == alloc::String::HASH {
            Self::String(value.borrow_string_ref()?.to_owned())
        } else if hash == Bytes::HASH {
            Self::Bytes(StoredBytes(
                value.borrow_ref::<Bytes>()?.as_slice().to_vec(),
            ))
        } else if hash == RuneVec::HASH {
            let vec = value.borrow_ref::<RuneVec>()?;
            Self::Vec(
                vec.iter()
                    .map(Self::from_value_ref)
                    .collect::<Result<_, _>>()?,
            )
        } else if hash == OwnedTuple::HASH {
            let tuple = value.borrow_ref::<OwnedTuple>()?;
            Self::Tuple(
                tuple
                    .iter()
                    .map(Self::from_value_ref)
                    .collect::<Result<_, _>>()?,
            )
        } else if hash == Object::HASH {
            let object = value.borrow_ref::<Object>()?;
            let mut out = HashMap::with_capacity(object.len());

            for (key, value) in object.iter() {
                out.insert(key.as_str().to_owned(), Self::from_value_ref(value)?);
            }

            Self::Object(out)
        } else if hash == Option::<Value>::HASH {
            let option = value.borrow_ref::<Option<Value>>()?;

            Self::Option(match &*option {
                Some(some) => Some(Box::new(Self::from_value_ref(some)?)),
                None => None,
            })
        } else {
            return Err(RuntimeError::panic(format!(
                "Value of type {} can't be stored",
                value.type_info()
            )));
        };

        Ok(out)
    }

    fn into_rune_value(self) -> Result<Value, RuntimeError> {
        let value = match self {
            Self::Unit => rune::to_value(())?,
            Self::Byte(b) => rune::to_value(b)?,
            Self::Char(c) => rune::to_value(c)?,
            Self::Bool(b) => rune::to_value(b)?,
            Self::Integer(n) => rune::to_value(n)?,
            Self::Unsigned(n) => rune::to_value(n)?,
            Self::Float(n) => rune::to_value(n)?,
            Self::String(s) => rune::to_value(s)?,
            Self::StaticString(s) => rune::to_value(s.inner)?,
            Self::Bytes(b) => rune::to_value(Bytes::from_vec(alloc::Vec::try_from(b.0)?))?,
            Self::Vec(values) => Value::vec(Self::into_rune_values(values)?)?,
            Self::Tuple(values) => Value::tuple(Self::into_rune_values(values.into_vec())?)?,
            Self::Object(object) => {
                let mut o = Object::with_capacity(object.len())?;

                for (key, value) in object {
                    o.insert(alloc::String::try_from(key)?, value.into_rune_value()?)?;
                }

                rune::to_value(o)?
            }
            Self::Option(option) => rune::to_value(match option {
                Some(some) => Some(some.into_rune_value()?),
                None => None,
            })?,
        };

        Ok(value)
    }

    fn into_rune_values(values: Vec<Self>) -> Result<alloc::Vec<Value>, RuntimeError> {
        let mut out = alloc::Vec::try_with_capacity(values.len())?;

        for value in values {
            out.try_push(value.into_rune_value()?)?;
        }

        Ok(out)
    }
}

impl FromValue for StoredValue {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        Self::from_value_ref(&value)
    }
}

impl ToValue for StoredValue {
    fn to_value(self) -> Result<Value, RuntimeError> {
        self.into_rune_value()
    }
}

impl MaybeTypeOf for StoredValue {
    fn maybe_type_of() -> alloc::Result<DocType> {
        Value::maybe_type_of()
    }
}

#[cfg(test)]
mod tests {
    use super::StoredValue as S;

    fn decode(hex: &str) -> S {
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect::<Vec<u8>>();
        serde_cbor::from_slice(&bytes).unwrap()
    }

    fn encode(value: &S) -> String {
        serde_cbor::to_vec(value)
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Encodings written by rune 0.12's `ConstValue`.
    #[test]
    fn legacy_encoding() {
        let int = "a167496e74656765721829";
        assert_eq!(decode(int), S::Integer(41));
        assert_eq!(encode(&S::Integer(41)), int);

        let key = "a16356656382a166537472696e6765636f756e74a166537472696e676775646f70726f67";
        let value = S::Vec(vec![S::String("count".into()), S::String("udoprog".into())]);
        assert_eq!(decode(key), value);
        assert_eq!(encode(&value), key);

        // `["count", user]` where "count" is a literal.
        let S::Vec(values) = decode("a16356656382a16c537461746963537472696e67a265696e6e657265636f756e7464686173681bfc800bd8621d9b35a166537472696e676775646f70726f67") else {
            panic!("expected vec");
        };
        assert!(matches!(&values[0], S::StaticString(..)));
        assert_eq!(values[1], S::String("udoprog".into()));

        // `(1, 2.5, true, 'x', b'a', b"hi", Some(#{a: ()}), None, [], "s")`
        let tuple = "a1655475706c658aa167496e746567657201a165466c6f6174f94100a164426f6f6cf5a164436861726178a164427974651861a1654279746573426869a1664f7074696f6ea1664f626a656374a1616164556e6974a1664f7074696f6ef6a16356656380a16c537461746963537472696e67a265696e6e6572617364686173681ba3c7365327b205f2";
        let S::Tuple(values) = decode(tuple) else {
            panic!("expected tuple");
        };
        assert_eq!(values[0], S::Integer(1));
        assert_eq!(values[1], S::Float(2.5));
        assert_eq!(values[2], S::Bool(true));
        assert_eq!(values[3], S::Char('x'));
        assert_eq!(values[4], S::Byte(b'a'));
        assert_eq!(values[5], S::Bytes(super::StoredBytes(b"hi".to_vec())));
        assert_eq!(
            values[6],
            S::Option(Some(Box::new(S::Object(
                [("a".to_owned(), S::Unit)].into_iter().collect()
            ))))
        );
        assert_eq!(values[7], S::Option(None));
        assert_eq!(values[8], S::Vec(Vec::new()));
        assert!(matches!(&values[9], S::StaticString(..)));
    }

    #[test]
    fn rune_roundtrip() {
        let value = S::Tuple(Box::new([
            S::Integer(-1),
            S::Unsigned(u64::MAX),
            S::Float(0.5),
            S::String("hello".into()),
            S::Bytes(super::StoredBytes(vec![1, 2])),
            S::Vec(vec![S::Bool(false), S::Char('c')]),
            S::Object([("k".to_owned(), S::Unit)].into_iter().collect()),
            S::Option(Some(Box::new(S::Integer(2)))),
            S::Option(None),
        ]));

        let rune_value = rune::to_value(value.clone()).unwrap();
        assert_eq!(rune::from_value::<S>(rune_value).unwrap(), value);
    }
}
