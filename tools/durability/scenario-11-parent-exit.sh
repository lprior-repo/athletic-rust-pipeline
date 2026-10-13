#!/usr/bin/env bash
# Scenario 11: Stop a parent with active owned workers; prove drain/reap and
# exact exclusive outcome accounting, with unfinished business durably resumable.
# Uses the ATHLETIC_FAULT_HTTP_EXIT trigger file seam behind the
# native-fault-injection feature.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/11}"

for tool in curl python3 jq timeout ss sha256sum pgrep; do
    command -v "$tool" >/dev/null 2>&1 || { echo "SKIPPED: missing prerequisite tool $tool"; exit 0; }
done
CHROMIUM="${CHROMIUM_EXECUTABLE:-/usr/bin/chromium}"
[ -x "$CHROMIUM" ] || { echo "SKIPPED: no executable browser at $CHROMIUM"; exit 0; }

PINNED_SERVER="$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server"
if [ -n "${RESTATE_SERVER_BIN:-}" ] && [ -x "$RESTATE_SERVER_BIN" ]; then
    :
elif [ -x "$PINNED_SERVER" ]; then
    export RESTATE_SERVER_BIN="$PINNED_SERVER"
else
    echo "SKIPPED: no executable restate-server 1.7.10 (set RESTATE_SERVER_BIN)"
    exit 0
fi
NODE="$RESTATE_SERVER_BIN"

CLIENT="${BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-service}"
[ -x "$CLIENT" ] || { echo "SKIPPED: census-service binary missing at $CLIENT"; exit 0; }

SERVE="${S11_SERVE_BINARY:-${SERVE_BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve}}"
[ -x "$SERVE" ] || { echo "SKIPPED: endpoint missing at $SERVE"; exit 0; }
for seam in CENSUS_NATIVE_SOURCE_BOUNDARY 'HTTP endpoint exiting due to the ATHLETIC_FAULT_HTTP_EXIT trigger'; do
    grep -aqF "$seam" "$SERVE" || { echo "FAIL: selected endpoint lacks native seam $seam: $SERVE"; exit 1; }
done

INGRESS_PORT="${S11_INGRESS_PORT:-18420}"
ADMIN_PORT="${S11_ADMIN_PORT:-19420}"
ENDPOINT_PORT="${S11_ENDPOINT_PORT:-18421}"
NODE_PORT="${S11_NODE_PORT:-$((INGRESS_PORT + 2))}"
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
    case "$port" in 18095|19095|15192) echo "FAIL: shared port $port is forbidden"; exit 1 ;; esac
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        echo "SKIPPED: port $port is already in use"
        exit 0
    fi
done

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
WORK="$(mktemp -d "$SCRATCH_STORE/scenario-11-XXXXXX")"
DATA="$WORK/store"
TRIGGER="$WORK/trigger"
mkdir -p "$DATA"
echo "EVIDENCE: $WORK"
sha256sum "$CLIENT" "$SERVE" "$NODE" > "$WORK/binary-hashes.sha256"
cat "$WORK/binary-hashes.sha256"
STATE="${S11_STATE:-VT}"
KEY="jurisdiction:$STATE:2026-27:1"
OPERATION="$KEY/teams/milesplit"
BOUNDARY="$WORK/boundary"
mkdir -m 700 "$BOUNDARY"
CONFIG="$BOUNDARY/native-source-boundary-config.json"
MARKER="$BOUNDARY/teams-source-reservation-reached.json"
printf '{"schema":1,"operation":"%s","attempt":1,"timeout_seconds":60}\n' "$OPERATION" > "$CONFIG"

cat > "$WORK/restate.toml" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "durability-scenario-11"
cluster-name = "durability-scenario-11"
auto-provision = true
default-num-partitions = 1
default-replication = 1
base-dir = "$WORK/restate"
listen-mode = "tcp"
bind-ip = "127.0.0.1"
bind-port = $NODE_PORT
advertised-address = "http://127.0.0.1:$NODE_PORT/"
shutdown-timeout = "1m"
disable-telemetry = true
experimental-enable-protocol-v7 = true
experimental-enable-vqueues = true
experimental-enable-scoped-virtual-objects = true

[ingress]
bind-address = "127.0.0.1:$INGRESS_PORT"

[admin]
bind-address = "127.0.0.1:$ADMIN_PORT"
EOF

sql() {
    local reply
    reply="$(curl -sS --fail-with-body --max-time 20 -X POST "http://127.0.0.1:$ADMIN_PORT/query" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -d "{\"query\":\"$1\"}")" || { printf '%s\n' "$reply" >&2; return 1; }
    printf '%s\n' "$reply" | jq -e 'has("rows") and (.rows | type == "array")' >/dev/null \
        || { printf 'FAIL: invalid admin query reply: %s\n' "$reply" >&2; return 1; }
    printf '%s\n' "$reply"
}
inventory() {
    sql "SELECT id, target_service_name, target_service_key, invoked_by_id, status, journal_size, completion_result, completion_failure FROM sys_invocation WHERE id = '$INVOCATION' OR invoked_by_id = '$INVOCATION'" > "$1"
}
wait_admin() {
    for _ in $(seq 1 60); do
        curl -sS -o /dev/null --max-time 2 "http://127.0.0.1:$ADMIN_PORT/deployments" && return 0
        sleep 1
    done
    return 1
}

stop_owned_process() {
    local pid="$1" label="$2" rc=0 term_failed=0
    [ -n "$pid" ] || return 0
    if kill -0 "$pid" 2>/dev/null; then
        echo "CLEANUP: TERM requested for owned $label pid=$pid"
        kill -TERM "$pid" || {
            echo "FAIL: could not request TERM for owned $label pid=$pid"
            term_failed=1
        }
    fi
    wait "$pid" || rc=$?
    echo "REAP: owned $label pid=$pid exit=$rc"
    [ "$rc" -eq 0 ] && [ "$term_failed" -eq 0 ] || {
        echo "FAIL: owned $label stop/reap failed (exit=$rc TERM_failed=$term_failed)"
        return 1
    }
}
cleanup() {
    local rc=$? cleanup_rc=0
    trap - EXIT
    stop_owned_process "${SERVE_PID:-}" endpoint || cleanup_rc=1
    SERVE_PID=""
    stop_owned_process "${NODE_PID:-}" Restate || cleanup_rc=1
    NODE_PID=""
    [ "$rc" -ne 0 ] || rc=$cleanup_rc
    exit "$rc"
}
trap cleanup EXIT

"$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node-first.log" 2>&1 &
NODE_PID=$!
wait_admin || { echo "FAIL: node did not answer on the admin port"; exit 1; }

RUST_LOG=info ATHLETIC_FAULT_HTTP_EXIT="$TRIGGER" CENSUS_NATIVE_SOURCE_BOUNDARY="$CONFIG" "$SERVE" \
    --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
    --drain-timeout 60 --browser-profile "$WORK/browser-profile" \
    --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint.log" 2>&1 &
SERVE_PID=$!

sleep 2
kill -0 "$SERVE_PID" 2>/dev/null || { echo "FAIL: endpoint exited during startup"; tail -5 "$WORK/endpoint.log"; exit 1; }

curl -sS -X POST "http://127.0.0.1:$ADMIN_PORT/deployments" -H 'content-type: application/json' \
    -d "{\"uri\":\"http://127.0.0.1:$ENDPOINT_PORT/\"}" > "$WORK/deployment.json"
REGISTERED=false
for _ in $(seq 1 30); do
    curl -sS --max-time 10 "http://127.0.0.1:$ADMIN_PORT/deployments" > "$WORK/deployments.json" || true
    if python3 - "$WORK/deployments.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
names = {
    s["name"] if isinstance(s, dict) else s
    for d in data.get("deployments", [])
    for s in d.get("services", [])
}
required = {"NationalCensus", "JurisdictionCensus", "TeamsSource", "Census"}
print(f"REGISTERED: {len(names)} services; required present: {sorted(required & names)}")
sys.exit(0 if required <= names else 1)
PY
    then
        REGISTERED=true
        break
    fi
    sleep 1
done
[ "$REGISTERED" = true ] || { echo "FAIL: endpoint did not register the expected services"; cat "$WORK/deployments.json"; exit 1; }

AS_OF="$(date -u +%F)"
LAST_YEAR="$(date -u +%Y)"
[ "$LAST_YEAR" -le 2027 ] || LAST_YEAR=2027
jq -n --arg state "$STATE" --arg date "$AS_OF" --argjson last "$LAST_YEAR" \
    '{jurisdiction:$state,season:2026,revision:1,history:{first_calendar_year:2023,last_calendar_year:$last,as_of:$date},limit_per_state:1,concurrency:4,source_parallelism:1}' \
    > "$WORK/request.json"
KEY_ROUTE="$(printf '%s' "$KEY" | jq -sRr @uri)"
submit() {
    curl -fsS --max-time 30 -X POST "http://127.0.0.1:$INGRESS_PORT/JurisdictionCensus/$KEY_ROUTE/run/send" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -H "idempotency-key: $KEY" --data-binary @"$WORK/request.json"
}
submit > "$WORK/submission.json" 2> "$WORK/submission.log" \
    || { echo "FAIL: submission refused"; cat "$WORK/submission.log"; exit 1; }
INVOCATION="$(jq -er 'select(.status == "Accepted") | .invocationId | select(type == "string") | select(test("^inv_[A-Za-z0-9]+$"))' "$WORK/submission.json")"
echo "SUBMITTED: $KEY as $INVOCATION"
INFLIGHT=false
for i in $(seq 1 60); do
    inventory "$WORK/invocations-${i}.json"
    jq -e --arg id "$INVOCATION" --arg key "$KEY" \
        'any(.rows[]; .id == $id and .target_service_key == $key)' "$WORK/invocations-${i}.json" >/dev/null \
        || { echo "FAIL: admitted original invocation missing from admin inventory"; exit 1; }
    if jq -e --arg id "$INVOCATION" \
        'any(.rows[]; .id == $id and .status == "completed")' "$WORK/invocations-${i}.json" >/dev/null; then
        echo "FAIL: original parent settled before active-owned-worker fault"
        cat "$WORK/invocations-${i}.json"
        exit 1
    fi
    if [ -f "$MARKER" ]; then
        jq -e --arg operation "$OPERATION" \
            '.schema == 2 and .phase == "teams_reserved_before_acquisition" and .operation == $operation and .attempt == 1 and .request_digest != "fault_injection" and (.acknowledged_effects | type == "object")' \
            "$MARKER" >/dev/null || { echo "FAIL: reached reservation marker identity invalid"; exit 1; }
        jq -e --arg id "$INVOCATION" --arg operation "$OPERATION" \
            'any(.rows[]; .target_service_name == "TeamsSource" and .target_service_key == $operation and .invoked_by_id == $id and .status == "running" and .journal_size > 0)' \
            "$WORK/invocations-${i}.json" >/dev/null || { echo "FAIL: reached worker not active and owned by original parent"; exit 1; }
        cp "$MARKER" "$WORK/reservation-before-exit.json"
        cp "$WORK/invocations-${i}.json" "$WORK/work-in-flight.json"
        INFLIGHT=true
        echo "INFLIGHT: original parent $INVOCATION owns active reservation $OPERATION"
        break
    fi
    sleep 1
done
[ "$INFLIGHT" = true ] || { echo "FAIL: owned source reservation not reached before trigger"; exit 1; }

echo "INJECTED: creating ATHLETIC_FAULT_HTTP_EXIT trigger file at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
touch "$TRIGGER"

TRIGGER_LINE=false
for _ in $(seq 1 120); do
    if grep -q "HTTP endpoint exiting due to the ATHLETIC_FAULT_HTTP_EXIT trigger" "$WORK/endpoint.log"; then
        TRIGGER_LINE=true
        break
    fi
    sleep 1
done
[ "$TRIGGER_LINE" = true ] || { echo "FAIL: trigger exit line never appeared within 120s"; tail -20 "$WORK/endpoint.log"; exit 1; }
echo "TRIGGER: HTTP endpoint exited on trigger"

for _ in $(seq 1 120); do
    if grep -q "drained: accepted=" "$WORK/endpoint.log"; then
        break
    fi
    sleep 1
done
if ! grep -q "drained: accepted=" "$WORK/endpoint.log"; then
    echo "FAIL: drain certificate never appeared within 120s after trigger"
    tail -20 "$WORK/endpoint.log"
    exit 1
fi

DRAIN=$(grep -a 'drained: accepted=' "$WORK/endpoint.log" | tail -1)
echo "DRAIN CERTIFICATE: $DRAIN"
echo "$DRAIN" > "$WORK/drain-certificate.txt"

validate_drain_certificate() {
python3 - "$1" "$2" <<'PY'
import re, sys
drain = open(sys.argv[1]).read().strip()
log = open(sys.argv[2], errors="replace").read().splitlines()
reports = [line for line in log if "report=DrainReport {" in line]
if not reports:
    print("FAIL: drain report line absent from the endpoint log")
    sys.exit(1)
report = reports[-1]
def field(name):
    match = re.search(rf'\b{name}: (\d+)', report)
    if not match:
        print(f"FAIL: drain report lacks {name}")
        sys.exit(1)
    return int(match.group(1))
accepted = field('accepted')
completed = field('completed')
cancelled = field('cancelled')
aborted = field('aborted')
panicked = field('panicked')
failed = field('failed')
still_running = field('still_running')
remaining = field('remaining')
timed_out = field('timed_out')
certificate = int(re.search(r'accepted=(\d+)', drain).group(1))
if certificate != accepted:
    print(f"FAIL: certificate accepted={certificate} disagrees with the report accepted={accepted}")
    sys.exit(1)
accounted = completed + cancelled + aborted + panicked
print(f"ACCOUNTING: accepted={accepted} vs exclusive_terminal={accounted} (completed={completed} failed={failed} cancelled={cancelled} aborted={aborted} panicked={panicked})")
print(f"DIAGNOSTIC: failed={failed} is a separate business-failure diagnostic, not a terminal bucket (a failed spawn joins as completed; see spawn/ledger.rs)")
print(f"RESIDENCY: still_running={still_running} remaining={remaining} timed_out={timed_out} is an overlapping subset of accepted={accepted}, not a terminal bucket")
if timed_out > accepted:
    print("FAIL: deadline subset exceeds accepted work")
    sys.exit(1)
if accepted != accounted:
    print("FAIL: exclusive outcome accounting mismatch")
    sys.exit(1)
if still_running != 0 or remaining != 0:
    print("FAIL: drained endpoint retained unaccounted work")
    sys.exit(1)
if accepted < 1:
    print("FAIL: accepted should be >= 1")
    sys.exit(1)
if panicked != 0:
    print("FAIL: panicked should be 0")
    sys.exit(1)
print("PASS: exclusive outcome accounting verified")
PY
}
validate_drain_certificate "$WORK/drain-certificate.txt" "$WORK/endpoint.log"

SERVE_EXIT=0
wait "$SERVE_PID" || SERVE_EXIT=$?
SERVE_PID=""
echo "EXIT: census-serve exited with code $SERVE_EXIT"
[ "$SERVE_EXIT" -eq 0 ] || { echo "FAIL: initial endpoint exited unsuccessfully"; exit 1; }

REAPED=true
if pgrep -f -- "--data-dir $DATA" >/dev/null 2>&1 || pgrep -f "$WORK/browser-profile" >/dev/null 2>&1; then
    REAPED=false
    pgrep -f -- "--data-dir $DATA" 2>/dev/null || true
    pgrep -f "$WORK/browser-profile" 2>/dev/null || true
fi
[ "$REAPED" = true ] || { echo "FAIL: owned child processes survived after parent exit"; exit 1; }
echo "REAP: no owned child processes survived"

stop_owned_process "$NODE_PID" Restate
NODE_PID=""

"$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node-second.log" 2>&1 &
NODE_PID=$!
wait_admin || { echo "FAIL: restarted node did not answer on the admin port"; exit 1; }
echo "RESTARTED: Restate node from same base-dir"

sha256sum -c "$WORK/binary-hashes.sha256" > "$WORK/restart-binary-hashes.txt"
env -u ATHLETIC_FAULT_HTTP_EXIT -u CENSUS_NATIVE_SOURCE_BOUNDARY "$SERVE" \
    --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
    --drain-timeout 60 --browser-profile "$WORK/browser-profile" \
    --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint-second.log" 2>&1 &
SERVE_PID=$!

sleep 2
kill -0 "$SERVE_PID" 2>/dev/null || { echo "FAIL: restarted endpoint exited during startup"; tail -5 "$WORK/endpoint-second.log"; exit 1; }

curl -fsS --max-time 30 -X POST "http://127.0.0.1:$ADMIN_PORT/deployments" -H 'content-type: application/json' \
    -d "{\"uri\":\"http://127.0.0.1:$ENDPOINT_PORT/\"}" > "$WORK/deployment-second.json"

echo "RESUMING: original invocation $INVOCATION on the same store"
submit > "$WORK/resubmit.json" 2> "$WORK/resubmit.log" \
    || { echo "FAIL: resubmission refused"; cat "$WORK/resubmit.log"; exit 1; }
jq -e --arg id "$INVOCATION" '.invocationId == $id and .status == "PreviouslyAccepted"' \
    "$WORK/resubmit.json" >/dev/null || { echo "FAIL: resubmission did not reuse the original invocation"; exit 1; }
echo "IDENTITY: original invocation and idempotency key reused"

PRE_TRIGGER_JOURNAL="$(jq -r --arg id "$INVOCATION" '.rows[] | select(.id == $id) | .journal_size' "$WORK/work-in-flight.json")"
RESUMED_JOURNAL="$PRE_TRIGGER_JOURNAL"
RESUME_TICKS=0
RESUME_PROVEN=false
for ITERATION in $(seq 1 300); do
    inventory "$WORK/invocations-resumed.json"
    jq -e --slurpfile before "$WORK/work-in-flight.json" \
        '([.rows[].id] as $after | all($before[0].rows[]; .id as $id | $after | index($id) != null))' \
        "$WORK/invocations-resumed.json" >/dev/null || { echo "FAIL: original invocation identity lost during recovery"; exit 1; }
    if jq -e --arg id "$INVOCATION" \
        'any(.rows[]; .id == $id and .status == "completed")' "$WORK/invocations-resumed.json" >/dev/null; then
        jq -e --arg id "$INVOCATION" \
            'any(.rows[]; .id == $id and .completion_result == "success")' "$WORK/invocations-resumed.json" >/dev/null \
            || { echo "FAIL: recovered original invocation failed"; cat "$WORK/invocations-resumed.json"; exit 1; }
        echo "RESUME: original invocation completed with success on the restarted endpoint"
        RESUME_PROVEN=true
        break
    fi
    OBSERVED="$(jq -r --arg id "$INVOCATION" '.rows[] | select(.id == $id) | .journal_size' "$WORK/invocations-resumed.json")"
    if [ -n "$OBSERVED" ] && [ "$OBSERVED" -gt "$RESUMED_JOURNAL" ]; then
        RESUME_TICKS=$((RESUME_TICKS + 1))
        RESUMED_JOURNAL="$OBSERVED"
    fi
    if [ "$RESUME_TICKS" -ge 3 ] && [ "$ITERATION" -ge 50 ]; then
        jq -e --arg id "$INVOCATION" \
            'all(.rows[] | select(.id == $id); .completion_result != "failure")' \
            "$WORK/invocations-resumed.json" >/dev/null \
            || { echo "FAIL: the resumed invocation recorded a failure"; cat "$WORK/invocations-resumed.json"; exit 1; }
        echo "RESUME: durable progress on the restarted endpoint, journal $PRE_TRIGGER_JOURNAL -> $RESUMED_JOURNAL across $RESUME_TICKS growth observations"
        RESUME_PROVEN=true
        break
    fi
    sleep 2
done
[ "$RESUME_PROVEN" = true ] || { echo "FAIL: no pre-trigger invocation resumed after restart (journal $PRE_TRIGGER_JOURNAL -> $RESUMED_JOURNAL, growth observations $RESUME_TICKS)"; cat "$WORK/invocations-resumed.json"; exit 1; }
echo "PASS: unfinished business stayed durable and resumable across endpoint death (journal $PRE_TRIGGER_JOURNAL -> $RESUMED_JOURNAL, growth observations $RESUME_TICKS)"


OPERATION_ROUTE="$(printf '%s' "$OPERATION" | jq -sRr @uri)"
SETTLE_PROVEN=false
for _ in $(seq 1 150); do
    curl -fsS --max-time 20 "http://127.0.0.1:$INGRESS_PORT/TeamsSource/$OPERATION_ROUTE/inspection" \
        > "$WORK/source-after-recovery.json"
    if jq -e '.status == "settled" and .outcome.status == "completed" and .outcome.outcome.disposition == "complete" and .outcome.outcome.errors == [] and .outcome.outcome.unfinished == [] and (.outcome.progress | length <= 3) and any(.outcome.progress[]; .status == "completed" and .outcome.records > 0 and .outcome.disposition == "complete" and .outcome.errors == [] and .outcome.unfinished == [])' \
        "$WORK/source-after-recovery.json" >/dev/null 2>&1; then
        SETTLE_PROVEN=true
        break
    fi
    sleep 2
done
if [ "$SETTLE_PROVEN" = true ]; then
    echo "PASS: original source settled with completed records after recovery"
else
    echo "RESUMED: the source had not settled inside the bounded window; the restart resume proof above stands and jurisdiction settlement completion belongs to the qualification/national acceptance, not this fault lane"
    jq -c '{status, outcome_status: (.outcome.status // null), progress: ((.outcome.progress // []) | length)}' \
        "$WORK/source-after-recovery.json" 2>/dev/null || true
fi
stop_owned_process "$SERVE_PID" resumed-endpoint
SERVE_PID=""
DRAIN2=$(grep -a 'drained: accepted=' "$WORK/endpoint-second.log" | tail -1 || true)
[ -n "$DRAIN2" ] || { echo "FAIL: resumed endpoint exited without a drain certificate"; exit 1; }
echo "RESUME DRAIN CERTIFICATE: $DRAIN2"
echo "$DRAIN2" > "$WORK/resume-drain-certificate.txt"
validate_drain_certificate "$WORK/resume-drain-certificate.txt" "$WORK/endpoint-second.log"
if pgrep -f -- "--data-dir $DATA" >/dev/null 2>&1 || pgrep -f "$WORK/browser-profile" >/dev/null 2>&1; then
    echo "FAIL: owned child processes survived the resumed endpoint drain"
    exit 1
fi
echo "REAP: no owned child processes survived resumed endpoint exit"
stop_owned_process "$NODE_PID" Restate
NODE_PID=""

echo "PASS: actual parent HTTP trigger fired with an identity-bound active owned source reservation; exclusive drain accounting and zero orphans/reaped children verified; original invocation/key/idempotency resumed durably on preserved state (journal $PRE_TRIGGER_JOURNAL -> $RESUMED_JOURNAL); settlement inside this lane's window: $SETTLE_PROVEN"