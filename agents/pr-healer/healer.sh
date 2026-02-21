#!/bin/bash
# agents/pr-healer/healer.sh (Host side)
# The simplified "Host Loop" daemon script.

set -e

# --- Configuration ---
WORKSPACE_FOLDER=$(pwd)
LOG_FILE="agents/pr-healer/healer.log"

# --- Initialization ---
echo "[$(date)] PR-Healer Daemon started" >> "$LOG_FILE"

# Trap Ctrl+C to exit gracefully
trap "echo '[Host] Caught SIGINT. Exiting daemon loop.'; exit 0" SIGINT

# --- Orchestration Loop ---
while :; do
    echo "=================================================="
    echo "[Host] Starting True Agentic PR-Healer Loop..."
    echo "[Host] Triggering Claude inside Dev Container..."
    
    # Remove the ALL_CLEAR flag before each run
    rm -f agents/pr-healer/ALL_CLEAR
    
    # Run Claude inside the container. 
    # Claude's intelligence takes over from here (discovering PRs, fixing, pushing).
    # We pass standard input from /dev/null as requested by the user, which works 
    # because devcontainer exec will not allocate an interactive TTY when stdin is closed here, 
    # automatically bypassing the "Yes, I accept" screen.
    devcontainer exec \
        --workspace-folder "$WORKSPACE_FOLDER" \
        --remote-env "GITHUB_TOKEN=$GITHUB_TOKEN" \
        claude --dangerously-skip-permissions "$(cat agents/pr-healer/prompt.txt)" < /dev/null
    
    # If Claude determines there's nothing left to do, it will touch this flag file.
    if [ -f "agents/pr-healer/ALL_CLEAR" ]; then
        echo "[Host] Claude reported all PRs are clean. Sleeping for 5 minutes before checking again..."
        rm -f agents/pr-healer/ALL_CLEAR
        sleep 300
        continue
    fi
    
    echo "[Host] Healer agent finished a turn. Restarting loop..."
    sleep 2
done
