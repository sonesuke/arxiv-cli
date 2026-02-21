#!/bin/bash
# agents/pr-healer/tools/record-progress.sh
# Appends a structured JSONL entry to progress.jsonl

PROGRESS_FILE="agents/pr-healer/progress.jsonl"

TASK="${1:-No task specified}"
FILES="${2:-}"
DECISIONS="${3:-}"
BLOCKERS="${4:-}"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Use jq to safely create JSON
ENTRY=$(jq -n -c \
  --arg ts "$TIMESTAMP" \
  --arg task "$TASK" \
  --arg files "$FILES" \
  --arg decisions "$DECISIONS" \
  --arg blockers "$BLOCKERS" \
  '{timestamp: $ts, task: $task, files: $files, decisions: $decisions, blockers: $blockers}')

echo "$ENTRY" >> "$PROGRESS_FILE"
echo "[record-progress] Logged: $TASK"
