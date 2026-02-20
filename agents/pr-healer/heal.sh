#!/bin/bash
# agents/pr-healer/heal.sh (Container side)

set -e

PR_NUMBER=$1

if [ -z "$PR_NUMBER" ]; then
    echo "[Container] Error: No PR number provided."
    exit 1
fi

echo "[Container] Healing PR #$PR_NUMBER..."

# 1. Checkout PR branch
gh pr checkout "$PR_NUMBER"
CURRENT_BRANCH=$(git branch --show-current)
echo "[Container] On branch $CURRENT_BRANCH"

# 2. Sync with main
echo "[Container] Syncing with main..."
git fetch origin main
# Use default pull behavior (merge) for simplicity in healer
git merge origin/main --no-edit || (echo "[Container] Merge conflict detected. Fallback to Claude." && git merge --abort)

# 3. Automated Fixes (Cheaper)
echo "[Container] Attempting automated fixes..."

# Run cargo fmt
cargo fmt

# Run clippy fix
cargo clippy --fix --allow-dirty || echo "[Container] Clippy fix had some issues, continuing..."

# Check if there are changes
if git diff --quiet; then
    echo "[Container] No automated fixes applied."
else
    echo "[Container] Automated fixes applied. Committing..."
    git add .
    git commit -m "fix(healer): automated formatting and clippy fixes"
fi

# 4. Check if still failing (Mocking 'test' as proxy for CI)
echo "[Container] Running tests to verify..."
if cargo test; then
    echo "[Container] Tests passed with automated fixes."
else
    echo "[Container] Tests still failing. Calling Claude Code for reasoning..."
    
    # Create prompt
    cat > agents/pr-healer/prompt.txt <<EOF
The CI is failing for this Pull Request. Please investigate the test failures and fix them.
You should:
1. Run 'cargo test' to see the failures.
2. Fix the code.
3. Verify the fix by running 'cargo test' again.
4. When done, exit.
EOF

    # Call Claude Code
    # Use --allow-dangerously-skip-permissions if necessary as per post-create alias
    claude --allow-dangerously-skip-permissions -p agents/pr-healer/prompt.txt
fi

# 5. Push Changes
if git diff origin/"$CURRENT_BRANCH"..HEAD --quiet; then
    echo "[Container] No changes to push."
else
    echo "[Container] Pushing healed changes..."
    git push origin "$CURRENT_BRANCH"
fi

echo "[Container] PR #$PR_NUMBER healing process finished."
