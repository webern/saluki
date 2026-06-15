//! Component-native configuration types.
//!
//! This crate is a leaf. It owns the plain-data configuration structs that ADP components are built
//! from, expressed in native Saluki terms: native field names, native types, no Datadog Agent key
//! names, and no [`saluki_config::GenericConfiguration`](https://docs.rs/saluki-config). Splitting
//! the config *type* from the component *implementation* (which lives in `saluki-components`) lets a
//! translator or topology builder reference a component's configuration without dragging in the
//! component's machinery.
//!
//! These types are an eligible translation target for any source language: a Datadog Agent config
//! translator produces them today, and an OpenTelemetry Collector, Observability Pipelines Worker,
//! or native Saluki config translator could produce them tomorrow. For that reason the types here
//! deliberately do not know *where* their values came from.
//!
//! # Forbidden dependencies
//!
//! This crate must not depend on `datadog-agent-config`, `saluki-config`, or `saluki-components`.
//! Keeping the dependency arrow pointing down toward this leaf is what makes the configuration
//! translation boundary real rather than aspirational.

#![deny(missing_docs)]
#![deny(warnings)]

pub mod checks;
pub mod common;
pub mod dogstatsd;
pub mod events;
pub mod forwarder;
pub mod logs;
pub mod metrics;
pub mod otlp;
pub mod service_checks;
pub mod traces;

pub use checks::ChecksConfig;
pub use common::{CompressionConfig, EndpointConfig, RetryConfig, TlsClientConfig};
pub use dogstatsd::{
    AggregateConfig, DogStatsDConfig, DogStatsDDebugLogConfig, DogStatsDMapperConfig, HistogramConfig,
    PrefixFilterConfig, TagFilterlistConfig,
};
pub use events::DatadogEventsEncoderConfig;
pub use forwarder::{DatadogForwarderConfig, MultiRegionFailoverConfig};
pub use logs::DatadogLogsEncoderConfig;
pub use metrics::{DatadogMetricsEncoderConfig, MetricsEnrichmentConfig};
pub use otlp::{OtlpConfig, OtlpProxyConfig};
pub use service_checks::DatadogServiceChecksEncoderConfig;
pub use traces::{
    ApmStatsEncoderConfig, DatadogTracesEncoderConfig, TraceObfuscationConfig, TraceSamplerConfig,
    TracesEnrichmentConfig,
};
