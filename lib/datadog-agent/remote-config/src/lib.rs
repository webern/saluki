//! Provides a client for remote configuration.

use std::collections::HashMap;

mod error;
mod message;
mod product;
#[cfg(test)]
mod test;

pub use error::{Error, Result};

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

    /// Subscribes to a product.
    ///
    /// The callback receives the complete current set whenever the product changes. An empty set means the product has
    /// no configurations assigned to this client.
    pub fn subscribe<F>(&mut self, _product_id: impl AsRef<str>, _callback: F) -> Result<()>
    where
        F: Fn(&ProductConfigurations) -> ApplyResult + Send + Sync + 'static,
    {
        todo!()
    }

    /// Runs the client until it is stopped or encounters an error.
    pub async fn run(&mut self) -> Result<()> {
        todo!()
    }
}
