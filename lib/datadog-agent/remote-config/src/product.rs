use serde::Serialize;
use serde_variant::to_variant_name;

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

/// Identifies one configuration assigned to a product.
///
/// Ordering is the order in which the client presents a product's configurations to its decoder.
// TODO: the shape of this identity is an open design detail: whether it carries the trailing name segment of the
// configuration's path in addition to the configuration ID, and whether a decoder also sees the configuration's
// version and length.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct ConfigId {
    pub(crate) id: String,
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
