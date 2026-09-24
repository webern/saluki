//! Provides a client for remote configuration.
//!
//! Configuration assigned to this deployment is delivered by polling the Datadog Agent, which relays it from the Datadog
//! backend. This crate hides that protocol: a subscriber names a product, supplies a [`ProductDecoder`] for its
//! payloads, and receives typed snapshots through a [`Subscription`]. The client's identity, its protocol cursor, its
//! cache advertisement, the paths configurations arrive under, and the numeric apply states it reports are all private.
//!
//! # Trust
//!
//! The client performs no TUF signature verification. It trusts the Agent, reached over an authenticated local IPC
//! channel, to have verified already. It does validate that each payload matches the length and SHA-256 hash published
//! in the accompanying targets metadata, which guards against a relaying bug rather than against an adversary.

#![deny(missing_docs)]

use datadog_agent_commons::ipc::client::RemoteAgentClient;

mod decoder;
mod error;
mod product;
mod protocol;
mod subscription;
#[cfg(test)]
mod tests;
mod worker;

pub use decoder::ProductDecoder;
pub use error::{ApplyError, AsApplyError, Error, Result};
pub use product::{ConfigId, ProductId};
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

// TODO: consider opt-in health notifications when a subscriber needs them (e.g. CWS enforcement).
impl RemoteConfigurationClient {
    /// Creates a client and its worker from a connected Datadog Agent client.
    ///
    /// The connection must be dedicated to Remote Configuration. Construction does not spawn the worker or probe
    /// Remote Configuration availability; the caller schedules the worker through a supervisor or its `run` method.
    pub fn new(_settings: RcClientConfiguration) -> (Self, RemoteConfigurationWorker) {
        todo!()
    }

    /// Subscribes to a product, decoding its assigned configurations with `P`.
    ///
    /// The decoder is named here, where how a product is read is the subject; the returned subscription is typed by the
    /// snapshot that decoder builds. Subscriptions can be added while the worker runs.
    ///
    /// A product may be subscribed only once per client. Several consumers of one product therefore share a single
    /// [`Subscription`] by cloning it, rather than each subscribing for themselves.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadySubscribed`] when the product is already subscribed on this client, which indicates
    /// that the caller should be receiving a clone of the existing subscription instead.
    // TODO: define the unsubscribe mechanism.
    pub fn subscribe<P>(&self, _product_id: ProductId) -> Result<Subscription<P::Snapshot, P::Error>>
    where
        P: ProductDecoder,
    {
        todo!()
    }
}
