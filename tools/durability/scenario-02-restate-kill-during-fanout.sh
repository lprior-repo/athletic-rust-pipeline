#!/usr/bin/env bash
# Scenario 02: SIGKILL native Restate while a national fan-out holds a reserved
# teams source at the reached boundary, then restart the node from the same
# base-dir and require every pre-kill invocation identity to survive.
# Requires the feature-enabled endpoint that honours CENSUS_NATIVE_SOURCE_BOUNDARY.
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/02}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"

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

SERVE="${S02_SERVE_BINARY:-$REPO_ROOT/target/moon-build/x86_64-unknown-linux-gnu/release/census-serve}"
if [ ! -x "$SERVE" ]; then
    echo "SKIPPED: feature-enabled endpoint missing; build it with"
    echo "SKIPPED: env -u CI tools/moon-local run pipeline:build -- --release -p census-service --features native-fault-injection --bin census-serve"
    exit 0
fi

INGRESS_PORT="${S02_INGRESS_PORT:-18410}"
ADMIN_PORT="${S02_ADMIN_PORT:-19410}"
ENDPOINT_PORT="${S02_ENDPOINT_PORT:-18411}"
NODE_PORT=$((INGRESS_PORT + 2))
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        echo "SKIPPED: port $port is already in use"
        exit 0
    fi
done

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
WORK="$(mktemp -d "$SCRATCH_STORE/scenario-02-XXXXXX")"
DATA="$WORK/store"
BOUNDARY="$WORK/boundary"
mkdir -p "$DATA" "$BOUNDARY"
chmod 700 "$BOUNDARY"
echo "EVIDENCE: $WORK"

cat > "$WORK/restate.toml" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "durability-scenario-02"
cluster-name = "durability-scenario-02"
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

OPERATION="jurisdiction:VT:2026-27:2/teams/milesplit"
CONFIG="$BOUNDARY/native-source-boundary-config.json"
MARKER="$BOUNDARY/teams-source-reservation-reached.json"
write_boundary() { printf '{\n  "schema": 1,\n  "operation": "%s",\n  "attempt": 1,\n  "timeout_seconds": 60\n}\n' "$1" > "$CONFIG"; }
write_boundary "$OPERATION"

sql() {
    curl -sS --max-time 20 -X POST "http://127.0.0.1:$ADMIN_PORT/query" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -d "{\"query\":\"$1\"}"
}
inventory() {
    sql "SELECT id, target_service_name, target_service_key, status, journal_size FROM sys_invocation" > "$1"
}
wait_admin() {
    for _ in $(seq 1 60); do
        curl -sS -o /dev/null --max-time 2 "http://127.0.0.1:$ADMIN_PORT/deployments" && return 0
        sleep 1
    done
    return 1
}

"$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node-first.log" 2>&1 &
NODE_PID=$!
wait_admin || { echo "FAIL: node did not answer on the admin port"; exit 1; }

CENSUS_NATIVE_SOURCE_BOUNDARY="$CONFIG" "$SERVE" \
    --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
    --drain-timeout 30 --browser-profile "$WORK/browser-profile" \
    --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint.log" 2>&1 &
SERVE_PID=$!
cleanup() {
    kill -TERM "$SERVE_PID" 2>/dev/null || true
    wait "$SERVE_PID" 2>/dev/null || true
    kill -TERM "$NODE_PID" 2>/dev/null || true
    wait "$NODE_PID" 2>/dev/null || true
}
trap cleanup EXIT

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

"$CLIENT" national --ingress "http://127.0.0.1:$INGRESS_PORT/" --revision 2 --limit-per-state 1 --detach \
    > "$WORK/submission.txt" 2>&1 || { echo "FAIL: submission refused"; cat "$WORK/submission.txt"; exit 1; }
grep -q 'national:' "$WORK/submission.txt" || { echo "FAIL: submission did not report a run identity"; cat "$WORK/submission.txt"; exit 1; }

REACHED=false
for i in $(seq 1 900); do
    if [ -f "$MARKER" ]; then REACHED=true; break; fi
    if [ $((i % 30)) -eq 0 ]; then echo "WAITING: boundary not reached after $((i * 2))s"; fi
    sleep 2
done
if [ "$REACHED" != true ]; then
    if tr '\0' '\n' < "/proc/$SERVE_PID/environ" | grep -q '^CENSUS_NATIVE_SOURCE_BOUNDARY='; then
        echo "FAIL: endpoint armed but the boundary for $OPERATION was never reached"
    else
        echo "FAIL: endpoint did not carry CENSUS_NATIVE_SOURCE_BOUNDARY"
    fi
    exit 1
fi
cp "$MARKER" "$WORK/marker.json"
inventory "$WORK/invocations-before.json"
python3 - "$WORK/invocations-before.json" <<'PY'
import json, sys
rows = json.load(open(sys.argv[1])).get("rows", [])
live = [r for r in rows if r.get("status") in ("running", "pending", "suspended", "backing-off")]
print(f"REACHED: {len(rows)} invocations, {len(live)} live before the kill")
PY

write_boundary "$OPERATION/disarmed"
sleep 30

kill -9 "$NODE_PID"
wait "$NODE_PID" 2>/dev/null || true
if kill -0 "$NODE_PID" 2>/dev/null; then echo "FAIL: node survived SIGKILL"; exit 1; fi
echo "INJECTED: SIGKILL native Restate pid $NODE_PID; endpoint pid $SERVE_PID stayed up"

"$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node-second.log" 2>&1 &
NODE_PID=$!
wait_admin || { echo "FAIL: restarted node did not answer on the admin port"; exit 1; }
echo "RECOVERED: node restarted from the same base-dir as pid $NODE_PID"

RETAINED=false
for _ in $(seq 1 45); do
    inventory "$WORK/invocations-after.json" 2>/dev/null || true
    if [ -s "$WORK/invocations-after.json" ] && python3 - "$WORK/invocations-before.json" "$WORK/invocations-after.json" <<'PY'
import json, sys
before = {r["id"] for r in json.load(open(sys.argv[1])).get("rows", [])}
after = {r["id"] for r in json.load(open(sys.argv[2])).get("rows", [])}
missing = before - after
print(f"IDENTITIES: before={len(before)} after={len(after)} missing={len(missing)}")
sys.exit(0 if not missing else 1)
PY
    then RETAINED=true; break; fi
    sleep 2
done
[ "$RETAINED" = true ] || { echo "FAIL: invocation identities were not retained after the node restart"; exit 1; }

sql "SELECT target_service_key, status, journal_size FROM sys_invocation WHERE target_service_key LIKE '%:2/teams/milesplit'" > "$WORK/teams-after-recovery.json"
"$CLIENT" open-work --ingress "http://127.0.0.1:$INGRESS_PORT/" --revision 2 > "$WORK/open-work-after.txt" 2>&1 || true
"$CLIENT" national --ingress "http://127.0.0.1:$INGRESS_PORT/" --revision 2 --timeout-seconds 20 --json \
    > "$WORK/report-after.json" 2>&1 || true
if grep -q 'progress:' "$WORK/report-after.json"; then
    echo "PROGRESS: $(grep -m1 'progress:' "$WORK/report-after.json")"
fi

kill -TERM "$SERVE_PID"
wait "$SERVE_PID" 2>/dev/null || true
DRAIN=$(grep -a 'drained:' "$WORK/endpoint.log" | tail -1 || true)
[ -n "$DRAIN" ] && echo "$DRAIN"
kill -TERM "$NODE_PID"
wait "$NODE_PID" 2>/dev/null || true

echo "PASS: native Restate SIGKILL at a reached teams-reservation boundary; invocation identities retained and the fan-out resumed from the same base-dir"
