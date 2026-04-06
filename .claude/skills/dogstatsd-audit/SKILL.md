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

Combine the files, filtering out any ConfKeys in `ignored-keys.txt`. For keys found in multiple
locations, prefer the most authoritative source:

- RefImpl: `common_settings.go` > `pkg/config/` > `cmd/agent/dist/datadog.yaml` > docs
- AdpImpl: config structs > call sites

Write to `{{tmp}}/all-conf-keys.json` with the following format. Each ConfKey should exist only once
giving its best source-of-truth locations from each Impl:

```json
[
	{
		"ConfKey": "histogram_aggregates",
		"RefImpl": null,
		"AdpImpl": "lib/saluki-components/src/transforms/aggregate/config.rs:79"
	},
	{
		"ConfKey": "dogstatsd_workers_count",
		"RefImpl": "pkg/config/setup/common_settings.go:1596",
		"AdpImpl": null
	},
	{
		"ConfKey": "dogstatsd_port",
		"RefImpl": "pkg/config/setup/common_settings.go:1524",
		"AdpImpl": "lib/saluki-components/src/sources/dogstatsd/mod.rs:175"
	}
]
```

STOP HERE: this skill is still under construction. The user wants to inspect the files in `{{tmp}}`