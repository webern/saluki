use serde::Serialize;
use serde_variant::to_variant_name;

use crate::{AsApplyError, Payloads};

/// Decodes and validates a product's complete configuration snapshot.
pub trait ProductConfiguration: Sized {
    /// The error this product's decoding and validation produces.
    ///
    /// Use [`ApplyError`](crate::ApplyError) when there is nothing richer to report; a product that wants to attribute
    /// a failure more precisely for its own diagnostics defines its own type instead.
    type Error: AsApplyError + Send + Sync + 'static;

    /// Decodes the assigned payloads into an accepted configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when the snapshot cannot be decoded or fails validation. The client reports the
    /// rejection to the Agent and delivers the error to subscribers; the subscriber does not acknowledge
    /// configurations separately.
    fn decode(payloads: Payloads<'_>) -> Result<Self, Self::Error>;
}

/// Represents product strings, such as `APM_SEMANTIC_CORE_DD`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ProductId {
    /// Trace sampling configuration.
    ApmSampling,

    /// Datadog-managed semantic convention mappings.
    ApmSemanticCoreDd,
    // TODO: how can users specify a product we haven't added as "first-class" yet?
    // Other(String),
}

impl AsRef<str> for ProductId {
    fn as_ref(&self) -> &str {
        to_variant_name(self).unwrap_or("UNKNOWN")
    }
}

#[cfg(test)]
mod tests {
    use crate::product::ProductId;

    #[test]
    fn product_id_impls_as_ref_str() {
        let product_id = ProductId::ApmSemanticCoreDd;
        let expected = "APM_SEMANTIC_CORE_DD";
        assert_eq!(expected, product_id.as_ref());
    }
}
