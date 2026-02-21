#!/bin/bash
# agents/pr-healer/healer.sh (Host side)
# The simplified "Host Loop" daemon script.

set -e

# --- Configuration ---
WORKSPACE_FOLDER=$(pwd)
GITHUB_TOKEN=$(gh auth token)

# Trap Ctrl+C to exit gracefully
trap 'echo "[Host] Caught SIGINT. Cleaning up..."; kill $CURRENT_PID 2>/dev/null; exit 0' SIGINT

# Variable to hold the current child process ID for the trap
CURRENT_PID=""

# --- Orchestration Loop ---
while :; do
    echo "=================================================="
    echo "[Host] Starting True Agentic PR-Healer Loop..."
    echo "[Host] Triggering Claude inside Dev Container..."
    
    # Remove the ALL_CLEAR flag before each run
    rm -f agents/pr-healer/ALL_CLEAR
    
    # Run Claude inside the container using -p (print mode).
    # -p mode skips the interactive workspace trust dialog and the
    # --dangerously-skip-permissions warning entirely, while still
    # executing tool calls (Bash, Read, Edit, etc.) autonomously.
    # Output is streamed as JSON so we can parse it later if needed.
    devcontainer exec \
        --workspace-folder "$WORKSPACE_FOLDER" \
        --remote-env "GITHUB_TOKEN=$GITHUB_TOKEN" \
        claude -p \
          --dangerously-skip-permissions \
          --verbose \
          --output-format stream-json \
          "$(cat agents/pr-healer/prompt.txt)" < /dev/null 2>&1 | jq . &
    
    CURRENT_PID=$!
    wait $CURRENT_PID
    
    # If Claude determines there's nothing left to do, it will touch this flag file.
    if [ -f "agents/pr-healer/ALL_CLEAR" ]; then
        echo "[Host] Claude reported all PRs are clean. Sleeping for 5 minutes before checking again..."
        rm -f agents/pr-healer/ALL_CLEAR
        sleep 300 &
        CURRENT_PID=$!
        wait $CURRENT_PID
        continue
    fi
    
    echo "[Host] Healer agent finished a turn. Restarting loop..."
    sleep 2 &
    CURRENT_PID=$!
    wait $CURRENT_PID
done
