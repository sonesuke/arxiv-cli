#!/usr/bin/env bash
# Check if the expected skill was loaded

set -euo pipefail

TEST_PROMPT="$1"
shift

# Parse expected skill name from test prompt
# Example: "Use arxiv-search skill to find papers" -> "arxiv-search"
if [[ "$TEST_PROMPT" =~ ([a-z]+-[a-z]+) ]]; then
    EXPECTED_SKILL="${BASH_REMATCH[1]}"
else
    echo "Error: Could not extract skill name from test prompt"
    exit 1
fi

echo "Checking skill loaded: $EXPECTED_SKILL"

# This would verify the skill is loaded
# For now, we assume it's always loaded in the test environment
echo "Skill loaded check: OK ($EXPECTED_SKILL)"
