//! Saluki-native supplemental configuration.

use bytesize::ByteSize;
use saluki_component_config::common::CompressionConfig;

use crate::authority::RuntimeConfigLanguage;

/// Saluki-native configuration that supplements the selected primary config language.
///
/// This is a real typed input the translator consumes alongside the primary config, so that no
/// component reads a private knob from a raw map. It is not a universal fixed key set: a setting is
/// Saluki-private when the selected primary language cannot express it, and that boundary differs by
/// language. The sections here are the ones the Datadog Agent language cannot express.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalukiPrivateConfiguration {
    /// The primary language this private configuration supplements.
    pub primary_language: RuntimeConfigLanguage,

    /// OTLP context-resolution tuning knobs.
    pub otlp: OtlpPrivateConfig,

    /// DogStatsD mapper tuning knobs.
    pub dogstatsd: DogStatsDPrivateConfig,

    /// Outbound payload compression knobs applied across encoders.
    pub compression: CompressionConfig,
}

impl SalukiPrivateConfiguration {
    /// Returns a default private configuration for the given primary language.
    pub fn for_language(primary_language: RuntimeConfigLanguage) -> Self {
        Self {
            primary_language,
            otlp: OtlpPrivateConfig::default(),
            dogstatsd: DogStatsDPrivateConfig::default(),
            compression: CompressionConfig::default(),
        }
    }
}

/// OTLP context-resolution tuning knobs that the Datadog Agent language does not express.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtlpPrivateConfig {
    /// Size of the string interner used for resolving metric contexts.
    pub context_string_interner_bytes: ByteSize,

    /// Maximum number of cached contexts.
    pub cached_contexts_limit: usize,

    /// Maximum number of cached tagsets.
    pub cached_tagsets_limit: usize,

    /// Whether heap allocations are permitted when the interner is full.
    pub allow_context_heap_allocations: bool,
}

impl Default for OtlpPrivateConfig {
    fn default() -> Self {
        Self {
            context_string_interner_bytes: ByteSize::mib(2),
            cached_contexts_limit: 500_000,
            cached_tagsets_limit: 500_000,
            allow_context_heap_allocations: true,
        }
    }
}

/// DogStatsD mapper tuning knobs that the Datadog Agent language does not express as first-class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DogStatsDPrivateConfig {
    /// Size of the string interner used while mapping.
    pub mapper_string_interner_bytes: ByteSize,

    /// Mapper result cache size.
    pub mapper_cache_size: usize,
}

impl Default for DogStatsDPrivateConfig {
    fn default() -> Self {
        Self {
            mapper_string_interner_bytes: ByteSize::kib(256),
            mapper_cache_size: 1000,
        }
    }
}
