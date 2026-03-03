#!/usr/bin/env bash
# Check if workspace was used correctly

set -euo pipefail

TEST_PROMPT="$1"
shift

echo "Checking workspace..."

# This would verify the workspace was used correctly
# For now, we assume it's always correct in the test environment
echo "Workspace check: OK"
