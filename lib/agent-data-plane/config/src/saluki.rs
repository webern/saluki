//! ADP-native runtime configuration.

/// Complete ADP-native runtime configuration.
///
/// Translators produce this from a primary config language plus any required
/// `SalukiPrivateConfiguration`. Runtime components should consume this model rather than source
/// config maps or source-language schema types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalukiConfiguration {
    /// TODO: figure out the actual struct fields needed.
    pub data_plane_enabled: bool,

    /// TODO: figure out the actual struct fields needed.
    pub enabled_pipelines: Vec<String>,
}
