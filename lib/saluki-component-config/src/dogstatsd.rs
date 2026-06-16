//! Native configuration for the DogStatsD pipeline (source + transforms + debug destination).

use std::path::PathBuf;
use std::time::Duration;

use bytesize::ByteSize;
use stringtheory::MetaString;

/// Native configuration for the DogStatsD source.
#[derive(Clone, Debug, PartialEq)]
pub struct DogStatsDConfig {
    /// Per-packet receive buffer size.
    pub buffer_size: usize,

    /// Number of receive buffers.
    pub buffer_count: usize,

    /// UDP listen port (0 disables UDP).
    pub port: u16,

    /// Socket receive buffer size (`SO_RCVBUF`).
    pub socket_receive_buffer_size: Option<usize>,

    /// Unix datagram socket path.
    pub socket_path: Option<MetaString>,

    /// Unix stream socket path.
    pub socket_stream_path: Option<MetaString>,

    /// Whether the series/sketches/events/service-check payload types are enabled.
    pub enabled_payloads: EnabledPayloads,

    /// Whether origin detection is enabled on the source.
    pub origin_detection_enabled: bool,

    /// Whether non-local traffic (binding to all interfaces) is permitted.
    pub non_local_traffic: bool,
}

impl Default for DogStatsDConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            buffer_count: 128,
            port: 8125,
            socket_receive_buffer_size: None,
            socket_path: None,
            socket_stream_path: None,
            enabled_payloads: EnabledPayloads::default(),
            origin_detection_enabled: false,
            non_local_traffic: false,
        }
    }
}

/// Which payload categories the DogStatsD source emits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnabledPayloads {
    /// Series metrics.
    pub series: bool,
    /// Sketch metrics.
    pub sketches: bool,
    /// Events.
    pub events: bool,
    /// Service checks.
    pub service_checks: bool,
}

impl Default for EnabledPayloads {
    fn default() -> Self {
        Self {
            series: true,
            sketches: true,
            events: true,
            service_checks: true,
        }
    }
}

/// Native configuration for the metrics aggregation transform.
#[derive(Clone, Debug, PartialEq)]
pub struct AggregateConfig {
    /// Aggregation window duration.
    pub window_duration: Duration,

    /// Primary flush interval.
    pub primary_flush_interval: Duration,

    /// Maximum number of contexts held in the aggregator.
    pub context_limit: usize,

    /// Whether incomplete windows are flushed on shutdown.
    pub flush_open_windows: bool,

    /// Idle keep-alive duration for counters before they expire.
    pub counter_expiry: Option<Duration>,

    /// Whether timestamped metrics bypass aggregation.
    pub passthrough_timestamped_metrics: bool,

    /// Idle flush timeout for the passthrough path.
    pub passthrough_idle_flush_timeout: Duration,

    /// Histogram aggregation configuration.
    pub histogram: HistogramConfig,
}

impl Default for AggregateConfig {
    fn default() -> Self {
        Self {
            window_duration: Duration::from_secs(10),
            primary_flush_interval: Duration::from_secs(15),
            context_limit: 1_000_000,
            flush_open_windows: false,
            counter_expiry: Some(Duration::from_secs(300)),
            passthrough_timestamped_metrics: false,
            passthrough_idle_flush_timeout: Duration::from_secs(2),
            histogram: HistogramConfig::default(),
        }
    }
}

/// Native configuration for histogram aggregation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistogramConfig {
    /// Aggregates emitted for each histogram (for example `max`, `median`, `avg`, `count`).
    pub statistics: Vec<MetaString>,

    /// Whether histogram samples are also copied to a distribution metric.
    pub copy_to_distribution: bool,

    /// Prefix applied to copied distribution metrics.
    pub copy_to_distribution_prefix: MetaString,
}

impl Default for HistogramConfig {
    fn default() -> Self {
        Self {
            statistics: vec![
                MetaString::from_static("max"),
                MetaString::from_static("median"),
                MetaString::from_static("avg"),
                MetaString::from_static("count"),
            ],
            copy_to_distribution: false,
            copy_to_distribution_prefix: MetaString::empty(),
        }
    }
}

/// Native configuration for the DogStatsD mapper transform.
#[derive(Clone, Debug, PartialEq)]
pub struct DogStatsDMapperConfig {
    /// Size of the string interner used while mapping.
    pub context_string_interner_bytes: ByteSize,

    /// Mapper result cache size.
    pub cache_size: usize,

    /// Mapper profiles. The detailed profile shape is summarized for the spike.
    pub profiles: Vec<MapperProfile>,
}

impl Default for DogStatsDMapperConfig {
    fn default() -> Self {
        Self {
            context_string_interner_bytes: ByteSize::kib(256),
            cache_size: 1000,
            profiles: Vec::new(),
        }
    }
}

/// A single DogStatsD mapper profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapperProfile {
    /// Profile name.
    pub name: MetaString,

    /// Metric-name prefix the profile applies to.
    pub prefix: MetaString,
}

/// Native configuration for the DogStatsD prefix/blocklist filter transform.
///
/// The lists themselves are runtime-updatable; this is the initial snapshot. Updates flow through a
/// typed, scoped update handle owned by the configuration system rather than a string-key watcher.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PrefixFilterConfig {
    /// Metric-name allowlist.
    pub metric_filterlist: Vec<MetaString>,

    /// Whether the allowlist matches by prefix.
    pub metric_filterlist_match_prefix: bool,

    /// Metric-name blocklist.
    pub metric_blocklist: Vec<MetaString>,

    /// Whether the blocklist matches by prefix.
    pub metric_blocklist_match_prefix: bool,
}

/// Native configuration for the metric tag filterlist transform.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TagFilterlistConfig {
    /// Per-metric tag filter entries. The detailed entry shape is summarized for the spike.
    pub entries: Vec<TagFilterEntry>,
}

/// A single tag-filterlist entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagFilterEntry {
    /// Metric name (or prefix) the entry applies to.
    pub name: MetaString,

    /// Tag keys retained for the matched metric.
    pub allowed_tags: Vec<MetaString>,
}

/// Native configuration for the DogStatsD debug-log destination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DogStatsDDebugLogConfig {
    /// Whether metrics-stats capture is enabled (runtime-updatable initial snapshot).
    pub metrics_stats_enabled: bool,

    /// Whether debug logging is enabled.
    pub logging_enabled: bool,

    /// Debug log file path.
    pub log_file: PathBuf,

    /// Maximum debug log file size.
    pub log_file_max_size: ByteSize,

    /// Maximum number of debug log file rolls.
    pub log_file_max_rolls: usize,
}
