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
- Phase 2: TBD

## Phase 1: Path Resolution and Git Check

## Find Repos

{{saluki}}: This skill lives in the saluki code repository, thus {{saluki}} is the root of this
repository.
{{datadog-agent}}: You will need the filepath to the Datadog Agent code repository. This is usually
named `datadog-agent` and often can be found at either `{{saluki}}../datadog-agent` or
`~/dd/datadog-agent`. If you can't find it, ask the user. If the user doesn't have it checked out
locally, this skill will not work. Error and exit explaining this to the user.
{{documentation}}: This is the code repository containing Datadog's public documentation. This is
usually named `documentation` and often can be found at either `{{saluki}}../documentation` or
`~/dd/documentation`. If you can't find it, ask the user. If the user doesn't have it checked out
locally, this skill will not work. Error and exit explaining this to the user.

Display these paths in a table as you have resolved them to the user and AskUserQuestion to
determine if they are correct.

## Double Check Git Status

For each of {{saluki}}, {{datadog-agent}}, and {{documentation}}, display to the user (in a table)
the git commit message of HEAD, the branch name if there is one, and whether the status is dirty or
not.

AskUserQuestion: Proceed?

DONE: the rest of the skill is a work in progress. Exit Success