#!/usr/bin/env bash
# Skill-Bench Test Runner
# Executes test cases and evaluates results

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CASES_DIR="$SCRIPT_DIR/cases"
TOOLS_DIR="$SCRIPT_DIR/tools"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
PASSED=0
FAILED=0
SKIPPED=0

# Usage
usage() {
    echo "Usage: $0 [<case-pattern>] [trials=<n>]"
    echo ""
    echo "Arguments:"
    echo "  case-pattern  - Glob pattern for test cases (default: \"*\")"
    echo "  trials=n      - Number of trials to run (default: 1)"
    echo ""
    echo "Examples:"
    echo "  $0                           # Run all test cases once"
    echo "  $0 \"arxiv-search\"           # Run arxiv-search test cases"
    echo "  $0 \"*\" trials=3             # Run all test cases 3 times"
}

# Parse arguments
CASE_PATTERN="*"
TRIALS=1

while [[ $# -gt 0 ]]; do
    case $1 in
        *=*)
            if [[ $1 == trials=* ]]; then
                TRIALS="${1#trials=}"
            else
                echo "Unknown parameter: $1" >&2
                usage
                exit 1
            fi
            ;;
        -*)
            echo "Unknown option: $1" >&2
            usage
            exit 1
            ;;
        *)
            CASE_PATTERN="$1"
            ;;
    esac
    shift
done

# Load test case from TOML file
load_case() {
    local case_file="$1"
    bash -c '
import toml
import sys
data = toml.load(sys.argv[1])
print("test_prompt=" + data.get("test_prompt", ""))
print("tool_calls=" + str(len(data.get("tool_calls", []))))
print("check=" + data.get("check", ""))
print("description=" + data.get("description", ""))
for i, tc in enumerate(data.get("tool_calls", [])):
    print("tool_" + str(i) + "_name=" + tc.get("name", ""))
    print("tool_" + str(i) + "_arguments=" + str(tc.get("arguments", {})))
' python3 "$case_file"
}

# Extract value from loaded case
get_value() {
    local -n ref=$1
    echo "${ref}" | grep "^$2=" | cut -d'=' -f2-
}

# Run single trial
run_trial() {
    local case_file="$1"
    local trial_num="$2"

    # Load test case
    local loaded_data
    loaded_data=$(load_case "$case_file")

    local test_prompt
    local tool_calls_count
    local check_script
    local description
    test_prompt=$(get_value loaded_data "test_prompt")
    tool_calls_count=$(get_value loaded_data "tool_calls")
    check_script=$(get_value loaded_data "check")
    description=$(get_value loaded_data "description")

    # Parse tool calls
    declare -a tool_names
    declare -a tool_args
    for ((i=0; i<tool_calls_count; i++)); do
        tool_names[$i]=$(get_value loaded_data "tool_${i}_name")
        tool_args[$i]=$(get_value loaded_data "tool_${i}_arguments")
    done

    local case_name
    case_name=$(basename "$(dirname "$case_file")")

    echo -e "\n${YELLOW}Running: $case_name${NC}"
    echo "Description: $description"
    echo "Trial: $trial_num/$TRIALS"
    echo "Test prompt: $test_prompt"

    # Execute check script
    local check_script_path="$TOOLS_DIR/$check_script"
    if [[ ! -f "$check_script_path" ]]; then
        echo -e "${RED}FAIL: Check script not found: $check_script${NC}"
        ((FAILED++))
        return 1
    fi

    # Run check with test prompt and expected tool calls
    local check_output
    check_output=$("$check_script_path" "$test_prompt" "${tool_names[@]}" "${tool_args[@]}" 2>&1)
    local check_exit_code=$?

    if [[ $check_exit_code -eq 0 ]]; then
        echo -e "${GREEN}PASS${NC}"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}FAIL${NC}"
        echo "$check_output"
        ((FAILED++))
        return 1
    fi
}

# Run test case
run_case() {
    local case_file="$1"

    for ((trial=1; trial<=TRIALS; trial++)); do
        run_trial "$case_file" "$trial"
    done
}

# Find all test cases
find_cases() {
    find "$CASES_DIR" -name "*.toml" -path "*/$CASE_PATTERN/*"
}

# Main
echo "======================================"
echo "Skill-Bench Test Runner"
echo "======================================"
echo "Case pattern: $CASE_PATTERN"
echo "Trials: $TRIALS"
echo ""

# Find and run test cases
local cases
cases=()
while IFS= read -r -d '' case; do
    cases+=("$case")
done < <(find "$CASES_DIR" -name "*.toml" -path "*/$CASE_PATTERN/*" -print0)

if [[ ${#cases[@]} -eq 0 ]]; then
    echo "No test cases found matching pattern: $CASE_PATTERN"
    exit 1
fi

for case in "${cases[@]}"; do
    run_case "$case"
done

# Summary
echo ""
echo "======================================"
echo "Summary"
echo "======================================"
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo "Skipped: $SKIPPED"
echo ""

if [[ $FAILED -gt 0 ]]; then
    echo -e "${RED}Some tests failed${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed${NC}"
    exit 0
fi
