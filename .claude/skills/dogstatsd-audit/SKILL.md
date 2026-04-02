---
name: dogstatsd-audit
description: >
  Audit DogStatsD feature parity between datadog-agent and agent-data-plane,
  using configuration keys as feature identifiers.
disable-model-invocation: true
allowed-tools: Read, Write, MultiEdit, Grep, Glob, LS, Bash, Agent, Task, AskUserQuestion
---
# /dogstatsd-audit

Usage: `/dogstatsd-audit <optional-prompt>`. The optional free-form prompt can adjust behavior,
limit scope, or provide additional context.

## Phase 1: Path Resolution and Git Check

Resolve three repo paths. {{saluki}} is this repo's root. For {{datadog-agent}} and
{{documentation}}, check `{{saluki}}/../<name>` then `~/dd/<name>`. If not found, ask the user.
If the user lacks either repo, error and exit.

Show a table of the three resolved paths. AskUserQuestion: are these correct?

Then show a table with each repo's HEAD commit message, branch name, and dirty status.

AskUserQuestion: Proceed?

## Phase 2: Build Supervisor Context

Read these files to understand DogStatsD and Datadog Agent:
- {{documentation}}/content/en/agent/architecture.md
- {{documentation}}/content/en/extend/dogstatsd/
- {{datadog-agent}}/pkg/config/
- {{datadog-agent}}/comp/dogstatsd/

Read these files to understand ADP, the performant alternative to datadog-agent's DogStatsD:
- {{saluki}}/docs/agent-data-plane/index.md
- {{saluki}}/docs/reference/architecture.md
- {{saluki}}/bin/agent-data-plane/

### Definitions

- **ADP** (Agent Data Plane): The agent-data-plane binary and its components.
- **RefImpl** (Reference Implementation): The DogStatsD implementation in `datadog-agent`.
- **AdpImpl** (ADP Implementation): The DogStatsD implementation in ADP.
- **ConfKey** (Configuration Key): Keys identifying configuration options. The primary source is
  {{datadog-agent}}/pkg/config/common_settings.go, though keys are defined elsewhere too.

The goal of this skill is to orchestrate large-scale discovery of RefImpl features, using ConfKeys
to identify them and track how they affect RefImpl behavior.

### FeatureState

Analyze AdpImpl to produce a matrix of ConfKey features, each in one of these states:

- **IMPLEMENTED**: Present in RefImpl and correctly implemented in AdpImpl.
- **ADP_ONLY**: Present in AdpImpl but not in RefImpl.
- **MISSING**: Present in RefImpl but not in AdpImpl.
- **DIVERGENT**: Present in both, but AdpImpl behavior differs from RefImpl.
- **PENDING**: Present in both, but behavioral analysis is incomplete.

AskUserQuestion: Summarize your understanding of the problem space in 100-300 words. Ask whether
it is correct, giving the user a chance to redirect if needed.

STOP HERE: the rest of this skill has not been written yet. EXIT_SUCCESS
