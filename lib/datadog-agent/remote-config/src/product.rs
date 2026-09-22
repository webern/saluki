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

    // TODO: This is annoying but if we are missing a product ID I think we may need a way for the user to provide it. Sucks. Should I just use constants instead of an enum so that this is more usable broadly?
    Other(String),
}

impl AsRef<str> for ProductId {
    fn as_ref(&self) -> &str {
        to_variant_name(self).unwrap_or("UNKNOWN")
    }
}

#[cfg(test)]
mod test {
    use crate::product::ProductId;

    #[test]
    fn product_id_impls_as_ref_str() {
        let product_id = ProductId::ApmSemanticCoreDd;
        let expected = "APM_SEMANTIC_CORE_DD";
        assert_eq!(expected, product_id.as_ref());
    }
}
