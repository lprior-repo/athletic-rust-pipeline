#!/usr/bin/env bash
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/08}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"

for tool in timeout jq tee sha256sum realpath mktemp; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "FAIL: missing scenario08 prerequisite $tool"
        exit 1
    fi
done
if [ -z "${MOON_RUST_CARGO:-}" ]; then
    echo "FAIL: invoke through env -u CI tools/moon-local run pipeline:durability -- scenario-08-global-budget"
    exit 1
fi
BINARY="${BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-service}"
if [ ! -x "$BINARY" ]; then
    echo "FAIL: native census-service executable missing: $BINARY"
    exit 1
fi
BINARY="$(realpath -e "$BINARY")"
export BINARY

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
EVIDENCE_DIR="$(mktemp -d "$SCRATCH_STORE/scenario-08-XXXXXX")"
echo "EVIDENCE: $EVIDENCE_DIR"
export CENSUS_ORIGIN_BUDGET_EVIDENCE="$EVIDENCE_DIR"
export TMPDIR="$EVIDENCE_DIR/tmp"
mkdir -p "$TMPDIR"
chmod 700 "$EVIDENCE_DIR"
read -r BINARY_HASH _ < <(sha256sum "$BINARY")
cd "$REPO_ROOT"
TEST=independent_cli_workflows_do_not_multiply_the_physical_origin_budget
LOG="$EVIDENCE_DIR/native-test.log"
set +e
timeout --signal=TERM --kill-after=30s 900s tools/moon-cargo test \
    -p census-service --test origin_budget --locked --offline -- \
    --exact "$TEST" --nocapture 2>&1 | tee "$LOG"
CODE=$?
set -e
if [ "$CODE" -ne 0 ]; then
    echo "FAIL: native origin-budget regression exited $CODE; retained $LOG"
    exit 1
fi
if ! grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$LOG" \
    || grep -qE '^SKIPPED:|^test result: FAILED' "$LOG"; then
    echo "FAIL: scenario08 did not execute exactly one non-skipped native regression; retained $LOG"
    exit 1
fi

shopt -s nullglob
CERTIFICATES=("$EVIDENCE_DIR"/native-origin-budget-*/origin-budget.json)
if [ "${#CERTIFICATES[@]}" -ne 1 ]; then
    echo "FAIL: scenario08 expected exactly one freshly created certificate"
    exit 1
fi
CERTIFICATE="${CERTIFICATES[0]}"
WORK="${CERTIFICATE%/*}"
if [ ! -f "$CERTIFICATE" ] || [ -L "$CERTIFICATE" ] \
    || [ ! -f "$WORK/http-ledger.jsonl" ] || [ -L "$WORK/http-ledger.jsonl" ]; then
    echo "FAIL: native certificate/physical ledger missing or symlinked"
    exit 1
fi
read -r CURRENT_HASH _ < <(sha256sum "$BINARY")
read -r LEDGER_HASH _ < <(sha256sum "$WORK/http-ledger.jsonl")
if [ "$CURRENT_HASH" != "$BINARY_HASH" ]; then
    echo "FAIL: census-service executable changed during qualification"
    exit 1
fi
if ! jq -e --arg binary "$BINARY" --arg hash "$BINARY_HASH" --arg ledger "$LEDGER_HASH" --arg work "$WORK" '
    def positive_pid: type == "number" and . > 0 and . == floor;
    def refused($origin; $pid):
        .exit_code == 1 and (.stderr | rtrimstr("\n") |
            ltrimstr("Error: origin " + $origin + " is held by another census-service process (holder: ") |
            rtrimstr(")") | fromjson | .pid == $pid and .origin == $origin);
    . as $certificate |
    .schema == 1 and .scenario == 8
    and (.scope | contains("no serving Restate endpoints or distributed coordination exercised"))
    and (.origin | test("^http://127\\.0\\.0\\.1:[0-9]+$"))
    and .configuration.delay_ms == 1000 and .configuration.clock_tolerance_ms == 50
    and .configuration.inflight_budget == 1 and .configuration.burst_budget == 1
    and .configuration.rival_workflows == 2 and .configuration.origin_lock_root == "var/locks"
    and .identities.cli.path == $binary and .identities.cli.sha256 == $hash
    and (.identities.regression.sha256 | test("^[0-9a-f]{64}$"))
    and .http_ledger_sha256 == $ledger
    and .facts.children_reaped == 4 and .facts.reached_connection == 2
    and .facts.owner.exit_code == 0 and .facts.replay.exit_code == 0
    and .facts.owner.store == .facts.replay.store
    and .facts.holder.pid == .facts.owner.pid and .facts.holder.origin == .origin
    and ([.facts.owner.pid, .facts.rivals[].pid, .facts.replay.pid] |
        length == 4 and all(.[]; positive_pid) and (unique | length) == 4)
    and ([.facts.owner, .facts.rivals[], .facts.replay] |
        all(.[]; .working_directory == $work))
    and ([.facts.owner.store, .facts.rivals[].store] | (unique | length) == 3)
    and (.facts.rivals | length == 2 and all(.[]; refused($certificate.origin; $certificate.facts.owner.pid)))
    and (.requests | length == 2 and map(.path) == ["/robots.txt", "/owner-target"]
        and all(.[]; .method == "GET" and .user_agent == "CensusOriginBudgetQualification/owner"
            and .response_status == 200 and .end_ns >= .start_ns))
    and .measurements.max_inflight == 1 and .measurements.max_burst == 1
    and .measurements.burst_window_ns == 950000000
    and .measurements.min_spacing_ns >= 950000000
    and .measurements.min_spacing_ns == (.requests[1].start_ns - .requests[0].start_ns)
    and .requests[0].end_ns <= .requests[1].start_ns
    and .measurements.admission_intervals == 1
    and .measurements.admissions_per_second <= (1000 / 950)
    and .facts.cache.metadata.status == 200 and .facts.cache.metadata.method == "GET"
    and .facts.cache.metadata.url == (.origin + "/owner-target")
    and .facts.cache.metadata.response_url == .facts.cache.metadata.url
    and .facts.cache.body_sha256 == .facts.cache.metadata.content_digest
    and (.facts.cache.metadata.fetched_at | type == "string" and length > 0)
' "$CERTIFICATE" >/dev/null; then
    echo "FAIL: exact native origin-budget certificate invalid: $CERTIFICATE"
    exit 1
fi
REGRESSION_BINARY="$(jq -er '.identities.regression.path' "$CERTIFICATE")"
read -r REGRESSION_HASH _ < <(sha256sum "$REGRESSION_BINARY")
if ! jq -e --arg hash "$REGRESSION_HASH" '.identities.regression.sha256 == $hash' "$CERTIFICATE" >/dev/null; then
    echo "FAIL: regression executable identity no longer matches certificate"
    exit 1
fi
CAPTURE_BODY="$(jq -er '.facts.cache.body_path' "$CERTIFICATE")"
CAPTURE_META="$(jq -er '.facts.cache.metadata_path' "$CERTIFICATE")"
for artifact in "$CAPTURE_BODY" "$CAPTURE_META"; do
    if [ ! -f "$artifact" ] || [ -L "$artifact" ]; then
        echo "FAIL: completed capture artifact missing or symlinked: $artifact"
        exit 1
    fi
done
read -r BODY_HASH _ < <(sha256sum "$CAPTURE_BODY")
read -r META_HASH _ < <(sha256sum "$CAPTURE_META")
if ! jq -e --arg body "$BODY_HASH" --arg meta "$META_HASH" \
    '.facts.cache.body_sha256 == $body and .facts.cache.metadata_sha256 == $meta' \
    "$CERTIFICATE" >/dev/null; then
    echo "FAIL: completed capture bytes no longer match native certificate"
    exit 1
fi

echo "PROOF: $CERTIFICATE"
echo "PASS: three independent native CLI acquisition workflows shared one origin; two typed owner-PID refusals emitted zero physical traffic; observed robots/target 200 admissions met 1000ms pacing (50ms clock tolerance), peak in-flight 1 and burst 1; owner lock released and cache replay preserved exact capture facts"
echo "SCOPE: one-host CLI workflows with a common production var/locks root; no multiple serving Restate endpoints or distributed coordination claim"
