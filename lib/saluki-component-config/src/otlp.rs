//! Native configuration for the OTLP source / proxy.

use bytesize::ByteSize;
use stringtheory::MetaString;

/// Native configuration for the OTLP ingest path.
#[derive(Clone, Debug, PartialEq)]
pub struct OtlpConfig {
    /// Whether OTLP metrics ingest is enabled.
    pub metrics_enabled: bool,

    /// Whether OTLP logs ingest is enabled.
    pub logs_enabled: bool,

    /// Whether OTLP traces ingest is enabled.
    pub traces_enabled: bool,

    /// gRPC receiver endpoint (host:port), if the gRPC receiver is enabled.
    pub grpc_endpoint: Option<MetaString>,

    /// HTTP receiver endpoint (host:port), if the HTTP receiver is enabled.
    pub http_endpoint: Option<MetaString>,

    /// Size of the string interner used for resolving metric contexts.
    pub context_string_interner_bytes: ByteSize,

    /// Maximum number of cached contexts.
    pub cached_contexts_limit: usize,

    /// Maximum number of cached tagsets.
    pub cached_tagsets_limit: usize,

    /// Whether heap allocations are permitted when the interner is full.
    pub allow_context_heap_allocations: bool,

    /// Proxy mode configuration, present when OTLP is proxied to the Core Agent rather than handled
    /// natively.
    pub proxy: Option<OtlpProxyConfig>,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: false,
            logs_enabled: false,
            traces_enabled: false,
            grpc_endpoint: None,
            http_endpoint: None,
            context_string_interner_bytes: ByteSize::mib(2),
            cached_contexts_limit: 500_000,
            cached_tagsets_limit: 500_000,
            allow_context_heap_allocations: true,
            proxy: None,
        }
    }
}

/// Native configuration for OTLP proxy mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtlpProxyConfig {
    /// Core Agent OTLP gRPC endpoint that proxied payloads are forwarded to.
    pub core_agent_otlp_grpc_endpoint: MetaString,

    /// Whether metrics are proxied.
    pub proxy_metrics: bool,

    /// Whether logs are proxied.
    pub proxy_logs: bool,

    /// Whether traces are proxied.
    pub proxy_traces: bool,

    /// Internal port used by the Core Agent traces intake.
    pub core_agent_traces_internal_port: u16,
}
