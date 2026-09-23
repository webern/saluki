//! Borrowed product snapshots and payload decoding helpers.

use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{DeserializeOwned, Visitor};
use serde::{Deserialize, Deserializer};

/// A borrowed, read-only snapshot of a product's configuration IDs and raw payload bytes.
///
/// An empty assignment has no entries; a removed configuration is absent from the next snapshot. The client retains
/// the underlying bytes to assemble later snapshots when the Agent omits cached payloads from its responses.
// TODO: add payload access and serde support for the decoding trait.
#[allow(dead_code)]
pub struct Payloads<'a> {
    pub(crate) contents: &'a HashMap<String, Vec<u8>>,
}

/// Deserializes a configuration's contents as JSON.
///
/// Remote Configuration contents are opaque bytes, so the format is a property of the product rather than of the
/// protocol. Wrap a field in `Json` to parse that configuration as JSON; a product using another format wraps its
/// fields in its own equivalent.
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<'de, T> Deserialize<'de> for Json<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(JsonVisitor(PhantomData))
    }
}

struct JsonVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for JsonVisitor<T>
where
    T: DeserializeOwned,
{
    type Value = Json<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON configuration contents")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        serde_json::from_slice(v).map(Json).map_err(E::custom)
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_bytes(v.as_bytes())
    }
}
