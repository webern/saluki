# Analyze Features

This is a substep of the `/dogstatsd-audit` skill. You receive a batch of configuration keys and
must independently analyze each one to determine its DogStatsD feature parity status between
RefImpl (datadog-agent) and AdpImpl (agent-data-plane in Saluki).

## Input

- `{{datadog-agent}}` = path to the datadog-agent repository
- `{{saluki}}` = path to the Saluki repository
- A batch of config keys to analyze (provided by the supervising agent)

## Important: Clean Room

You have NO prior knowledge of whether a key exists on either side. The supervising agent may have
hints, but you must independently verify. For every key, search BOTH codebases regardless of what
you've been told.

## Per-Key Analysis Procedure

For each config key in your batch:

### 1. Search RefImpl

Search `{{datadog-agent}}` for the key. Look in:
- `pkg/config/setup/` for registration calls (`BindEnvAndSetDefault`, `SetDefault`, `BindEnv`, etc.)
- `comp/dogstatsd/` for runtime reads (`GetString`, `GetBool`, `GetInt`, etc.)
- Anywhere else it appears in `.go` files

If found, read the surrounding code to understand:
- What is the default value?
- What does it control? (trace the usage from config read to behavioral effect)
- Is it used directly by DogStatsD, or indirectly via shared infrastructure?

### 2. Search AdpImpl

Search `{{saluki}}` for the key. Look in:
- `#[serde(rename = "key")]` attributes on Deserialize structs
- `get_typed("key")`, `try_get_typed("key")`, `get_typed_or_default("key")` calls
- Field names on Deserialize structs (if no rename, the field name is the key)
- Any other accessor pattern on `GenericConfiguration`

If found, read the surrounding code to understand:
- What is the default value?
- What does it control?
- How does the behavior compare to RefImpl?

### 3. Determine Status

Based on your analysis:

- **Implemented**: Key exists in both, behavior is functionally equivalent.
- **Missing**: Key exists in RefImpl but not in AdpImpl.
- **Divergent**: Key exists in both but behavior differs in a meaningful way.
- **ADP Only**: Key exists in AdpImpl but not in RefImpl.

### 4. Write Outputs

For each key, produce:

**Description** (required, max 32 characters): A terse plain-English summary of what the key
controls. Examples:
- `UDP listen port`
- `Receive buffer size (bytes)`
- `Tag cardinality for origin`
- `Max cached DSD contexts`

**Notes** (optional, max 32 characters): Only populate if the status is Divergent or something is
otherwise surprising/noteworthy. Leave blank for straightforward Implemented/Missing/ADP Only keys.
Examples:
- `ADP default differs: 256 vs 128`
- `ADP ignores when standalone`
- `` (blank for most keys)

**Discussion** (optional): Only write a discussion if the feature is noteworthy. This means:
- Divergent behavior that users should know about
- Missing features that are important or surprising
- Subtle differences in defaults, edge cases, or semantics
- ADP-only features that warrant explanation

A discussion should include relevant code snippets from both sides (if applicable), explain the
difference concretely, and note any user-visible impact. Keep it focused. Most keys will NOT need a
discussion.

## Output Format

Respond with a JSON array. One object per key:

```json
[
  {
    "ConfKey": "dogstatsd_port",
    "Status": "Implemented",
    "Description": "UDP listen port",
    "Notes": "",
    "Discussion": null
  },
  {
    "ConfKey": "dogstatsd_mapper_profiles",
    "Status": "Missing",
    "Description": "Metric name to tag mapping",
    "Notes": "",
    "Discussion": null
  },
  {
    "ConfKey": "dogstatsd_buffer_size",
    "Status": "Divergent",
    "Description": "Receive buffer size (bytes)",
    "Notes": "ADP default 8192 vs Agent 8192",
    "Discussion": "### dogstatsd_buffer_size\n\nIn the Agent, this controls...\n\n```go\n// agent code\n```\n\nIn ADP, this controls...\n\n```rust\n// adp code\n```\n\nThe difference is..."
  }
]
```

Rules:
- `Description` must be non-empty and at most 32 characters
- `Notes` must be at most 32 characters (empty string if not needed)
- `Discussion` is `null` for non-noteworthy keys, or a markdown string (starting with `### key_name`) for noteworthy ones
- Discussion markdown should use `###` heading (h3) for the key name

## Getting Additional Context

You are running as a subagent and may ask questions from the supervising agent if you need more
context.

## Completion

Return your JSON array to the supervising agent when done.
