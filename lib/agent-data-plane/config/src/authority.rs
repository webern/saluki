//! Runtime configuration authority and language selection.

/// Describes where the authoritative runtime configuration comes from.
///
/// The load-bearing lifecycle distinction is [`LocalSnapshot`](RuntimeConfigAuthority::LocalSnapshot)
/// versus [`ConfigStream`](RuntimeConfigAuthority::ConfigStream): a local snapshot implies no
/// long-lived provider attachment, while a stream-backed authority owns whatever provider
/// attachment is required to receive the initial snapshot and future updates.
///
/// "Standalone mode" is not modeled here. It is a legacy bootstrap input that selects an authority;
/// semantically it maps to `LocalSnapshot(DatadogAgent)` with no Agent attachment. The combination
/// "`LocalSnapshot(DatadogAgent)` + Agent attachment" is intentionally not expressible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeConfigAuthority {
    /// Runtime configuration is read once from a local snapshot in the given language.
    LocalSnapshot(RuntimeConfigLanguage),

    /// Runtime configuration is streamed from a long-lived provider.
    ConfigStream(ConfigStreamAuthority),
}

impl RuntimeConfigAuthority {
    /// Returns the primary config language this authority resolves to.
    pub const fn language(&self) -> RuntimeConfigLanguage {
        match self {
            Self::LocalSnapshot(language) => *language,
            Self::ConfigStream(ConfigStreamAuthority::DatadogAgent) => RuntimeConfigLanguage::DatadogAgent,
        }
    }

    /// Returns whether resolving this authority requires a long-lived provider attachment.
    pub const fn requires_attachment(&self) -> bool {
        matches!(self, Self::ConfigStream(_))
    }
}

/// Identifies the primary configuration language being translated into [`SalukiConfiguration`].
///
/// [`SalukiConfiguration`]: crate::saluki::SalukiConfiguration
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
///
/// "Remote Agent" is one *provider* of a stream, not the authority concept itself; today the
/// Datadog Agent is the only stream provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigStreamAuthority {
    /// Datadog Agent config stream.
    DatadogAgent,
}
