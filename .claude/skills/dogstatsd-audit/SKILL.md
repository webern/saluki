---
name: dogstatsd-audit
description: >
  Audit DogStatsD feature parity between datadog-agent and agent-data-plane,
  using configuration keys as feature identifiers.
disable-model-invocation: true
allowed-tools: Read, Write, MultiEdit, Grep, Glob, LS, Bash, Agent, Task, AskUserQuestion
---
# /dogstatsd-audit

Usage: `/dogstatsd-audit <prompt>`. The prompt is optional and can adjust behavior, limit scope, or
add context.

## Action: Path Resolution and Git Check

Resolve three repo paths. `{{saluki}}` is this repo's root. For `{{datadog-agent}}` and
`{{documentation}}`, check `{{saluki}}/../<name>` then `~/dd/<name>`. If not found, ask the user for
a custom path. If still unavailable, report which is missing and stop.

Show a table with: each repo's resolved path, HEAD commit (message + branch), and dirty status. Use
AskUserQuestion to confirm before proceeding.

You may store temporary files in `{{tmp}}`=`{{saluki}}/target/.temp/dogstatsd-audit`. Delete {{tmp}}
if exists. Create {{tmp}}

## Initial Definitions

- **ADP** (Agent Data Plane): The `agent-data-plane` binary and its components.
- **RefImpl** (Reference Implementation): The DogStatsD implementation in `datadog-agent`.
- **AdpImpl** (ADP Implementation): The DogStatsD implementation in ADP.
- **ConfKey** (Configuration Key): A `datadog-agent` configuration key. The primary index is
  `{{datadog-agent}}/pkg/config/common_settings.go`, but keys also appear throughout
  `{{datadog-agent}}/comp/dogstatsd/` and elsewhere

### FeatureState (used in later phases)

- **IMPLEMENTED**: Present in RefImpl and correctly implemented in AdpImpl.
- **ADP_ONLY**: Present in AdpImpl but not in RefImpl.
- **MISSING**: Present in RefImpl but not in AdpImpl.
- **DIVERGENT**: Present in both, but AdpImpl behavior differs from RefImpl.
- **UNSURE**: Present in both, but behavioral analysis is incomplete.

## Action: Gather Background Knowledge

Read these files to understand RefImpl. Follow references as needed.

- `{{documentation}}/content/en/extend/dogstatsd/_index.md` — feature overview, metric types
- `{{datadog-agent}}/pkg/config/setup/common_settings.go` — primary config key registry; DogStatsD
  keys are around lines 1523-1625
- `{{datadog-agent}}/pkg/config/model/types.go` — Reader/Setup interfaces (all Get*/Set* methods)
- `{{datadog-agent}}/comp/dogstatsd/server/server.go` — main server, reads config at runtime
- `{{datadog-agent}}/comp/dogstatsd/config/config.go` — DogStatsD config accessor

Read these files to understand AdpImpl:
- `{{saluki}}/bin/agent-data-plane/src/config.rs` — top-level ADP config struct
- `{{saluki}}/bin/agent-data-plane/src/cli/run.rs` — topology construction, config loading pipeline

Use AskUserQuestion: briefly summarize your understanding of the audit goal and the two
implementations. Ask the user to confirm or correct.

## Definition: ConfKey csv

A ConfKey csv file looks like this:

```csv
"dogstatsd_tag_cardinality","{{datadog-agent}}/pkg/config/setup/common_settings.go:536"
"system_probe_config.internal_profiling.enabled","{{datadog-agent}}/pkg/config/setup/system_probe.go:109"
```

## Action: Discover

**Collect ALL ConfKeys across the entire codebase, not just DogStatsD-related ones.** DogStatsD keys
can't always be identified by name alone — filtering happens in a later phase.

Create a sub-agent for each task. Store output in `{{tmp}}`.

- Find all ConfKeys in {{datadog-agent}} by running ./find-refimpl-configs.md
- Find all ConfKeys in {{saluki}} by running ./find-adpimpl-configs.md

- AskUserQuestion - give the user the output filenames and ask the user if they look OK before
proceeding.

Combine the files. For keys found in multiple locations, prefer the most authoritative source:

- RefImpl: `common_settings.go` > `pkg/config/` > `cmd/agent/dist/datadog.yaml` > docs
- AdpImpl: config structs > call sites

### known-configs.csv

`known-configs.csv` lives alongside this skill file and is the persistent ledger of all classified
keys. It has three fields, all quoted:

```csv
"config-key","relevant","reason"
"dogstatsd_port","true","directly configures DogStatsD listener"
"security_agent.enabled","false","security agent only, no DogStatsD path"
```

- `relevant=true`: the key could affect DogStatsD behavior. These are the focus of downstream
  analysis.
- `relevant=false`: the key is clearly unrelated to DogStatsD. These are ignored in downstream
  analysis.
- A key NOT present in `known-configs.csv` is **unreviewed** and needs classification.

### Building all-conf-keys.json

Cross-reference the discovered keys against `known-configs.csv`. Write to
`{{tmp}}/all-conf-keys.json`. Include keys where `relevant=true` AND keys not yet in
`known-configs.csv` (unreviewed). Exclude keys where `relevant=false`.

Each entry includes a `"Status"` field: `"known"` or `"unreviewed"`.

```json
[
	{
		"ConfKey": "histogram_aggregates",
		"Status": "known",
		"RefImpl": null,
		"AdpImpl": "lib/saluki-components/src/transforms/aggregate/config.rs:79"
	},
	{
		"ConfKey": "dogstatsd_workers_count",
		"Status": "known",
		"RefImpl": "pkg/config/setup/common_settings.go:1596",
		"AdpImpl": null
	},
	{
		"ConfKey": "some_new_key",
		"Status": "unreviewed",
		"RefImpl": "pkg/config/setup/common_settings.go:400",
		"AdpImpl": null
	}
]
```

## Action: Classify Unreviewed Keys

If `all-conf-keys.json` contains no `"unreviewed"` keys, skip this section.

The goal is to classify each unreviewed key as relevant or irrelevant to DogStatsD behavior. This
requires code analysis — name prefixes alone are not sufficient. A key like `forwarder_num_workers`
has no `dogstatsd` prefix but directly affects how DogStatsD metrics are forwarded.

### Phase 1: Batch Triage

Filter `all-conf-keys.json` to only `"Status": "unreviewed"` entries. Split them into batches of
~30-50 keys. For each batch, create a sub-agent with the following instructions:

> For each key in this batch, determine whether it could plausibly affect DogStatsD behavior. Read
> the code at the listed source location(s) and trace how the key is used. A key is relevant to
> DogStatsD if it influences any of:
>
> - Metric reception (listeners, ports, sockets, buffers, protocols)
> - Metric parsing or decoding (DogStatsD wire format, sample rates, timestamps)
> - Metric aggregation, enrichment, or tagging (context resolution, tag cardinality, host tags)
> - Metric forwarding or serialization (forwarder, endpoints, payloads, compression, retry)
> - Origin detection or container enrichment
> - General infrastructure that DogStatsD depends on (API keys, proxy, TLS, secrets, logging that
>   would affect DogStatsD components)
>
> Respond with one CSV line per key:
> `"key_name","true/false","reasoning (20-70 chars)"`
>
> Where `true` means RELEVANT (it does or could affect DogStatsD), and `false` means NOT RELEVANT.
>
> When uncertain, err on the side of `true` (relevant). It is much worse to miss a relevant key
> than to include an irrelevant one.

Give each sub-agent access to both `{{datadog-agent}}` and `{{saluki}}` so it can read usage sites.

### Phase 2: Assemble and Review

Collect all sub-agent CSV outputs and concatenate into
`{{tmp}}/new-key-recommendations.csv`:

```csv
"api_key","true","shared infra: DogStatsD forwarder needs this"
"security_agent.enabled","false","security agent only, no DogStatsD path"
"dogstatsd_port","true","directly configures DogStatsD listener"
"network_config.enable_http_monitoring","false","system probe network monitoring only"
```

Use AskUserQuestion: report how many keys were analyzed, how many recommended-relevant vs
recommended-irrelevant, and ask the user to review the file before proceeding.

### Phase 3: Update known-configs.csv and all-conf-keys.json

After the user approves (they may have edited the recommendations file):

1. Append the entries from `{{tmp}}/new-key-recommendations.csv` to `known-configs.csv`, sorted
   alphabetically by key. Do not remove or modify existing entries in `known-configs.csv`.

2. Update `{{tmp}}/all-conf-keys.json`: for every key that was just added to `known-configs.csv`,
   change its `"Status"` from `"unreviewed"` to `"known"`. Remove entries whose key is now
   `relevant=false` in `known-configs.csv` (they are no longer needed in the working set).

After this step, `all-conf-keys.json` should contain only `"Status": "known"` entries — the
relevant keys that downstream phases will analyze.

## Action: Analyze and Document Feature Parity (TBD)

Instructions for this phase are under construction. The intent is:

For each relevant key in `all-conf-keys.json`, analyze the RefImpl and AdpImpl to determine the
FeatureState (IMPLEMENTED, MISSING, DIVERGENT, ADP_ONLY, UNSURE). Update
`docs/reference/dogstatsd-features.md` with the results — a customer-facing reference documenting
DogStatsD feature parity between the Datadog Agent and Agent Data Plane.

STOP HERE: instructions for this phase are TBD.