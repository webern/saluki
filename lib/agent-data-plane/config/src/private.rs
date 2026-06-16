//! Saluki-native supplemental configuration.

use crate::authority::RuntimeConfigLanguage;

/// Saluki-native configuration that supplements the selected primary config language.
///
/// This is not a universal fixed set of keys. A setting is "Saluki-private" when the selected
/// primary language cannot express it. That boundary can differ between Datadog Agent, OTel
/// Collector, OPW/Vector, and native Saluki configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalukiPrivateConfiguration {
    /// TODO: figure out the actual struct fields needed.
    pub primary_language: RuntimeConfigLanguage,

    /// TODO: figure out the actual struct fields needed.
    pub supplemental_keys: Vec<String>,
}
