//! Native configuration for ADP CLI/debug helpers.

use std::path::{Path, PathBuf};

/// Native DogStatsD CLI/debug configuration.
#[derive(Clone, Debug, Default)]
pub struct DogStatsDCliConfiguration {
    dogstatsd_socket_path: Option<PathBuf>,
}

impl DogStatsDCliConfiguration {
    /// Creates native DogStatsD CLI/debug configuration.
    pub fn new(dogstatsd_socket_path: Option<PathBuf>) -> Self {
        Self { dogstatsd_socket_path }
    }

    /// Returns the configured DogStatsD socket path, if present.
    pub fn dogstatsd_socket_path(&self) -> Option<&Path> {
        self.dogstatsd_socket_path.as_deref()
    }
}
