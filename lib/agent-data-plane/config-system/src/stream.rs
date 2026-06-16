//! Configuration stream handle.
//!
//! The handle is source-provider agnostic at the configuration-system boundary; today the only
//! provider is the Datadog Agent config stream. The configuration system is the sole receiver of
//! inbound config updates (a fixed invariant): typed, per-component scoped delivery is layered on
//! top of this single receiver by the dynamic-update routing layer.

use agent_data_plane_config::ConfigStreamAuthority;
use saluki_config::dynamic::ConfigUpdate;
use tokio::sync::mpsc;

/// Long-lived handle for a streamed runtime configuration authority.
pub struct ConfigStreamHandle {
    authority: ConfigStreamAuthority,
    updates: mpsc::Receiver<ConfigUpdate>,
}

impl std::fmt::Debug for ConfigStreamHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigStreamHandle")
            .field("authority", &self.authority)
            .finish_non_exhaustive()
    }
}

impl ConfigStreamHandle {
    /// Creates a stream handle wrapping the inbound update receiver.
    pub fn new(authority: ConfigStreamAuthority, updates: mpsc::Receiver<ConfigUpdate>) -> Self {
        Self { authority, updates }
    }

    /// Returns the authority backing this stream.
    pub fn authority(&self) -> &ConfigStreamAuthority {
        &self.authority
    }

    /// Takes ownership of the inbound update receiver.
    ///
    /// The dynamic-update routing layer consumes this to re-translate snapshots and route typed,
    /// scoped deltas to per-component handles.
    pub fn into_updates(self) -> mpsc::Receiver<ConfigUpdate> {
        self.updates
    }
}
