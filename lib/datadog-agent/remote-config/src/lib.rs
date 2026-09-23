//! Provides a client for remote configuration.

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use serde::de::{DeserializeOwned, Deserializer, Visitor};
use serde::Deserialize;
use tokio::sync::watch;

mod error;
mod payloads;
mod product;
mod protocol;
#[cfg(test)]
mod test;

pub use error::{Error, Result};
pub use product::ProductId;

/// Polls the Datadog Agent for Remote Configuration product updates.
pub struct RemoteConfigurationClient {}

impl RemoteConfigurationClient {
    /// Creates a client without connecting to the Datadog Agent.
    pub fn from_configuration(_configuration: ()) -> Result<Self> {
        todo!()
    }

    /// Subscribes to a product, deserializing its configurations into `T`.
    ///
    /// `T` is deserialized from a map whose keys are configuration IDs and whose values are the raw configuration
    /// contents. Name the IDs as fields when they're known, or use a map type when they aren't:
    ///
    /// ```ignore
    /// #[derive(Deserialize)]
    /// struct SemanticCore {
    ///     #[serde(rename = "semantic_core.v1")]
    ///     core: Json<Mappings>,
    /// }
    ///
    /// type Sampling = HashMap<String, Json<Rates>>;
    /// ```
    ///
    /// The returned receiver holds the current value: a subscriber that arrives late still sees it. Deserializing `T`
    /// is what acknowledges the configuration upstream, so a failure is reported as an error against the offending
    /// configuration and the receiver keeps its previous value.
    ///
    /// Dropping every receiver for a product unsubscribes from it.
    pub fn subscribe<T>(&self, _product_id: ProductId) -> Result<watch::Receiver<Arc<T>>>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        todo!()
    }

    /// Creates a client with a test transport.
    // TODO: use this constructor when tests introduce a mock transport.
    #[allow(dead_code)]
    pub(crate) fn for_testing(_configuration: (), _mock: ()) -> Result<Self> {
        todo!()
    }
}
