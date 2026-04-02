---
name: dogstatsd-audit
description: >
  Conduct a user-invoced audit of the features implemented in the DogStatsD feature of datadog-agent
  and compare that with the features available in agent-data-plane's DogStatsD component. This
  analysis uses configuration keys as feature identifiers.
disable-model-invocation: true
allowed-tools: Read, Write, MultiEdit, Grep, Glob, LS, Bash, Agent, Task, AskUserQuestion
---
# /dogstatsd-audit

Usage `/dogstatsd-audit <optional-prompt>`. The optional, free-form prompt may be used to adjust the
agent's behavior or context at the start of the skill, to ask it to perform only part of the skill,
or anything else that seems to work.

Program flow overview:
- Phase 1: Path resolution - finding the relevant code repositories
- Phase 2: Supervisor Context Building

## Phase 1: Path Resolution and Git Check

Resolve three repo paths. {{saluki}} is this repo's root. For each of {{datadog-agent}} and
{{documentation}}, check `{{saluki}}/../<name>` then `~/dd/<name>`. If not found, ask the user. If
the user doesn't have either repo locally, error and exit.

Show a table of the three resolved paths. AskUserQuestion: are these correct?

Then show a table with each repo's HEAD commit message, branch name, and dirty status.

AskUserQuestion: Proceed?

## Phase 2: Supervising Agent Context

You need to understand DogStatsD, Datadog Agent, read these files:
- {{documentation}}/content/en/agent/architecture.md
- {{documentation}}/content/en/extend/dogstatsd/
- {{datadog-agent}}/pkg/config/
- {{datadog-agent}}/comp/dogstatsd/

You need to understand the agent-data-plane project's goal of providing a more performant
alternative to datadog-agent's DogStatsD. Read these files:
- {{saluki}}/docs/agent-data-plane/index.md
- {{saluki}}/docs/reference/architecture.md
- {{saluki}}/bin/agent-data-plane/

Definitions:
- ADP=Agent Data Plate
- Reference Implementation, or RefImpl: The implementation of Datadog Agent and DogStatsD found in
  `datadog-agent`
- ADP Implementation, or AdpImpl: The implementation of DogStatsD (and supporting features) in ADP.
- Configuration Key, or ConfKey: The keys used to identify configuration options. The closest thing
  to a source of truth for these is {{datadog-agent}}/pkg/config/common_settings.go but they are
  defined in other places as well.

The purpose of this skill is going to be for you to orchestrate a large-scale discovery of RefImpl
features, using ConfKeys to identify them and track how they affect RefImpl behavior.

You will then perform an analysis of AdpImpl to create a matrix of RefImpl features that are:
- IMPL: correctly implemented in ADP
- MISS: missing or ignored in ADP
- DIVR: exist in AdpImpl but behave differently than RefImpl

AskUserQuestion: Provide a description of your understanding of the problem space using 100-300
words and ask if you correctly understand the problem. If no, ask the user for more feedback and
repeat.

STOP HERE: the rest of this skill has not been written yet. EXIT_SUCCESS