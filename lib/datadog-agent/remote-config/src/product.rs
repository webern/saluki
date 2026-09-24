use std::fmt;
use std::ops::Deref;

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
/// A configuration's identity is its configuration ID: the `semantic.v1` in
/// `employee/APM_SEMANTIC_CORE_DD/semantic.v1/config`. Nothing else about where the configuration came from is part of
/// its identity, because nothing else is something the protocol can act on -- an apply status is reported against a
/// product and this ID alone.
///
/// Ordering is the order in which the client presents a product's configurations to its decoder.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConfigId(pub(crate) String);

impl Deref for ConfigId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ConfigId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::product::{ConfigId, ProductId};

    #[test]
    fn product_id_impls_as_ref_str() {
        let product_id = ProductId::ApmSemanticCoreDd;
        let expected = "APM_SEMANTIC_CORE_DD";
        assert_eq!(expected, product_id.as_ref());
    }

    #[test]
    fn config_id_derefs_to_str_for_matching() {
        let config_id = ConfigId("semantic.v1".to_string());

        assert!(matches!(&*config_id, "semantic.v1"));
        assert_eq!("semantic.v1", config_id.to_string());
    }

    #[test]
    fn config_ids_order_ascending_by_id() {
        let mut ids = [
            ConfigId("registry.v3".to_string()),
            ConfigId("registry.v1".to_string()),
            ConfigId("registry.v2".to_string()),
        ];
        ids.sort();

        let sorted: Vec<&str> = ids.iter().map(|id| &**id).collect();
        assert_eq!(vec!["registry.v1", "registry.v2", "registry.v3"], sorted);
    }
}
