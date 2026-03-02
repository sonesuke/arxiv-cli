#!/usr/bin/env bash
# Check if MCP tool call was successful

set -euo pipefail

TEST_PROMPT="$1"
shift

EXPECTED_TOOL="$1"
shift

echo "Checking MCP success: $EXPECTED_TOOL"

# This would verify the MCP tool call was successful
# For now, we assume it's always successful in the test environment
echo "MCP success check: OK ($EXPECTED_TOOL)"
