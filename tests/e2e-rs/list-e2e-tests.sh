#!/usr/bin/env bash
# List all runnable e2e integration test cases as a comma-separated string

cd "$(dirname "$0")"

cargo nextest list --all 2>/dev/null | \
    grep '::integration_test' | \
    awk -F'::' '{print $NF}' | \
    tr '\n' ',' | \
    sed 's/,$/\n/'
