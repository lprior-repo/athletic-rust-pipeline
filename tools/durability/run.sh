#!/usr/bin/env bash
# runner for tools/durability/ — executes every scenario script, prints a table,
# exits nonzero on any FAIL or SKIPPED.
#
# Usage:
#   tools/durability/run.sh                          # run all scenarios
#   tools/durability/run.sh scenario-NN-name.sh      # run one scenario
#
# Environment overrides:
#   SCRATCH_STORE   — base directory for scenario stores (default /tmp/durability-scenario)
#   BINARY          — census-service binary (default: look on PATH, then target/release)
#   SERVE_BINARY    — census-serve binary (default: $BINARY)
#   RESTATE_BINARY  — restate-server binary (default: look on PATH; optional)
#   RESTATE_SERVER_BIN — pinned restate-server 1.7.10 (default: same as RESTATE_BINARY)
#   CORPUS_FIXTURE  — corpus fixture directory (default: fixtures/alpha/)
#   ADMIN_PORT      — Restate admin port (default: 19095)
#   SERVICE_PORT    — Census endpoint port (default: 9080)
#   TMPDIR          — temporary directory for scenario isolation (default: /tmp/durability-scenario/tmp)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"


# Defaults
SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario}"
mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
TMPDIR="${TMPDIR:-$SCRATCH_STORE/tmp}"
mkdir -p "$TMPDIR"
TMPDIR="$(cd "$TMPDIR" && pwd)"
BINARY_PATH=""
if command -v census-service >/dev/null 2>&1; then
    BINARY_PATH="$(command -v census-service)"
elif test -x "$REPO_ROOT/target/release/census-service"; then
    BINARY_PATH="$REPO_ROOT/target/release/census-service"
elif test -x "$REPO_ROOT/target/debug/census-service"; then
    BINARY_PATH="$REPO_ROOT/target/debug/census-service"
fi
BINARY="${BINARY:-$BINARY_PATH}"
SERVE_BINARY="${SERVE_BINARY:-$(dirname "$BINARY")/census-serve}"
if [ -z "$BINARY" ]; then
    echo "SKIP: no built endpoint binary (checked PATH, target/release/census-service, target/debug/census-service)"
    # Still run all scenarios so they report their own reasons
fi
RESTATE_BINARY="${RESTATE_BINARY:-${RESTATE_SERVER_BIN:-$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server}}"
RESTATE_SERVER_BIN="${RESTATE_SERVER_BIN:-$RESTATE_BINARY}"
CORPUS_FIXTURE="${CORPUS_FIXTURE:-$REPO_ROOT/fixtures/alpha}"
ADMIN_PORT="${ADMIN_PORT:-19095}"
SERVICE_PORT="${SERVICE_PORT:-9080}"

# Validate census-service binary (graceful skip if absent; scenarios check BINARY_AVAILABLE)
BINARY_AVAILABLE=false
if [ -n "$BINARY" ] && "$BINARY" --help >/dev/null 2>&1; then
    BINARY_AVAILABLE=true
fi

# Restate binary is optional; scenarios that need it will skip if unavailable
RESTATE_AVAILABLE=true
if ! "$RESTATE_BINARY" --help >/dev/null 2>&1; then
    RESTATE_AVAILABLE=false
fi

# Discover scenario scripts
if [ $# -gt 0 ]; then
    SCENARIOS=()
    for arg in "$@"; do
        base="$arg"
        [[ "$base" != *.sh ]] && base="${base}.sh"
        path="$SCRIPT_DIR/$base"
        if [ -f "$path" ]; then
            SCENARIOS+=("$path")
        else
            echo "ERROR: scenario script not found: $path" >&2
            exit 2
        fi
    done
else
    mapfile -t SCENARIOS < <(
        find "$SCRIPT_DIR" -maxdepth 1 -name 'scenario-*.sh' -type f | sort
    )
    if [ ${#SCENARIOS[@]} -eq 0 ]; then
        echo "ERROR: no scenario scripts found in $SCRIPT_DIR" >&2
        exit 2
    fi
fi

# Export environment for scenario scripts
export SCRATCH_STORE TMPDIR BINARY SERVE_BINARY RESTATE_BINARY RESTATE_SERVER_BIN CORPUS_FIXTURE ADMIN_PORT SERVICE_PORT REPO_ROOT BINARY_AVAILABLE RESTATE_AVAILABLE

# Results collection
declare -A RESULTS  # name -> PASS|FAIL|SKIPPED
declare -A REASONS  # name -> reason (for SKIPPED/FAIL)

run_scenario() {
    local script="$1"
    local name
    name="$(basename "$script" .sh)"

    echo "──────────────────────────────────────────────────────────"
    echo "Running: $name"
    echo "──────────────────────────────────────────────────────────"

    local output
    local rc=0
    set +e
    output=$(bash "$script" 2>&1) || rc=$?
    set -e
    rc=${rc:-0}

    echo "$output"

    # Non-zero child exit code is always a failure
    if [ "$rc" -ne 0 ]; then
        RESULTS["$name"]="FAIL"
        REASONS["$name"]="exit code $rc"
        return
    fi

    # Must have PASS in output and NO SKIPPED to count as passing.
    # Any SKIPPED output means unverified seams — the scenario is unverified.
    if echo "$output" | grep -q "^SKIPPED:"; then
        RESULTS["$name"]="SKIPPED"
        REASONS["$name"]=$(echo "$output" | grep "^SKIPPED:" | head -1 | sed 's/^SKIPPED: //')
    elif echo "$output" | grep -q "^PASS:"; then
        RESULTS["$name"]="PASS"
        REASONS["$name"]=""
    else
        RESULTS["$name"]="SKIPPED"
        REASONS["$name"]="no PASS or SKIPPED marker in output"
    fi
}

# Execute all scenarios
for script in "${SCENARIOS[@]}"; do
    run_scenario "$script"
done

# Print the summary table
echo ""
echo "══════════════════════════════════════════════════════════"
echo "DURABILITY HARNESS — RESULTS"
echo "══════════════════════════════════════════════════════════"
printf "%-50s %7s  %s\n" "SCENARIO" "STATUS" "DETAIL"
printf "%-50s %7s  %s\n" "$(printf '%0.s─' {1..50})" "$(printf '%0.s─' {1..7})" "────────"

ANY_FAIL=0
for name in $(echo "${!RESULTS[@]}" | tr ' ' '\n' | sort); do
    status="${RESULTS[$name]}"
    reason="${REASONS[$name]:-}"
    printf "%-50s %7s" "$name" "$status"
    if [ "$status" = "SKIPPED" ] || [ "$status" = "FAIL" ]; then
        printf "  %s" "$reason"
    fi
    printf "\n"
    if [ "$status" = "FAIL" ] || [ "$status" = "SKIPPED" ]; then
        ANY_FAIL=1
    fi
done

echo ""
pass_count=0; fail_count=0; skip_count=0
for status in "${RESULTS[@]}"; do
    case "$status" in
        PASS)    pass_count=$((pass_count + 1)) ;;
        FAIL)    fail_count=$((fail_count + 1)) ;;
        SKIPPED) skip_count=$((skip_count + 1)) ;;
    esac
done

echo "Total: $pass_count PASS, $fail_count FAIL, $skip_count SKIPPED (${#RESULTS[@]} scenarios)"
echo ""

exit $ANY_FAIL
