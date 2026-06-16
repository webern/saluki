//! Native configuration for the checks IPC source.

use saluki_io::net::ListenAddress;

/// Native configuration for the checks IPC source.
#[derive(Clone, Debug)]
pub struct ChecksConfig {
    /// Listen address for the checks IPC gRPC endpoint.
    pub grpc_endpoint: ListenAddress,
}
