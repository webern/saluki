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

## Step 1: Discover the Config API Surface

Before searching for usages, read the `GenericConfiguration` impl in `lib/saluki-config/src/lib.rs`
to discover **every public method** that takes a config key string argument. As of this writing, the
known methods are `get_typed`, `try_get_typed`, `get_typed_or_default`, `as_typed`, and
`watch_for_updates` — but new methods may have been added. Build your own complete list from the
source.

Also look for any wrapper functions or helpers elsewhere in the codebase that delegate to
`GenericConfiguration`. For example, search for functions whose body calls one of the methods you
found above — these wrappers may be the actual call sites in component code.

## Step 2: Search for Configuration Keys

There are two families of patterns. You must search for both.

### Pattern A: Serde rename on Deserialize structs

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

**Search:** Grep all `.rs` files under `lib/` and `bin/agent-data-plane/` for `serde(rename`.
For each match, extract the string literal as the ConfKey and record file:line as the location.

Also check for `Deserialize` structs loaded via `as_typed` that have fields WITHOUT `rename` — the
field name is the key in those cases.

### Pattern B: Manual key queries

Some config structs don't derive `Deserialize`. Instead, their `from_configuration()` method calls
accessor functions on `GenericConfiguration` with string literal key names.

```rust
// Examples using the methods known at time of writing:
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

**Search:** Using the full method list you discovered in Step 1, grep all `.rs` files under `lib/`
and `bin/agent-data-plane/` for each method name followed by `("`. For each match, extract the
string literal as the ConfKey and record file:line as the location.

## Step 3: Validate Completeness

After collecting keys from Steps 2A and 2B, do a sanity check:

- Search for any string literals that look like config keys (lowercase, underscores or dots, no
  spaces) being passed to a `GenericConfiguration` or similar type that you may have missed.
- Spot-check 2-3 component `mod.rs` or `config.rs` files to see if there's a pattern you haven't
  accounted for.
- If you find a new pattern, go back and search for it comprehensively.

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