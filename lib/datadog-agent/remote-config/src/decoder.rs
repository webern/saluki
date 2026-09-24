//! The subscriber's decoding contract.

use crate::{AsApplyError, ConfigId};

/// Decodes one product's assigned configurations into a snapshot.
///
/// The client drives an implementation once per published snapshot: it constructs a decoder with [`Default`], calls
/// [`decode`](Self::decode) once per assigned configuration in ascending [`ConfigId`] order, and then calls
/// [`build`](Self::build). Ascending order is part of the contract, so a reduction that keeps the last valid
/// configuration is well defined.
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
    /// Use [`ApplyError`](crate::ApplyError) when there is nothing richer to report; a product that wants to describe a
    /// failure more precisely for its own diagnostics defines its own type instead.
    type Error: AsApplyError + Send + Sync + 'static;

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
    /// # Errors
    ///
    /// Returns [`Self::Error`] to reject the snapshot as a whole, which leaves subscribers holding the last snapshot
    /// that was accepted.
    fn build(self) -> Result<Self::Snapshot, Self::Error>;
}
