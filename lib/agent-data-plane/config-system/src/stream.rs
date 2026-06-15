//! Configuration stream handles.

use agent_data_plane_config::ConfigStreamAuthority;

/// Long-lived handle for a streamed runtime configuration authority.
///
/// The handle is source-provider agnostic at the configuration-system boundary. Today the only
/// provider is the Datadog Agent config stream.
#[derive(Clone, Debug)]
pub struct ConfigStreamHandle {
    /// TODO: figure out the actual struct fields needed.
    pub authority: ConfigStreamAuthority,

    /// TODO: figure out the actual struct fields needed.
    pub initial_snapshot_received: bool,
}
