#!/bin/bash
# agents/pr-healer/healer.sh (Host side)
# The "Ralph Loop" orchestration script.

set -e

WORKSPACE_FOLDER=$(pwd)
PROGRESS_FILE="agents/pr-healer/progress.txt"
LOG_FILE="agents/pr-healer/healer.log"

touch "$PROGRESS_FILE"

echo "[$(date)] healer.sh started" >> "$LOG_FILE"

# Function to check if a PR has already been processed for its current head commit
is_processed() {
    local pr_number=$1
    local head_sha=$2
    grep -q "${pr_number}:${head_sha}" "$PROGRESS_FILE"
}

# Function to mark a PR as processed
mark_processed() {
    local pr_number=$1
    local head_sha=$2
    echo "${pr_number}:${head_sha}" >> "$PROGRESS_FILE"
}

# Ensure devcontainer is up
echo "[Host] Ensuring devcontainer is up..."
# devcontainer up --workspace-folder "$WORKSPACE_FOLDER"

# Get a list of open PRs with CI failures
# We can filter by status using gh pr list --json statusCheckRollup
# However, simpler is just to get all open PRs and check each inside the loop context if needed.
PR_LIST=$(gh pr list --state open --json number,headRefOid --jq '.[] | "\(.number):\(.headRefOid)"')

for pr_info in $PR_LIST; do
    PR_NUMBER=${pr_info%%:*}
    PR_HEAD_SHA=${pr_info#*:}

    if is_processed "$PR_NUMBER" "$PR_HEAD_SHA"; then
        echo "[Host] PR #$PR_NUMBER (SHA: $PR_HEAD_SHA) already processed. Skipping."
        continue
    fi

    echo "[Host] PR #$PR_NUMBER (SHA: $PR_HEAD_SHA) status check..."
    
    # Check if CI actually failed
    CI_STATUS=$(gh pr view "$PR_NUMBER" --json statusCheckRollup --jq '.statusCheckRollup[] | select(.status == "COMPLETED") | .conclusion' | sort | uniq)
    
    if [[ "$CI_STATUS" == *"FAILURE"* ]]; then
        echo "[Host] PR #$PR_NUMBER has failed checks. Starting healer..."
        
        # Get branch name on host
        BRANCH_NAME=$(gh pr view "$PR_NUMBER" --json headRefName --jq .headRefName)
        
        # Execute healer logic inside the container
        if devcontainer exec --workspace-folder "$WORKSPACE_FOLDER" bash agents/pr-healer/heal.sh "$PR_NUMBER" "$BRANCH_NAME"; then
            echo "[Host] PR #$PR_NUMBER healing logic completed. Pushing from host..."
            
            # Record current branch to return later
            ORIGINAL_BRANCH=$(git branch --show-current)
            
            # Push the changes from host
            git checkout "$BRANCH_NAME"
            if git push origin "$BRANCH_NAME"; then
                echo "[Host] PR #$PR_NUMBER healing successful and pushed."
                mark_processed "$PR_NUMBER" "$PR_HEAD_SHA"
            else
                echo "[Host] PR #$PR_NUMBER push failed."
            fi
            
            # Back to original branch
            git checkout "$ORIGINAL_BRANCH"
        else
            echo "[Host] PR #$PR_NUMBER healing failed or no changes needed."
            # We don't mark as processed so it can be retried if needed, 
            # or maybe we should to avoid infinite loops on unfixable errors.
            # For now, let's just log it.
        fi
    else
        echo "[Host] PR #$PR_NUMBER CI is passing or in progress. Skipping."
    fi
done

echo "[$(date)] healer.sh completed" >> "$LOG_FILE"
