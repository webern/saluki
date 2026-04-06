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

## Source 1: Go Code Registration

The primary config registry is `pkg/config/setup/common_settings.go` (~1900 lines, ~1100
registration calls). Other files in `pkg/config/setup/` register keys for specific subsystems (APM,
system probe, etc.).

### Registration patterns to search for

Search all `.go` files under `pkg/config/` for these function calls. The first string argument is
the ConfKey.

**Pattern 1 — most common (~95% of keys):**
```go
config.BindEnvAndSetDefault("dogstatsd_port", 8125)
config.BindEnvAndSetDefault("dogstatsd_buffer_size", 1024*8)
config.BindEnvAndSetDefault("dogstatsd_origin_detection", false)
```

**Pattern 2 — default without env binding:**
```go
config.SetDefault("key_name", defaultValue)
```

**Pattern 3 — env binding without default:**
```go
config.BindEnv("dogstatsd_mapper_profiles")
```

**Pattern 4 — custom env parsing (the key is still the first argument):**
```go
config.ParseEnvAsSlice("dogstatsd_mapper_profiles", func(in string) []interface{} { ... })
config.ParseEnvAsStringSlice("key_name", func(string) []string { ... })
config.ParseEnvAsMapStringInterface("key_name", func(string) map[string]interface{} { ... })
```

**Search commands:** Grep all `.go` files under `pkg/config/` for each of:
```
BindEnvAndSetDefault("
SetDefault("
BindEnv("
ParseEnvAsSlice("
ParseEnvAsStringSlice("
ParseEnvAsMapStringInterface("
```

For each match, extract the first string literal as the ConfKey and record file:line as the
location.

### Keys found at runtime read sites

Some keys may only appear at read sites, not at registration sites. As a secondary pass, also search
`comp/dogstatsd/` for config reads:

```go
// The Reader interface methods — all take a key string as the first argument:
.GetString("key")
.GetBool("key")
.GetInt("key")
.GetFloat64("key")
.GetDuration("key")
.GetStringSlice("key")
.GetStringMap("key")
.GetStringMapString("key")
```

Only include keys from read sites that were NOT already found at registration sites.

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