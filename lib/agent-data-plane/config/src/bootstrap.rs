//! Bootstrap configuration model.
//!
//! Bootstrap is a lifecycle phase, not a trust domain: it can hold Datadog-domain and Saluki-domain
//! keys. It is the typed, allowlisted configuration ADP must read *before* it can resolve a runtime
//! authority — stand up logging and metrics, and (for a stream-backed authority) connect to the
//! Core Agent. It sits outside the translation facade and is consumed directly by startup; it does
//! not flow through the translator into [`SalukiConfiguration`](crate::saluki::SalukiConfiguration).

use std::path::PathBuf;
use std::time::Duration;

use saluki_io::net::ListenAddress;

use crate::authority::RuntimeConfigAuthority;
use crate::logging::RuntimeLoggingConfig;

/// Typed pre-authority configuration.
#[derive(Clone, Debug)]
pub struct BootstrapConfiguration {
    /// Startup decisions that determine how runtime configuration is resolved.
    pub startup: BootstrapStartupConfiguration,

    /// Logging configuration used to stand up logging before the authoritative config arrives.
    ///
    /// `log_level` is dual-lifecycle: it is read here to bring logging up early, and again
    /// authoritatively at runtime (logging reloads after the snapshot arrives).
    pub logging: RuntimeLoggingConfig,

    /// Early process telemetry configuration.
    pub telemetry: BootstrapTelemetryConfiguration,

    /// Datadog Agent IPC connection parameters, present only when a stream-backed authority was
    /// selected and ADP must connect to the Agent.
    pub ipc: Option<BootstrapIpcConfiguration>,
}

/// Startup decisions that determine how runtime configuration is resolved.
#[derive(Clone, Debug)]
pub struct BootstrapStartupConfiguration {
    /// The selected runtime configuration authority.
    pub runtime_config_authority: RuntimeConfigAuthority,

    /// Privileged (secure) control API listen address. Needed at bootstrap because it is published
    /// to the Agent during remote-agent registration.
    pub secure_api_listen_address: ListenAddress,
}

/// Early telemetry settings needed before runtime configuration is online.
///
/// Open question (carried from the design): whether fixed early defaults can serve until
/// [`SalukiConfiguration`](crate::saluki::SalukiConfiguration) is online, which would let telemetry
/// be dropped from bootstrap entirely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapTelemetryConfiguration {
    /// Metrics verbosity level read before the authoritative config exists.
    pub metrics_level: Option<String>,
}

/// Native, pre-authority Datadog Agent IPC connection parameters.
///
/// This is a source-agnostic shape; the configuration system maps it onto the Datadog Agent IPC
/// client configuration when establishing the connection. Keeping it native here means
/// `agent-data-plane-config` does not depend on the Datadog Agent commons crate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapIpcConfiguration {
    /// gRPC endpoint URI for the Agent IPC service.
    pub ipc_endpoint: Option<String>,

    /// Command port used to derive a localhost endpoint when `ipc_endpoint` is unset.
    pub cmd_port: Option<u16>,

    /// Path to the IPC auth token file.
    pub auth_token_file_path: PathBuf,

    /// Path to the IPC certificate file, if overridden.
    pub ipc_cert_file_path: Option<PathBuf>,

    /// Number of connection retry attempts.
    pub connect_retry_attempts: usize,

    /// Backoff between connection retries.
    pub connect_retry_backoff: Duration,

    /// Maximum inbound gRPC message size.
    pub grpc_max_message_size: usize,

    /// vsock CID, when the IPC transport runs over vsock (Linux only).
    pub vsock_cid: Option<u32>,
}
