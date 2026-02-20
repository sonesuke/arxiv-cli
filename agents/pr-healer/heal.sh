#!/bin/bash
# agents/pr-healer/heal.sh (Container side)

set -e

# --- Configuration ---
PR_NUMBER=$1
BRANCH_NAME=$2
ITERATION_CAP=5
PROGRESS_FILE="agents/pr-healer/progress.txt"
PROMPT_FILE="agents/pr-healer/prompt.txt"
LOG_FILE="agents/pr-healer/healer.log"

# --- Modular Functions (Procedural) ---

setup_environment() {
    echo "[Container] Setting up environment for PR #$PR_NUMBER..."
    if [ -z "$PR_NUMBER" ] || [ -z "$BRANCH_NAME" ]; then
        echo "[Container] Error: PR number or branch name missing."
        exit 1
    fi
    git fetch origin "$BRANCH_NAME"
    git checkout "$BRANCH_NAME"
    echo "[Container] Checked out branch: $(git branch --show-current)"
}

sync_with_main() {
    echo "[Container] Syncing with main..."
    git fetch origin main
    git merge origin/main --no-edit || {
        echo "[Container] Merge conflict detected. Manual/Claude intervention needed."
        return 1
    }
}

apply_automated_fixes() {
    local changed=false
    echo "[Container] Attempting automated fixes (fmt, clippy fix)..."
    
    cargo fmt
    cargo clippy --fix --allow-dirty || echo "[Container] Clippy fix warning, continuing..."
    
    if ! git diff --quiet; then
        echo "[Container] Automated fixes applied. Committing..."
        git add .
        git commit -m "fix(healer): automated formatting and clippy fixes"
        changed=true
    fi
    return 0
}

check_quality() {
    ./agents/pr-healer/verify.sh
}

run_autonomous_healer() {
    echo "[Container] Starting autonomous reasoning loop (Ralph Loop)..."
    
    for ((i=1; i<=ITERATION_CAP; i++)); do
        echo "[Container] Iteration $i of $ITERATION_CAP..."
        
        if check_quality; then
            echo "[Container] Healing complete! All quality gates passed."
            return 0
        fi

        echo "[Container] Calling Claude Code for reasoning..."
        # Execute Claude with the defined prompt
        result=$(claude --allow-dangerously-skip-permissions -p "$PROMPT_FILE")
        
        echo "--- Claude Output Iteration $i ---" >> "$LOG_FILE"
        echo "$result" >> "$LOG_FILE"
        
        if [[ "$result" == *"<promise>COMPLETE</promise>"* ]]; then
            echo "[Container] Claude reported COMPLETE."
            # Final check anyway
            if check_quality; then
                return 0
            else
                echo "[Container] Claude claimed complete but quality check still fails."
            fi
        fi
        
        if [ $i -eq $ITERATION_CAP ]; then
            echo "[Container] Reached iteration cap ($ITERATION_CAP)."
            return 1
        fi
    done
}

# --- Main Execution ---

setup_environment
sync_with_main
apply_automated_fixes
run_autonomous_healer

echo "[Container] PR #$PR_NUMBER healing process finished locally."
