#!/usr/bin/env bash
# Scenario 11: Stop a parent with active owned workers; prove drain/reap and
# exact exclusive outcome accounting, with unfinished business durably resumable.
# Uses the ATHLETIC_FAULT_HTTP_EXIT trigger file seam behind the
# native-fault-injection feature.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/11}"

for tool in curl python3 timeout ss; do
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

SERVE="${SERVE_BINARY:-$REPO_ROOT/target/moon-build/x86_64-unknown-linux-gnu/release/census-serve}"
if [ ! -x "$SERVE" ]; then
    echo "SKIPPED: feature-enabled endpoint missing; build it with"
    echo "SKIPPED: env -u CI tools/moon-local run pipeline:build -- --release -p census-service --features native-fault-injection --bin census-serve"
    exit 0
fi

INGRESS_PORT="${S11_INGRESS_PORT:-18420}"
ADMIN_PORT="${S11_ADMIN_PORT:-19420}"
ENDPOINT_PORT="${S11_ENDPOINT_PORT:-18421}"
NODE_PORT=$((INGRESS_PORT + 2))
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
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
    curl -sS --max-time 20 -X POST "http://127.0.0.1:$ADMIN_PORT/query" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -d "{\"query\":\"$1\"}"
}
inventory() {
    sql "SELECT id, target_service_name, target_service_key, status, journal_size, progress FROM sys_invocation" > "$1"
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

ATHLETIC_FAULT_HTTP_EXIT="$TRIGGER" "$SERVE" \
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

CANDIDATES="VT RI DE DC SD WY ND AK"
STATE=""
SUBMIT_LOG="$WORK/submission.log"
for candidate in $CANDIDATES; do
    echo "SUBMITTING: jurisdiction $candidate"
    "$CLIENT" jurisdiction "$candidate" --ingress "http://127.0.0.1:$INGRESS_PORT/" \
        --season 2026 --revision 1 --limit-per-state 1 --detach \
        > "$SUBMIT_LOG" 2>&1 && { STATE="$candidate"; break; }
    if grep -q 'held by another census-service process' "$SUBMIT_LOG"; then
        echo "SKIPPED: $candidate held by another process"
    else
        echo "WARNING: $candidate failed unexpectedly:"
        cat "$SUBMIT_LOG"
    fi
done
[ -n "$STATE" ] || { echo "FAIL: no jurisdiction could be submitted"; exit 1; }
echo "SUBMITTED: jurisdiction $STATE revision 1"
grep -q "jurisdiction:{$STATE}" "$SUBMIT_LOG" || grep -q "jurisdiction:$STATE" "$SUBMIT_LOG" || { echo "FAIL: submission did not report a run identity"; cat "$SUBMIT_LOG"; exit 1; }

INFLIGHT=false
for i in $(seq 1 900); do
    inventory "$WORK/invocations-${i}.json"
    FOUND=$(python3 - "$WORK/invocations-${i}.json" <<'PY'
import json, sys
rows = json.load(open(sys.argv[1])).get("rows", [])
live = []
for r in rows:
    if r.get("status") in ("running", "backing-off", "pending", "suspended"):
        js = r.get("journal_size", 0)
        if isinstance(js, (int, float)) and js > 0:
            live.append(r)
if live:
    print(json.dumps(live, indent=2))
PY
)
    if [ -n "$FOUND" ]; then
        INFLIGHT=true
        cp "$WORK/invocations-${i}.json" "$WORK/work-in-flight.json"
        echo "$FOUND" > "$WORK/live-invocations.json"
        echo "INFLIGHT: live invocations captured at iteration $i"
        python3 - "$WORK/work-in-flight.json" <<'PY'
import json, sys
rows = json.load(open(sys.argv[1])).get("rows", [])
live = [r for r in rows if r.get("status") in ("running", "backing-off", "pending", "suspended")]
if live:
    print(f"LIVE: {len(live)} invocation(s) active:")
    for r in live[:3]:
        print(f"  {r.get('id', '?')} {r.get('target_service_name', '?')} status={r.get('status', '?')} journal={r.get('journal_size', '?')}")
PY
        break
    fi
    if [ $((i % 60)) -eq 0 ]; then echo "WAITING: no live invocations after $((i * 2))s"; fi
    sleep 2
done
[ "$INFLIGHT" = true ] || { echo "FAIL: no live invocations found before trigger"; exit 1; }

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
python3 - "$1" <<'PY'
import re, sys
drain = open(sys.argv[1]).read().strip()
accepted = int(re.search(r'accepted=(\d+)', drain).group(1))
completed = int(re.search(r'completed=(\d+)', drain).group(1))
cancelled = int(re.search(r'cancelled=(\d+)', drain).group(1))
timed_out = int(re.search(r'timed_out=(\d+)', drain).group(1))
aborted = int(re.search(r'aborted=(\d+)', drain).group(1))
panicked = int(re.search(r'panicked=(\d+)', drain).group(1))
accounted = completed + cancelled + aborted + panicked
print(f"ACCOUNTING: accepted={accepted} vs exclusive_terminal={accounted} (completed={completed} cancelled={cancelled} aborted={aborted} panicked={panicked})")
print(f"DEADLINE: timed_out={timed_out} is an overlapping subset of accepted={accepted}, not a terminal bucket")
if timed_out > accepted:
    print("FAIL: deadline subset exceeds accepted work")
    sys.exit(1)
if accepted != accounted:
    print("FAIL: exclusive outcome accounting mismatch")
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
validate_drain_certificate "$WORK/drain-certificate.txt"

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

"$SERVE" \
    --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
    --drain-timeout 60 --browser-profile "$WORK/browser-profile" \
    --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint-second.log" 2>&1 &
SERVE_PID=$!

sleep 2
kill -0 "$SERVE_PID" 2>/dev/null || { echo "FAIL: restarted endpoint exited during startup"; tail -5 "$WORK/endpoint-second.log"; exit 1; }

curl -sS -X POST "http://127.0.0.1:$ADMIN_PORT/deployments" -H 'content-type: application/json' \
    -d "{\"uri\":\"http://127.0.0.1:$ENDPOINT_PORT/\"}" > "$WORK/deployment-second.json" || true

echo "RESUMING: re-submitting jurisdiction $STATE revision 1 on same store"
"$CLIENT" jurisdiction "$STATE" --ingress "http://127.0.0.1:$INGRESS_PORT/" \
    --season 2026 --revision 1 --limit-per-state 1 --detach \
    > "$WORK/resubmit.log" 2>&1 || {
    echo "FAIL: resubmission rejected"; cat "$WORK/resubmit.log"; exit 1;
}
echo "RESUBMIT LOG:"
cat "$WORK/resubmit.log"

ORIGINAL_IDENTITY=$(grep -oE 'jurisdiction[^ "]*' "$SUBMIT_LOG" | head -1 || true)
RESUBMIT_IDENTITY=$(grep -oE 'jurisdiction[^ "]*' "$WORK/resubmit.log" | head -1 || true)
echo "IDENTITY: original=$ORIGINAL_IDENTITY resubmit=$RESUBMIT_IDENTITY"
if [ -z "$ORIGINAL_IDENTITY" ] || [ "$ORIGINAL_IDENTITY" != "$RESUBMIT_IDENTITY" ]; then
    echo "FAIL: resubmission did not reuse the original run identity"
    exit 1
fi
echo "IDENTITY: resubmission reused the same run identity"

RESUME_PROVEN=false
for _ in $(seq 1 60); do
    inventory "$WORK/invocations-resumed.json" || true
    if python3 - "$WORK/work-in-flight.json" "$WORK/invocations-resumed.json" <<'PY'
import json, sys
before_rows = json.load(open(sys.argv[1])).get("rows", [])
after = {r["id"]: r for r in json.load(open(sys.argv[2])).get("rows", [])}
terminal = ("completed", "failed", "cancelled", "timed-out", "aborted", "panicked")
progressed = []
for r in before_rows:
    a = after.get(r["id"])
    if a is None:
        continue
    if (
        a.get("status") in terminal
        or a.get("status") != r.get("status")
        or a.get("journal_size", 0) > r.get("journal_size", 0)
    ):
        progressed.append(
            (r["id"], r.get("status"), a.get("status"), r.get("journal_size"), a.get("journal_size"))
        )
if progressed:
    for row in progressed:
        print(f"RESUMED: {row[0]} {row[1]} -> {row[2]} journal {row[3]} -> {row[4]}")
    sys.exit(0)
sys.exit(1)
PY
    then
        RESUME_PROVEN=true
        break
    fi
    sleep 2
done
[ "$RESUME_PROVEN" = true ] || { echo "FAIL: no pre-trigger invocation resumed after restart"; exit 1; }

"$CLIENT" open-work --ingress "http://127.0.0.1:$INGRESS_PORT/" --season 2026 --revision 1 \
    > "$WORK/open-work-resumed.txt" 2>&1 || true
echo "OPEN WORK AFTER RESUME:"
cat "$WORK/open-work-resumed.txt"

stop_owned_process "$SERVE_PID" resumed-endpoint
SERVE_PID=""
DRAIN2=$(grep -a 'drained: accepted=' "$WORK/endpoint-second.log" | tail -1 || true)
[ -n "$DRAIN2" ] || { echo "FAIL: resumed endpoint exited without a drain certificate"; exit 1; }
echo "RESUME DRAIN CERTIFICATE: $DRAIN2"
echo "$DRAIN2" > "$WORK/resume-drain-certificate.txt"
validate_drain_certificate "$WORK/resume-drain-certificate.txt"
stop_owned_process "$NODE_PID" Restate
NODE_PID=""

echo "PASS: parent HTTP server exited on trigger; both drain certificates satisfied accepted == completed+cancelled+aborted+panicked with accepted >= 1, panicked == 0 and overlapping timed_out <= accepted; both owned endpoints drained/reaped; the same run identity resumed on the same store and a pre-trigger invocation advanced"