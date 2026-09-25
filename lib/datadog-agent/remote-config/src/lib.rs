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

use std::time::Duration;

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

/// Settings for a [`RemoteConfigurationClient`].
///
/// Sorry for the weird name but it seemed better that RemoteConfigurationClientConfiguration.
///
/// Start from [`Default`] and assign the fields to change. Out-of-range values are clamped rather than rejected, so
/// construction stays infallible.
///
/// Whatever the settings, the worker polls once immediately when it starts, and retries every second until its first
/// successful poll.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RcClientConfiguration {
    /// How long the worker waits between successful polls.
    ///
    /// Shorter intervals deliver changes sooner at the cost of more requests to the Agent. The Agent refreshes from the
    /// backend far less often than this, so lowering it rarely helps. Values below one second are raised to one second.
    ///
    /// Defaults to 5 seconds.
    pub poll_interval: Duration,

    /// The longest the worker waits between polls while polls are failing.
    ///
    /// After consecutive failures the wait doubles, with jitter, from `poll_interval` up to this ceiling, and resets
    /// on the next success. The worker also waits this long between attempts when the Agent has remote configuration
    /// disabled. Values below `poll_interval` are raised to `poll_interval`.
    ///
    /// Defaults to 90 seconds.
    pub max_backoff: Duration,
}

impl Default for RcClientConfiguration {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
            max_backoff: Duration::from_secs(90),
        }
    }
}

/// A cloneable handle for subscribing to Remote Configuration products.
#[derive(Clone)]
#[non_exhaustive]
pub struct RemoteConfigurationClient {}

// TODO: consider opt-in health notifications when a subscriber needs them (e.g. CWS enforcement).
// TODO: consider a per-product option to keep the last configuration when the Agent reports it expired.
impl RemoteConfigurationClient {
    /// Creates a client and its worker from a connected Datadog Agent client.
    ///
    /// The connection must be dedicated to Remote Configuration. Construction does not spawn the worker or probe
    /// Remote Configuration availability; the caller schedules the worker through a supervisor or its `run` method.
    ///
    /// The client's identity is private and chosen here: a random ID kept for the life of the client, and the
    /// application name and version of the running binary.
    // TODO: generate the client ID with `uuid` (v4) and read the name and version from `saluki_metadata`.
    // TODO: clamp `poll_interval` and `max_backoff` as documented on `RcClientConfiguration`.
    pub fn new(_agent_client: RemoteAgentClient, _config: RcClientConfiguration) -> (Self, RemoteConfigurationWorker) {
        todo!()
    }

    /// Subscribes to a product, decoding its assigned configurations with `P`.
    ///
    /// The decoder is named here, where how a product is read is the subject; the returned subscription is typed by the
    /// snapshot that decoder builds. Subscriptions can be added while the worker runs.
    ///
    /// A product may have only one live subscription per client. Several consumers of one product therefore share a
    /// single [`Subscription`] by cloning it, rather than each subscribing for themselves. Dropping the last clone
    /// unsubscribes, after which the product may be subscribed again.
    ///
    /// Subscribing wakes the worker to poll immediately rather than at its next scheduled poll. A product no other
    /// client of the Agent has requested may still take up to the Agent's own backend refresh interval to arrive.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadySubscribed`] when the product still has a live subscription on this client, which
    /// indicates that the caller should be receiving a clone of the existing subscription instead.
    // TODO: replace a registry entry whose last clone was dropped before the worker noticed.
    pub fn subscribe<P>(&self, _product_id: ProductId) -> Result<Subscription<P::Snapshot, P::Error>>
    where
        P: ProductDecoder,
    {
        todo!()
    }
}
