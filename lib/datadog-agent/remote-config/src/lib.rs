//! Provides a client for remote configuration.

#![deny(missing_docs)]

use std::sync::Arc;

use datadog_agent_commons::ipc::client::RemoteAgentClient;
use tokio::sync::watch;

mod error;
mod payloads;
mod product;
mod protocol;
#[cfg(test)]
mod tests;
mod worker;

pub use error::{ApplyError, Error, Result};
pub use payloads::{Json, Payloads};
pub use product::{ProductConfiguration, ProductId};
pub use worker::RemoteConfigurationWorker;

/// A cloneable handle for subscribing to Remote Configuration products.
#[derive(Clone)]
#[non_exhaustive]
pub struct RemoteConfigurationClient {}

impl RemoteConfigurationClient {
    /// Creates a client and its worker from a connected Datadog Agent client.
    ///
    /// The connection must be dedicated to Remote Configuration. Construction does not spawn the worker or probe
    /// Remote Configuration availability; the caller schedules the worker through a supervisor or its `run` method.
    pub fn new(_agent_client: RemoteAgentClient) -> (Self, RemoteConfigurationWorker) {
        todo!()
    }

    /// Subscribes to a product, decoding its complete configuration snapshot into `T`.
    ///
    /// Subscriptions can be added while the worker runs. Decoding accepts or rejects each snapshot and supplies the
    /// information needed for the client to report apply status to the Agent.
    // TODO: settle the return type, initial absence of a value, and delivery of decoding errors to subscribers.
    // TODO: define the unsubscribe mechanism.
    pub fn subscribe<T>(&self, _product_id: ProductId) -> Result<watch::Receiver<Arc<T>>>
    where
        T: ProductConfiguration + Send + Sync + 'static,
    {
        todo!()
    }
}
