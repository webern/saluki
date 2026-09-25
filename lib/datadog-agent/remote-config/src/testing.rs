//! Test support for subscribers, enabled by the `test-util` feature.

use std::marker::PhantomData;

use crate::{ApplyError, ProductDecoder, Subscription};

/// Publishes into a [`Subscription`] by hand, so a subscriber can test its component without an Agent.
///
/// The subscription returned alongside the publisher is the production type, so it behaves as it does in production:
/// [`current`](Subscription::current) keeps the last accepted snapshot through a rejection, a clone observes every
/// publish, and once the publisher is dropped [`changed`](Subscription::changed) never resolves, as when the client's
/// worker has stopped.
///
/// A component under test should take a [`Subscription`] rather than a
/// [`RemoteConfigurationClient`](crate::RemoteConfigurationClient). Production wiring subscribes once per product and
/// hands out clones, and a component built that way can be tested with this publisher alone.
///
/// # Examples
///
/// ```ignore
/// let (publisher, subscription) = TestPublisher::<SemanticCore, SemanticCoreError>::new();
/// let mut component = MyComponent::new(subscription);
///
/// // Publish a finished snapshot, or a rejection:
/// publisher.accept(SemanticCore::default());
/// publisher.reject(SemanticCoreError::MissingAttributes);
///
/// // Or publish whatever the product's decoder makes of these payloads:
/// publisher.assign::<SemanticCoreDecoder>([("semantic.v1", payload), ("metrics.v1", other)]);
/// ```
// TODO: hold the `watch::Sender` whose receiver backs the returned subscription.
#[non_exhaustive]
pub struct TestPublisher<T, E = ApplyError> {
    _snapshot: PhantomData<fn(T, E)>,
}

impl<T, E> TestPublisher<T, E> {
    /// Creates a publisher and the subscription it publishes into.
    ///
    /// The subscription starts with no accepted snapshot, so [`current`](Subscription::current) returns `None` until
    /// the first successful publish.
    pub fn new() -> (Self, Subscription<T, E>) {
        todo!()
    }

    /// Publishes an accepted snapshot.
    ///
    /// Every clone of the subscription is notified, and [`current`](Subscription::current) returns this snapshot.
    pub fn accept(&self, _snapshot: T) {
        todo!()
    }

    /// Publishes a rejection.
    ///
    /// Every clone of the subscription receives the error from [`changed`](Subscription::changed), and
    /// [`current`](Subscription::current) keeps returning the last accepted snapshot.
    pub fn reject(&self, _error: E) {
        todo!()
    }

    /// Runs `P` over an assignment of configurations and publishes the outcome, exactly as the client's worker would.
    ///
    /// Each item is a configuration ID, such as `semantic.v1`, and its payload. The decoding rules are the production
    /// code, not a copy: a fresh decoder receives the configurations in ascending ID order, a configuration that
    /// [`decode`](ProductDecoder::decode) rejects is skipped while the rest are still decoded, and then
    /// [`build`](ProductDecoder::build) decides the outcome. A successful build publishes as [`accept`](Self::accept)
    /// does, and a failed build publishes as [`reject`](Self::reject) does. A panic in the decoder is caught and
    /// publishes nothing, so subscribers are not notified.
    ///
    /// An empty assignment is valid and is what a product with no configurations assigned receives, including when the
    /// Agent reports its configuration expired.
    ///
    /// Unlike the worker, this decodes on every call, even when the assignment is unchanged. Production publishes
    /// identical snapshots after a worker restart, so subscribers must tolerate them regardless.
    ///
    /// The apply statuses the client would report to the Agent are not returned; they are the client's concern.
    ///
    /// # Panics
    ///
    /// Panics if two items share a configuration ID. Configuration IDs are unique within a product's assignment, so a
    /// duplicate indicates a mistake in the test.
    pub fn assign<P>(&self, _assignment: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<[u8]>)>)
    where
        P: ProductDecoder<Snapshot = T, Error = E>,
    {
        todo!()
    }
}
