#!/bin/bash
# agents/pr-healer/gh-helper.sh
# Encapsulates GitHub CLI operations for PR-Healer.

list_open_prs() {
    # Returns a list of "PR_NUMBER:HEAD_SHA"
    gh pr list --state open --json number,headRefOid --jq '.[] | "\(.number):\(.headRefOid)"'
}

get_pr_branch() {
    local pr_number=$1
    gh pr view "$pr_number" --json headRefName --jq .headRefName
}

get_pr_sha() {
    local pr_number=$1
    gh pr view "$pr_number" --json headRefOid --jq .headRefOid
}

check_ci_failure() {
    local pr_number=$1
    local ci_status
    # Gets all conclusions of completed checks
    ci_status=$(gh pr view "$pr_number" --json statusCheckRollup --jq '.statusCheckRollup[] | select(.status == "COMPLETED") | .conclusion' | sort | uniq)
    
    if [[ "$ci_status" == *"FAILURE"* ]]; then
        return 0 # Failure detected
    fi
    return 1 # No failure
}

# --- This script is intended to be sourced or called with arguments ---

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    # If run directly as a script, provide a simple CLI interface for testing
    COMMAND=$1
    shift
    case "$COMMAND" in
        list) list_open_prs ;;
        branch) get_pr_branch "$@" ;;
        sha) get_pr_sha "$@" ;;
        check_failure) check_ci_failure "$@" && echo "FAILURE" || echo "CLEAN" ;;
        *) echo "Usage: $0 {list|branch|sha|check_failure}"; exit 1 ;;
    esac
fi
