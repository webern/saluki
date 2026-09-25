//! Test support for subscribers, enabled by the `test-util` feature.

use std::sync::Arc;

use tokio::sync::watch;

use crate::decoder::{evaluate, Outcome};
use crate::subscription::Snapshot;
use crate::{ApplyError, ConfigId, ProductDecoder, Subscription};

/// Publishes into a [`Subscription`] by hand, so a subscriber can test its component without an Agent.
///
/// The subscription returned alongside the publisher is the production type: [`current`](Subscription::current) keeps
/// the last accepted snapshot through a rejection. After the publisher is dropped and pending publications are
/// observed, [`changed`](Subscription::changed) waits indefinitely. Slow consumers may skip intermediate publications.
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
#[non_exhaustive]
pub struct TestPublisher<T, E = ApplyError> {
    sender: watch::Sender<Snapshot<T, E>>,
}

impl<T, E> TestPublisher<T, E> {
    /// Creates a publisher and the subscription it publishes into.
    ///
    /// The subscription starts with no accepted snapshot, so [`current`](Subscription::current) returns `None` until
    /// the first successful publish.
    pub fn new() -> (Self, Subscription<T, E>) {
        let (sender, receiver) = watch::channel(Snapshot {
            accepted: None,
            rejection: None,
        });
        (Self { sender }, Subscription { receiver })
    }

    /// Publishes an accepted snapshot.
    ///
    /// [`current`](Subscription::current) returns this snapshot. Subscribers waiting for a change are notified.
    pub fn accept(&self, snapshot: T) {
        self.sender.send_modify(|state| {
            state.accepted = Some(Arc::new(snapshot));
            state.rejection = None;
        });
    }

    /// Publishes a rejection.
    ///
    /// Subscribers waiting for a change receive the error from [`changed`](Subscription::changed), and
    /// [`current`](Subscription::current) keeps returning the last accepted snapshot.
    pub fn reject(&self, error: E) {
        self.sender.send_modify(|state| {
            state.rejection = Some(Arc::new(error));
        });
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
    pub fn assign<P>(&self, assignment: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<[u8]>)>)
    where
        P: ProductDecoder<Snapshot = T, Error = E>,
    {
        let mut assignment: Vec<_> = assignment
            .into_iter()
            .map(|(id, payload)| (ConfigId::new(id.as_ref()), payload.as_ref().to_vec()))
            .collect();
        assignment.sort_by(|(left, _), (right, _)| left.cmp(right));
        assert!(
            assignment.windows(2).all(|pair| pair[0].0 != pair[1].0),
            "duplicate configuration ID"
        );

        let assignment = assignment
            .iter()
            .map(|(id, payload)| (id.clone(), payload.as_slice()))
            .collect();
        match evaluate::<P>(assignment).outcome {
            Outcome::Accepted(snapshot) => self.accept(snapshot),
            Outcome::Rejected(error) => self.reject(error),
            Outcome::Panicked => {}
        }
    }
}
