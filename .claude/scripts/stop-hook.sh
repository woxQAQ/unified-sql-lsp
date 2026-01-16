#!/usr/bin/env bash
# Output JSON to block the stop and feed prompt back
# The "reason" field contains the prompt that will be sent back to Claude
cd /home/woxQAQ/unified-sql-lsp

if ! output=$(make check 2>&1); then
  jq -n \
    --arg err "checks are failed, error reason: <output>$output</output>, please run \`make check\` to debug" \
    '{
      "decision": "block",
      "reason": $err,
      "systemMessage": "Failed to pass all the checks"
    }'
  exit 0
fi

exit 0
