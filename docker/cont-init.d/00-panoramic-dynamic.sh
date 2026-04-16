#!/bin/bash

# Panoramic dynamic variable resolution.
#
# Evaluates PANORAMIC_DYNAMIC_* env vars as shell commands and writes the
# results to /airlock/dynamic/<KEY>. Then resolves {{PANORAMIC_DYNAMIC_*}}
# references found in the remaining env vars and writes the resolved values to
# /run/adp/env/ so s6-envdir can override them before ADP starts.
#
# This script is a no-op when no PANORAMIC_DYNAMIC_* vars are present.

# Always signal readiness on exit, regardless of how we get there.
mkdir -p /airlock/dynamic
trap 'touch /airlock/dynamic/.ready' EXIT

# Collect all PANORAMIC_DYNAMIC_* variable names.
dynamic_vars=$(env | grep '^PANORAMIC_DYNAMIC_' | cut -d= -f1)

# Exit early if none are defined — nothing to do.
if [ -z "$dynamic_vars" ]; then
    exit 0
fi

mkdir -p /run/adp/env

# Phase 1: Evaluate each PANORAMIC_DYNAMIC_* command and store the result.
for var in $dynamic_vars; do
    key="${var#PANORAMIC_DYNAMIC_}"
    cmd="${!var}"
    result=$(eval "$cmd" 2>/dev/null)
    printf "%s" "$result" > "/airlock/dynamic/$key"
done

# Phase 2: Resolve {{PANORAMIC_DYNAMIC_*}} references in all other env vars
# and write resolved values to /run/adp/env/ for s6-envdir.
env | while IFS='=' read -r var value; do
    # Skip PANORAMIC_DYNAMIC_* vars themselves.
    case "$var" in PANORAMIC_DYNAMIC_*) continue ;; esac

    # Skip vars that don't contain a template reference.
    case "$value" in *'{{PANORAMIC_DYNAMIC_'*) ;; *) continue ;; esac

    resolved="$value"
    for file in /airlock/dynamic/*; do
        [ -f "$file" ] || continue
        key=$(basename "$file")
        val=$(cat "$file")
        resolved="${resolved//\{\{PANORAMIC_DYNAMIC_${key}\}\}/$val}"
    done

    printf "%s" "$resolved" > "/run/adp/env/$var"
done
