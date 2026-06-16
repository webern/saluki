# Mapless Configuration Translation System — spike status

Implements `datadog/projects/confra/design/mapless.md` on branch `m/confra-spike-2c`, starting from
`361fabd92` (generated `DatadogConfiguration` deserializer).

## What is built and verified

The configuration translation boundary is realized end-to-end and the **whole workspace compiles**
(`cargo build --workspace`). Three new crates plus the witness driver and an IPC split:

- `lib/saluki-component-config` (step 1): component-native config structs as plain data. Leaf; no
  Datadog key names, no `GenericConfiguration`, no `saluki-components` dependency. Eligible
  translation target for OTel/OPW/SalukiNative.
- `lib/agent-data-plane/config` (step 2): the ADP-native target model — `SalukiConfiguration`
  (embeds the leaf structs), `BootstrapConfiguration`, `SalukiPrivateConfiguration`,
  `RuntimeConfigAuthority`/`RuntimeConfigLanguage`, native `RuntimeLoggingConfig`. Depends only down
  on `saluki-component-config`.
- `datadog-agent-config` witness (step 3): `build.rs` now generates `DatadogConfigConsumer` (one
  `consume_<key>` method per `support: full`/`partial` key — 144 today) plus `drive()`. No default
  method bodies, so the translator must give every supported key a destination or it fails to
  compile. Generated file: `src/witness.rs`.
- `datadog-agent-commons` (step 4): `RemoteAgentClient::connect(typed_config)` split out from
  `from_configuration`; typed constructors added to `RemoteAgentClientConfiguration` and
  `IpcAuthConfiguration` so IPC config can be built without a raw map.
- `lib/agent-data-plane/config-system` (steps 5–7, 9): the adapter — the only crate that touches
  `GenericConfiguration`.
  - `bootstrap.rs`: `BootstrapInputs`, local source loading, `BootstrapConfiguration` parsing,
    authority selection (standalone → `LocalSnapshot(DatadogAgent)`, otherwise
    `ConfigStream(DatadogAgent)`), pipeline-gate reading.
  - `datadog_agent.rs`: `DatadogAgentConnection` — the config-authority half of the old
    `RemoteAgentBootstrap` (connect, register, session handle, config stream creation).
  - `stream.rs`: `ConfigStreamHandle`.
  - `translate.rs`: the witness-driven `Translator` (implements all 144 `consume_*` methods) plus
    `translate_datadog`. Folds `SalukiPrivateConfiguration` in. Tested.
  - `system.rs`: `ConfigurationSystem::start()` — the single startup seam returning
    `StartedConfigurationSystem { bootstrap, saluki, attachments }`.
  - `dynamic.rs`: `ConfigUpdateRouter` + typed `ScopedConfigHandle`s. Resolves the design's open
    routing question with re-translate-snapshot/diff/route. Tested: forwarder API-key refresh and
    log-level paths fire while an unrelated DogStatsD slice provably does not change.
  - `examples/startup_collapse.rs`: the realized `run.rs` collapse seam, compiling against the real
    API.

Tests: `cargo test -p agent-data-plane-config-system` (3 tests) pass.

### Invariants honored

- Dependency arrows point down; no generic crate depends up on an ADP/Datadog crate.
- `GenericConfiguration` appears only inside `config-system`.
- No `from_native(total, generic)` hybrid signatures.
- Every supported Datadog key has a compile-enforced destination via the witness.

## Steps 8 and 10: cutover wired into the binary (data path), with transitional remainders

The binary now builds the bulk of its data topology from a translated `SalukiConfiguration` and the
whole workspace compiles. `run.rs` calls
`agent_data_plane_config_system::translate_from_generic(&config, …)` to obtain `SalukiConfiguration`
and constructs these components via native `from_native(&slice)` (no `GenericConfiguration`):

- Datadog forwarder; metrics/logs/events/service-checks encoders; checks IPC source.
- Full DogStatsD pipeline: source, prefix filter, mapper, tag filterlist, aggregate, post-aggregate
  filter.
- Native OTLP source.

Each migrated component grew a `from_native` constructor in `saluki-components` (or the bin); the
old `from_configuration` is retained behind `#[allow(dead_code)]` during the transition.

### `run.rs` is GenericConfiguration-free; the shell is relocated to `cli/runtime.rs`

`bin/agent-data-plane/src/cli/run.rs` now contains only the `RunCommand` definition (zero
`GenericConfiguration` references). The runtime orchestration moved to `cli/runtime.rs`, which builds
the data topology from the native `SalukiConfiguration` but still consumes `GenericConfiguration` for
the runtime shell (below). This relocation keeps the command surface typed; it does not by itself
make the shell native — that is the remaining work, and `runtime.rs` documents it inline. Decisions
are logged in `datadog/projects/confra/design/spike-2c-claude.md`.

### Now native (no longer raw-config) — added after the initial cutover

- **Overlay classifier validation** moved into `config-system` (`validate_against_overlay`, run by
  `translate_from_generic`), per the design. `runtime.rs` no longer flattens/classifies source keys.
- **Memory bounds** are native: `SalukiConfiguration::memory` (`MemoryConfig`) is populated by the
  config system, and `runtime.rs` builds `MemoryBoundsConfiguration::from_parts(...)` from it.
- **Trace transforms** (sampler, obfuscation, APM stats) and the **traces encoder** are now built via
  `from_native` (Default + native fields — a documented fidelity reduction for the heavy
  `ApmConfig`/`ObfuscationConfig` rule sets).

### Transitional remainders (still consume the raw configuration in `runtime.rs`)

These are the honest gaps to a *no-concessions* GenericConfiguration-free binary. `run.rs` itself is
free of it; these live in `cli/runtime.rs`:

- **Bootstrap/authority resolution** (`DataPlaneConfiguration::from_configuration`, the
  `RemoteAgentBootstrap` + dynamic-config-stream dance). Replaced in the end state by
  `ConfigurationSystem::start()`.
- **Environment provider** (host/workload/autodiscovery) and **internal supervisor** (`ConfigWorker`,
  `DynamicLogLevelWorker`, `IpcAuthConfiguration` + server TLS). The env/host/workload providers and
  the workload collectors (tagger/workloadmeta, plus containerd/cgroups/feature-detector/PID-resolver)
  each construct their own IPC client and read config pervasively — routing them through the shared
  `DatadogAgentConnection` is the design's open question and a genuine `saluki-env` rewrite.
- **Host tags** (queries the Agent; native path is a disabled stub pending the shared connection).
- **MRF** and the **OTLP proxy branch**: MRF's gateway retains a map for runtime watching; the OTLP
  proxy gRPC endpoint isn't a witnessed key. Both need per-component watcher/translator work to go
  native (the scoped `ScopedConfigHandle`s from `dynamic.rs` are the replacement for the watchers).

### To finish (no-concessions end state)

- Give the trace transforms / encoder full native config types and `from_native`; widen the
  obfuscation/mapper/tag-filterlist native shapes to full fidelity.
- Resolve the shared `DatadogAgentConnection` question, then add
  `ADPEnvironmentProvider::from_saluki_configuration` and `create_internal_supervisor_from_saluki`
  consuming `SalukiConfiguration` + `StartedAttachments`; move the status/flare/telemetry service
  impls onto the typed `DatadogAgentConnection`.
- Move MRF + OTLP-proxy onto native config; wire the `ScopedConfigHandle`s from `dynamic.rs` into the
  forwarder/prefix-filter/tag-filterlist components to replace their retained-map watchers.
- Collapse `run.rs` to the `examples/startup_collapse.rs` shape (`ConfigurationSystem::start()` →
  typed outputs only), delete the `remote_agent_enabled` / `use_new_config_stream_endpoint` gates,
  and remove the `#[allow(dead_code)]` `from_configuration` constructors.

### Known spike simplifications

- Local loading omits `KEY_ALIASES` / `DatadogRemapper` (source-precedence migration detail).
- Native obfuscation, mapper-profile, and tag-filterlist entry shapes are summarized, not exhaustive.
- ~60 of the 144 witness methods are explicit no-ops for keys ADP does not yet model natively (the
  witness still requires the method to exist — that is the point).
