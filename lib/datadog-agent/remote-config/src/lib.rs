//! Provides a client for remote configuration.

#![deny(missing_docs)]

use datadog_agent_commons::ipc::client::RemoteAgentClient;

mod error;
mod payloads;
mod product;
mod protocol;
mod subscription;
#[cfg(test)]
mod tests;
mod worker;

pub use error::{ApplyError, AsApplyError, Error, Result};
pub use payloads::{Json, Payloads};
pub use product::{ProductConfiguration, ProductId};
pub use subscription::Subscription;
pub use worker::RemoteConfigurationWorker;

/// Configuration for the Remote Configuration Client.
///
/// Sorry for the weird name but it seemed better that RemoteConfigurationClientConfiguration!
pub struct RcClientConfiguration {
    /// The remote agent client
    pub remote_agent_client: RemoteAgentClient,
    // TODO: poll frequency? any other fields?
}

/// A cloneable handle for subscribing to Remote Configuration products.
#[derive(Clone)]
#[non_exhaustive]
pub struct RemoteConfigurationClient {}

impl RemoteConfigurationClient {
    /// Creates a client and its worker from a connected Datadog Agent client.
    ///
    /// The connection must be dedicated to Remote Configuration. Construction does not spawn the worker or probe
    /// Remote Configuration availability; the caller schedules the worker through a supervisor or its `run` method.
    pub fn new(_settings: RcClientConfiguration) -> (Self, RemoteConfigurationWorker) {
        todo!()
    }

    /// Subscribes to a product, decoding its complete configuration snapshot into `T`.
    ///
    /// Subscriptions can be added while the worker runs. Decoding accepts or rejects each snapshot and supplies the
    /// information needed for the client to report apply status to the Agent.
    ///
    /// A product may be subscribed only once per client. Several consumers of one product therefore share a single
    /// [`Subscription`] by cloning it, rather than each subscribing for themselves.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadySubscribed`] when the product is already subscribed on this client, which indicates
    /// that the caller should be receiving a clone of the existing subscription instead.
    // TODO: define the unsubscribe mechanism.
    pub fn subscribe<T>(&self, _product_id: ProductId) -> Result<Subscription<T>>
    where
        T: ProductConfiguration + Send + Sync + 'static,
    {
        todo!()
    }
}
