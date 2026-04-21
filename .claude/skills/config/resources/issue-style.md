# GitHub Issue Style Guide

Guidelines for drafting and filing issues in `DataDog/saluki` for DogStatsD ConfKey gaps.

## Writing Style

- **Terse, professional, technical.** These are public. Every word must earn its place.
- **Enough context to understand the issue** without reading any external audit document.
- **No filler.** No "it would be nice if" or "we should consider." State the problem, state what
  needs to happen.
- **Title format**: A proper, grammatical sentence ending with a period. Use backticks around config
  key names in titles. Examples:
  - `Allow configuring the TLS handshake timeout for HTTP clients.`
  - `Read `bind_host` from config instead of hardcoding the listen address.`
- Use backticks for config key names, code references, and file paths in the body as well.
- Reference the core Datadog Agent as "the core agent" (not "the Go agent" or "datadog-agent").
- Include GitHub permalinks to relevant code when available.

## Label Reference

Use labels that genuinely apply.

**Type labels (pick one):**
- `type/bug` — Bug fixes (e.g. silent metric drops, wrong defaults)
- `type/enhancement` — New functionality or support
- `type/investigation` — Needs further investigation to categorize
- `type/chore` — Administrative/maintenance tasks
- `type/meta` — Not yet fully-formed ideas

**Area labels (pick all that apply):**
- `area/config` — Configuration
- `area/components` — Sources, transforms, and destinations
- `area/core` — Core functionality, event model
- `area/io` — General I/O and networking
- `area/memory` — Memory bounds and management
- `area/docs` — Reference documentation
- `area/observability` — Internal observability

**Component labels (pick all that apply):**
- `source/dogstatsd` — DogStatsD source
- `transform/aggregate` — Aggregate transform
- `transform/dogstatsd-mapper` — DogStatsD Mapper synchronous transform
- `transform/dogstatsd-prefix-filter` — DogStatsD Prefix/Filter transform
- `transform/host-enrichment` — Host Enrichment synchronous transform
- `transform/host-tags` — Host Tags synchronous transform
- `forwarder/datadog` — Datadog forwarder
- `encoder/datadog-metrics` — Datadog Metrics encoder
- `destination/dogstatsd-stats` — DogStatsD Statistics destination

**Effort labels (pick one):**
- `effort/simple` — Trivial changes, should be fine if it compiles and tests pass
- `effort/intermediate` — Non-expert can work on it but might need guidance
- `effort/complex` — Requires guidance and careful review

**Status labels (use when appropriate):**
- `status/blocked` — Blocked on another issue or upstream dependency

## `gh` Command Template

```bash
ISSUE_URL=$(gh issue create \
  --repo DataDog/saluki \
  --title "TITLE" \
  --label "label1,label2,label3" \
  --body "$(cat <<'EOF'
BODY_TEXT_HERE
EOF
)")
```
