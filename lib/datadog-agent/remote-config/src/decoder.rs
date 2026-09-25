//! The subscriber's decoding contract.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::{ApplyError, ConfigId};

/// Decodes one product's assigned configurations into a snapshot.
///
/// The client drives an implementation once per published snapshot: it constructs a decoder with [`Default`], calls
/// [`decode`](Self::decode) once per assigned configuration in ascending [`ConfigId`] order, and then calls
/// [`build`](Self::build). Ascending order is part of the contract, so a reduction that keeps the last valid
/// configuration is well defined.
///
/// A product is decoded again only when its assigned configurations or their contents change, so a rejected
/// assignment is not retried until it changes. When the Agent reports its configuration expired, every product is
/// decoded as an empty assignment.
///
/// Decoding runs on the client's worker task and delays polling for every product while it runs, so implementations
/// **MUST NOT** block. A panic in [`decode`](Self::decode) or [`build`](Self::build) is caught: the client discards the
/// decoder, rejects every configuration in the assignment, and publishes nothing, so subscribers keep the last accepted
/// snapshot and are not notified.
///
/// # Design
///
/// A product's assignment is several configurations, each with its own payload, and the protocol carries an apply
/// status for each one separately. Decoding therefore accumulates configuration by configuration rather than consuming
/// the assignment as a whole: a malformed configuration is attributed to itself and skipped, and the product is still
/// built from the rest. Because the client drives the loop, that attribution cannot be forgotten or misdirected.
///
/// Accumulating into the decoder, rather than returning one decoded value per configuration, is what lets a product
/// whose configurations have different shapes hold each of them in a field of its own type instead of funneling them
/// through a shared enum.
pub trait ProductDecoder: Default + Send + 'static {
    /// The configuration snapshot published to subscribers.
    ///
    /// This type carries no bounds of the client's, so a product may publish a struct of its own, a collection, or a
    /// type from another crate.
    type Snapshot: Send + Sync + 'static;

    /// The error this product's decoding and validation produces.
    ///
    /// Use [`String`] when there is nothing richer to report; a product that wants to describe a failure more
    /// precisely for its own diagnostics defines its own type and implements [`ApplyError`] for it.
    type Error: ApplyError + Send + Sync + 'static;

    /// Accumulates one of the product's assigned configurations.
    ///
    /// The client calls this at most once per distinct `id` within a single snapshot, so a decoder may hold one slot per
    /// configuration ID without a second configuration silently displacing the first.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] to reject this configuration alone. The client attributes the rejection to it and skips
    /// it, and still decodes the product's remaining configurations.
    fn decode(&mut self, id: &ConfigId, payload: &[u8]) -> Result<(), Self::Error>;

    /// Validates the accumulated configurations and produces the snapshot to publish.
    ///
    /// Validation that spans configurations belongs here, including a required configuration being absent: a product
    /// assigned nothing at all is built from a default decoder that received no calls to [`decode`](Self::decode), so
    /// whether an empty assignment is acceptable is this method's decision.
    ///
    /// The client acknowledges successfully decoded configurations only when this method succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] to reject the snapshot. The client rejects every successfully decoded configuration
    /// with this error's apply reason; configurations rejected by [`decode`](Self::decode) keep their own errors.
    /// No new snapshot is published, and subscribers retain the last accepted snapshot.
    ///
    /// This method cannot reject selected configurations while publishing the rest. Selective rejection belongs in
    /// [`decode`](Self::decode).
    fn build(self) -> Result<Self::Snapshot, Self::Error>;
}

/// What one run of a decoder over a product's assignment produced.
pub(crate) struct Evaluation<T, E> {
    pub(crate) outcome: Outcome<T, E>,
    /// Each assigned configuration's rejection, if any, in ascending ID order.
    pub(crate) verdicts: Vec<(ConfigId, Option<String>)>,
}

pub(crate) enum Outcome<T, E> {
    /// `build` succeeded; the snapshot is published.
    Accepted(T),

    /// `build` failed; the error is published as a rejection and `current` keeps the last accepted snapshot.
    Rejected(E),

    /// `decode` or `build` panicked; nothing is published and subscribers are not notified.
    Panicked,
}

/// Runs a fresh decoder over one product's assignment exactly as the client does.
///
/// This is the only implementation of the decoding rules, shared by the worker and by
/// [`TestPublisher::assign`](crate::TestPublisher::assign), so that what a subscriber tests is what production runs:
/// configurations in ascending [`ConfigId`] order, a rejected configuration skipped while the rest are still decoded,
/// then `build`, with a panic in either caught.
pub(crate) fn evaluate<P: ProductDecoder>(mut assignment: Vec<(ConfigId, &[u8])>) -> Evaluation<P::Snapshot, P::Error> {
    assignment.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut verdicts = Vec::with_capacity(assignment.len());
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut decoder = P::default();
        for (id, payload) in &assignment {
            let rejection = decoder.decode(id, payload).err().map(|error| error.apply_error());
            verdicts.push((id.clone(), rejection));
        }
        match decoder.build() {
            Ok(snapshot) => Outcome::Accepted(snapshot),
            Err(error) => {
                let reason = error.apply_error();
                for (_, rejection) in &mut verdicts {
                    if rejection.is_none() {
                        *rejection = Some(reason.clone());
                    }
                }
                Outcome::Rejected(error)
            }
        }
    }));

    let outcome = match result {
        Ok(outcome) => outcome,
        Err(_) => {
            verdicts = assignment
                .into_iter()
                .map(|(id, _)| (id, Some("Product decoder panicked.".to_owned())))
                .collect();
            Outcome::Panicked
        }
    };

    Evaluation { outcome, verdicts }
}
