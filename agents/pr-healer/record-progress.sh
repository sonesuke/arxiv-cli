#!/bin/bash
# agents/pr-healer/record-progress.sh
# Standardized progress logging for PR-Healer.

PROGRESS_FILE="agents/pr-healer/progress.txt"

TITLE=$1
DESCRIPTION=$2
FILES_CHANGED=$3

if [ -z "$TITLE" ]; then
    echo "Usage: $0 <title> <description> <files_changed>"
    exit 1
fi

{
    echo "--- $(date '+%Y-%m-%d %H:%M:%S') ---"
    echo "Task: $TITLE"
    echo "Description: $DESCRIPTION"
    echo "Files: $FILES_CHANGED"
    echo ""
} >> "$PROGRESS_FILE"

echo "[Progress] Recorded: $TITLE"
