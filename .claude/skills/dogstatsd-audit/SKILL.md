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
`{{documentation}}`, check `{{saluki}}/../<name>` then `~/dd/<name>`. If not found, ask the user. If
either repo is unavailable, report which is missing and stop.

Show a table of the three resolved paths. Use AskUserQuestion to confirm they are correct.

Then show a table with each repo's HEAD commit message, branch name, and dirty status. Use
AskUserQuestion to ask whether to proceed.

You may store temporary files in `{{tmp}}`=`{{saluki}}/target/.temp/dogstatsd-audit`. Delete {{tmp}}
if exists. Create {{tmp}}

## Initial Definitions

- **ADP** (Agent Data Plane): The `agent-data-plane` binary and its components.
- **RefImpl** (Reference Implementation): The DogStatsD implementation in `datadog-agent`.
- **AdpImpl** (ADP Implementation): The DogStatsD implementation in ADP.
- **ConfKey** (Configuration Key): A `datadog-agent` configuration key. The primary index is
  `{{datadog-agent}}/pkg/config/common_settings.go`, but keys also appear throughout
  `{{datadog-agent}}/comp/dogstatsd/` and elsewhere

### FeatureState

Each ConfKey maps to one of these states:

- **IMPLEMENTED**: Present in RefImpl and correctly implemented in AdpImpl.
- **ADP_ONLY**: Present in AdpImpl but not in RefImpl.
- **MISSING**: Present in RefImpl but not in AdpImpl.
- **DIVERGENT**: Present in both, but AdpImpl behavior differs from RefImpl.
- **UNSURE**: Present in both, but behavioral analysis is incomplete.

This skill discovers RefImpl features by enumerating ConfKeys and analyzing how each affects RefImpl
behavior. The goal is to audit AdpImpl against RefImpl.

## Action: Gather Background Knowledge

You will be commanding a swarm of sub-agents. Here is the background context that you will need.

Read these files to understand RefImpl:
- `{{documentation}}/content/en/agent/architecture.md`
- `{{documentation}}/content/en/extend/dogstatsd/` -- index and overview files
- `{{datadog-agent}}/pkg/config/` -- `.go` files
- `{{datadog-agent}}/comp/dogstatsd/` -- `.go` files

Read these files to understand AdpImpl:
- `{{saluki}}/docs/agent-data-plane/index.md`
- `{{saluki}}/docs/reference/architecture.md`
- `{{saluki}}/bin/agent-data-plane/` -- entry point files

Use AskUserQuestion: summarize your understanding of the audit goal, the two implementations, and
the FeatureState categories in 100-300 words. Ask the user to confirm or correct it.

## Definition: ConfKey csv

A ConfKey csv file looks like this:

```csv
"dogstatsd_tag_cardinality","{{datadog-agent}}/pkg/config/setup/common_settings.go:536"
"system_probe_config.internal_profiling.enabled","{{datadog-agent}}/pkg/config/setup/system_probe.go:109"
```

## Action: Discover

Create a sub-agent for each of the following tasks. Store their output in `{{tmp}}`.
Include ALL ConfKeys, not just DogStatsD-relevant keys.

- Discover all ConfKeys in {{datadog-agent}}/pkg/config/
- Discover all ConfKeys in {{datadog-agent}}/cmd/agent/dist/datadog.yaml : this is an example
  configuration YAML file with most configuration sections commented out. Use YAML flattening to
  produce dot-separated paths as we see in common_settings.go
- Find all ConfKeys in {{saluki}} by running ./find-saluki-configs.md

Combine the files:
- filtering out any ConfKeys in `ignored-keys.txt`...
- find the best single-source-of-truth representation of each RefImpl key with the following
  preference-order:
  - {{datadog-agent}}/pkg/config/setup/common_settings.go
  - {{datadog-agent}}/pkg/config/
  - {{datadog-agent}}/cmd/agent/dist/datadog.yaml
  - {{documentation}}

- find the best single-source-of-truth representation of each AdpImpl key with the following
  preference-order: <!-- TODO: better understanding here -->
  - {{saluki}}

Write to `{{tmp}}/all-conf-keys.json` with the following format. Each ConfKey should exist only once giving it's best
source-of-truth locations from each Impl:

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
