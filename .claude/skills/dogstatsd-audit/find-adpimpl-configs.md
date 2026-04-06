# Find AdpImpl Configs

This is a substep of the `/dogstatsd-audit` skill. Your job is to discover the configuration keys
(ConfKeys) that are used by agent-data-plane and its components at runtime.

## Input

- `{{saluki}}` = the path to the root of the Saluki codebase repository
- `{{tmp}}` = your output directory

## System Overview

`bin/agent-data-plane` is the binary whose configuration we are concerned with. It composes
components found in `lib/saluki-components` and uses other libs found in `lib/*`. The core
configuration primitives are in `lib/saluki-config`.

Configuration values originate from a `datadog.yaml` file and/or `DD_`-prefixed environment
variables. At runtime they are stored in a flat key-value bag and accessed by components via
`GenericConfiguration`.

## How to Identify Configuration Keys

There are two patterns. You must search for both.

### Pattern 1: Serde rename on Deserialize structs

Config structs derive `Deserialize` and use `#[serde(rename = "...")]` to map fields to config key
names. The struct is loaded as a whole via `config.as_typed::<T>()`.

```rust
#[derive(Deserialize)]
pub struct DogStatsDConfiguration {
    #[serde(rename = "dogstatsd_port", default = "default_port")]
    port: u16,

    #[serde(rename = "dogstatsd_buffer_size", default = "default_buffer_size")]
    buffer_size: usize,

    // flatten embeds another struct's keys into this one
    #[serde(flatten)]
    origin_enrichment: OriginEnrichmentConfiguration,
}
```

**The config key is the string inside `rename = "..."`.**

If a field has NO `rename` attribute, the Rust field name itself is the key. For example
`pub api_key: String` without a rename means the key is `api_key`.

**Important:** `#[serde(flatten)]` means a sub-struct's fields are inlined. You must follow these
to find all keys — they won't appear in the outer struct.

**Search command:** Grep all `.rs` files under `lib/` and `bin/agent-data-plane/` for:
```
#[serde(rename = "
```

For each match, extract the string literal as the ConfKey and record file:line as the location.

Also check for `Deserialize` structs loaded via `as_typed` that have fields WITHOUT `rename` — the
field name is the key in those cases.

### Pattern 2: Manual key queries

Some config structs don't derive `Deserialize`. Instead, their `from_configuration()` method calls
accessor functions on `GenericConfiguration` with string literal key names.

```rust
// Examples of the three accessor functions:
config.get_typed("api_key")?
config.try_get_typed("data_plane.enabled")?.unwrap_or(false)
config.get_typed_or_default("log_level")
```

**The config key is the string literal passed to the function.**

Dotted keys like `data_plane.otlp.enabled` represent YAML nesting:
```yaml
data_plane:
  otlp:
    enabled: true
```

**Search commands:** Grep all `.rs` files under `lib/` and `bin/agent-data-plane/` for each of:
```
get_typed("
try_get_typed("
get_typed_or_default("
```

For each match, extract the string literal as the ConfKey and record file:line as the location.

### What NOT to include

- Test files (`#[cfg(test)]` modules, files in `tests/` directories)
- Keys that appear only in comments or doc strings
- Internal framework keys that are not user-facing configuration

## Output

Write to `{{tmp}}/adpimpl-config-keys.csv`. Each line is a quoted ConfKey and its file:line location,
relative to `{{saluki}}`:

```csv
"dogstatsd_buffer_size","lib/saluki-components/src/sources/dogstatsd/mod.rs:157"
"dogstatsd_port","lib/saluki-components/src/sources/dogstatsd/mod.rs:175"
"data_plane.enabled","bin/agent-data-plane/src/config.rs:35"
```

If the same ConfKey appears in multiple locations, include the most authoritative one — prefer the
declaration site (serde rename or struct definition) over a secondary read site.

## Getting Additional Context

You are running as a subagent and may ask questions from the supervising agent or user if you need
more context.

## Completion

Report to the supervising agent when your work is complete.