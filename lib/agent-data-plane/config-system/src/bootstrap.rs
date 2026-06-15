//! Bootstrap inputs for the configuration system.

use std::path::PathBuf;

/// Process-local inputs used to start configuration resolution.
///
/// This will eventually include paths, environment selection, and other local source controls. It
/// should not expose `saluki_config::GenericConfiguration`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapInputs {
    /// TODO: figure out the actual struct fields needed.
    pub config_file_path: PathBuf,

    /// TODO: figure out the actual struct fields needed.
    pub env_var_prefix: String,
}
