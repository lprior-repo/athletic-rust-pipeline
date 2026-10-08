#!/usr/bin/env bash
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/17}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"

for tool in timeout jq tee; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "FAIL: missing scenario17 prerequisite $tool"
        exit 1
    fi
done
if [ -z "${MOON_RUST_CARGO:-}" ]; then
    echo "FAIL: invoke through env -u CI tools/moon-local run pipeline:durability -- scenario-17-recovery-tests"
    exit 1
fi

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
EVIDENCE_DIR=$(mktemp -d "$SCRATCH_STORE/scenario-17-XXXXXX")
echo "EVIDENCE: $EVIDENCE_DIR"
export TMPDIR="$EVIDENCE_DIR/tmp"
export CENSUS_NATIVE_RECOVERY_EVIDENCE="$EVIDENCE_DIR/certificates"
mkdir -p "$TMPDIR" "$CENSUS_NATIVE_RECOVERY_EVIDENCE"
chmod 700 "$CENSUS_NATIVE_RECOVERY_EVIDENCE"
cd "$REPO_ROOT"

run_lane() {
    local test="$1" log="$EVIDENCE_DIR/$2.log" code
    set +e
    timeout --signal=TERM 900s tools/moon-cargo test -p census-service \
        --features native-fault-injection --test recovery --locked --offline -- \
        --exact "$test" --nocapture 2>&1 | tee "$log"
    code=$?
    set -e
    if [ "$code" -ne 0 ]; then
        echo "FAIL: $test exited $code; retained $log"
        exit 1
    fi
    if ! grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$log" \
        || grep -qE '^SKIPPED:|^test result: FAILED' "$log"; then
        echo "FAIL: $test did not execute exactly once without skips; retained $log"
        exit 1
    fi
}

run_lane interrupted_worker::sigkill_mid_batch_worker_restart_completes_the_remaining_units source
run_lane interrupted_worker::sigkill_derived_generation_restart_never_exposes_partial_state derived

for phase in source_batch_staged_before_commit source_chunk_committed_before_next derived_batch_staged_before_publish; do
    certificate="$CENSUS_NATIVE_RECOVERY_EVIDENCE/$phase.json"
    if [ ! -f "$certificate" ] || [ -L "$certificate" ]; then
        echo "FAIL: reached boundary certificate absent or not an owned regular file: $phase"
        exit 1
    fi
    if ! jq -e --arg phase "$phase" '
        def exact_set: type == "array" and length > 0 and . == (sort | unique);
        .schema == 1 and .phase == $phase
        and .marker.schema == 1 and .marker.phase == $phase
        and (.marker.pid | type == "number" and . > 0 and . == floor)
        and .killed_pid == .marker.pid and .signal == 9
        and (.marker.operation | type == "string" and length > 0)
        and (.marker.input_digest | type == "string" and test("^[0-9a-f]{64}$"))
        and .same_operation_recovered == true
        and .atomic_visibility_proven == true
        and .exact_remaining_recovery_proven == true
        and (.expected_ids | exact_set) and .expected_ids == .recovered_ids
        and (.expected_receipts | exact_set) and .expected_receipts == .recovered_receipts
        and (if $phase == "derived_batch_staged_before_publish" then
            (.previous_generation | type == "number" and . > 0 and . == floor)
            and .staged_generation == .marker.generation
            and .staged_generation > .previous_generation
            and .visible_generation_after_kill == .previous_generation
            and .staged_generation_not_visible == true
            and .complete_generation_published == true
        else
            (.marker.ordinal | type == "number" and . > 0 and . == floor)
        end)
    ' "$certificate" >/dev/null; then
        echo "FAIL: scenario17 exact reached/recovered boundary certificate invalid: $phase"
        exit 1
    fi
    echo "PROOF: exact reached SIGKILL and recovery certificate: $phase ($certificate)"
done

echo "PASS: actual source batch, completed chunk and unpublished derived-generation SIGKILL boundaries reached; exact remaining recovery and atomic visibility certified"
