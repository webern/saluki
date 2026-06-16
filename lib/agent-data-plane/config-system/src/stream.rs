//! Configuration stream handles.

use agent_data_plane_config::ConfigStreamAuthority;

/// Long-lived handle for a streamed runtime configuration authority.
///
/// The handle is source-provider agnostic at the configuration-system boundary. Today the only
/// provider is the Datadog Agent config stream.
#[derive(Clone, Debug)]
pub struct ConfigStreamHandle {
    authority: ConfigStreamAuthority,
    initial_snapshot_received: bool,
}

impl ConfigStreamHandle {
    /// Creates a config stream handle.
    pub const fn new(authority: ConfigStreamAuthority, initial_snapshot_received: bool) -> Self {
        Self {
            authority,
            initial_snapshot_received,
        }
    }

    /// Returns the backing stream authority.
    pub fn authority(&self) -> ConfigStreamAuthority {
        self.authority.clone()
    }

    /// Returns whether the initial authoritative snapshot has been received.
    pub const fn initial_snapshot_received(&self) -> bool {
        self.initial_snapshot_received
    }

    /// Returns this handle with updated initial-snapshot status.
    pub const fn with_initial_snapshot_received(mut self, initial_snapshot_received: bool) -> Self {
        self.initial_snapshot_received = initial_snapshot_received;
        self
    }
}
