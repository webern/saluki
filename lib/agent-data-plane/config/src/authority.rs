//! Runtime configuration authority and language selection.

/// Describes where the authoritative runtime configuration comes from.
///
/// Local snapshots do not imply any long-lived provider attachment. Stream-backed authorities own
/// whatever provider attachment is required to receive the initial snapshot and future updates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeConfigAuthority {
    /// Runtime configuration is read from a local snapshot.
    LocalSnapshot(RuntimeConfigLanguage),

    /// Runtime configuration is read from a configuration stream.
    ConfigStream(ConfigStreamAuthority),
}

/// Identifies the primary configuration language being translated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeConfigLanguage {
    /// Datadog Agent configuration.
    DatadogAgent,

    /// OpenTelemetry Collector configuration.
    OpenTelemetryCollector,

    /// Observability Pipelines Worker / Vector configuration.
    ObservabilityPipelinesWorker,

    /// Native Saluki configuration.
    SalukiNative,
}

/// Identifies a supported streamed configuration authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigStreamAuthority {
    /// Datadog Agent config stream.
    DatadogAgent,
}
