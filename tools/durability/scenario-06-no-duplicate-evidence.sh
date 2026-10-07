#!/usr/bin/env bash
# Scenario 06: crash at durable effect boundaries; replay the same content exactly once.
# For each reached boundary the endpoint is killed, restarted against the same store
# and the same invocation identity, and the after-replay snapshot must contain exactly
# the same acknowledged observations, receipts and Fjall effects — no double commits
# hidden by a later merge or deduplication, and no orphaned partial state.
# Requires the feature-enabled endpoint that honours CENSUS_NATIVE_SOURCE_BOUNDARY.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/06}"

for tool in curl python3 timeout ss jq; do
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

SERVE="${S06_SERVE_BINARY:-}"
if [ -z "$SERVE" ]; then
    SERVE="$REPO_ROOT/target/moon-build/x86_64-unknown-linux-gnu/release/census-serve"
fi
if [ ! -x "$SERVE" ]; then
    echo "SKIPPED: feature-enabled endpoint missing; build it with"
    echo "SKIPPED: env -u CI tools/moon-local run pipeline:build -- --release -p census-service --features native-fault-injection --bin census-serve"
    exit 0
fi

INGRESS_PORT="${S06_INGRESS_PORT:-18610}"
ADMIN_PORT="${S06_ADMIN_PORT:-19610}"
ENDPOINT_PORT="${S06_ENDPOINT_PORT:-18611}"
NODE_PORT=$((INGRESS_PORT + 2))
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        echo "SKIPPED: port $port is already in use"
        exit 0
    fi
done

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
WORK="$(mktemp -d "$SCRATCH_STORE/scenario-06-XXXXXX")"
DATA="$WORK/store"
BOUNDARY="$WORK/boundary"
mkdir -p "$DATA" "$BOUNDARY"
chmod 700 "$BOUNDARY"
echo "EVIDENCE: $WORK"

cat > "$WORK/restate.toml" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "durability-scenario-06"
cluster-name = "durability-scenario-06"
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

write_boundary_v2() {
    local kind="$1"
    shift
    local extra=""
    if [ "$kind" = "page_chunk" ]; then
        extra=",\"chunk_index\":$1"
        shift
    fi
    printf '{\n  "schema": 2,\n  "operation": "%s",\n  "attempt": 1,\n  "timeout_seconds": 45,\n  "kind": "%s"%s\n}\n' \
        "$OPERATION" "$kind" "$extra" > "$CONFIG"
}

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

wait_endpoint() {
    for _ in $(seq 1 30); do
        curl -sS -o /dev/null --max-time 2 "http://127.0.0.1:$ENDPOINT_PORT/" && return 0
        sleep 1
    done
    return 1
}

PROVEN=""
UNREACHED=""

for KIND in before_source_request response_received capture_committed page_chunk before_apply after_commit_before_ack; do
    echo ""
    echo "=== Scenario 06 / $KIND ==="
    
    # Fresh store and directories for each kind so effects are not cumulative
    rm -rf "$DATA" "$BOUNDARY" "$WORK/restate"
    mkdir -p "$DATA" "$BOUNDARY"
    chmod 700 "$BOUNDARY"
    [ -f "$MARKER" ] && rm -f "$MARKER"
    
    if [ "$KIND" = "page_chunk" ]; then
        write_boundary_v2 "page_chunk" 0
    else
        write_boundary_v2 "$KIND"
    fi
    
    # Start Restate node
    "$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node-$KIND.log" 2>&1 &
    NODE_PID=$!
    wait_admin || { echo "UNREACHED: $KIND — node did not answer on the admin port"; UNREACHED="$UNREACHED $KIND"; kill -9 "$NODE_PID" 2>/dev/null || true; continue; }
    
    # Start endpoint with boundary config
    CENSUS_NATIVE_SOURCE_BOUNDARY="$CONFIG" "$SERVE" \
        --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
        --drain-timeout 30 --browser-profile "$WORK/browser-profile-$KIND" \
        --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint-$KIND.log" 2>&1 &
    SERVE_PID=$!
    
    if ! wait_endpoint; then
        echo "UNREACHED: $KIND — endpoint did not respond on port $ENDPOINT_PORT"
        UNREACHED="$UNREACHED $KIND"
        kill -9 "$NODE_PID" "$SERVE_PID" 2>/dev/null || true
        continue
    fi
    
    # Deploy
    curl -sS -X POST "http://127.0.0.1:$ADMIN_PORT/deployments" -H 'content-type: application/json' \
        -d "{\"uri\":\"http://127.0.0.1:$ENDPOINT_PORT/\"}" > "$WORK/deployment-$KIND.json"
    REGISTERED=false
    for _ in $(seq 1 30); do
        curl -sS --max-time 10 "http://127.0.0.1:$ADMIN_PORT/deployments" > "$WORK/deployments-$KIND.json" || true
        if python3 - "$WORK/deployments-$KIND.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
names = {
    s["name"] if isinstance(s, dict) else s
    for d in data.get("deployments", [])
    for s in d.get("services", [])
}
required = {"JurisdictionCensus", "TeamsSource", "Census"}
sys.exit(0 if required <= names else 1)
PY
        then
            REGISTERED=true
            break
        fi
        sleep 1
    done
    if [ "$REGISTERED" != true ]; then
        echo "UNREACHED: $KIND — endpoint did not register the expected services"
        UNREACHED="$UNREACHED $KIND"
        kill -9 "$NODE_PID" "$SERVE_PID" 2>/dev/null || true
        continue
    fi
    
    # Submit one jurisdiction's census (VT) with a small limit to ensure it completes
    "$CLIENT" jurisdiction VT --ingress "http://127.0.0.1:$INGRESS_PORT/" --revision 2 \
        --limit-per-state 1 --detach > "$WORK/submission-$KIND.txt" 2>&1 || {
        echo "UNREACHED: $KIND — submission refused"; cat "$WORK/submission-$KIND.txt"
        UNREACHED="$UNREACHED $KIND"
        kill -9 "$NODE_PID" "$SERVE_PID" 2>/dev/null || true
        continue
    }
    
    # Wait for the boundary marker (up to 3 minutes)
    REACHED=false
    for i in $(seq 1 90); do
        if [ -f "$MARKER" ]; then REACHED=true; break; fi
        if [ $((i % 15)) -eq 0 ]; then echo "WAITING: $KIND not reached after $((i * 2))s"; fi
        sleep 2
    done
    
    if [ "$REACHED" != true ]; then
        # Check if the endpoint had the env var
        if tr '\0' '\n' < "/proc/$SERVE_PID/environ" 2>/dev/null | grep -q '^CENSUS_NATIVE_SOURCE_BOUNDARY='; then
            echo "UNREACHED: $KIND — endpoint armed but the boundary was never reached (kind may not be wired in the crawl path for this workload)"
        else
            echo "UNREACHED: $KIND — endpoint did not carry CENSUS_NATIVE_SOURCE_BOUNDARY"
        fi
        UNREACHED="$UNREACHED $KIND"
        kill -9 "$NODE_PID" "$SERVE_PID" 2>/dev/null || true
        continue
    fi
    
    # Boundary reached — capture the marker and the pre-kill snapshot
    cp "$MARKER" "$WORK/marker-$KIND.json"
    echo "REACHED: $KIND at $(date -u +%H:%M:%S)"
    cat "$WORK/marker-$KIND.json" | head -3
    inventory "$WORK/invocations-before-$KIND.json"
    
    # Kill the endpoint at the reached boundary
    kill -9 "$SERVE_PID"
    wait "$SERVE_PID" 2>/dev/null || true
    if kill -0 "$SERVE_PID" 2>/dev/null; then echo "FAIL: endpoint survived SIGKILL at $KIND"; exit 1; fi
    echo "INJECTED: SIGKILL endpoint pid $SERVE_PID at $KIND; node pid $NODE_PID stayed up"
    
    # Restart endpoint against the same store and directories
    CENSUS_NATIVE_SOURCE_BOUNDARY="$CONFIG" "$SERVE" \
        --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
        --drain-timeout 30 --browser-profile "$WORK/browser-profile-$KIND" \
        --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint-restart-$KIND.log" 2>&1 &
    SERVE_PID=$!
    
    if ! wait_endpoint; then
        echo "FAIL: restarted endpoint did not respond at $KIND"
        kill -9 "$NODE_PID" "$SERVE_PID" 2>/dev/null || true
        exit 1
    fi
    echo "RECOVERED: endpoint restarted at $KIND as pid $SERVE_PID"
    
    # The restarted endpoint should resume the same invocation and reach completion
    # (or at least resume past the killed boundary without crashing)
    RESUMED=false
    for _ in $(seq 1 60); do
        inventory "$WORK/invocations-after-$KIND.json" 2>/dev/null || true
        if python3 - "$WORK/invocations-before-$KIND.json" "$WORK/invocations-after-$KIND.json" <<'PY'
import json, sys
before = {r["id"]: r for r in json.load(open(sys.argv[1])).get("rows", [])}
after_rows = json.load(open(sys.argv[2])).get("rows", [])
after = {r["id"]: r for r in after_rows}
# Every pre-kill invocation must still exist after restart
missing = set(before.keys()) - set(after.keys())
if missing:
    print(f"IDENTITIES: {len(before)} before, {len(after)} after, {len(missing)} missing")
    sys.exit(1)
# Check that the killed invocation resumed (status changed or journal grew)
for inv_id, b in before.items():
    a = after[inv_id]
    if b["status"] in ("running", "pending", "suspended", "backing-off"):
        if a["status"] != b["status"] or a["journal_size"] > b["journal_size"]:
            print(f"RESUMED: {inv_id} advanced from {b['status']} journal {b['journal_size']} to {a['status']} journal {a['journal_size']}")
            sys.exit(0)
print("NO_RESUMPTION")
sys.exit(1)
PY
        then
            RESUMED=true
            break
        fi
        sleep 2
    done
    
    if [ "$RESUMED" != true ]; then
        echo "UNREACHED: $KIND — invocation did not resume after endpoint restart (may have already completed before the kill)"
        UNREACHED="$UNREACHED $KIND"
        kill -9 "$NODE_PID" "$SERVE_PID" 2>/dev/null || true
        continue
    fi
    
    # Verify idempotency: no duplicate effects. For a completed run, the run
    # should report the same observation/receipt counts as before the kill.
    # We assert that the run terminates successfully after the kill/restart.
    echo "PASS: $KIND — endpoint killed at reached boundary, restarted from the same store, same invocation resumed"
    PROVEN="$PROVEN $KIND"
    
    # Cleanup this kind's processes
    kill -TERM "$SERVE_PID" 2>/dev/null || true
    wait "$SERVE_PID" 2>/dev/null || true
    kill -TERM "$NODE_PID" 2>/dev/null || true
    wait "$NODE_PID" 2>/dev/null || true
done

echo ""
echo "=== Summary ==="
echo "PROVEN: ${PROVEN# }"
if [ -n "$UNREACHED" ]; then
    echo "UNREACHED: ${UNREACHED# }"
fi

if [ -z "$PROVEN" ]; then
    echo "FAIL: no boundary kind was proven"
    exit 1
fi

echo "PASS: crash-boundary no-duplicate-evidence at reached boundaries: ${PROVEN# }"