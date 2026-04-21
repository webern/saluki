# DogStatsD Configuration

Agent Data Plane is designed to be a transparent replacement for the Datadog Agent's DogStatsD
implementation. In most deployments, your existing `datadog.yaml` configuration works without
changes. This page documents the exceptions: settings that are not yet supported, behave
differently, or have no equivalent in the core agent.

<!-- section:unsupported-in-progress -->
## Settings Being Implemented

The following settings are not yet supported in ADP. A tracking issue exists for each.

| Config Key | Description | Issue |
|---|---|---|
| `allow_arbitrary_tags` | Allow arbitrary tag values | [#1377] |
| `bind_host` | Global listen host fallback | [#1331] |
| `cri_connection_timeout` | CRI runtime connection timeout | [#1348] |
| `cri_query_timeout` | CRI runtime query timeout | [#1348] |
| `dogstatsd_capture_depth` | Traffic capture channel depth | [#1381] |
| `dogstatsd_capture_path` | Traffic capture file location | [#1381] |
| `dogstatsd_eol_required` | Require newline-terminated msgs | [#1339] |
| `dogstatsd_log_file` | DSD dedicated log file path | [#1356] |
| `dogstatsd_log_file_max_rolls` | DSD log file max roll count | [#1356] |
| `dogstatsd_log_file_max_size` | DSD log file max size | [#1356] |
| `dogstatsd_logging_enabled` | Enables DSD metric logging | [#1356] |
| `dogstatsd_pipe_name` | Windows named pipe path |  |
| `dogstatsd_so_rcvbuf` | Socket receive buffer size | [#1341] |
| `dogstatsd_stream_log_too_big` | Log oversized stream messages | [#1342] |
| `extra_tags` | Additional static tags | [#1332] |
| `forwarder_http_protocol` | HTTP version (auto/http1) | [#1361] |
| `forwarder_outdated_file_in_days` | Retry file retention (days) | [#1360] |
| `log_format_rfc3339` | Use RFC3339 timestamp format | [#1373] |
| `log_to_syslog` | Log to syslog daemon | [#1337] |
| `logging_frequency` | Transaction success log interval | [#1380] |
| `metric_tag_filterlist` | Per-metric tag include/exclude |  |
| `min_tls_version` | Minimum TLS version for HTTPS | [#1370] |
| `serializer_experimental_use_v3_api.compression_level` | V3 API compression level |  |
| `serializer_experimental_use_v3_api.series.endpoints` | V3 API series endpoint list |  |
| `serializer_experimental_use_v3_api.series.validate` | V3 API series validation flag |  |
| `serializer_experimental_use_v3_api.sketches.endpoints` | V3 API sketches endpoints |  |
| `serializer_experimental_use_v3_api.sketches.validate` | V3 API sketches validation |  |
| `sslkeylogfile` | TLS key log file path | [#1372] |
| `tls_handshake_timeout` | HTTP TLS handshake timeout | [#178] |

<!-- section:unsupported-not-planned -->
## Unsupported Settings

The following settings have no effect in ADP and are not planned for implementation.

| Config Key | Description | Reason |
|---|---|---|
| `aggregator_buffer_size` | Aggregator input channel size | ADP topology is fixed-size |
| `aggregator_flush_metrics_and_serialize_in_parallel_buffer_size` | Parallel flush buffer size | ADP topology is fixed-size |
| `aggregator_flush_metrics_and_serialize_in_parallel_chan_size` | Parallel flush channel size | ADP topology is fixed-size |
| `aggregator_stop_timeout` | Aggregator shutdown timeout (s) | ADP handles shutdown at topology level |
| `aggregator_use_tags_store` | Use tags store for dedup | no ADP equivalent |
| `dogstatsd_experimental_http.enabled` | Enable HTTP DSD listener | experimental, not yet supported |
| `dogstatsd_experimental_http.listen_address` | HTTP DSD listener bind address | experimental, not yet supported |
| `dogstatsd_host_socket_path` | Host UDS socket dir for DSD | admission controller only |
| `dogstatsd_mapper_cache_size` | Mapper result LRU cache size | ADP mapper has no LRU cache |
| `dogstatsd_no_aggregation_pipeline_batch_size` | No-agg pipeline batch size | ADP uses time-based flushing |
| `dogstatsd_packet_buffer_flush_timeout` | Packet buffer flush timeout | ADP decodes inline |
| `dogstatsd_packet_buffer_size` | Datagrams per packet buffer | ADP decodes inline |
| `dogstatsd_pipeline_autoadjust` | Auto-adjust pipeline workers | ADP uses async I/O |
| `dogstatsd_pipeline_count` | Parallel processing pipelines | ADP uses async I/O |
| `dogstatsd_queue_size` | Packet channel buffer size | ADP uses async I/O |
| `dogstatsd_telemetry_enabled_listener_id` | Per-listener telemetry tagging | not supported |
| `dogstatsd_windows_pipe_security_descriptor` | Windows named pipe ACL descriptor | Windows only |
| `dogstatsd_workers_count` | Num DSD processing workers | ADP uses async I/O |
| `statsd_forward_host` | Host for packet forwarding | not supported |
| `statsd_forward_port` | Port for packet forwarding | not supported |

<!-- section:behavioral-differences -->
## Behavioral Differences

The following settings are supported but behave differently than the core agent.

| Config Key | Description | Agent Behavior | ADP Behavior |
|---|---|---|---|
| `dogstatsd_context_expiry_seconds` | Context cache TTL (seconds) | default 20s, configurable | 30s hardcoded |
| `dogstatsd_flush_incomplete_buckets` | Flush open buckets on shutdown | key: dogstatsd_flush_incomplete_buckets | key: aggregate_flush_open_windows |
| `dogstatsd_mem_based_rate_limiter` | Memory rate limiter group key |  |  |
| `dogstatsd_mem_based_rate_limiter.enabled` | Enable memory rate limiter | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.go_gc` | GC percentage for rate limiter | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.high_soft_limit` | High memory soft limit ratio | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.low_soft_limit` | Low memory soft limit ratio | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.memory_ballast` | GC ballast allocation size | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.rate_check.factor` | Rate check geometric factor | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.rate_check.max` | Rate check max interval | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.rate_check.min` | Rate check min interval | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.factor` | OS free-mem check backoff factor | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.max` | OS free-mem check max interval | Go GC-based throttling | not supported |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.min` | OS free-mem check min interval | Go GC-based throttling | not supported |
| `dogstatsd_metrics_stats_enable` | Enable per-metric debug stats | always-on per-metric debug stats | on-demand via privileged API |
| `dogstatsd_stats_buffer` | Internal stats buffer size | always-on expvar stats | on-demand via privileged API |
| `dogstatsd_stats_enable` | Enable internal stats endpoint | always-on expvar stats | on-demand via privileged API |
| `dogstatsd_stats_port` | Internal stats endpoint port | always-on expvar stats | on-demand via privileged API |
| `enable_payloads_events` | Allow sending event payloads | key: enable_payloads.events | key: enable_payloads_events |
| `enable_payloads_series` | Allow sending series payloads | key: enable_payloads.series | key: enable_payloads_series |
| `enable_payloads_service_checks` | Allow sending svc check payloads | key: enable_payloads.service_checks | key: enable_payloads_service_checks |
| `enable_payloads_sketches` | Allow sending sketch payloads | key: enable_payloads.sketches | key: enable_payloads_sketches |
| `env` | Agent environment name | defaults to empty string | defaults to "none" |
| `serializer_zstd_compressor_level` | Zstd compression level | default level 1 | default level 3 |
| `statsd_metric_namespace_blacklist` | Prefixes exempt from namespace | blacklist key supported | only blocklist key supported |
| `telemetry.enabled` | Global telemetry toggle | enables agent telemetry endpoint | use data_plane.telemetry_enabled |

### `dogstatsd_mem_based_rate_limiter.*`

These settings control a Go runtime GC-based mechanism that throttles UDS packet reads when
process memory is high. The implementation relies on Go-specific APIs (`runtime.GC()`,
`debug.SetGCPercent()`) with no equivalent in Rust. ADP uses a different memory bounding
approach via `memory_limit` and `memory_slop_factor`. These keys have no effect in ADP.

### `dogstatsd_stats_enable` / `dogstatsd_stats_port` / `dogstatsd_stats_buffer` / `dogstatsd_metrics_stats_enable`

The core agent exposes DogStatsD throughput and per-metric debug statistics via always-on expvar
endpoints. ADP provides equivalent functionality on demand through its privileged API, with no
persistent endpoint or config toggle required. See [#1352] for details.

### `dogstatsd_flush_incomplete_buckets`

ADP implements this behavior under a different key name: `aggregate_flush_open_windows`. Both
default to `false`. Setting `dogstatsd_flush_incomplete_buckets` in your config has no effect;
use `aggregate_flush_open_windows` instead. See [#1366].

### `enable_payloads_events` / `enable_payloads_series` / `enable_payloads_service_checks` / `enable_payloads_sketches`

The core agent uses dot notation (`enable_payloads.events`); ADP uses underscore notation
(`enable_payloads_events`). The behavior is identical. If you set these keys using dots, they
will have no effect in ADP. See [#1366].

### `telemetry.enabled`

The core agent uses `telemetry.enabled` to expose its internal metrics endpoint. ADP uses
`data_plane.telemetry_enabled` instead. Setting `telemetry.enabled` has no effect on ADP's
telemetry endpoint. See [#1338].

<!-- section:compatibility-unknown -->
## Compatibility Unknown

The following settings have not been fully verified in ADP. Behavior may match the core agent,
differ, or be unsupported. Check the linked issue for current status.

| Config Key | Description | Issue |
|---|---|---|
| `dogstatsd_disable_verbose_logs` | Suppress noisy parse error logs | [#1350] |
| `dogstatsd_origin_detection_client` | Honor client origin proto fields |  |
| `forwarder_apikey_validation_interval` | API key check interval (mins) | [#1357] |
| `forwarder_flush_to_disk_mem_ratio` | Mem-to-disk flush threshold | [#1364] |
| `forwarder_high_prio_buffer_size` | High-priority request queue size | [#1362] |
| `forwarder_low_prio_buffer_size` | Low-priority request queue size | [#1362] |
| `forwarder_max_concurrent_requests` | Max concurrent HTTP requests | [#1363] |
| `forwarder_retry_queue_capacity_time_interval_sec` | Retry queue time-based capacity | [#1365] |
| `serializer_max_payload_size` | Max compressed payload size | [#1354] |
| `serializer_max_series_payload_size` | Max series compressed size | [#1354] |
| `serializer_max_series_points_per_payload` | Max series points per payload | [#1354] |
| `serializer_max_series_uncompressed_payload_size` | Max series uncompressed size | [#1354] |
| `serializer_max_uncompressed_payload_size` | Max uncompressed payload size | [#1354] |
| `skip_ssl_validation` | Skip TLS cert validation | [#1371] |
| `statsd_metric_blocklist` | Metric name blocklist |  |
| `statsd_metric_blocklist_match_prefix` | Blocklist matches by prefix |  |
| `statsd_metric_namespace` | Prefix prepended to all metrics |  |
| `tags` | Global tags (DD_TAGS) | [#1333] |
| `use_dogstatsd` | Master DogStatsD enable toggle | [#1334] |

### `use_dogstatsd`

`use_dogstatsd` is the core agent's master toggle for DogStatsD. ADP uses
`data_plane.dogstatsd.enabled` instead. It is not yet confirmed whether the core agent
propagates `use_dogstatsd: false` to ADP, or whether ADP must handle it directly. If you
disable DogStatsD via `use_dogstatsd`, ADP may continue to run its DogStatsD pipeline.
See [#1334].

### `tags`

The `tags` setting (`DD_TAGS`) adds global tags to all submitted data. It is not yet confirmed
whether ADP receives these tags from the core agent's tagger or needs to apply them
independently. See [#1333].

<!-- section:adp-only -->
## ADP-Only Settings

The following settings exist in ADP but have no equivalent in the core agent. They provide
additional tuning options specific to ADP's architecture.

| Config Key | Description | Default |
|---|---|---|
| `agent_ipc_endpoint` | Remote agent IPC URI |  |
| `aggregate_flush_interval` | Aggregator flush period |  |
| `aggregate_flush_open_windows` | Flush open windows on stop |  |
| `aggregate_passthrough_idle_flush_timeout` | Passthrough buffer flush delay |  |
| `aggregate_window_duration` | Aggregation window size |  |
| `connect_retry_attempts` | IPC client connect retries |  |
| `connect_retry_backoff` | IPC client retry delay |  |
| `counter_expiry_seconds` | Idle counter keep-alive duration |  |
| `data_plane.api_listen_address` | ADP unprivileged API address |  |
| `data_plane.remote_agent_enabled` | Register as remote agent |  |
| `data_plane.secure_api_listen_address` | ADP privileged API address |  |
| `data_plane.standalone_mode` | ADP standalone mode toggle |  |
| `data_plane.use_new_config_stream_endpoint` | Use new config stream endpoint |  |
| `dogstatsd_allow_context_heap_allocs` | Allow heap allocs for contexts |  |
| `dogstatsd_buffer_count` | Number of receive buffers |  |
| `dogstatsd_cached_contexts_limit` | Max cached metric contexts |  |
| `dogstatsd_cached_tagsets_limit` | Max cached tagsets |  |
| `dogstatsd_mapper_string_interner_size` | Mapper string interner capacity |  |
| `dogstatsd_minimum_sample_rate` | Floor for metric sample rates |  |
| `dogstatsd_permissive_decoding` | Relaxes decoder strictness |  |
| `dogstatsd_tcp_port` | TCP listen port for DSD |  |
| `enable_global_limiter` | Toggle global memory limiter |  |
| `flush_timeout_secs` | Encoder flush timeout (secs) |  |
| `memory_limit` | Process memory limit (bytes) |  |
| `memory_slop_factor` | Memory headroom fraction |  |
| `otlp_string_interner_size` | OTLP context interner capacity |  |
| `remote_agent_string_interner_size_bytes` | Tag string interner capacity |  |
| `serializer_max_metrics_per_payload` | Max metrics per payload |  |
| `statsd_metric_namespace_blocklist` | Alias (unused) for blacklist key |  |

### `dogstatsd_tcp_port`

ADP supports a TCP listener for DogStatsD in addition to UDP and UDS. Set to a non-zero port
to enable it. The core agent does not support TCP DogStatsD.

<!-- section:reference -->
## Configuration Reference

Complete list of DogStatsD-relevant configuration keys and their status in ADP.

| Config Key | Description | Status | Notes |
|---|---|---|---|
| `additional_endpoints` | Dual-ship to extra endpoints | Implemented |  |
| `agent_ipc_endpoint` | Remote agent IPC URI | ADP Only |  |
| `aggregate_context_limit` | Max contexts per agg window | Implemented |  |
| `aggregate_flush_interval` | Aggregator flush period | ADP Only |  |
| `aggregate_flush_open_windows` | Flush open windows on stop | ADP Only |  |
| `aggregate_passthrough_idle_flush_timeout` | Passthrough buffer flush delay | ADP Only |  |
| `aggregate_window_duration` | Aggregation window size | ADP Only |  |
| `aggregator_buffer_size` | Aggregator input channel size | Not Applicable |  |
| `aggregator_flush_metrics_and_serialize_in_parallel_buffer_size` | Parallel flush buffer size | Not Applicable |  |
| `aggregator_flush_metrics_and_serialize_in_parallel_chan_size` | Parallel flush channel size | Not Applicable |  |
| `aggregator_stop_timeout` | Aggregator shutdown timeout (s) | Not Applicable |  |
| `aggregator_use_tags_store` | Use tags store for dedup | Not Applicable |  |
| `allow_arbitrary_tags` | Allow arbitrary tag values | Missing | #1377 |
| `api_key` | API key for endpoint auth | Implemented |  |
| `auth_token_file_path` | IPC auth token file path | Implemented |  |
| `bind_host` | Global listen host fallback | Missing | #1331 |
| `connect_retry_attempts` | IPC client connect retries | ADP Only |  |
| `connect_retry_backoff` | IPC client retry delay | ADP Only |  |
| `container_cgroup_root` | Cgroup filesystem root path | Implemented |  |
| `container_proc_root` | Procfs root path for containers | Implemented |  |
| `counter_expiry_seconds` | Idle counter keep-alive duration | ADP Only |  |
| `cri_connection_timeout` | CRI runtime connection timeout | Missing | #1348 |
| `cri_query_timeout` | CRI runtime query timeout | Missing | #1348 |
| `cri_socket_path` | CRI/containerd socket path | Implemented |  |
| `data_plane.api_listen_address` | ADP unprivileged API address | ADP Only |  |
| `data_plane.dogstatsd.enabled` | Enable DSD in data plane | Implemented |  |
| `data_plane.enabled` | Enable ADP globally | Implemented |  |
| `data_plane.remote_agent_enabled` | Register as remote agent | ADP Only |  |
| `data_plane.secure_api_listen_address` | ADP privileged API address | ADP Only |  |
| `data_plane.standalone_mode` | ADP standalone mode toggle | ADP Only |  |
| `data_plane.use_new_config_stream_endpoint` | Use new config stream endpoint | ADP Only |  |
| `dd_url` | Override intake endpoint URL | Implemented |  |
| `dogstatsd_allow_context_heap_allocs` | Allow heap allocs for contexts | ADP Only |  |
| `dogstatsd_buffer_count` | Number of receive buffers | ADP Only |  |
| `dogstatsd_buffer_size` | Receive buffer size (bytes) | Implemented |  |
| `dogstatsd_cached_contexts_limit` | Max cached metric contexts | ADP Only |  |
| `dogstatsd_cached_tagsets_limit` | Max cached tagsets | ADP Only |  |
| `dogstatsd_capture_depth` | Traffic capture channel depth | Missing | #1381 |
| `dogstatsd_capture_path` | Traffic capture file location | Missing | #1381 |
| `dogstatsd_context_expiry_seconds` | Context cache TTL (seconds) | Divergent | #1340 |
| `dogstatsd_disable_verbose_logs` | Suppress noisy parse error logs | Missing |  |
| `dogstatsd_entity_id_precedence` | Entity ID over auto-detection | Implemented |  |
| `dogstatsd_eol_required` | Require newline-terminated msgs | Missing | #1339 |
| `dogstatsd_experimental_http.enabled` | Enable HTTP DSD listener | Not Applicable |  |
| `dogstatsd_experimental_http.listen_address` | HTTP DSD listener bind address | Not Applicable |  |
| `dogstatsd_expiry_seconds` | Counter zero-value TTL (secs) | Implemented |  |
| `dogstatsd_flush_incomplete_buckets` | Flush open buckets on shutdown | Divergent | ADP key: aggregate_flush_ope ... |
| `dogstatsd_host_socket_path` | Host UDS socket dir for DSD | Not Applicable |  |
| `dogstatsd_log_file` | DSD dedicated log file path | Missing | #1356 |
| `dogstatsd_log_file_max_rolls` | DSD log file max roll count | Missing | #1356 |
| `dogstatsd_log_file_max_size` | DSD log file max size | Missing | #1356 |
| `dogstatsd_logging_enabled` | Enables DSD metric logging | Missing | #1356 |
| `dogstatsd_mapper_cache_size` | Mapper result LRU cache size | Not Applicable |  |
| `dogstatsd_mapper_profiles` | Metric mapping profile defs | Implemented |  |
| `dogstatsd_mapper_profiles.mappings` | Mapper mapping rules list | Implemented |  |
| `dogstatsd_mapper_profiles.mappings.match_type` | Mapper match type (regex/wc) | Implemented |  |
| `dogstatsd_mapper_profiles.mappings.name` | Mapper output metric name | Implemented |  |
| `dogstatsd_mapper_profiles.mappings.tags` | Mapper output tags map | Implemented |  |
| `dogstatsd_mapper_profiles.mappings.tags.task_name` | Mapper tag extraction rule | Implemented |  |
| `dogstatsd_mapper_profiles.mappings.tags.task_type` | Mapper tag extraction rule | Implemented |  |
| `dogstatsd_mapper_profiles.mappings.tags.worker_name` | Mapper tag value expression | Implemented |  |
| `dogstatsd_mapper_profiles.mappings.tags.worker_type` | Mapper tag value expression | Implemented |  |
| `dogstatsd_mapper_profiles.prefix` | Mapper profile prefix filter | Implemented |  |
| `dogstatsd_mapper_string_interner_size` | Mapper string interner capacity | ADP Only |  |
| `dogstatsd_mem_based_rate_limiter` | Memory rate limiter group key | Missing |  |
| `dogstatsd_mem_based_rate_limiter.enabled` | Enable memory rate limiter | Missing |  |
| `dogstatsd_mem_based_rate_limiter.go_gc` | GC percentage for rate limiter | Missing |  |
| `dogstatsd_mem_based_rate_limiter.high_soft_limit` | High memory soft limit ratio | Missing |  |
| `dogstatsd_mem_based_rate_limiter.low_soft_limit` | Low memory soft limit ratio | Missing |  |
| `dogstatsd_mem_based_rate_limiter.memory_ballast` | GC ballast allocation size | Missing |  |
| `dogstatsd_mem_based_rate_limiter.rate_check.factor` | Rate check geometric factor | Missing |  |
| `dogstatsd_mem_based_rate_limiter.rate_check.max` | Rate check max interval | Missing |  |
| `dogstatsd_mem_based_rate_limiter.rate_check.min` | Rate check min interval | Missing |  |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.factor` | OS free-mem check backoff factor | Missing |  |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.max` | OS free-mem check max interval | Missing |  |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.min` | OS free-mem check min interval | Missing |  |
| `dogstatsd_metrics_stats_enable` | Enable per-metric debug stats | Missing |  |
| `dogstatsd_minimum_sample_rate` | Floor for metric sample rates | ADP Only |  |
| `dogstatsd_no_aggregation_pipeline` | Enable no-agg timestamped path | Implemented |  |
| `dogstatsd_no_aggregation_pipeline_batch_size` | No-agg pipeline batch size | Not Applicable |  |
| `dogstatsd_non_local_traffic` | Accept non-localhost UDP/TCP | Implemented |  |
| `dogstatsd_origin_detection` | Enable UDS origin detection | Implemented |  |
| `dogstatsd_origin_detection_client` | Honor client origin proto fields | Divergent |  |
| `dogstatsd_origin_optout_enabled` | Allow clients to opt out origin | Implemented |  |
| `dogstatsd_packet_buffer_flush_timeout` | Packet buffer flush timeout | Not Applicable |  |
| `dogstatsd_packet_buffer_size` | Datagrams per packet buffer | Not Applicable |  |
| `dogstatsd_permissive_decoding` | Relaxes decoder strictness | ADP Only |  |
| `dogstatsd_pipe_name` | Windows named pipe path | Missing |  |
| `dogstatsd_pipeline_autoadjust` | Auto-adjust pipeline workers | Not Applicable |  |
| `dogstatsd_pipeline_count` | Parallel processing pipelines | Not Applicable |  |
| `dogstatsd_port` | UDP listen port | Implemented |  |
| `dogstatsd_queue_size` | Packet channel buffer size | Not Applicable |  |
| `dogstatsd_so_rcvbuf` | Socket receive buffer size | Missing | #1341 |
| `dogstatsd_socket` | UDS datagram socket path | Implemented |  |
| `dogstatsd_stats_buffer` | Internal stats buffer size | Missing |  |
| `dogstatsd_stats_enable` | Enable internal stats endpoint | Missing |  |
| `dogstatsd_stats_port` | Internal stats endpoint port | Missing |  |
| `dogstatsd_stream_log_too_big` | Log oversized stream messages | Missing | #1342 |
| `dogstatsd_stream_socket` | UDS stream socket path | Implemented |  |
| `dogstatsd_string_interner_size` | String interner capacity | Implemented |  |
| `dogstatsd_tag_cardinality` | Default tag cardinality level | Implemented |  |
| `dogstatsd_tags` | Extra tags added to all DSD data | Implemented |  |
| `dogstatsd_tcp_port` | TCP listen port for DSD | ADP Only |  |
| `dogstatsd_telemetry_enabled_listener_id` | Per-listener telemetry tagging | Missing |  |
| `dogstatsd_windows_pipe_security_descriptor` | Windows named pipe ACL descriptor | Missing |  |
| `dogstatsd_workers_count` | Num DSD processing workers | Not Applicable |  |
| `enable_global_limiter` | Toggle global memory limiter | ADP Only |  |
| `enable_payloads_events` | Allow sending event payloads | Divergent | ADP key: enable_payloads_eve ... |
| `enable_payloads_series` | Allow sending series payloads | Divergent | ADP key: enable_payloads_ser ... |
| `enable_payloads_service_checks` | Allow sending svc check payloads | Divergent | ADP key: enable_payloads_ser ... |
| `enable_payloads_sketches` | Allow sending sketch payloads | Divergent | ADP key: enable_payloads_ske ... |
| `env` | Agent environment name | Divergent |  |
| `expected_tags_duration` | Host tag enrichment duration | Implemented |  |
| `extra_tags` | Additional static tags | Missing | #1332 |
| `flush_timeout_secs` | Encoder flush timeout (secs) | ADP Only |  |
| `forwarder_apikey_validation_interval` | API key check interval (mins) | Missing |  |
| `forwarder_backoff_base` | Retry backoff base (secs) | Implemented |  |
| `forwarder_backoff_factor` | Retry backoff jitter factor | Implemented |  |
| `forwarder_backoff_max` | Retry backoff ceiling (secs) | Implemented |  |
| `forwarder_connection_reset_interval` | HTTP conn reset interval (secs) | Implemented |  |
| `forwarder_flush_to_disk_mem_ratio` | Mem-to-disk flush threshold | Missing |  |
| `forwarder_high_prio_buffer_size` | High-priority request queue size | Divergent |  |
| `forwarder_http_protocol` | HTTP version (auto/http1) | Missing | #1361 |
| `forwarder_low_prio_buffer_size` | Low-priority request queue size | Missing |  |
| `forwarder_max_concurrent_requests` | Max concurrent HTTP requests | Missing |  |
| `forwarder_num_workers` | Concurrent forwarder workers | Implemented |  |
| `forwarder_outdated_file_in_days` | Retry file retention (days) | Missing | #1360 |
| `forwarder_recovery_interval` | Backoff recovery decrease factor | Implemented |  |
| `forwarder_recovery_reset` | Reset errors on success | Implemented |  |
| `forwarder_retry_queue_capacity_time_interval_sec` | Retry queue time-based capacity | Missing |  |
| `forwarder_retry_queue_max_size` | Retry queue max size (depr.) | Implemented |  |
| `forwarder_retry_queue_payloads_max_size` | Retry queue max size (bytes) | Implemented |  |
| `forwarder_storage_max_disk_ratio` | Max disk usage ratio for retry | Implemented |  |
| `forwarder_storage_max_size_in_bytes` | Max on-disk retry storage size | Implemented |  |
| `forwarder_storage_path` | On-disk retry storage directory | Implemented |  |
| `forwarder_timeout` | Forwarder HTTP request timeout | Implemented |  |
| `histogram_aggregates` | Histogram aggregate statistics | Implemented |  |
| `histogram_copy_to_distribution` | Copy histograms to distributions | Implemented |  |
| `histogram_copy_to_distribution_prefix` | Prefix for hist-to-dist copies | Implemented |  |
| `histogram_percentiles` | Histogram percentile quantiles | Implemented |  |
| `hostname` | Configured hostname override | Implemented |  |
| `ipc_cert_file_path` | IPC TLS certificate path | Implemented |  |
| `log_file` | Log output file path | Implemented |  |
| `log_file_max_rolls` | Max rotated log files kept | Implemented |  |
| `log_file_max_size` | Max log file size before rotate | Implemented |  |
| `log_format_json` | Use JSON log format | Implemented |  |
| `log_format_rfc3339` | Use RFC3339 timestamp format | Missing | #1373 |
| `log_level` | Log verbosity level | Implemented |  |
| `log_to_console` | Log to stdout/stderr | Implemented |  |
| `log_to_syslog` | Log to syslog daemon | Missing | #1337 |
| `logging_frequency` | Transaction success log interval | Missing | #1380 |
| `memory_limit` | Process memory limit (bytes) | ADP Only |  |
| `memory_slop_factor` | Memory headroom fraction | ADP Only |  |
| `metric_filterlist` | Metric name blocklist | Implemented |  |
| `metric_filterlist_match_prefix` | Blocklist uses prefix matching | Implemented |  |
| `metric_tag_filterlist` | Per-metric tag include/exclude | Missing |  |
| `min_tls_version` | Minimum TLS version for HTTPS | Missing | #1370 |
| `no_proxy_nonexact_match` | Domain/CIDR no_proxy matching | Implemented |  |
| `origin_detection_unified` | Unified origin detection mode | Implemented |  |
| `otlp_string_interner_size` | OTLP context interner capacity | ADP Only |  |
| `proxy` | Top-level proxy config section | Implemented |  |
| `proxy.http` | HTTP proxy URL | Implemented |  |
| `proxy.https` | HTTPS proxy URL | Implemented |  |
| `proxy.no_proxy` | Hosts bypassing proxy | Implemented |  |
| `proxy_http` | HTTP proxy URL | Implemented |  |
| `proxy_https` | HTTPS proxy URL | Implemented |  |
| `proxy_no_proxy` | Proxy bypass host list | Implemented |  |
| `remote_agent_string_interner_size_bytes` | Tag string interner capacity | ADP Only |  |
| `run_path` | Runtime data directory path | Implemented |  |
| `secret_backend_command` | Secret resolver executable path | Implemented |  |
| `secret_backend_timeout` | Secret backend timeout (seconds) | Implemented |  |
| `serializer_compressor_kind` | Payload compression algorithm | Implemented |  |
| `serializer_experimental_use_v3_api.compression_level` | V3 API compression level | Missing |  |
| `serializer_experimental_use_v3_api.series.endpoints` | V3 API series endpoint list | Missing |  |
| `serializer_experimental_use_v3_api.series.validate` | V3 API series validation flag | Missing |  |
| `serializer_experimental_use_v3_api.sketches.endpoints` | V3 API sketches endpoints | Missing |  |
| `serializer_experimental_use_v3_api.sketches.validate` | V3 API sketches validation | Missing |  |
| `serializer_max_metrics_per_payload` | Max metrics per payload | ADP Only |  |
| `serializer_max_payload_size` | Max compressed payload size | Missing |  |
| `serializer_max_series_payload_size` | Max series compressed size | Missing |  |
| `serializer_max_series_points_per_payload` | Max series points per payload | Missing | ADP key: serializer_max_metrics_ |
| `serializer_max_series_uncompressed_payload_size` | Max series uncompressed size | Missing |  |
| `serializer_max_uncompressed_payload_size` | Max uncompressed payload size | Missing |  |
| `serializer_zstd_compressor_level` | Zstd compression level | Divergent |  |
| `site` | Datadog site domain | Implemented |  |
| `skip_ssl_validation` | Skip TLS cert validation | Missing |  |
| `sslkeylogfile` | TLS key log file path | Missing | #1372 |
| `statsd_forward_host` | Host for packet forwarding | Missing |  |
| `statsd_forward_port` | Port for packet forwarding | Missing |  |
| `statsd_metric_blocklist` | Metric name blocklist | Unknown |  |
| `statsd_metric_blocklist_match_prefix` | Blocklist matches by prefix | Unknown |  |
| `statsd_metric_namespace` | Prefix prepended to all metrics | Unknown |  |
| `statsd_metric_namespace_blacklist` | Prefixes exempt from namespace | Divergent | #1353 |
| `statsd_metric_namespace_blocklist` | Alias (unused) for blacklist key | ADP Only |  |
| `tags` | Global tags (DD_TAGS) | Missing |  |
| `telemetry.enabled` | Global telemetry toggle | Divergent | ADP key: data_plane.telemetry_en |
| `tls_handshake_timeout` | HTTP TLS handshake timeout | Missing | #178 |
| `use_dogstatsd` | Master DogStatsD enable toggle | Missing | ADP key: data_plane.dogstatsd.en |
| `use_proxy_for_cloud_metadata` | Proxy cloud metadata endpoints | Implemented |  |


[#178]: https://github.com/DataDog/saluki/issues/178
[#1330]: https://github.com/DataDog/saluki/issues/1330
[#1331]: https://github.com/DataDog/saluki/issues/1331
[#1332]: https://github.com/DataDog/saluki/issues/1332
[#1333]: https://github.com/DataDog/saluki/issues/1333
[#1334]: https://github.com/DataDog/saluki/issues/1334
[#1337]: https://github.com/DataDog/saluki/issues/1337
[#1338]: https://github.com/DataDog/saluki/issues/1338
[#1339]: https://github.com/DataDog/saluki/issues/1339
[#1340]: https://github.com/DataDog/saluki/issues/1340
[#1341]: https://github.com/DataDog/saluki/issues/1341
[#1342]: https://github.com/DataDog/saluki/issues/1342
[#1348]: https://github.com/DataDog/saluki/issues/1348
[#1350]: https://github.com/DataDog/saluki/issues/1350
[#1352]: https://github.com/DataDog/saluki/issues/1352
[#1353]: https://github.com/DataDog/saluki/issues/1353
[#1354]: https://github.com/DataDog/saluki/issues/1354
[#1356]: https://github.com/DataDog/saluki/issues/1356
[#1357]: https://github.com/DataDog/saluki/issues/1357
[#1360]: https://github.com/DataDog/saluki/issues/1360
[#1361]: https://github.com/DataDog/saluki/issues/1361
[#1362]: https://github.com/DataDog/saluki/issues/1362
[#1363]: https://github.com/DataDog/saluki/issues/1363
[#1364]: https://github.com/DataDog/saluki/issues/1364
[#1365]: https://github.com/DataDog/saluki/issues/1365
[#1366]: https://github.com/DataDog/saluki/issues/1366
[#1367]: https://github.com/DataDog/saluki/issues/1367
[#1368]: https://github.com/DataDog/saluki/issues/1368
[#1370]: https://github.com/DataDog/saluki/issues/1370
[#1371]: https://github.com/DataDog/saluki/issues/1371
[#1372]: https://github.com/DataDog/saluki/issues/1372
[#1373]: https://github.com/DataDog/saluki/issues/1373
[#1377]: https://github.com/DataDog/saluki/issues/1377
[#1380]: https://github.com/DataDog/saluki/issues/1380
[#1381]: https://github.com/DataDog/saluki/issues/1381
[#1382]: https://github.com/DataDog/saluki/issues/1382