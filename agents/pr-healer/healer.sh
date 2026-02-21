#!/bin/bash
# agents/pr-healer/healer.sh (Host side)
# The simplified "Host Loop" daemon script.

set -e
set -o pipefail

# --- Pre-flight Checks ---
check_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "[Error] Required command '$1' not found. Please install it." >&2
        return 1
    fi
}

check_command "gh" || exit 1
check_command "devcontainer" || exit 1
check_command "jq" || exit 1

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    echo "[Error] Docker is not running or accessible. Please start Docker Desktop." >&2
    exit 1
fi

GITHUB_TOKEN=$(gh auth token)
WORKSPACE_FOLDER="${WORKSPACE_FOLDER:-$(pwd)}"

echo "[Host] Ensuring dev container is up for $WORKSPACE_FOLDER..."
devcontainer up --workspace-folder "$WORKSPACE_FOLDER"

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
    
    # Run Claude inside the container. 
    # Use a temporary file for stderr to avoid swallowing it in jq pipe while keeping jq for stdout.
    TEMP_ERR=$(mktemp)
    
    # We run 'devcontainer exec' in the foreground so we can catch its exit code.
    # We still use jq to format the JSON stream from Claude.
    if ! devcontainer exec \
        --workspace-folder "$WORKSPACE_FOLDER" \
        --remote-env "GITHUB_TOKEN=$GITHUB_TOKEN" \
        claude -p \
          --dangerously-skip-permissions \
          --verbose \
          --output-format stream-json \
          "$(cat agents/pr-healer/prompt.txt)" < /dev/null 2>"$TEMP_ERR" | jq . ; then
        
        EXIT_CODE=$?
        echo "[Host] Error: Claude agent or devcontainer failed with exit code $EXIT_CODE." >&2
        if [ -s "$TEMP_ERR" ]; then
            echo "[Host] Detailed error log:" >&2
            cat "$TEMP_ERR" >&2
        fi
        rm -f "$TEMP_ERR"
        
        # Determine if we should retry or stop. 
        # For now, stop on errors to avoid infinite loops of failure.
        echo "[Host] Terminating loop due to error." >&2
        exit $EXIT_CODE
    fi
    rm -f "$TEMP_ERR"
    
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
