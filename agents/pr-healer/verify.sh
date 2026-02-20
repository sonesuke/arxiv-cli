#!/bin/bash
# agents/pr-healer/verify.sh
# Standardized quality gates for PR-Healer.

set -e

echo "[Verify] Running quality gates..."

# 1. Formatting
echo "[Verify] Checking formatting..."
cargo fmt --all -- --check || {
    echo "[Verify] Failed: Formatting check failed."
    exit 1
}

# 2. Linting (Clippy)
echo "[Verify] Checking lints (clippy)..."
cargo clippy -- -D warnings || {
    echo "[Verify] Failed: Clippy found warnings or errors."
    exit 1
}

# 3. Tests
echo "[Verify] Running tests..."
cargo test || {
    echo "[Verify] Failed: Tests failed."
    exit 1
}

echo "[Verify] Success! All quality gates passed."
exit 0
