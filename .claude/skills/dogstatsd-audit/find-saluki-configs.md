# Find Saluki Configs

This is a substep of the `/dogstatsd-audit` skill. Your job is to discover the configuration keys
(ConfKeys) that are used by agent-data-plane and it's components at runtime.

## Input

{{saluki}}=the path to the root of the Saluki codebase repository
{{tmp}}=your the output directory

## System Overview

`bin/agent-data-plane` is the binary whose configuration we are concerned with. It composes
components found in `lib/saluki-components` and uses other libs found in `lib/*`. In particular, the
core configuration primitives are in `lib/saluki-config`.

## How to Identify Configuration

ConfigKeys and configuration values are present in the main Datadog Agent configuration file,
usually named `datadog.yaml`. These can be overridden with environment variables. The pattern is as
follows,

if `datadog.yaml` has the following:

```yaml
config_section:
    key_a: true
    key_b: false
```

Then we may see the strings `config_section.key_a` and `config_section.key_b` in Saluki code.
Furthermore, these may turn into environment variables in which `DD_` is the global default prefix
and dots are turned into `_` so we might also see `DD_CONFIG_SECTION_KEY_A` or
`DD_CONFIG_SECTION_KEY_B`. The `DD_` prefix may be added or removed in the code leading to strings
like `CONFIG_SECTION_KEY_A` (without the prefix).

In {{saluki}} ConfigKeys are not centralized anywhere. They're exist across:
- lib/saluki-components/src/ — component config structs (DogStatsD, forwarders, etc.)
- bin/agent-data-plane/src/config.rs — top-level ADP config
- Various config.rs or mod.rs files inside component directories

In general, they may be found in two ways. Sometimes they exist as `#[serde(rename = "..."]` on the
fields of a config struct. At the site where the struct is used you will see CLAUDE: SHOW AN EXAMPLE
HERE

If a single field is being accessed without a config struct, the call site will look like this:
CLAUDE: give examples of `get_typed` or `try_get_typed`. Enumerate the functions that exist.

## Your Output

You must find all ConfigKeys used in the {{saluki}} codebase and list them in an output file. The
output format is csv pairs of {{conf-key}},{{location}} where location is the relative path to the
file where it was found and the line number, like this:

```csv
"dogstatsd_buffer_size","lib/saluki-components/src/sources/dogstatsd/mod.rs:157"
```

The filename is {{tmp}}/saluki-config-keys.csv

## Getting Additional Context

You are running as a subagent and may ask questions from the supervising agent or user if you need
more context.

## Completion

Report to the supervising agent when your work is complete.