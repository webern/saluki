//! Private types that users should not need to worry about.

/// One configuration assigned to a client.
pub(crate) struct RcState {
    /// The configuration version.
    pub(crate) version: u64,

    /// The opaque product-specific payload.
    pub(crate) contents: Vec<u8>,
}
