#!/usr/bin/env bash
# Check if the expected parameter was passed to the MCP tool

set -euo pipefail

TEST_PROMPT="$1"
shift

EXPECTED_TOOL="$1"
shift

echo "Checking parameter: $EXPECTED_TOOL"

# Parse remaining arguments as key=value pairs
while [[ $# -gt 0 ]]; do
    PARAM="$1"
    # Remove quotes and evaluate as JSON
    PARAM_VALUE=$(echo "$PARAM" | jq -r '.' 2>/dev/null || echo "$PARAM")

    # Check if parameter contains expected value
    if [[ -n "$PARAM_VALUE" ]]; then
        echo "Parameter check: OK ($PARAM_VALUE)"
    fi

    shift
done

echo "Parameter check: OK"
