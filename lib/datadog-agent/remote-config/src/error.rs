use snafu::Snafu;

use crate::ProductId;

/// An error produced by the remote configuration client.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    /// The product already has a subscription on this client.
    ///
    /// A product carries one apply status per configuration, so only one subscriber may decode it. To drive several
    /// consumers from one product, subscribe once where those consumers are wired together and give each of them a
    /// clone of that [`Subscription`](crate::Subscription): every clone observes every snapshot, and the product is
    /// still decoded once per snapshot.
    ///
    /// The client does not return the existing subscription, because it cannot know that the type the second caller
    /// asks for is the type the first caller subscribed with. This error therefore means the call sites need
    /// restructuring so that one of them owns the subscription and passes clones to the others, rather than that the
    /// call should be retried.
    #[snafu(display("Product {} is already subscribed on this client.", product.as_ref()))]
    AlreadySubscribed {
        /// The product that is already subscribed.
        product: ProductId,
    },
}

/// A rejection reason reported to the Agent.
///
/// The client determines which configurations are rejected: a [`decode`](crate::ProductDecoder::decode) error rejects
/// that configuration, while a [`build`](crate::ProductDecoder::build) error rejects every successfully decoded
/// configuration. Build errors do not replace individual decode errors.
// TODO: define the representation of rejection reasons.
#[derive(Clone, Debug, Snafu)]
#[snafu(display("Remote configuration was rejected."))]
#[non_exhaustive]
pub struct ApplyError;

/// Converts a subscriber's decoding error into the client's wire-facing rejection.
///
/// A subscriber keeps the full fidelity of its own error type while the client reports a lossy form of it, since the
/// protocol carries an apply error as a single string.
pub trait AsApplyError {
    /// Describes this error as a rejection the client can report to the Agent.
    fn as_apply_error(&self) -> ApplyError;
}

impl AsApplyError for ApplyError {
    fn as_apply_error(&self) -> ApplyError {
        self.clone()
    }
}

/// A result produced by the remote configuration client.
pub type Result<T> = std::result::Result<T, Error>;
