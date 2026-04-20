# DogStatsD Doc Guide

This file describes the structure and maintenance rules for
`docs/agent-data-plane/configuration/dogstatsd.md`. It is read by the `/dogstatsd-audit` skill
before making any edits and can be read by humans to understand the conventions before editing
manually.

## Document Purpose

The doc is customer-facing. Its audience is an operator who has enabled ADP and wants to know
whether their existing DogStatsD configuration will work, or why something is not behaving as
expected. Team tracking is limited to GitHub issue numbers where relevant -- no priorities, project
status, or internal discussion belongs here.

## Section Structure

Each section follows this pattern:

1. A short introductory sentence or two explaining what the section covers.
2. A table for quick scanning.
3. Optional `### \`key_name\`` sub-sections below the table for keys that need prose explanation.

Not every table row needs a sub-section. Only add one when a one-liner is insufficient -- for
example, when the behavior difference has a non-obvious cause, when the customer needs to take a
specific action, or when the divergence involves a unit or semantic change.

Sub-section prose is **human-authored narrative**. The skill must preserve it unless the
`feature_state` or `action` for that key changed in a way that makes the existing text factually
wrong. Do not rewrite prose to match sub-agent wording.

## Section Anchors

Each section in the doc is marked with an HTML comment anchor immediately before its heading. These
are invisible in rendered output and give the skill an unambiguous location target.

| Anchor                                     | Section                                  |
|--------------------------------------------|------------------------------------------|
| `<!-- section:unsupported-in-progress -->` | Unsupported Settings -- being worked on  |
| `<!-- section:unsupported-not-planned -->` | Unsupported Settings -- not planned      |
| `<!-- section:behavioral-differences -->`  | Behavioral Differences                   |
| `<!-- section:compatibility-unknown -->`   | Compatibility Unknown                    |
| `<!-- section:adp-only -->`                | ADP-Only Settings                        |
| `<!-- section:reference -->`               | Configuration Reference                  |

## Table Schemas

### Unsupported Settings -- being worked on

Keys that are `feature_state=MISSING` and `action=IMPLEMENT` with an open GitHub issue.

| Config Key | Description | Issue |
|------------|-------------|-------|

Columns:
- **Config Key**: backtick-quoted key name
- **Description**: from `known-configs.json` `description` field, max 32 chars
- **Issue**: reference-style link, e.g. `[#1331]` -- URL defined at the bottom of the doc

### Unsupported Settings -- not planned

Keys that are `feature_state=MISSING` or `NOT_APPLICABLE` and `action=NONE` where a customer might
plausibly expect support. Do not list every NOT_APPLICABLE key -- only those relevant enough that a
customer might wonder why they have no effect.

| Config Key | Description | Reason |
|------------|-------------|--------|

Columns:
- **Config Key**: backtick-quoted key name
- **Description**: from `known-configs.json` `description` field, max 32 chars
- **Reason**: one short phrase, customer-facing. e.g. "Windows only", "Go runtime specific",
  "handled by core agent"

### Behavioral Differences

Keys that are `feature_state=DIVERGENT` and `action=DOCUMENT` or `DOCUMENTED`, plus any
`action=INVESTIGATE` keys where a divergence has been confirmed.

| Config Key | Description | Agent Behavior | ADP Behavior |
|------------|-------------|----------------|--------------|

Columns:
- **Config Key**: backtick-quoted key name; if ADP uses a different key name, note it in the
  sub-section, not this column
- **Description**: from `known-configs.json` `description` field, max 32 chars
- **Agent Behavior**: one short phrase
- **ADP Behavior**: one short phrase

### Compatibility Unknown

Keys that are `feature_state=UNKNOWN` and `action=INVESTIGATE`, or `MISSING`/`DIVERGENT` with
`action=INVESTIGATE` where we have not yet confirmed behavior. These are keys we have not fully
verified in ADP.

| Config Key | Description |
|------------|-------------|

Columns:
- **Config Key**: backtick-quoted key name
- **Description**: from `known-configs.json` `description` field, max 32 chars

### ADP-Only Settings

Keys that are `feature_state=ADP_ONLY`. These are configuration options that exist in ADP but have
no equivalent in the core agent.

| Config Key | Description | Default |
|------------|-------------|---------|

Columns:
- **Config Key**: backtick-quoted key name
- **Description**: from `known-configs.json` `description` field, max 32 chars
- **Default**: the default value, if known and useful

### Configuration Reference

All DogStatsD-relevant keys with their current status. Includes PARITY, MISSING, DIVERGENT, and
ADP_ONLY entries. Does not include NOT_APPLICABLE keys.

| Config Key | Description | Status | Notes |
|------------|-------------|--------|-------|

Columns:
- **Config Key**: backtick-quoted key name
- **Description**: from `known-configs.json` `description` field, max 32 chars
- **Status**: one of `Implemented`, `Missing`, `Divergent`, `ADP Only`
- **Notes**: short clarifying note, max 32 chars, or empty

## Mapping: known-configs.json -> Doc Section

| feature_state  | action                 | Section anchor                        | Notes                              |
|----------------|------------------------|---------------------------------------|------------------------------------|
| MISSING        | IMPLEMENT              | `section:unsupported-in-progress`     |                                    |
| MISSING        | INVESTIGATE            | `section:unsupported-in-progress`     | Omit issue column if none filed    |
| MISSING        | NONE                   | `section:unsupported-not-planned`     | Only if customer-visible           |
| DIVERGENT      | DOCUMENT or DOCUMENTED | `section:behavioral-differences`      |                                    |
| DIVERGENT      | IMPLEMENT              | `section:behavioral-differences`      | Note fix is in progress            |
| DIVERGENT      | INVESTIGATE            | `section:behavioral-differences`      | Note behavior is under review      |
| ADP_ONLY       | NONE                   | `section:adp-only`                    |                                    |
| PARITY         | NONE                   | `section:reference`                   | Reference table only               |
| NOT_APPLICABLE | NONE                   | --                                    | Omit unless customer-visible       |
| UNKNOWN        | INVESTIGATE            | --                                    | Omit until resolved                |

Every key that appears in a section table also appears in the Configuration Reference table.

## Link Block

All GitHub issue URLs are defined as Markdown reference links at the very bottom of the doc,
after all content. This keeps tables and prose narrow and readable. Use reference-style links
everywhere in the document body.

Format:

```markdown
[#1331]: https://github.com/DataDog/saluki/issues/1331
[#1332]: https://github.com/DataDog/saluki/issues/1332
```

When adding a new issue reference, append its definition to the link block. When an issue is
referenced in multiple places in the doc, define it once at the bottom. Keep the block sorted
numerically by issue number.

## Preservation Rules

1. **Table rows** are data. Update them when `known-configs.json` changes -- add rows for new keys,
   update Status/Notes when feature_state or action changes, remove rows only if a key is removed
   entirely from `known-configs.json`.

2. **Sub-section prose** is narrative. Preserve it unless the key's `feature_state` or `action`
   changed in a way that makes it factually wrong. Do not rewrite to match sub-agent phrasing.

3. **Section intros and headings** are human-authored. Never modify them.

4. **Issue links** should reflect the current open/closed state of the referenced issue. A closed
   issue may warrant moving a key from "being worked on" to "Implemented" in the reference table,
   but confirm via `gh issue view` before making that change.
