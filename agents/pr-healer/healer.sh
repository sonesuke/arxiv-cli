#!/bin/bash
# agents/pr-healer/healer.sh (Host side)
# The "Ralph Loop" orchestration script.

set -e

# --- Configuration ---
WORKSPACE_FOLDER=$(pwd)
PROGRESS_FILE="agents/pr-healer/progress.txt"
LOG_FILE="agents/pr-healer/healer.log"
GH_HELPER="./agents/pr-healer/gh-helper.sh"

# --- Initialization ---

init() {
    touch "$PROGRESS_FILE"
    echo "[$(date)] healer.sh started" >> "$LOG_FILE"
}

# --- Persistence ---

is_processed() {
    local pr_number=$1
    local head_sha=$2
    grep -q "${pr_number}:${head_sha}" "$PROGRESS_FILE"
}

mark_processed() {
    local pr_number=$1
    local head_sha=$2
    echo "${pr_number}:${head_sha}" >> "$PROGRESS_FILE"
}

# --- Orchestration ---

push_healed_branch() {
    local branch_name=$1
    local pr_number=$2
    local head_sha=$3
    
    echo "[Host] PR #$pr_number: Quality confirmed. Ready to push branch '$branch_name'..."
    ORIGINAL_BRANCH=$(git branch --show-current)
    
    git checkout "$branch_name"
    if git push origin "$branch_name"; then
        echo "[Host] PR #$pr_number: Push successful."
        mark_processed "$pr_number" "$head_sha"
    else
        echo "[Host] PR #$pr_number: Push failed."
    fi
    
    git checkout "$ORIGINAL_BRANCH"
}

process_single_pr() {
    local pr_info=$1
    local pr_number=${pr_info%%:*}
    local head_sha=${pr_info#*:}

    if is_processed "$pr_number" "$head_sha"; then
        echo "[Host] PR #$pr_number: Already processed for SHA $head_sha. Skipping."
        return
    fi

    echo "[Host] PR #$pr_number: Investigating status..."
    
    if "$GH_HELPER" check_failure "$pr_number"; then
        echo "[Host] PR #$pr_number: CI failure detected. Initiating healing loop..."
        
        local branch_name
        branch_name=$("$GH_HELPER" branch "$pr_number")
        
        # Execute healer logic inside the container
        # Note: We pass both PR_NUMBER and BRANCH_NAME to heal.sh
        if devcontainer exec --workspace-folder "$WORKSPACE_FOLDER" bash agents/pr-healer/heal.sh "$pr_number" "$branch_name"; then
            push_healed_branch "$branch_name" "$pr_number" "$head_sha"
        else
            echo "[Host] PR #$pr_number: Healing process failed or timed out."
        fi
    else
        echo "[Host] PR #$pr_number: Status is green/pending. No action taken."
    fi
}

# --- Main Entry Point ---

main() {
    init

    echo "[Host] Fetching open pull requests..."
    PR_LIST=$("$GH_HELPER" list)

    if [ -z "$PR_LIST" ]; then
        echo "[Host] No open pull requests found."
    else
        for pr_info in $PR_LIST; do
            process_single_pr "$pr_info"
        done
    fi

    echo "[$(date)] healer.sh completed" >> "$LOG_FILE"
}

main "$@"
