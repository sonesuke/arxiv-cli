#!/bin/bash
# agents/pr-healer/tools/load-progress.sh
# Reads and displays the most recent progress entries from progress.jsonl

PROGRESS_FILE="agents/pr-healer/progress.jsonl"

if [ ! -f "$PROGRESS_FILE" ]; then
    echo "[load-progress] No progress file found. This is a fresh start."
    exit 0
fi

LINES=$(wc -l < "$PROGRESS_FILE" | tr -d ' ')

if [ "$LINES" -eq 0 ]; then
    echo "[load-progress] Progress file is empty. This is a fresh start."
    exit 0
fi

echo "[load-progress] Showing last 5 entries (of $LINES total):"
tail -n 5 "$PROGRESS_FILE" | jq .
