# Analyze Features

Substep of `/dogstatsd-audit`. You receive a batch of config keys and perform clean room analysis of
each one for DogStatsD feature parity between RefImpl (datadog-agent) and AdpImpl
(agent-data-plane/Saluki).

Search BOTH codebases for every key regardless of what the supervising agent tells you.

## Input

- `{{datadog-agent}}` = path to datadog-agent repo
- `{{saluki}}` = path to Saluki repo
- `{{outdir}}/{{outfile}}` = your output filepath
- A batch of config keys (provided by the supervising agent)

## Per-Key Analysis

### Where to search

**RefImpl** (`{{datadog-agent}}`):
- `pkg/config/setup/` — registration: `BindEnvAndSetDefault`, `SetDefault`, `BindEnv`
- `comp/dogstatsd/` — runtime reads: `GetString`, `GetBool`, `GetInt`, etc.
- Any other `.go` file referencing the key

**AdpImpl** (`{{saluki}}`):
- `#[serde(rename = "key")]` on Deserialize structs
- `get_typed("key")`, `try_get_typed("key")`, `get_typed_or_default("key")`
- Bare field names on Deserialize structs (field name = key when no rename)

Do a deep code analysis on how the configuration setting affects both systems.

### Determine Status

- **Implemented**: Exists in both, affected behavior functionally equivalent.
- **Missing**: Exists in RefImpl but not AdpImpl.
- **Divergent**: Exists in both, behavior differs meaningfully.
- **ADP Only**: Exists in AdpImpl but not RefImpl.

Commit to a status. If you truly cannot determine equivalence with confidence after thorough
analysis, use **Unsure** — but this should be rare.

### Write Outputs

**Description** (required, max 32 chars): Terse summary of what the key controls.
Examples: `UDP listen port`, `Tag cardinality for origin`, `Max cached DSD contexts`

**Notes** (optional, max 32 chars): Only for Divergent or surprising cases. Blank otherwise.
Examples: `ADP default differs: 256 vs 128`, `ADP ignores when standalone`

**Discussion** (optional, null for most keys): Only for noteworthy features — divergent behavior,
surprising omissions, subtle semantic differences. Include code snippets from both sides and explain
user-visible impact. Keep focused.

## Output Format

JSON array, one object per key:

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
    "ConfKey": "dogstatsd_buffer_size",
    "Status": "Divergent",
    "Description": "Receive buffer size (bytes)",
    "Notes": "ADP default 8192 vs Agent 4096",
    "Discussion": "### dogstatsd_buffer_size\n\nIn the Agent...\n```go\n// code\n```\n\nIn ADP...\n```rust\n// code\n```\n\nThe difference is..."
  }
]
```

- `Description`: non-empty, max 32 chars
- `Notes`: max 32 chars, empty string if not needed
- `Discussion`: `null` or markdown string starting with `### key_name`
