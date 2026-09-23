//! Typed subscriptions to a product's configuration.

use std::sync::Arc;

use tokio::sync::watch;

use crate::ProductConfiguration;

/// The published state of one product.
///
/// A rejection never evicts `accepted`, so retaining the last known-good configuration is the client's behavior rather
/// than each consumer's bookkeeping.
// TODO: remove dead_code guard once the worker publishes snapshots.
#[allow(dead_code)]
struct Snapshot<T: ProductConfiguration> {
    /// The most recently accepted configuration, absent until the first snapshot is accepted.
    accepted: Option<Arc<T>>,

    /// The rejection of the most recent snapshot, absent while the most recent snapshot was accepted.
    rejection: Option<Arc<T::Error>>,
}

/// A handle that observes one product's configuration as the client publishes it.
///
/// A consumer reads [`current`](Self::current) and then loops on [`changed`](Self::changed). A subscription created
/// after the client has already published treats that value as observed and receives no notification for it, so a
/// consumer that only awaited `changed` would wait for a snapshot that may never arrive.
///
/// Cloning shares one subscription between several consumers: each clone tracks its own position and observes every
/// snapshot, while decoding still happens once per snapshot.
// TODO: remove dead_code guard once subscriptions are wired to the worker.
#[allow(dead_code)]
pub struct Subscription<T: ProductConfiguration> {
    receiver: watch::Receiver<Snapshot<T>>,
}

impl<T: ProductConfiguration> Subscription<T> {
    /// Returns the most recently accepted configuration.
    ///
    /// Returns `None` until the first snapshot is accepted. `None` does not mean the product has no configuration
    /// assigned: an empty assignment is a snapshot that [`ProductConfiguration::decode`] receives as an empty map and
    /// may accept.
    pub fn current(&self) -> Option<Arc<T>> {
        todo!()
    }

    /// Waits for the next published snapshot.
    ///
    /// Never resolves once the client's worker has stopped, so a caller may `select!` on it unconditionally.
    ///
    /// # Errors
    ///
    /// Returns the subscriber's own decoding error when the snapshot was rejected. The client reports that rejection to
    /// the Agent independently; a consumer with nothing to report can ignore it. [`current`](Self::current) continues
    /// to return the last accepted configuration.
    pub async fn changed(&mut self) -> Result<Arc<T>, Arc<T::Error>> {
        todo!()
    }
}

impl<T: ProductConfiguration> Clone for Subscription<T> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
        }
    }
}
