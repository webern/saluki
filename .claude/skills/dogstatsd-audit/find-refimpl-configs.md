# Find RefImpl Configs

This is a substep of the `/dogstatsd-audit` skill. Your job is to discover all configuration keys
(ConfKeys) registered in the datadog-agent Go codebase and in the example datadog.yaml file.

## Input

- `{{datadog-agent}}` = the path to the root of the datadog-agent repository
- `{{tmp}}` = your output directory

## System Overview

The Datadog Agent uses Viper (via a custom wrapper) for configuration. Keys are registered in Go
source code with default values and env var bindings, and can also appear in the example
`datadog.yaml` configuration file.

## Step 1: Discover the Config API Surface

Before searching for usages, read the interface definitions in `pkg/config/model/types.go` to
discover every method on the `Setup` interface (registration methods) and the `Reader` interface
(accessor methods). Build your own complete list from the source — do not rely solely on the
examples below, as methods may have been added or renamed.

Also look for any wrapper functions or helpers that delegate to these interfaces. For example,
`pkg/config/setup/config_accessor.go` provides top-level accessor functions. Check if other files
in `pkg/config/setup/` add helpers you should also search for.

## Step 2: Search for Config Key Registration

The primary config registry is `pkg/config/setup/common_settings.go` (~1900 lines, ~1100
registration calls). Other files in `pkg/config/setup/` register keys for specific subsystems (APM,
system probe, etc.).

Using the `Setup` interface methods you discovered in Step 1, search all `.go` files under
`pkg/config/` for calls to each registration method. The first string argument is the ConfKey.

As of this writing, the known registration patterns are:

```go
// Most common (~95% of keys):
config.BindEnvAndSetDefault("dogstatsd_port", 8125)

// Default without env binding:
config.SetDefault("key_name", defaultValue)

// Env binding without default:
config.BindEnv("dogstatsd_mapper_profiles")

// Custom env parsing (key is still the first argument):
config.ParseEnvAsSlice("key_name", func(in string) []interface{} { ... })
config.ParseEnvAsStringSlice("key_name", func(string) []string { ... })
config.ParseEnvAsMapStringInterface("key_name", func(string) map[string]interface{} { ... })
```

But there may be additional registration methods on the `Setup` interface — search for all of them.

For each match, extract the first string literal as the ConfKey and record file:line as the
location.

## Step 3: Search for Config Key Reads

Some keys may only appear at read sites, not at registration sites. Using the `Reader` interface
methods you discovered in Step 1, search `comp/dogstatsd/` and `pkg/config/setup/` for accessor
calls.

As of this writing, the known accessor patterns are:

```go
.GetString("key")
.GetBool("key")
.GetInt("key")
.GetFloat64("key")
.GetDuration("key")
.GetStringSlice("key")
.GetStringMap("key")
.GetStringMapString("key")
```

But there may be additional accessor methods — search for all of them.

Only include keys from read sites that were NOT already found at registration sites.

## Step 4: Validate Completeness

After collecting keys from Steps 2 and 3, do a sanity check:

- Pick 3-5 well-known DogStatsD keys (e.g. `dogstatsd_port`, `dogstatsd_buffer_size`,
  `use_dogstatsd`) and verify they appear in your output with correct locations.
- Scan `common_settings.go` manually (or skim sections) to check for registration patterns you
  might have missed — e.g. keys registered via a loop, a helper function, or a different call
  pattern.
- If you find a new pattern, go back and search for it comprehensively.

## Source 2: Example YAML File

The file `cmd/agent/dist/datadog.yaml` is a ~1600-line example config with most sections
commented out. It uses standard YAML nesting.

Parse this file to extract all config key paths. Use dot-separated flattening to match the format
used in Go code:

```yaml
# In the YAML:
proxy:
  http: http://example.com
  https: https://example.com

# Becomes these ConfKeys:
# proxy.http
# proxy.https
```

Note: commented-out keys (lines starting with `#`) should still be included — this is an example
file where most settings are intentionally commented out.

### What NOT to include

- Test files
- Keys that appear only in comments describing other keys
- Internal/framework keys not meant for user configuration

## Output

Write TWO files:

### `{{tmp}}/refimpl-go-config-keys.csv`

Keys discovered from Go source code:

```csv
"dogstatsd_port","pkg/config/setup/common_settings.go:1524"
"dogstatsd_buffer_size","pkg/config/setup/common_settings.go:1526"
"use_dogstatsd","pkg/config/setup/common_settings.go:1523"
```

### `{{tmp}}/refimpl-yaml-config-keys.csv`

Keys discovered from the example YAML file:

```csv
"api_key","cmd/agent/dist/datadog.yaml:15"
"proxy.http","cmd/agent/dist/datadog.yaml:42"
```

All paths should be relative to `{{datadog-agent}}`.

## Getting Additional Context

You are running as a subagent and may ask questions from the supervising agent or user if you need
more context.

## Completion

Report to the supervising agent when your work is complete.