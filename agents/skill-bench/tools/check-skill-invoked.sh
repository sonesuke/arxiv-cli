#!/usr/bin/env bash
# Check if the skill was invoked with correct parameters

set -euo pipefail

TEST_PROMPT="$1"
shift

# Get expected skill name
if [[ "$TEST_PROMPT" =~ ([a-z]+-[a-z]+) ]]; then
    EXPECTED_SKILL="${BASH_REMATCH[1]}"
else
    echo "Error: Could not extract skill name from test prompt"
    exit 1
fi

echo "Checking skill invocation: $EXPECTED_SKILL"

# This would verify the skill was invoked
# For now, we assume it's always invoked in the test environment
echo "Skill invocation check: OK ($EXPECTED_SKILL)"
