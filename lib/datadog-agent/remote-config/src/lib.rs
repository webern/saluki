//! Provides a client for remote configuration.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::de::DeserializeOwned;

mod error;
mod message;
mod product;
#[cfg(test)]
mod test;

pub use error::{Error, Result};

/// Identifies one configuration within a product.
pub struct ConfigId(String);

/// One configuration assigned to this client.
pub struct Payload {}

impl Payload {
    /// Returns the configuration's version.
    pub fn version(&self) -> u64 {
        todo!()
    }

    /// Returns the raw configuration contents.
    pub fn contents(&self) -> &[u8] {
        todo!()
    }
}

/// Every configuration currently assigned to this client for one product.
///
/// A handler receives this:
/// ```text
///    Payloads  (len = 3)
///      ├── config_id "dynamic_rates"      version 41   contents
///      ├── config_id "team_web_override"  version 7    contents
///      └── config_id "team_db_override"   version 3    contents
/// ```
///
/// The set is complete, not a delta. An empty set means the product has nothing assigned.
pub struct Payloads {}

impl Payloads {
    /// Returns the payload for `id`, if it is assigned.
    pub fn get(&self, _id: &ConfigId) -> Option<&Payload> {
        todo!()
    }

    /// Iterates over every assigned payload.
    pub fn iter(&self) -> impl Iterator<Item = (&ConfigId, &Payload)> {
        todo!();
        #[allow(unreachable_code)]
        std::iter::empty()
    }

    /// Returns the number of assigned payloads.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// Returns `true` if the product has nothing assigned.
    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

/// Reports that a handler could not apply the payloads it was given.
///
/// The message reaches the Datadog backend and is shown against the configuration, so write it for an operator.
pub struct ApplyError {}

/// Applies one product's configuration.
///
/// A product has exactly one handler. `apply` runs whenever the assignment or any payload changes, and its return
/// value becomes the apply status reported upstream: `Ok` acknowledges, `Err` reports the error.
///
/// Handlers must be idempotent: the same set can be applied twice.
#[async_trait]
pub trait ConfigHandler: Send + Sync + 'static {
    /// Applies the complete set of payloads currently assigned to this client.
    async fn apply(&self, payloads: &Payloads) -> std::result::Result<(), ApplyError>;
}

/// Builds a [`ConfigHandler`] that deserializes every payload as JSON before calling `f`.
///
/// If any payload fails to deserialize, `f` is not called and the error is reported upstream.
// TODO: settle whether a single bad payload fails the whole set or only itself.
pub fn json<T, F>(_f: F) -> impl ConfigHandler
where
    T: DeserializeOwned + Send + Sync + 'static,
    F: Fn(HashMap<ConfigId, T>) -> std::result::Result<(), ApplyError> + Send + Sync + 'static,
{
    todo!();
    #[allow(unreachable_code)]
    JsonHandler::<T, F> {
        _marker: std::marker::PhantomData,
    }
}

struct JsonHandler<T, F> {
    _marker: std::marker::PhantomData<(T, F)>,
}

#[async_trait]
impl<T, F> ConfigHandler for JsonHandler<T, F>
where
    T: DeserializeOwned + Send + Sync + 'static,
    F: Fn(HashMap<ConfigId, T>) -> std::result::Result<(), ApplyError> + Send + Sync + 'static,
{
    async fn apply(&self, _payloads: &Payloads) -> std::result::Result<(), ApplyError> {
        todo!()
    }
}

/// Polls the Datadog Agent for Remote Configuration product updates.
pub struct RemoteConfigurationClient {}

impl RemoteConfigurationClient {
    /// Creates a client without connecting to the Datadog Agent.
    pub fn from_configuration(_configuration: ()) -> Result<Self> {
        todo!()
    }

    /// Creates a client with a test transport.
    // TODO: use this constructor when tests introduce a mock transport.
    #[allow(dead_code)]
    pub(crate) fn for_testing(_configuration: (), _mock: ()) -> Result<Self> {
        todo!()
    }
}
