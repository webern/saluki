# Agent guidelines for Saluki

## What is Saluki

Saluki is a high-performance telemetry data plane written in Rust. It receives metrics, logs,
traces, and events via protocols like DogStatsD and OTLP, processes them through a configurable
pipeline, and forwards them to Datadog. The main binary is `agent-data-plane` (ADP).

## Guiding principles

- Correctness and safety over performance. Make it work, then make it fast — performance
  without correctness is unacceptable.
- Documentation is not optional. Saluki must be easy to understand and maintain by humans
  and LLMs alike.
- Communicate before committing. Surface plans, trade-offs, and open questions early —
  especially for architectural decisions and non-trivial refactors.

## Project layout

```
bin/
  agent-data-plane/       # Main production binary (entry point: src/main.rs)
  correctness/            # Test binaries: panoramic, ground-truth, millstone, etc.
lib/
  saluki-core/            # Component traits, topology, data model, runtime
  saluki-components/      # Concrete component implementations (sources, transforms, etc.)
  saluki-app/             # Application bootstrap, logging, metrics, memory, API
  saluki-config/          # Config loading (figment-based: YAML + env + secrets)
  saluki-context/         # Context resolution and string interning
  saluki-io/              # Network I/O, codecs, listeners
  saluki-error/           # GenericError (anyhow wrapper) + ErrorContext trait
  saluki-env/             # Platform abstractions (Linux/macOS)
  saluki-metadata/        # Build metadata (version, git hash via build.rs)
  saluki-metrics/         # Metrics utilities
  saluki-tls/             # TLS support (including FIPS mode)
  protos/                 # Protobuf definitions: datadog, containerd, otlp
  ddsketch/               # DDSketch histogram implementation
  stringtheory/           # String manipulation utilities
  ottl/                   # OpenTelemetry Transform Language parser/evaluator
test/
  integration/            # Docker-based integration tests (Panoramic framework)
  correctness/            # Deterministic correctness test cases
  smp/                    # Single Machine Performance benchmarks
```

## Architecture

### Component pipeline

Saluki processes data through a directed acyclic graph of typed components. There are
7 component types, forming two parallel paths:

**Event path:** Source → Transform → Destination
**Payload path:** Relay → Decoder → Transform → Encoder → Forwarder

Each component type has a trait in `saluki-core/src/components/` and concrete implementations
in `saluki-components/src/`. Components are wired together via `TopologyBlueprint`, validated
as a DAG, built, then spawned as independent tokio tasks.

### Adding a new component

1. Define a config struct deriving `Deserialize` in the appropriate `saluki-components` subdirectory
2. Implement the builder trait (e.g., `SourceBuilder`) — declares outputs/inputs and builds
   the component
3. Implement the component trait (e.g., `Source`) — the `async fn run()` loop
4. Register via `TopologyBlueprint::add_source()` (or equivalent) in the binary

Follow existing components as templates. DogStatsD source
(`lib/saluki-components/src/sources/dogstatsd/mod.rs`) is a comprehensive example.

### Data model

Core event types are in `saluki-core/src/data_model/event/`:
- `Event` enum: Metric, EventD, ServiceCheck, Log, Trace, TraceStats
- `Payload` enum: Raw, Http, Grpc
- `EventType` bitmask validates component compatibility at graph build time

### Configuration flow

YAML file → environment variables → secrets resolution → `figment` → serde `Deserialize`
→ typed config struct → component builder. Config structs use `#[serde(default = "...")]`
for defaults. No `Configurable` trait — just derive `Deserialize`.

### Error handling

`GenericError` (anyhow wrapper) is the standard error type. Use `snafu` for component-specific
error enums. Propagate context with `.error_context("what failed")`.

### Async runtime

Tokio multi-threaded runtime. Components run as spawned tasks managed by `JoinSet`.
Shutdown is coordinated: sources stop first, then downstream components drain.

## Build and test

### Commands

Use `make` targets for all standard workflows:

| Command | Purpose |
|---------|---------|
| `make build-adp` | Debug build |
| `make build-adp-release` | Release build |
| `make test` | Unit tests (excludes property tests) |
| `make test-property` | Property-based tests (release mode) |
| `make test-all` | Full suite: unit, property, doc, miri, loom |
| `make fast-edit-test` | Quick validation: format + lint + deny + tests |
| `make check-all` | All checks: format, clippy, features, deny, licenses |
| `make fmt` | Format code (uses nightly rustfmt) |
| `make check-fmt` | Check formatting |
| `make check-clippy` | Run clippy |

### Tooling details

- **Test runner:** `cargo nextest`, not `cargo test`. The Makefile handles this.
- **Formatter:** `cargo +nightly fmt` — nightly is required for the rustfmt config.
- **Rust toolchain:** Pinned in `rust-toolchain.toml` (currently 1.93.0).
- **Protobuf updates:** `make update-protos` (vendored definitions with pinned versions).

## Coding conventions

### Rustdoc

All public items **must** be documented — enforced by `#![deny(missing_docs)]`.

- First line: complete sentence summarizing what the item is or does
- Structure: brief summary, blank line, detailed explanation, then formal sections
- Use `# Errors`, `# Panics`, `# Examples`, `# Design`, `# Missing` as appropriate

Configuration fields **must** document:

- What the field controls and its impact on behavior
- The default value (explicitly stated)
- Edge cases and boundary values (e.g., "If set to `0`, X is disabled")
- Guidance for who should change it (e.g., "high-throughput workloads may increase this")

Trade-offs: name both sides explicitly, quantify when possible, acknowledge workload dependency.

Error messages: describe what went wrong in plain language, provide actionable guidance,
reference specific config fields.

### Formatting and linting

- Max line width: 120 (`rustfmt.toml`)
- Import grouping: std, then external, then crate (`group_imports = "StdExternalCrate"`)
- Clippy allows: `new_without_default`, `uninlined_format_args`, `map_entry`, `mutable_key_type`
- Clippy threshold: `too-many-arguments-threshold = 8`
- Cargo.toml entries sorted (`cargo sort --workspace`)

### PR conventions

- Title format: `scope: description` (scopes validated by CI)
- Fill in the PR template: summary, change type, test plan, references
- Run `make fast-edit-test` before pushing

## Technical documentation

### Voice and tone

- Use "you" for instructions, "we" for project-collective statements
- Write as a knowledgeable colleague — direct and practical, not curt or condescending
- Use active voice: "Run the command", not "The command should be run"
- Use present tense: describe what things do, not what they will do
- Avoid "please" in instructions, "simply/easy/just", excessive exclamation marks, jargon,
  and pop culture references

### Markdown formatting

- Headings: sentence case; never skip levels; H1 for page title only
- Code formatting: backticks for code, commands, file paths, config keys, type names
- Emphasis: bold for UI elements and key terms on first use; italics sparingly
- Lists: numbered for sequential steps; bullets for non-sequential items
- Conditions first: "To enable debug mode, set `debug: true`" not the reverse
- Admonitions: `> [!WARNING]`, `> [!NOTE]`, `> [!TIP]`
- Links: descriptive text (not "click here"); relative paths for internal docs
- Code blocks: always specify language; keep examples minimal and focused

### Requirement levels

Use RFC 2119 keywords sparingly and in bold when normative:

- **must**: absolute requirement
- **should**: strongly recommended, but valid exceptions exist
- **may**: truly optional
