//! Private types that users should not need to worry about.

use crate::{ConfigId, ProductId};

/// The full path a configuration arrives under.
///
/// Configurations are identified on the wire as `datadog/<org_id>/<PRODUCT>/<config_id>/<name>`, or as
/// `employee/<PRODUCT>/<config_id>/<name>` for employee-signed products, which carry no organization segment. The client
/// keys its own state by the whole path because cache advertisement echoes paths back to the Agent, and publishes only
/// the configuration ID segment as a [`ConfigId`].
///
/// The trailing name segment is conventionally the literal `config` but is not required to be, and the client never acts
/// on it: an apply status cannot be reported against it, so it is parsed and discarded.
// TODO: parse this from a response, and reject a product's entire colliding set when two paths in one response share a
// configuration ID.
#[allow(dead_code)]
pub(crate) struct ConfigPath {
    /// The path exactly as it appeared on the wire, which is the form cache advertisement must echo back.
    pub(crate) raw: String,

    /// The product the configuration belongs to.
    pub(crate) product: ProductId,

    /// The configuration ID, which is the only segment a subscriber sees.
    pub(crate) config_id: ConfigId,
}

/// One configuration assigned to this client.
// TODO: use this state when implementing response caching.
#[allow(dead_code)]
pub(crate) struct RcState {
    /// The configuration version.
    ///
    /// A version may be bumped while the contents stay identical, which does not count as a change: the configuration is
    /// not re-decoded and any status already reported for it carries onto the new version.
    pub(crate) version: u64,

    /// The payload length published in the targets metadata.
    pub(crate) length: u64,

    /// The payload SHA-256 hash published in the targets metadata.
    ///
    /// Doubles as the change detector. A differing hash is what makes a configuration eligible for re-decoding and what
    /// clears a status previously reported for it.
    pub(crate) sha256: [u8; 32],

    /// The opaque product-specific payload.
    ///
    /// Retained because the Agent omits unchanged files that the client advertises as cached. A decoder is rebuilt from
    /// these bytes whenever a product's assignment changes, so the client caches payloads rather than decoded values and
    /// has no decoded state to invalidate.
    pub(crate) contents: Vec<u8>,
}
