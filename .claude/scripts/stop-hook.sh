#!/usr/bin/env bash
# Output JSON to block the stop and feed prompt back
# The "reason" field contains the prompt that will be sent back to Claude
if ! output=$(make check 2>&1); then
  jq -n \
    --arg err "checks are failed, error reason: <output>$output</output>, please run \`make check\` to debug" \
    '{
      "decision": "block",
      "reason": $err,
      "systemMessage": "Failed to pass all the checks"
    }'
fi

if ! output=$(make test-e2e 2>&1); then
  jq -n \
    --arg err "e2e tests are failed, error reason: <output>$output</output>. please run \`make test-e2e\` to debug" \
    '{
      "decision": "block",
      "reason": $err,
      "systemMessage": "Failed to pass all the checks"
    }'
fi
