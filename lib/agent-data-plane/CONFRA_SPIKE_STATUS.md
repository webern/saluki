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

## Remaining migration (steps 8 and 10 proper)

These rewrite the existing binary and `saluki-components` in lockstep and are a multi-PR migration,
not completed in this spike session:

- **Step 8 — component cutover.** For each of the ~32 component config structs (inventory: DogStatsD
  source/aggregate/mapper/prefix-filter/tag-filterlist/debug-log, OTLP source/relay/forwarder/decoder,
  Datadog forwarder, metrics/logs/events/service-checks/traces/stats encoders, traces
  enrich/sample/obfuscate, checks IPC): split the plain-data config from the builder, move the data
  type to `saluki-component-config` (or construct from the existing native slice), and replace
  `from_configuration(&GenericConfiguration)` with a pure `from_native(&NativeSlice)` constructor.
  The retained-`GenericConfiguration` capabilities (forwarder API-key refresh, MRF, dsd debug-log,
  prefix/tag watchers) are replaced by the typed `ScopedConfigHandle`s from `dynamic.rs`.
- **Step 10 — `run.rs` collapse.** Replace the current `run.rs` body with the
  `examples/startup_collapse.rs` shape: `ConfigurationSystem::start()` → typed outputs → topology +
  internal supervisor. Add `ADPEnvironmentProvider::from_saluki_configuration` and
  `create_internal_supervisor_from_saluki` consuming `SalukiConfiguration` + `StartedAttachments`,
  and move the Remote Agent service implementations (status/flare/telemetry) to consume the typed
  `DatadogAgentConnection`. Delete the `remote_agent_enabled` / `use_new_config_stream_endpoint`
  gates.

### Known spike simplifications

- Local loading omits `KEY_ALIASES` / `DatadogRemapper` (source-precedence migration detail).
- Native obfuscation, mapper-profile, and tag-filterlist entry shapes are summarized, not exhaustive.
- ~60 of the 144 witness methods are explicit no-ops for keys ADP does not yet model natively (the
  witness still requires the method to exist — that is the point).
