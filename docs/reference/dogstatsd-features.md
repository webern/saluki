# DogStatsD Feature Parity

This document tracks feature parity between the Datadog Agent's DogStatsD implementation and Agent
Data Plane (ADP). Use it to determine whether ADP supports the DogStatsD features your workload
depends on.

Last updated: 2026-04-06

The `/dogstatsd-audit` skill can automatically inspect and update this, but it is also OK to edit
this by hand.

## Status Legend

| Status          | Meaning                                                                        |
|-----------------|--------------------------------------------------------------------------------|
| **Implemented** | Feature is present in ADP and behaves the same as the Datadog Agent.           |
| **Missing**     | Feature exists in the Datadog Agent but is not yet implemented in ADP.         |
| **Divergent**   | Feature exists in both but behavior differs. See the notes column for details. |
| **ADP Only**    | Feature is unique to ADP and does not exist in the Datadog Agent.              |

## Features

| Config Key | Description | Status | Notes |
|---|---|---|---|
| `additional_endpoints` | Dual-ship to extra endpoints | Implemented |  |
| `agent_ipc_endpoint` | Remote agent IPC URI | ADP Only | Agent uses agent_ipc.host+port |
| `aggregate_context_limit` | Max contexts per agg window | ADP Only |  |
| `aggregate_flush_interval` | Aggregator flush period | ADP Only |  |
| `aggregate_flush_open_windows` | Flush open windows on stop | ADP Only |  |
| `aggregate_passthrough_idle_flush_timeout` | Passthrough buffer flush delay | ADP Only |  |
| `aggregate_window_duration` | Aggregation window size | ADP Only |  |
| `aggregator_buffer_size` | Aggregator input channel size | Missing |  |
| `aggregator_flush_metrics_and_serialize_in_parallel_buffer_size` | Parallel flush buffer size | Missing |  |
| `aggregator_flush_metrics_and_serialize_in_parallel_chan_size` | Parallel flush channel size | Missing |  |
| `aggregator_stop_timeout` | Aggregator shutdown timeout (s) | Missing |  |
| `aggregator_use_tags_store` | Use tags store for dedup | Missing |  |
| `allow_arbitrary_tags` | Allow arbitrary tag values | Missing |  |
| `api_key` | API key for endpoint auth | Implemented |  |
| `auth_token_file_path` | IPC auth token file path | Implemented |  |
| `batch_max_concurrent_send` | Max concurrent log batch sends | Missing | ADP uses forwarder_num_workers |
| `batch_max_content_size` | Max log batch content bytes | Missing |  |
| `batch_max_size` | Max events per log batch | Missing |  |
| `batch_wait` | Log batch flush interval (secs) | Missing |  |
| `bind_host` | Global listen host fallback | Missing | ADP hardcodes 127.0.0.1 |
| `cloud_provider_metadata` | Enabled cloud metadata providers | Missing |  |
| `cluster_name` | Kubernetes cluster name tag | Missing | ADP gets it from Agent via IPC |
| `compression_kind` | Log forwarder compression algo | Missing | ADP uses serializer_compressor_k |
| `compression_level` | Log forwarder gzip level | Missing | ADP uses serializer_zstd_compres |
| `connect_retry_attempts` | IPC client connect retries | ADP Only |  |
| `connect_retry_backoff` | IPC client retry delay | ADP Only |  |
| `connection_reset_interval` | Logs transport conn reset | Missing | Saluki has forwarder_ variant |
| `container_cgroup_root` | Cgroup filesystem root path | Implemented |  |
| `container_env_as_tags` | Map container env vars to tags | Missing |  |
| `container_exclude` | Global container exclusion filte | Missing |  |
| `container_exclude_metrics` | Metrics-specific container exclu | Missing |  |
| `container_include` | Global container inclusion filte | Missing |  |
| `container_include_metrics` | Metrics-specific container inclu | Missing |  |
| `container_labels_as_tags` | Map container labels to tags | Missing |  |
| `container_pid_mapper` | Custom PID-to-container mapper | Missing |  |
| `container_proc_root` | Procfs root path for containers | Implemented |  |
| `counter_expiry_seconds` | Idle counter keep-alive duration | ADP Only | Alias for dogstatsd_expiry_secon |
| `cri_connection_timeout` | CRI runtime connection timeout | Missing |  |
| `cri_query_timeout` | CRI runtime query timeout | Missing |  |
| `cri_socket_path` | CRI/containerd socket path | Implemented |  |
| `data_plane.api_listen_address` | ADP unprivileged API address | ADP Only |  |
| `data_plane.dogstatsd.enabled` | Enable DSD in data plane | Implemented |  |
| `data_plane.enabled` | Enable ADP globally | Implemented |  |
| `data_plane.remote_agent_enabled` | Register as remote agent | ADP Only |  |
| `data_plane.secure_api_listen_address` | ADP privileged API address | ADP Only |  |
| `data_plane.standalone_mode` | ADP standalone mode toggle | ADP Only |  |
| `data_plane.use_new_config_stream_endpoint` | Use new config stream endpoint | ADP Only |  |
| `dd_url` | Override intake endpoint URL | Implemented |  |
| `dev_mode_no_ssl` | Disable SSL for forwarding | Missing |  |
| `disable_cluster_name_tag_key` | Suppress cluster_name tag | Missing |  |
| `dogstatsd_allow_context_heap_allocs` | Allow heap allocs for contexts | ADP Only |  |
| `dogstatsd_buffer_count` | Number of receive buffers | ADP Only |  |
| `dogstatsd_buffer_size` | Receive buffer size (bytes) | Implemented |  |
| `dogstatsd_cached_contexts_limit` | Max cached metric contexts | ADP Only | ADP default 500000 |
| `dogstatsd_cached_tagsets_limit` | Max cached tagsets | ADP Only | ADP default 500000 |
| `dogstatsd_capture_depth` | Traffic capture channel depth | Missing |  |
| `dogstatsd_capture_path` | Traffic capture file location | Missing |  |
| `dogstatsd_context_expiry_seconds` | Context cache TTL (seconds) | Divergent | ADP hardcodes 30s vs Agent 20s |
| `dogstatsd_disable_verbose_logs` | Suppress noisy parse error logs | Missing |  |
| `dogstatsd_entity_id_precedence` | Entity ID over auto-detection | Implemented |  |
| `dogstatsd_eol_required` | Require newline-terminated msgs | Missing |  |
| `dogstatsd_experimental_http.enabled` | Enable HTTP DSD listener | Missing |  |
| `dogstatsd_experimental_http.listen_address` | HTTP DSD listener bind address | Missing |  |
| `dogstatsd_expiry_seconds` | Counter zero-value TTL (secs) | Implemented |  |
| `dogstatsd_flush_incomplete_buckets` | Flush open buckets on shutdown | Divergent | ADP uses aggregate_flush_open_* |
| `dogstatsd_host_socket_path` | Host UDS socket dir for DSD | Missing |  |
| `dogstatsd_log_file` | DSD dedicated log file path | Missing |  |
| `dogstatsd_log_file_max_rolls` | DSD log file max roll count | Missing |  |
| `dogstatsd_log_file_max_size` | DSD log file max size | Missing |  |
| `dogstatsd_logging_enabled` | Enables DSD metric logging | Missing |  |
| `dogstatsd_mapper_cache_size` | Mapper result LRU cache size | Missing | ADP mapper has no cache |
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
| `dogstatsd_mem_based_rate_limiter.` | Memory rate limiter group key | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.enabled` | Enable memory rate limiter | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.go_gc` | GC percentage for rate limiter | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.high_soft_limit` | High memory soft limit ratio | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.low_soft_limit` | Low memory soft limit ratio | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.memory_ballast` | GC ballast allocation size | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.rate_check.factor` | Rate check geometric factor | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.rate_check.max` | Rate check max interval | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.rate_check.min` | Rate check min interval | Missing | Go-runtime-specific feature |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.factor` | OS free-mem check backoff factor | Missing |  |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.max` | OS free-mem check max interval | Missing |  |
| `dogstatsd_mem_based_rate_limiter.soft_limit_freeos_check.min` | OS free-mem check min interval | Missing |  |
| `dogstatsd_metrics_stats_enable` | Enable per-metric debug stats | Missing |  |
| `dogstatsd_minimum_sample_rate` | Floor for metric sample rates | ADP Only |  |
| `dogstatsd_no_aggregation_pipeline` | Enable no-agg timestamped path | Implemented |  |
| `dogstatsd_no_aggregation_pipeline_batch_size` | No-agg pipeline batch size | Missing |  |
| `dogstatsd_non_local_traffic` | Accept non-localhost UDP/TCP | Implemented |  |
| `dogstatsd_origin_detection` | Enable UDS origin detection | Implemented |  |
| `dogstatsd_origin_detection_client` | Honor client origin proto fields | Divergent | ADP always parses; no toggle |
| `dogstatsd_origin_optout_enabled` | Allow clients to opt out origin | Implemented |  |
| `dogstatsd_packet_buffer_flush_timeout` | Packet buffer flush timeout | Missing |  |
| `dogstatsd_packet_buffer_size` | Datagrams per packet buffer | Missing |  |
| `dogstatsd_permissive_decoding` | Relaxes decoder strictness | ADP Only | ADP-only; default true |
| `dogstatsd_pipe_name` | Windows named pipe path | Missing |  |
| `dogstatsd_pipeline_autoadjust` | Auto-adjust pipeline workers | Missing |  |
| `dogstatsd_pipeline_count` | Parallel processing pipelines | Missing |  |
| `dogstatsd_port` | UDP listen port | Implemented |  |
| `dogstatsd_queue_size` | Packet channel buffer size | Missing |  |
| `dogstatsd_so_rcvbuf` | Socket receive buffer size | Missing |  |
| `dogstatsd_socket` | UDS datagram socket path | Implemented |  |
| `dogstatsd_stats_buffer` | Internal stats buffer size | Missing |  |
| `dogstatsd_stats_enable` | Enable internal stats endpoint | Missing |  |
| `dogstatsd_stats_port` | Internal stats endpoint port | Missing |  |
| `dogstatsd_stream_log_too_big` | Log oversized stream messages | Missing |  |
| `dogstatsd_stream_socket` | UDS stream socket path | Implemented |  |
| `dogstatsd_string_interner_size` | String interner capacity | Divergent | Agent=4096 entries, ADP=2MiB byt |
| `dogstatsd_tag_cardinality` | Default tag cardinality level | Implemented |  |
| `dogstatsd_tags` | Extra tags added to all DSD data | Implemented |  |
| `dogstatsd_tags.mappings` | Tag mapping rules for DSD metric | Missing | Not a real Agent config key |
| `dogstatsd_tcp_port` | TCP listen port for DSD | ADP Only | Agent has no TCP DSD listener |
| `dogstatsd_telemetry_enabled_listener_id` | Per-listener telemetry tagging | Missing |  |
| `dogstatsd_windows_pipe_security_descriptor` | Windows named pipe ACL descripto | Missing | ADP is Linux-only, N/A |
| `dogstatsd_workers_count` | Num DSD processing workers | Missing | ADP uses async, no worker pools |
| `enable_global_limiter` | Toggle global memory limiter | ADP Only | Agent has no equivalent |
| `enable_payloads_events` | Allow sending event payloads | Divergent | ADP key uses _ not . separator |
| `enable_payloads_series` | Allow sending series payloads | Divergent | ADP key uses _ not . separator |
| `enable_payloads_service_checks` | Allow sending svc check payloads | Divergent | ADP key uses _ not . separator |
| `enable_payloads_sketches` | Allow sending sketch payloads | Divergent | ADP key uses _ not . separator |
| `env` | Agent environment name | Implemented | ADP default 'none' vs Agent '' |
| `expected_tags_duration` | Host tag enrichment duration | Implemented |  |
| `extra_tags` | Additional static tags | Missing |  |
| `flush_timeout_secs` | Encoder flush timeout (secs) | ADP Only |  |
| `forwarder_apikey_validation_interval` | API key check interval (mins) | Missing |  |
| `forwarder_backoff_base` | Retry backoff base (secs) | Implemented |  |
| `forwarder_backoff_factor` | Retry backoff jitter factor | Implemented |  |
| `forwarder_backoff_max` | Retry backoff ceiling (secs) | Implemented |  |
| `forwarder_connection_reset_interval` | HTTP conn reset interval (secs) | Implemented |  |
| `forwarder_flush_to_disk_mem_ratio` | Mem-to-disk flush threshold | Missing |  |
| `forwarder_high_prio_buffer_size` | High-priority request queue size | Divergent | ADP default 16 vs Agent 100 |
| `forwarder_http_protocol` | HTTP version (auto/http1) | Missing |  |
| `forwarder_low_prio_buffer_size` | Low-priority request queue size | Missing |  |
| `forwarder_max_concurrent_requests` | Max concurrent HTTP requests | Missing |  |
| `forwarder_num_workers` | Concurrent forwarder workers | Implemented |  |
| `forwarder_outdated_file_in_days` | Retry file retention (days) | Missing |  |
| `forwarder_recovery_interval` | Backoff recovery decrease factor | Implemented |  |
| `forwarder_recovery_reset` | Reset errors on success | Implemented |  |
| `forwarder_requeue_buffer_size` | Requeue channel buffer size | Missing |  |
| `forwarder_retry_queue_capacity_time_interval_sec` | Retry queue time-based capacity | Missing |  |
| `forwarder_retry_queue_max_size` | Retry queue max size (deprecated | Implemented |  |
| `forwarder_retry_queue_payloads_max_size` | Retry queue max size (bytes) | Implemented |  |
| `forwarder_stop_timeout` | Forwarder shutdown timeout (s) | Missing |  |
| `forwarder_storage_max_disk_ratio` | Max disk usage ratio for retry | Implemented |  |
| `forwarder_storage_max_size_in_bytes` | Max on-disk retry storage size | Implemented |  |
| `forwarder_storage_path` | On-disk retry storage directory | Implemented |  |
| `forwarder_timeout` | Forwarder HTTP request timeout | Implemented |  |
| `histogram_aggregates` | Histogram aggregate statistics | Implemented |  |
| `histogram_copy_to_distribution` | Copy histograms to distributions | Implemented |  |
| `histogram_copy_to_distribution_prefix` | Prefix for hist-to-dist copies | Implemented |  |
| `histogram_percentiles` | Histogram percentile quantiles | Implemented |  |
| `host_aliases` | Extra host aliases for metadata | Missing |  |
| `hostname` | Configured hostname override | Implemented |  |
| `hostname_file` | File-based hostname fallback | Missing |  |
| `hostname_force_config_as_canonical` | Force config hostname canonical | Missing |  |
| `hostname_fqdn` | Use FQDN as hostname | Missing |  |
| `hostname_trust_uts_namespace` | Trust UTS namespace hostname | Missing |  |
| `ignore_host_etc` | Skip /host/etc in containers | Missing |  |
| `input_chan_size` | EP forwarder input chan size | Missing | Not used by DogStatsD at all |
| `ipc_cert_file_path` | IPC TLS certificate path | Implemented |  |
| `log_file` | Log output file path | Implemented |  |
| `log_file_max_rolls` | Max rotated log files kept | Implemented |  |
| `log_file_max_size` | Max log file size before rotate | Implemented |  |
| `log_format_json` | Use JSON log format | Implemented |  |
| `log_format_rfc3339` | Use RFC3339 timestamp format | Missing |  |
| `log_level` | Log verbosity level | Implemented |  |
| `log_payloads` | Debug-log flushed payloads | Missing |  |
| `log_to_console` | Log to stdout/stderr | Implemented |  |
| `log_to_syslog` | Log to syslog daemon | Missing | ADP has TODO for syslog |
| `logging_frequency` | Transaction success log interval | Missing |  |
| `memory_limit` | Process memory limit (bytes) | ADP Only |  |
| `memory_slop_factor` | Memory headroom fraction | ADP Only |  |
| `metric_filterlist` | Metric name blocklist | Implemented |  |
| `metric_filterlist_match_prefix` | Blocklist uses prefix matching | Implemented |  |
| `metric_tag_filterlist` | Per-metric tag include/exclude | Missing |  |
| `min_tls_version` | Minimum TLS version for HTTPS | Missing |  |
| `no_proxy_nonexact_match` | Enable domain/CIDR no_proxy matc | Implemented |  |
| `origin_detection_unified` | Unified origin detection mode | Implemented |  |
| `otlp_string_interner_size` | OTLP context interner capacity | ADP Only |  |
| `proc_root` | Root path to /proc filesystem | Missing |  |
| `procfs_path` | Path to procfs for system checks | Missing |  |
| `proxy` | Top-level proxy config section | Implemented |  |
| `proxy.http` | HTTP proxy URL | Implemented |  |
| `proxy.https` | HTTPS proxy URL | Implemented |  |
| `proxy.no_proxy` | Hosts bypassing proxy | Implemented |  |
| `proxy_http` | HTTP proxy URL | Implemented |  |
| `proxy_https` | HTTPS proxy URL | Implemented |  |
| `proxy_no_proxy` | Proxy bypass host list | Implemented |  |
| `remote_agent_string_interner_size_bytes` | Tag string interner capacity | ADP Only |  |
| `run_path` | Runtime data directory path | Implemented |  |
| `secret_backend_arguments` | Args to secret backend command | Missing |  |
| `secret_backend_command` | Secret resolver executable path | Implemented |  |
| `secret_backend_command_allow_group_exec_perm` | Allow group exec on backend bina | Missing |  |
| `secret_backend_config` | Config map for secret backend | Missing |  |
| `secret_backend_output_max_size` | Max secret backend response size | Missing |  |
| `secret_backend_remove_trailing_line_break` | Strip trailing newline from secr | Missing |  |
| `secret_backend_skip_checks` | Skip security checks on backend | Missing |  |
| `secret_backend_timeout` | Secret backend timeout (seconds) | Implemented |  |
| `secret_backend_type` | Native secrets backend selector | Missing |  |
| `secret_refresh_interval` | Secrets periodic refresh (secs) | Missing |  |
| `secret_refresh_on_api_key_failure_interval` | Secrets refresh on API key fail | Missing |  |
| `secret_refresh_scatter` | Randomize secret refresh timing | Missing |  |
| `sender_backoff_base` | Log sender backoff base (secs) | Missing |  |
| `sender_backoff_factor` | Log sender backoff jitter factor | Missing |  |
| `sender_backoff_max` | Log sender max backoff (secs) | Missing |  |
| `sender_recovery_interval` | Log sender recovery interval | Missing |  |
| `sender_recovery_reset` | Log sender full error reset | Missing |  |
| `serializer_compressor_kind` | Payload compression algorithm | Implemented |  |
| `serializer_experimental_use_v3_api.compression_level` | V3 API compression level | Missing |  |
| `serializer_experimental_use_v3_api.series.endpoints` | V3 API series endpoint list | Missing |  |
| `serializer_experimental_use_v3_api.series.validate` | V3 API series validation flag | Missing |  |
| `serializer_experimental_use_v3_api.sketches.endpoints` | V3 API sketches endpoints | Missing |  |
| `serializer_experimental_use_v3_api.sketches.validate` | V3 API sketches validation | Missing |  |
| `serializer_max_metrics_per_payload` | Max metrics per payload | ADP Only | ADP only; default 10000 |
| `serializer_max_payload_size` | Max compressed payload size | Missing | ADP uses hardcoded 3.2MB |
| `serializer_max_series_payload_size` | Max series compressed size | Missing | ADP hardcodes same default |
| `serializer_max_series_points_per_payload` | Max series points per payload | Missing | ADP has similar via max_metrics |
| `serializer_max_series_uncompressed_payload_size` | Max series uncompressed size | Missing | ADP hardcodes same default |
| `serializer_max_uncompressed_payload_size` | Max uncompressed payload size | Missing | ADP hardcodes 60MB vs Agent 4MB |
| `serializer_zstd_compressor_level` | Zstd compression level | Divergent | ADP default 3 vs Agent 1 |
| `server_timeout` | IPC API server timeout | Missing |  |
| `site` | Datadog site domain | Implemented |  |
| `skip_ssl_validation` | Skip TLS cert validation | Missing |  |
| `sslkeylogfile` | TLS key log file path | Missing |  |
| `statsd_forward_host` | Host for packet forwarding | Missing |  |
| `statsd_forward_port` | Port for packet forwarding | Missing |  |
| `statsd_metric_blocklist` | Metric name blocklist | Missing |  |
| `statsd_metric_blocklist_match_prefix` | Blocklist matches by prefix | Missing |  |
| `statsd_metric_namespace` | Prefix prepended to all metrics | Missing |  |
| `statsd_metric_namespace_blacklist` | Prefixes exempt from namespace | Missing |  |
| `statsd_metric_namespace_blocklist` | Alias (unused) for blacklist key | Missing | Not registered in Agent either |
| `syslog_uri` | URI for syslog output | Missing |  |
| `tag_value_split_separator` | Per-tag value split chars | Missing |  |
| `tag_value_split_separator.foo` | Split char for tag 'foo' | Missing | Sub-key of map config |
| `tags` | Global tags (DD_TAGS) | Missing |  |
| `telemetry.dogstatsd.aggregator_channel_latency_buckets` | Histogram buckets for agg latenc | Missing |  |
| `telemetry.dogstatsd.listeners_channel_latency_buckets` | Histogram buckets for listener l | Missing |  |
| `telemetry.dogstatsd.listeners_latency_buckets` | DSD listener latency buckets | Missing |  |
| `telemetry.dogstatsd_origin` | DSD origin detection telemetry | Missing |  |
| `telemetry.enabled` | Global telemetry toggle | Divergent | ADP uses data_plane.telemetry_en |
| `tls_handshake_timeout` | HTTP TLS handshake timeout | Missing |  |
| `use_compression` | Payload compression toggle | Missing | ADP uses serializer_compressor_k |
| `use_dogstatsd` | Master DogStatsD enable toggle | Missing | ADP uses data_plane.dogstatsd.en |
| `use_improved_cgroup_parser` | Trie-based cgroup path parser | Missing |  |
| `use_proxy_for_cloud_metadata` | Proxy cloud metadata endpoints | Implemented |  |
| `use_v2_api` | V2 API for log/forwarder endpoin | Missing |  |
| `use_v2_api.series` | V2 series API for metrics | Missing | ADP always uses v2 series API |
| `workloadmeta.remote.recv_without_timeout` | Workloadmeta stream recv timeout | Missing |  |
| `zstd_compression_level` | Zstd compression level | Divergent | ADP key: serializer_zstd_compres |

## Discussion

### dogstatsd_port

We have a diverfence as follows, in the Agent we see

```go
// blah some go code
```

and in ADP we see

```rust
// blah some Rust code
```

Blah blah discussion

### agent_ipc_endpoint

The Go Agent exposes `agent_ipc.host` (default `localhost`) and `agent_ipc.port` (default `0`) as separate config keys under the `agent_ipc` group in `pkg/config/setup/common_settings.go`:

```go
config.BindEnvAndSetDefault("agent_ipc.host", "localhost")
config.BindEnvAndSetDefault("agent_ipc.port", 0)
```

Saluki combines these into a single `agent_ipc_endpoint` URI in `lib/saluki-env/src/helpers/remote_agent/client.rs`:

```rust
#[serde(rename = "agent_ipc_endpoint", with = "http_serde_ext::uri",
        default = "default_agent_ipc_endpoint")]
ipc_endpoint: Uri,
// default: https://127.0.0.1:5001
```

This key has no equivalent in the Go Agent. The Go Agent uses `cmd_port` (default 5001) for the IPC endpoint, while Saluki defaults its combined URI to `https://127.0.0.1:5001`. Functionally equivalent intent but different config surface.

### batch_max_concurrent_send

In the Agent, `batch_max_concurrent_send` is registered under prefixed log/event forwarder config keys (e.g. `logs_config.batch_max_concurrent_send`) and controls the maximum number of concurrent HTTP sends for log batches. Default is `0` (unlimited).

```go
// pkg/config/setup/config.go
config.BindEnvAndSetDefault(prefix+"batch_max_concurrent_send", DefaultBatchMaxConcurrentSend) // 0
```

In ADP, there is no `batch_max_concurrent_send` config key. The closest equivalent is `forwarder_num_workers` (default 1) which controls per-endpoint concurrency:

```rust
// lib/saluki-components/src/common/datadog/config.rs
#[serde(default = "default_endpoint_concurrency", rename = "forwarder_num_workers")]
endpoint_concurrency: usize, // default 1
```

The Agent's key is specific to log pipeline batching, while ADP uses a different concurrency model via its forwarder configuration.

### bind_host

In the Agent, `bind_host` is a global fallback used when `dogstatsd_non_local_traffic` is false. The DogStatsD UDP listener calls `GetBindHost(cfg)` which returns the `bind_host` config value (defaulting to `localhost`):

```go
// comp/dogstatsd/listeners/udp.go
if cfg.GetBool("dogstatsd_non_local_traffic") {
    url = net.JoinHostPort("0.0.0.0", port)
} else {
    url = net.JoinHostPort(configutils.GetBindHost(cfg), port)
}
```

In ADP, the DogStatsD source ignores `bind_host` entirely and hardcodes the listen address:

```rust
// lib/saluki-components/src/sources/dogstatsd/mod.rs
let address = if self.non_local_traffic {
    ListenAddress::Udp(([0, 0, 0, 0], self.port).into())
} else {
    ListenAddress::Udp(([127, 0, 0, 1], self.port).into())
};
```

Users who set `bind_host` to a specific interface address will not see that respected by ADP.

### cluster_name

In the Agent, `cluster_name` is a top-level config key (default empty string) used to set the cluster name for Kubernetes environments. It is read by `clustername.GetClusterName()` and applied as a host-level tag.

```go
// pkg/config/setup/common_settings.go
config.BindEnvAndSetDefault("cluster_name", "")
```

In ADP, `cluster_name` does not exist as a config key. It appears only as a tag name constant in OTLP attribute mappings (e.g., `KUBERNETES_DD_TAGS` includes `"cluster_name"` as a passthrough tag). ADP running alongside the Agent receives host tags (including cluster_name) via the IPC/gRPC connection to the Agent, so it does not need to resolve this independently. However, in standalone mode this tag would be absent.

### compression_kind

In the Agent, `compression_kind` is a prefixed key (e.g., `logs_config.compression_kind`) that controls the compression algorithm for log forwarding. Default is `"zstd"`.

```go
// pkg/config/setup/config.go
config.BindEnvAndSetDefault(prefix+"compression_kind", DefaultLogCompressionKind) // "zstd"
```

In ADP, the equivalent functionality exists but under a different config key name: `serializer_compressor_kind` (default `"zstd"`):

```rust
// lib/saluki-components/src/encoders/datadog/metrics/mod.rs
#[serde(rename = "serializer_compressor_kind", default = "default_serializer_compressor_kind")]
compressor_kind: String, // default "zstd"
```

The underlying behavior is equivalent (both support gzip and zstd), but the config key name differs. Users setting `compression_kind` will not affect ADP.

### compression_level

In the Agent, `compression_level` is a prefixed key (e.g., `logs_config.compression_level`) controlling the gzip compression level. Default is `6`. A separate `zstd_compression_level` key (default `1`) controls zstd level.

```go
// pkg/config/setup/config.go
config.BindEnvAndSetDefault(prefix+"compression_level", DefaultGzipCompressionLevel) // 6
config.BindEnvAndSetDefault(prefix+"zstd_compression_level", DefaultZstdCompressionLevel) // 1
```

In ADP, compression level is controlled via `serializer_zstd_compressor_level` (default `1`), and there is no separate gzip level key:

```rust
// lib/saluki-components/src/encoders/datadog/metrics/mod.rs
#[serde(rename = "serializer_zstd_compressor_level", default = "default_zstd_compressor_level")]
zstd_compressor_level: i32, // default 1
```

ADP does not honor `compression_level` -- it uses its own key name. The defaults align for zstd (both default to level 1) but the key names are incompatible.

### connection_reset_interval

In the Agent, `connection_reset_interval` is registered as a prefixed logs config key via `bindEnvAndSetLogsConfigKeys` (e.g., `logs_config.connection_reset_interval`). The forwarder has a separate key `forwarder_connection_reset_interval` (default 0, disabled).

In Saluki, only the `forwarder_connection_reset_interval` key exists:
```rust
#[serde(
    default = "default_forwarder_connection_reset_interval",
    rename = "forwarder_connection_reset_interval"
)]
connection_reset_interval_secs: u64,
```

The bare `connection_reset_interval` key (logs transport variant) has no equivalent in Saluki. Since Saluki does not implement the logs pipeline, this is expected.

### container_env_as_tags

In the Agent, `container_env_as_tags` is a `map[string]string` (default empty) registered in `common_settings.go`. It is used by the tagger (`workloadmeta_main.go`) and container env filter (`env_vars_filter.go`) to extract container environment variables and add them as tags to metrics.

Saluki has no implementation of this key. There is no Rust code referencing `container_env_as_tags` or `env_as_tags`. This means container environment variable-based tagging is not supported in ADP, which affects tag enrichment for DogStatsD metrics from containerized workloads.

### container_labels_as_tags

In the Agent, `container_labels_as_tags` is a `map[string]string` (default empty) registered in `common_settings.go`. It is used by the tagger (`workloadmeta_main.go`) to extract container labels and map them to metric tags.

Saluki has no implementation of this key. No Rust code references `container_labels_as_tags` or `labels_as_tags`. This means container label-based tag enrichment is not available in ADP.

### counter_expiry_seconds

In the Agent, the equivalent key is `dogstatsd_expiry_seconds` (default 300s). It controls how long idle counters emit zero values before being removed. There is no `counter_expiry_seconds` key in the Agent.

In Saluki, `counter_expiry_seconds` is the primary field name with `dogstatsd_expiry_seconds` as an alias:
```rust
#[serde(alias = "dogstatsd_expiry_seconds", default = "default_counter_expiry_seconds")]
counter_expiry_seconds: Option<u64>,
```

Both default to 300 seconds and have the same semantics. The key name `counter_expiry_seconds` is ADP-only, but the alias ensures backward compatibility with Agent configs using `dogstatsd_expiry_seconds`.

### data_plane.dogstatsd.enabled

In the Agent, this key is registered in `pkg/config/setup/common_settings.go`:
```go
config.BindEnvAndSetDefault("data_plane.dogstatsd.enabled", false)
```
It is read in `comp/dogstatsd/config/config.go` to determine whether DogStatsD should be disabled in the Core Agent (because ADP is handling it):
```go
dsdEnabledDataPlane := c.config.GetBool("data_plane.enabled") && c.config.GetBool("data_plane.dogstatsd.enabled")
```

In ADP (`bin/agent-data-plane/src/config.rs`):
```rust
enabled: config.try_get_typed("data_plane.dogstatsd.enabled")?.unwrap_or(false),
```

Both default to `false`. The semantics are complementary: in the Agent it disables the internal DSD server, in ADP it enables the DSD pipeline. Functionally equivalent.

### dd_url

In the Agent, `dd_url` is registered in `pkg/config/setup/common_settings.go`:
```go
config.BindEnv("dd_url", "DD_DD_URL", "DD_URL")
```
It is consumed by `pkg/config/utils/endpoints.go` via `GetInfraEndpoint` to override the default site-based endpoint:
```go
func GetInfraEndpoint(c pkgconfigmodel.Reader) string {
    return GetMainEndpoint(c, InfraURLPrefix, "dd_url")
}
```

In ADP, `dd_url` is a field on `EndpointConfiguration` (`lib/saluki-components/src/common/datadog/endpoints.rs`):
```rust
#[serde(default)]
dd_url: Option<String>,
```
When set, it takes precedence over `site` for the primary endpoint. Both implementations use `dd_url` as an override for the site-based endpoint. Functionally equivalent.

### dev_mode_no_ssl

In the Agent, `dev_mode_no_ssl` is a logs-config-prefixed setting registered in `pkg/config/setup/config.go`:
```go
config.SetDefault(prefix+"dev_mode_no_ssl", false)
```
It is used in the logs subsystem (`comp/logs/agent/config/config_keys.go`) to disable SSL on log forwarding endpoints.

The bare `dev_mode_no_ssl` (no prefix) is relevant to the DogStatsD pipeline because it can affect how the metrics forwarder connects to intake. In ADP, there is no equivalent config key -- ADP always uses HTTPS for endpoint connections and has no dev-mode SSL bypass. For production use this is fine, but it means ADP cannot be used with non-SSL dev endpoints.

### disable_cluster_name_tag_key

In the Agent, this is registered in `pkg/config/setup/common_settings.go`:
```go
config.BindEnvAndSetDefault("disable_cluster_name_tag_key", false)
```
It controls whether the `cluster_name` tag (in addition to `kube_cluster_name`) is added to metrics. When `true`, only `kube_cluster_name` is emitted. This is read by the tagger (`comp/core/tagger/collectors/workloadmeta_extract.go`) and host tags (`comp/metadata/host/hostimpl/hosttags/tags.go`).

In ADP, there is no reference to this config key in any Rust source file. ADP's tagger/enrichment logic does not check for this flag, so ADP always emits both `cluster_name` and `kube_cluster_name` tags regardless of this setting.

### dogstatsd_allow_context_heap_allocs

This config key does not exist in the datadog-agent codebase. It is an ADP-specific setting defined in `lib/saluki-components/src/sources/dogstatsd/mod.rs`:
```rust
#[serde(
    rename = "dogstatsd_allow_context_heap_allocs",
    default = "default_allow_context_heap_allocations"
)]
allow_context_heap_allocations: bool,
```
Default: `true`. It controls whether the string interner fallback to heap allocations is allowed. When `false`, metrics whose context cannot be interned are dropped. The Agent does not have this concept because it uses a different memory model for context resolution.

### dogstatsd_buffer_count

This config key does not exist in the datadog-agent codebase (no Go files or YAML templates reference it). It is an ADP-specific setting defined in `lib/saluki-components/src/sources/dogstatsd/mod.rs`:
```rust
#[serde(rename = "dogstatsd_buffer_count", default = "default_buffer_count")]
buffer_count: usize,
```
Default: `128`. It controls the number of pre-allocated I/O buffers for receiving DogStatsD messages. The Agent uses a different I/O model with goroutine-based packet handling and does not expose an equivalent buffer count setting.

### dogstatsd_cached_contexts_limit

This key is not registered in the Agent's `pkg/config/setup/common_settings.go` -- there is no `BindEnvAndSetDefault` call for it.

In ADP/Saluki, this key is actively used:
```rust
#[serde(
    rename = "dogstatsd_cached_contexts_limit",
    default = "default_cached_contexts_limit"
)]
cached_contexts_limit: usize,
```
With a default of 500,000. It is consumed in `resolver.rs`:
```rust
.with_cached_contexts_limit(cached_contexts_limit)
```

This is an ADP-originated configuration providing explicit control over the context resolver cache size. The Agent manages context caching internally without a user-facing config key.

### dogstatsd_cached_tagsets_limit

This key is not registered in the Agent's `pkg/config/setup/common_settings.go`. There is no `BindEnvAndSetDefault` call for it.

In ADP/Saluki, it is actively used:
```rust
#[serde(rename = "dogstatsd_cached_tagsets_limit", default = "default_cached_tagsets_limit")]
cached_tagsets_limit: usize,
```
With a default of 500,000. It is consumed in `resolver.rs`:
```rust
.with_cached_tagsets_limit(cached_tagsets_limit)
```

This is an ADP-originated configuration for explicit tagset cache sizing that has no Agent counterpart.

### dogstatsd_context_expiry_seconds

In the Agent, this is a configurable setting registered with a default of 20 seconds:
```go
config.BindEnvAndSetDefault("dogstatsd_context_expiry_seconds", 20)
```
It controls how long DogStatsD metric contexts are kept in memory before being evicted.

In ADP/Saluki, there is no serde field for this config key. Instead, the context expiration is hardcoded in `resolver.rs`:
```rust
const RESOLVER_CACHE_EXPIRATION: Duration = Duration::from_secs(30);
```
This value is used directly:
```rust
.with_idle_context_expiration(RESOLVER_CACHE_EXPIRATION)
.with_idle_tagsets_expiration(RESOLVER_CACHE_EXPIRATION)
```

The Agent allows runtime configuration with a 20-second default; ADP hardcodes 30 seconds. Users who have tuned this setting in the Agent will find ADP does not honor their value.

### dogstatsd_expiry_seconds

In the Agent, this is registered with a default of 300 seconds:
```go
config.BindEnvAndSetDefault("dogstatsd_expiry_seconds", 300)
```
It controls how long counters continue to be sampled to zero if not received.

In ADP/Saluki, this key is recognized via a serde alias on the aggregate transform:
```rust
#[serde(alias = "dogstatsd_expiry_seconds", default = "default_counter_expiry_seconds")]
counter_expiry_seconds: Option<u64>,
```
The default is also `Some(300)`. The behavior is functionally equivalent: idle counters emit zero values until expiry. The `alias` approach means either `dogstatsd_expiry_seconds` or `counter_expiry_seconds` can be used.

### dogstatsd_flush_incomplete_buckets

In the Agent, this is registered with a default of `false`:
```go
config.BindEnvAndSetDefault("dogstatsd_flush_incomplete_buckets", false)
```
It controls whether incomplete aggregation buckets are flushed on DogStatsD shutdown.

In ADP/Saluki, the equivalent functionality exists but under a different config key name:
```rust
#[serde(rename = "aggregate_flush_open_windows", default)]
flush_open_windows: bool,
```
Both default to `false` and serve the same purpose -- flushing partial/open aggregation windows at shutdown. However, ADP does not recognize the `dogstatsd_flush_incomplete_buckets` key. Users migrating from the Agent who have this set to `true` would need to use `aggregate_flush_open_windows` instead.

### dogstatsd_mapper_cache_size

In the Agent, the mapper maintains an LRU cache keyed by metric name. `dogstatsd_mapper_cache_size` (default 1000) controls the capacity:
```go
cacheSize := s.config.GetInt("dogstatsd_mapper_cache_size")
mapperInstance, err := mapper.NewMetricMapper(mappings, cacheSize)
```
The cache avoids re-running regex matching for previously seen metric names.

In ADP/Saluki, `DogStatsDMapperConfiguration` has no cache concept. Every metric is matched against profile regexes on every event. The key `dogstatsd_mapper_cache_size` is not referenced anywhere in the Saluki codebase. For high-cardinality mapper workloads this could be a performance difference, though the Rust regex engine may compensate.

### dogstatsd_mapper_string_interner_size

This key exists only in ADP/Saluki and has no counterpart in the Go Agent. In the Go Agent, the DogStatsD mapper does not use a dedicated string interner; it relies on the per-worker `stringInterner` whose size is controlled by `dogstatsd_string_interner_size` (default 4096 entries). The mapper results are cached separately via `dogstatsd_mapper_cache_size`.

In ADP, the mapper transform has its own `ContextResolver` with a dedicated string interner:
```rust
#[serde(rename = "dogstatsd_mapper_string_interner_size",
        default = "default_context_string_interner_size")]
context_string_interner_bytes: ByteSize,
```
Default is 64 KiB. This controls memory pre-allocated for interning mapped metric names and tags within the mapper transform specifically. This is an ADP architectural choice since the mapper is a separate transform component with its own context resolver rather than being inline in the server worker.

### dogstatsd_mem_based_rate_limiter.

This is a namespace/group key for all `dogstatsd_mem_based_rate_limiter.*` sub-keys. The entire memory-based rate limiter feature is Go-runtime-specific: it monitors cgroup memory usage and throttles UDS packet reads when memory pressure is high. It uses Go-specific APIs like `runtime.GC()`, `debug.FreeOSMemory()`, and `debug.SetGCPercent()`. None of the sub-keys under this namespace exist in ADP/Saluki. ADP uses Rust with a different memory model and does not implement this GC-based throttling mechanism.

### dogstatsd_minimum_sample_rate

This key exists only in ADP/Saluki and has no equivalent in the Datadog Agent.

In Saluki, it clamps extremely small sample rates to prevent memory blowup:
```rust
const fn default_dogstatsd_minimum_sample_rate() -> f64 {
    0.000000003845
}
```

When a metric is sent with a sample rate lower than this value, it is clamped to prevent tracking an unbounded number of equivalent samples (~260M max). The Agent has no such protection against very small sample rates.

### dogstatsd_no_aggregation_pipeline_batch_size

In the Agent, this controls how many metrics are batched in no-aggregation pipeline payloads sent to the intake:
```go
noAggWorker = newNoAggregationStreamWorker(
    pkgconfigsetup.Datadog().GetInt("dogstatsd_no_aggregation_pipeline_batch_size"),
    ...
)
```
Default is 2048.

Saluki reads `dogstatsd_no_aggregation_pipeline` to enable passthrough of timestamped metrics, but uses a time-based flush (`aggregate_passthrough_idle_flush_timeout`, default 1s) rather than a count-based batch size. There is no equivalent config key for controlling batch size in ADP.

### dogstatsd_origin_detection_client

In the Agent, this boolean (default `false`) gates whether the DogStatsD parser honors client-provided origin fields (`c:`, `e:`, `|card:`):
```go
case p.dsdOriginEnabled && bytes.HasPrefix(optionalField, localDataPrefix):
    localData = p.parseLocalData(...)
case p.dsdOriginEnabled && bytes.HasPrefix(optionalField, externalDataPrefix):
    externalData = p.parseExternalData(...)
case p.dsdOriginEnabled && bytes.HasPrefix(optionalField, cardinalityPrefix):
    cardinality = string(...)
```

In Saluki, the parser always parses these fields unconditionally:
```rust
b'c' if chunk.len() > 1 && chunk[1] == b':' => {
    let (_, local_data) = all_consuming(preceded(tag("c:"), local_data)).parse(chunk)?;
    maybe_local_data = Some(local_data);
}
```

There is no config toggle in ADP. The behavior is equivalent to `dogstatsd_origin_detection_client: true` at all times. Users who explicitly disable this in the Agent (the default) would get different behavior in ADP.

### dogstatsd_permissive_decoding

This key exists only in Saluki (ADP). It is not registered or referenced anywhere in the datadog-agent codebase.

In ADP, it defaults to `true` and controls whether the DogStatsD codec operates in permissive mode:
```rust
const fn default_dogstatsd_permissive_decoding() -> bool {
    true
}

#[serde(
    rename = "dogstatsd_permissive_decoding",
    default = "default_dogstatsd_permissive_decoding"
)]
permissive_decoding: bool,
```

When enabled, the codec uses a relaxed metric name parser (`permissive_metric_name`) that accepts characters the strict parser would reject. This is designed to match the Agent's actual parsing behavior, even though the Agent itself has no equivalent toggle -- the Agent's parser is inherently lenient.

### dogstatsd_string_interner_size

In the Agent, this controls the number of entries in a per-worker LRU string interner cache:
```go
stringInternerCacheSize := cfg.GetInt("dogstatsd_string_interner_size")
// default: 4096
```
Each worker gets its own interner sized to this count.

In ADP/Saluki, this controls the total byte capacity of a shared arena-style string interner:
```rust
#[serde(rename = "dogstatsd_string_interner_size", default = "default_context_string_interner_size")]
context_string_interner_bytes: ByteSize,
// default: 2 MiB
```

The semantics differ fundamentally: the Agent uses an entry count (4096 strings per worker), while ADP uses a byte budget (2 MiB total). A user migrating this setting from Agent to ADP would need to understand the unit change. The ADP approach is more memory-predictable but the numeric value is not portable between the two.

### dogstatsd_tags.mappings

This key does not exist as an actual configuration key in either the Agent or ADP. In the Agent, the related feature is `dogstatsd_mapper_profiles` which provides metric name-to-tag mapping via wildcard/regex profiles. That feature is analyzed separately. The `dogstatsd_tags.mappings` key appears to be a phantom entry in the known-configs inventory and does not correspond to any real config path in either codebase.

### dogstatsd_tcp_port

The Agent does not support a TCP listener for DogStatsD at all -- there is no `dogstatsd_tcp_port` config key registered anywhere in the Agent codebase. DogStatsD in the Agent only supports UDP, UDS (datagram and stream), and Windows named pipes.

In ADP/Saluki, TCP is supported as an additional listener type:
```rust
#[serde(rename = "dogstatsd_tcp_port", default = "default_tcp_port")]
tcp_port: u16,
// default: 0 (disabled)
```
When set to a non-zero value, a TCP listener is created. This is an ADP-only extension that provides an additional transport option not available in the reference implementation.

### dogstatsd_workers_count

In the Agent, this undocumented config overrides the auto-calculated number of DogStatsD worker goroutines:
```go
if configWC := s.config.GetInt("dogstatsd_workers_count"); configWC != 0 {
    s.log.Debug("Forcing the amount of DogStatsD workers to:", configWC)
    workersCount = configWC
}
// default: 0 (auto-calculate based on pipeline count)
```

ADP/Saluki does not have this config key. It uses an async architecture with Tokio rather than a worker-goroutine pool model, so the concept does not directly apply. The key appears in ADP test fixtures (`datadog.yaml` files) but is not read by any ADP code.

### enable_global_limiter

This key exists only in ADP/Saluki and has no counterpart in the Agent:
```rust
#[serde(default = "default_enable_global_limiter")]
enable_global_limiter: bool,
// default: true
```
When enabled, ADP uses a `MemoryLimiter` that tracks process memory and exerts backpressure when usage exceeds the configured `memory_limit`. The Agent does not have an equivalent global memory limiter mechanism -- it relies on per-component buffering and Go's GC.

### enable_payloads_events

In the Agent, this feature uses a dot-separated key `enable_payloads.events` (default `true`), read by the serializer to control whether event payloads are forwarded:
```go
config.BindEnvAndSetDefault("enable_payloads.events", true)
enableEvents: config.GetBool("enable_payloads.events"),
```

In ADP/Saluki, the equivalent uses an underscore-separated key `enable_payloads_events`:
```rust
#[serde(default = "default_enable_payloads_events")]
enable_payloads_events: bool,
// default: true
```

The behavior is functionally equivalent (filtering events before forwarding), but the config key name differs: `enable_payloads.events` (Agent) vs `enable_payloads_events` (ADP). Users migrating configs would need to adjust the key name.

### enable_payloads_series

In the Agent, this is `enable_payloads.series` (default `true`):
```go
config.BindEnvAndSetDefault("enable_payloads.series", true)
```

In ADP/Saluki, it is `enable_payloads_series`:
```rust
#[serde(default = "default_enable_payloads_series")]
enable_payloads_series: bool,
```

Same behavioral parity but different key naming convention (dot vs underscore).

### enable_payloads_service_checks

In the Agent, this is `enable_payloads.service_checks` (default `true`):
```go
config.BindEnvAndSetDefault("enable_payloads.service_checks", true)
```

In ADP/Saluki, it is `enable_payloads_service_checks`:
```rust
#[serde(default = "default_enable_payloads_service_checks")]
enable_payloads_service_checks: bool,
```

Same behavioral parity but different key naming convention (dot vs underscore).

### enable_payloads_sketches

In the Agent, this is `enable_payloads.sketches` (default `true`):
```go
config.BindEnvAndSetDefault("enable_payloads.sketches", true)
```

In ADP/Saluki, it is `enable_payloads_sketches`:
```rust
#[serde(default = "default_enable_payloads_sketches")]
enable_payloads_sketches: bool,
```

Same behavioral parity but different key naming convention (dot vs underscore).

### env

In the Agent, `env` defaults to `""` and is read via `GetString("env")` in multiple places: host tags, trace config, profiler, SBOM processor, and orchestrator checks. It is a global setting that tags telemetry with the deployment environment.

In ADP/Saluki, the `env` key is deserialized as a bare field on `DatadogApmStatsEncoderConfiguration` and `DatadogTraceConfiguration` via `config.as_typed()`. It defaults to `"none"` rather than `""`. The field is used to populate the `agent_env` in APM stats and trace payloads.

The default value differs (`"none"` in ADP vs `""` in the Agent), and the scope is narrower in ADP (only traces/stats encoders vs. global usage in the Agent). However, for DogStatsD metric forwarding, neither implementation uses `env` directly on the metrics path, so the practical impact for DogStatsD parity is minimal.

### flush_timeout_secs

This config key exists only in ADP/Saluki. It controls how long the metrics, stats, and traces encoders wait before flushing a partially-filled request payload. It appears on `DatadogMetricsConfiguration`, `DatadogApmStatsEncoderConfiguration`, and `DatadogTraceConfiguration`, all defaulting to 2 seconds. A value of 0 is treated as 10ms (near-immediate flush).

The Agent does not have an equivalent top-level config key. The Agent's forwarder uses a different batching/flushing strategy based on its internal serializer and domain forwarder architecture. This is an ADP-specific tuning knob for controlling batching latency vs. payload efficiency.

### forwarder_high_prio_buffer_size

In the Agent, `forwarder_high_prio_buffer_size` defaults to 100 and controls the channel buffer size for high-priority transactions in `domainForwarder.init()`:
```go
highPrioBuffSize := f.config.GetInt("forwarder_high_prio_buffer_size")
f.highPrio = make(chan transaction.Transaction, highPrioBuffSize)
```

In ADP/Saluki, the same config key is read via `#[serde(rename = "forwarder_high_prio_buffer_size")]` on `ForwarderConfiguration.endpoint_buffer_size`, but it defaults to 16 (via `default_endpoint_buffer_size`):
```rust
const fn default_endpoint_buffer_size() -> usize {
    16
}
```

Additionally, ADP uses a single buffer size for all endpoint requests (no separate high/low priority queues), whereas the Agent maintains distinct high-priority and low-priority channel buffers. The Agent's `forwarder_low_prio_buffer_size` (also default 100) has no counterpart in ADP.

### forwarder_low_prio_buffer_size

In the Agent, `forwarder_low_prio_buffer_size` defaults to 100 and controls the channel buffer for low-priority transactions, separate from the high-priority buffer.

In ADP/Saluki, there is no separate low-priority queue. The forwarder uses a single `endpoint_buffer_size` (mapped from `forwarder_high_prio_buffer_size`). There is no priority-based routing of transactions; all requests go through one buffer. This key is entirely absent from ADP.

### hostname

In the Agent, the `hostname` config key is read from `pkg/util/hostname/common.go` as the first priority source for hostname resolution:
```go
configName := pkgconfigsetup.Datadog().GetString("hostname")
```
The resolved hostname is then injected into the DogStatsD server as `defaultHostname`.

In ADP/Saluki, the `FixedHostProvider` reads the same key:
```rust
let hostname = config.get_typed::<String>("hostname")?;
```
This is used when running in standalone mode. When running alongside the Agent, `RemoteAgentHostProvider` fetches the hostname via gRPC from the Agent instead, which means the Agent's full hostname resolution chain (including `hostname` config) is used indirectly. The behavior is functionally equivalent.

### input_chan_size

In the Agent, `input_chan_size` (default 100) is registered in `pkg/config/setup/config.go` as a logs/EP forwarder config key with a prefix:
```go
config.BindEnvAndSetDefault(prefix+"input_chan_size", DefaultInputChanSize) // Only used by EP Forwarder for now, not used by logs
```
It is read in `comp/logs/agent/config/config_keys.go` and is not referenced anywhere in the `comp/dogstatsd/` component. This key has no relevance to DogStatsD functionality.

### log_format_rfc3339

In the Agent, `log_format_rfc3339` (default `false`) controls whether log timestamps use RFC3339 format instead of the default format. It is read in `pkg/util/log/setup/log_format.go`:
```go
dateFmt := formatters.Date(cfg.GetBool("log_format_rfc3339"))
```

In ADP/Saluki, the `LoggingConfiguration` struct in `lib/saluki-app/src/logging/config.rs` has no `log_format_rfc3339` field. The timestamp format is hardcoded in `lib/saluki-app/src/logging/layer.rs` using the pattern `%Y-%m-%d %H:%M:%S %Z`, which is not RFC3339. There is no way to switch to RFC3339 via configuration. This is a minor omission since most users rely on the default format.

### log_payloads

In the Agent, `log_payloads` (default `false`) causes the aggregator to debug-log every flushed service check, event, and series payload. It is read in `pkg/aggregator/aggregator.go`:
```go
if pkgconfigsetup.Datadog().GetBool("log_payloads") {
    log.Debug("Flushing the following Service Checks:")
```

ADP/Saluki has no equivalent config key or behavior. Since ADP does not run the full aggregation pipeline in the same way (it focuses on DogStatsD intake and forwarding), this debug feature has no direct analog.

### log_to_syslog

In the Agent, `log_to_syslog` (default `false`) enables sending logs to the system syslog daemon. It is read in `pkg/util/log/setup/log_nix.go` and `log_windows.go`.

In ADP/Saluki, there is an explicit TODO comment in `lib/saluki-app/src/logging/mod.rs`:
```rust
// TODO: Support for logging to syslog.
```

The `LoggingConfiguration` struct does not have a `log_to_syslog` field, and no syslog backend is wired up. This is a known gap with a planned fix.

### logging_frequency

In the Agent, `logging_frequency` (default `500`) controls how often successful transaction POSTs are logged. It is read in `comp/forwarder/defaultforwarder/transaction/transaction.go`:
```go
loggingFrequency := config.GetInt64("logging_frequency")
```
Every Nth successful transaction gets logged at info level to avoid log spam.

ADP/Saluki does not implement this config key. ADP's forwarding path does not have the same transaction-level logging mechanism, so this key has no current analog.

### memory_limit

This config key does not exist in the Datadog Agent's `pkg/config/setup/` registration. The Agent relies on external container memory limits and Go's runtime memory management rather than an explicit config key.

In ADP/Saluki, `memory_limit` is a core part of the memory bounds system defined in `lib/saluki-app/src/memory.rs`:
```rust
#[serde(default)]
memory_limit: Option<ByteSize>,
```
It controls the overall memory ceiling for the process. When set, ADP verifies that all component memory bounds fit within this limit (after applying `memory_slop_factor`). If not set, ADP also attempts to detect cgroup limits via the `DOCKER_DD_AGENT` environment variable.

### memory_slop_factor

This config key does not exist in the Datadog Agent. It is an ADP-specific concept.

In ADP/Saluki, `memory_slop_factor` (default `0.25`) is defined in `lib/saluki-app/src/memory.rs`:
```rust
#[serde(default = "default_memory_slop_factor")]
memory_slop_factor: f64,
```
It reduces the effective memory limit to account for untracked allocations. A value of 0.25 means 25% of `memory_limit` is withheld, so components must fit within 75% of the configured limit. This is part of ADP's proactive memory bounds verification system.

### metric_tag_filterlist

In the Agent, `metric_tag_filterlist` is a list of objects that map metric names to tag include/exclude rules:
```go
err := structure.UnmarshalKey(config, "metric_tag_filterlist", &tagFilterListEntries)
```
Each entry has a metric name, an action (include/exclude), and a list of tags. This allows fine-grained control over which tags are kept or removed on a per-metric basis.

In Saluki, the `DogStatsDPrefixFilterConfiguration` struct does not include a `metric_tag_filterlist` field. The prefix filter transform only handles `metric_filterlist` (name-based blocking) and `statsd_metric_blocklist`, but not tag-level filtering. There is no tag-filtering transform equivalent in the ADP codebase.

### otlp_string_interner_size

This config key exists only in Saluki's OTLP source configuration:
```rust
#[serde(
    rename = "otlp_string_interner_size",
    default = "default_context_string_interner_size"
)]
context_string_interner_bytes: ByteSize,
```
It controls the memory budget for the string interner used to deduplicate metric names and tags in the OTLP pipeline. The datadog-agent has no equivalent key -- its OTLP pipeline does not use a configurable string interner for context resolution. This is an ADP-specific optimization.

### proc_root

In the Agent, `proc_root` (default `/proc`) is used by the ECS metadata detection module to find the default gateway:
```go
gw, err := system.GetDefaultGateway(pkgconfigsetup.Datadog().GetString("proc_root"))
```

Saluki does not read `proc_root`. Its cgroups configuration uses `container_proc_root` instead, which is a different config key with different semantics (container-oriented procfs path). The `proc_root` key is not referenced anywhere in the Saluki codebase.

### procfs_path

In the Agent, `procfs_path` defaults to `/host/proc` inside containers or `/proc` otherwise. It is used by Python check collectors, the network check, memory check, and CPU check to locate the proc filesystem:
```go
if pkgconfigsetup.Datadog().IsSet("procfs_path") {
    procfsPath = pkgconfigsetup.Datadog().GetString("procfs_path")
}
```

Saluki does not use `procfs_path`. Its cgroups helper reads `container_proc_root` instead. Since Saluki does not run Python checks or system-level core checks, `procfs_path` is not applicable to its use case.

### proxy

In the Agent, `proxy` is a top-level config section with sub-keys `http`, `https`, and `no_proxy`. The `LoadProxyFromEnv` function reads the nested `proxy` section and also supports `DD_PROXY_HTTP`, `DD_PROXY_HTTPS`, `DD_PROXY_NO_PROXY`, `HTTP_PROXY`, `HTTPS_PROXY`, and `NO_PROXY` env vars.

In Saluki, the `proxy` section is handled via key aliases that flatten `proxy.http` to `proxy_http`, `proxy.https` to `proxy_https`, and `proxy.no_proxy` to `proxy_no_proxy` at config load time. The `DatadogRemapper` also maps `HTTP_PROXY`/`HTTPS_PROXY` env vars to the flat keys. The `ProxyConfiguration` struct then reads the flat keys via serde. Functionally equivalent.

### remote_agent_string_interner_size_bytes

This key exists only in Saluki/ADP. It controls the size of the string interner used by the remote agent workload provider, primarily for tags.

```rust
// lib/saluki-env/src/workload/providers/remote_agent/mod.rs
const DEFAULT_STRING_INTERNER_SIZE_BYTES: NonZeroUsize = NonZeroUsize::new(512 * 1024).unwrap();
let string_interner_size_bytes = config
    .try_get_typed::<NonZeroUsize>("remote_agent_string_interner_size_bytes")?
    .unwrap_or(DEFAULT_STRING_INTERNER_SIZE_BYTES);
```

The datadog-agent has no equivalent config key. This is an ADP-specific tunable for memory management of interned tag strings, defaulting to 512KB.

### secret_backend_arguments

In the agent, `secret_backend_arguments` provides command-line arguments to the secret backend process:

```go
// pkg/config/setup/common_settings.go
config.BindEnvAndSetDefault("secret_backend_arguments", []string{})
// pkg/config/setup/config.go
Arguments: config.GetStringSlice("secret_backend_arguments"),
```

Saluki's `ExternalProcessResolverConfiguration` only deserializes `secret_backend_command` and `secret_backend_timeout`. It does not accept `secret_backend_arguments` and spawns the backend command with no arguments:

```rust
// lib/saluki-config/src/secrets/resolver/external.rs
pub struct ExternalProcessResolverConfiguration {
    secret_backend_command: PathBuf,
    secret_backend_timeout: u64,
}
let mut command = Command::new(&self.config.secret_backend_command)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
```

Users who rely on passing arguments to their secret backend binary will find this unsupported in ADP.

### sender_backoff_base

In the Agent, `sender_backoff_base` is a sub-key used with a prefix (e.g. `logs_config.sender_backoff_base`). It controls the base duration for exponential backoff in the logs sender HTTP client. Default is 1.0 seconds.

In Saluki, the equivalent retry/backoff configuration exists but uses the `forwarder_backoff_base` key name (from the metrics forwarder path), not `sender_backoff_base`. Saluki's `RetryConfiguration` in `retry.rs` uses `forwarder_backoff_base` with a default of 2.0 seconds. Since these are logs-sender-specific keys and Saluki does not have a separate logs sender pipeline with these config keys, they are missing.

### sender_backoff_factor

In the Agent, `sender_backoff_factor` is a logs-sender-specific sub-key (e.g. `logs_config.sender_backoff_factor`). It controls the randomness/jitter factor for the exponential backoff. Default is 2.0.

Saluki has `forwarder_backoff_factor` for its general forwarder retry logic but no `sender_backoff_factor` for a logs sender pipeline.

### serializer_max_metrics_per_payload

This config key exists only in Saluki/ADP, not in the Datadog Agent.

In Saluki, it controls the maximum number of input metrics encoded into a single request payload for both series and sketches endpoints:
```rust
#[serde(
    rename = "serializer_max_metrics_per_payload",
    default = "default_max_metrics_per_payload"
)]
max_metrics_per_payload: usize,
```
Default is 10,000. It is wired into `RequestBuilder::with_max_inputs_per_payload()`.

The Agent has `serializer_max_series_points_per_payload` (default 10,000) which serves a similar role for series, but there is no single unified key covering both series and sketches. The ADP key is a simplification that unifies the limit.

### serializer_max_payload_size

In the Agent, this controls the maximum compressed payload size and defaults to 2.5 MiB (2*megaByte + megaByte/2 = 2,621,440 bytes):
```go
config.BindEnvAndSetDefault("serializer_max_payload_size", 2*megaByte+megaByte/2)
```

Saluki does not read this config key. Instead, it uses hardcoded constants per endpoint type:
- Sketches: `DEFAULT_INTAKE_COMPRESSED_SIZE_LIMIT = 3,200,000` (3 MiB)
- Series: `SERIES_V2_COMPRESSED_SIZE_LIMIT = 512,000` (500 KiB)

The Agent's single configurable value applies broadly to non-series payloads, while Saluki hardcodes distinct limits. The sketches limit in Saluki (3.2 MB) differs from the Agent default (2.5 MB), meaning the key is missing and the effective behavior also diverges.

### serializer_max_uncompressed_payload_size

In the Agent, this controls the maximum uncompressed payload size and defaults to 4 MiB:
```go
config.BindEnvAndSetDefault("serializer_max_uncompressed_payload_size", 4*megaByte)
```

Saluki does not read this config key. Instead, for non-series payloads (sketches, events, service checks, logs, traces), it uses:
```rust
pub const DEFAULT_INTAKE_UNCOMPRESSED_SIZE_LIMIT: usize = 62_914_560; // 60 MiB
```

This is a significant difference: Saluki allows payloads up to 60 MiB uncompressed vs the Agent's 4 MiB. The series endpoint uses `SERIES_V2_UNCOMPRESSED_SIZE_LIMIT = 5,242,880` (5 MiB) which matches the Agent's `serializer_max_series_uncompressed_payload_size` default. The config key is not only missing but the effective hardcoded default for non-series payloads is 15x larger.

### serializer_zstd_compressor_level

Both implementations read this config key.

In the Agent, the default is 1:
```go
DefaultZstdCompressionLevel = 1
config.BindEnvAndSetDefault("serializer_zstd_compressor_level", DefaultZstdCompressionLevel)
```

In Saluki, the default is 3:
```rust
const fn default_zstd_compressor_level() -> i32 {
    3
}
#[serde(rename = "serializer_zstd_compressor_level", default = "default_zstd_compressor_level")]
zstd_compressor_level: i32,
```

Saluki reads this key in multiple encoder configs (metrics, events, service_checks, logs, traces). The higher default means ADP will use more CPU for compression by default but produce smaller payloads. Users who haven't explicitly set this key will see different compression behavior.

### skip_ssl_validation

In the Agent, this boolean (default false) disables TLS certificate verification for outgoing HTTPS connections:
```go
config.BindEnvAndSetDefault("skip_ssl_validation", false)
```
It is also overridden by FIPS mode (`fips.tls_verify`).

Saluki has the `danger_accept_invalid_certs()` capability in `ClientTLSConfigBuilder`, but it is only used programmatically in specific places (proxy connections and CLI IPC connections), not driven by the `skip_ssl_validation` config key. The main forwarder HTTP client in `TransactionForwarder::from_config` does not wire this config key, so users cannot disable TLS validation for metric forwarding via configuration.

### statsd_metric_blocklist

In the Agent, `statsd_metric_blocklist` is a string slice that specifies metric names to block from processing. It is the legacy name for `metric_filterlist`; the newer key takes precedence when set. It is read in `comp/filterlist/impl/filterlist.go`:

```go
filterlist = config.GetStringSlice("statsd_metric_blocklist")
filterlistPrefix = config.GetBool("statsd_metric_blocklist_match_prefix")
```

The filter list can also be updated dynamically via Remote Config.

In Saluki, no metric blocklist/filterlist mechanism exists. There is no equivalent configuration key or filtering logic.

### statsd_metric_namespace

In the Agent, `statsd_metric_namespace` sets a string prefix prepended to every DogStatsD metric name (a `.` separator is auto-appended). Read in `comp/dogstatsd/server/server.go`:

```go
metricPrefix := cfg.GetString("statsd_metric_namespace")
if metricPrefix != "" && !strings.HasSuffix(metricPrefix, ".") {
    metricPrefix = metricPrefix + "."
}
```

Saluki has no equivalent namespace-prefixing feature.

### statsd_metric_namespace_blacklist

In the Agent, this string slice lists metric prefixes that should NOT have the `statsd_metric_namespace` prefix applied. Default is `StandardStatsdPrefixes` (datadog.agent, datadog.dogstatsd, jvm, kafka, etc.). Read in `comp/dogstatsd/server/server.go`:

```go
metricPrefixBlacklist := cfg.GetStringSlice("statsd_metric_namespace_blacklist")
```

Saluki has no namespace prefixing and therefore no blacklist either.

### statsd_metric_namespace_blocklist

This key does not actually exist in the Agent codebase. Only `statsd_metric_namespace_blacklist` is registered and read. This appears to be an assumed "blocklist" variant that was never implemented. Neither the Agent nor Saluki use this key.

### tag_value_split_separator

In the Agent, `tag_value_split_separator` is a `map[string]string` that maps tag names to separator characters. When a tag value contains the separator, it is split into multiple tags. Used by the tagger in `comp/core/tagger/taglist/taglist.go` and host tags in `comp/metadata/host/hostimpl/hosttags/tags.go`:

```go
splitList := conf.GetStringMapString("tag_value_split_separator")
```

This is a global agent setting, not DogStatsD-specific. Saluki has no equivalent.

### tags

In the Agent, `tags` (DD_TAGS) is a global string slice of tags applied across all components. For DogStatsD specifically, it is included in `extraTags` only when running in Fargate environments, via `GetStaticTagsSlice` which calls `GetConfiguredTags`:

```go
if staticTags := tagutil.GetStaticTagsSlice(context.TODO(), cfg); staticTags != nil {
    extraTags = append(extraTags, staticTags...)
}
```

Saluki uses `dogstatsd_tags` (via serde rename) for additional tags, but does not read the global `tags` config key. In non-Fargate contexts, the Agent also does not apply `tags` to DogStatsD metrics directly.

### telemetry.enabled

In the Agent, `telemetry.enabled` (default `false`) is a global toggle that gates all internal telemetry, including DogStatsD origin telemetry:

```go
// comp/dogstatsd/server/server.go
originTelemetry: cfg.GetBool("telemetry.enabled") &&
    cfg.GetBool("telemetry.dogstatsd_origin"),
```

In ADP, the analogous setting is `data_plane.telemetry_enabled` (default `false`), read via `try_get_typed("data_plane.telemetry_enabled")`. This controls internal observability (Prometheus endpoint, remote agent telemetry provider), but ADP does not read `telemetry.enabled` at all.

The user-visible impact is that the same YAML key (`telemetry.enabled`) will not toggle telemetry in ADP; operators must use `data_plane.telemetry_enabled` instead.

### use_compression

In the Agent, `use_compression` is registered by `bindEnvAndSetLogsConfigKeys` with various prefixes (default `true`). It controls whether outgoing payloads are compressed. The serializer path for metrics uses a separate `serializer_compressor_kind` key instead.

In ADP, compression is always enabled for metric payloads via `serializer_compressor_kind` (default `zstd`) and `serializer_zstd_compressor_level`. There is no `use_compression` toggle to disable compression entirely. The user cannot set `use_compression: false` to disable payload compression in ADP.

### use_dogstatsd

In the Agent, `use_dogstatsd` (default `true`) is the master toggle for DogStatsD. When ADP is running, the Agent checks both `use_dogstatsd` and `data_plane.dogstatsd.enabled` to determine which component handles DSD traffic:

```go
// comp/dogstatsd/config/config.go
func (c *Config) Enabled() bool {
    return c.config.GetBool("use_dogstatsd")
}
```

In ADP, the DogStatsD enable/disable is controlled solely by `data_plane.dogstatsd.enabled` (default `false`). ADP does not read `use_dogstatsd`. If an operator sets `use_dogstatsd: false` expecting DogStatsD to stop everywhere, ADP will not respect that setting.

### use_v2_api.series

In the Agent, `use_v2_api.series` (default `true`) controls whether the serializer uses the v1 or v2 API for time series metrics:

```go
// pkg/serializer/serializer.go
useV1API := !s.config.GetBool("use_v2_api.series")
```

In ADP, the metrics encoder always uses the v2 series protobuf format. There is no configuration to fall back to the v1 JSON-based series API. This is fine for normal operation since v2 has been the default, but operators cannot force v1 API usage through ADP.

### zstd_compression_level

In the Agent, `zstd_compression_level` is registered by `bindEnvAndSetLogsConfigKeys` with various prefixes (e.g., `logs_config.zstd_compression_level`). For the metrics serializer, the Agent uses `serializer_zstd_compressor_level` instead.

In ADP, the compression level is controlled by `serializer_zstd_compressor_level` (default 3), matching the Agent's serializer behavior. However, ADP does not read the `zstd_compression_level` key that the Agent uses for log/forwarder endpoints. The metrics path is functionally equivalent, but the log-forwarding compression level config key differs.

## Action Items

Here we will create a check-box list of features to implement perhaps?
