//! Bootstrap inputs for the configuration system.

use std::path::PathBuf;

/// Process-local inputs used to start configuration resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapInputs {
    /// Local Datadog-shaped bootstrap configuration file.
    pub config_file_path: PathBuf,

    /// Environment variable prefix for Datadog-shaped local bootstrap settings.
    pub env_var_prefix: &'static str,
}

impl BootstrapInputs {
    /// Creates bootstrap inputs.
    pub fn new(config_file_path: PathBuf, env_var_prefix: &'static str) -> Self {
        Self {
            config_file_path,
            env_var_prefix,
        }
    }
}
