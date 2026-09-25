//! Provides a client for remote configuration.
//!
//! Configuration assigned to this deployment is delivered by polling the Datadog Agent, which fetches it from the Datadog
//! backend. This crate hides that protocol: a subscriber names a product, supplies a [`ProductDecoder`] for its
//! payloads, and receives typed snapshots through a [`Subscription`]. The client's identity, its protocol cursor, its
//! cache advertisement, the paths configurations arrive under, and the numeric apply states it reports are all private.
//!
//! # Testing
//!
//! With the `test-util` feature enabled, [`TestPublisher`] creates a [`Subscription`] that a test publishes into by
//! hand, either with finished snapshots and rejections or by running a decoder over payloads exactly as the client
//! does. A component that takes a `Subscription` can therefore be tested without an Agent.
//!
//! # Trust
//!
//! The client performs no TUF signature verification. It trusts the Agent, reached over an authenticated local IPC
//! channel, to have verified already. It does validate that each payload matches the length and SHA-256 hash published
//! in the accompanying targets metadata, which guards against a bug in delivery rather than against an adversary.

#![deny(missing_docs)]

use std::time::Duration;

use datadog_agent_commons::ipc::client::RemoteAgentClient;

mod decoder;
mod error;
mod product;
mod protocol;
mod source;
mod subscription;
#[cfg(test)]
mod test;
#[cfg(any(test, feature = "test-util"))]
mod test_util;
mod worker;

pub use decoder::ProductDecoder;
pub use error::{ApplyError, Error, Result};
pub use product::{ConfigId, ProductId};
pub use subscription::Subscription;
#[cfg(any(test, feature = "test-util"))]
pub use test_util::TestPublisher;
pub use worker::RemoteConfigurationWorker;

/// Settings for a [`RemoteConfigurationClient`].
///
/// Sorry for the weird name but it seemed better that RemoteConfigurationClientConfiguration.
///
/// Use [`new`](Self::new) to set the polling schedule, or [`Default`] for the standard schedule. Invalid intervals are
/// rejected at construction.
///
/// Whatever the settings, the worker polls once immediately when it starts, and retries every second until its first
/// successful poll.
#[derive(Clone, Debug)]
#[non_exhaustive]
// TODO: remove dead_code guard when the worker reads the validated settings.
#[allow(dead_code)]
pub struct RcClientConfiguration {
    /// How long the worker waits between successful polls.
    ///
    /// Shorter intervals deliver changes sooner at the cost of more requests to the Agent. The Agent refreshes from the
    /// backend far less often than this, so lowering it rarely helps. Must be at least one second.
    ///
    /// Defaults to 5 seconds.
    poll_interval: Duration,

    /// The longest the worker waits between polls while polls are failing.
    ///
    /// After consecutive failures the wait doubles, with jitter, from `poll_interval` up to this ceiling, and resets
    /// on the next success. The worker also waits this long between attempts when the Agent has remote configuration
    /// disabled. Must be at least `poll_interval`.
    ///
    /// Defaults to 90 seconds.
    max_backoff: Duration,
}

impl RcClientConfiguration {
    /// Creates settings for polling the Agent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPollInterval`] if `poll_interval` is shorter than one second, or
    /// [`Error::InvalidMaxBackoff`] if `max_backoff` is shorter than `poll_interval`.
    pub fn new(poll_interval: Duration, max_backoff: Duration) -> Result<Self> {
        if poll_interval < Duration::from_secs(1) {
            return Err(Error::InvalidPollInterval { poll_interval });
        }
        if max_backoff < poll_interval {
            return Err(Error::InvalidMaxBackoff {
                poll_interval,
                max_backoff,
            });
        }
        Ok(Self {
            poll_interval,
            max_backoff,
        })
    }
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
    pub fn new(_agent_client: RemoteAgentClient, _config: RcClientConfiguration) -> (Self, RemoteConfigurationWorker) {
        todo!()
    }

    /// Subscribes to a product, decoding its assigned configurations with `P`.
    ///
    /// The product is named by its protocol string. Pass a [`ProductId`] for a product this crate knows about, or the
    /// string itself, such as `"APM_SAMPLING"`, for one it does not. The two spellings name the same product, so they
    /// share the one-subscription limit below.
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
    pub fn subscribe<P>(&self, _product_id: impl AsRef<str>) -> Result<Subscription<P::Snapshot, P::Error>>
    where
        P: ProductDecoder,
    {
        todo!()
    }
}
