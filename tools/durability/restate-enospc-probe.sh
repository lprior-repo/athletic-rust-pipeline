#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C RUST_LOG=info

: "${RESTATE_SERVER_BIN:?}" "${SERVE_BINARY:?}" "${TEST_DIR:?}" "${TMPFS_DIR:?}" "${FJALL_DIR:?}"
: "${ADMIN_PORT:?}" "${INGRESS_PORT:?}" "${NODE_PORT:?}" "${SERVE_PORT:?}"
RESTATE_LOG="$TEST_DIR/restate.log"
ENDPOINT_LOG="$TEST_DIR/endpoint.log"
CONFIG="$TEST_DIR/restate.toml"
BASELINE_KEY="disk-pressure-baseline"
SERVER_PID=""
ENDPOINT_PID=""
cleanup() {
    local rc=$?
    trap - EXIT
    for pid in "$SERVER_PID" "$ENDPOINT_PID"; do
        if [ -n "$pid" ]; then
            kill -KILL "$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
        fi
    done
    if [ "$rc" -ne 0 ]; then
        cat "$RESTATE_LOG" "$ENDPOINT_LOG" 2>/dev/null || true
    fi
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
if ! mount -t tmpfs -o size=256m tmpfs "$TMPFS_DIR"; then
    echo "SKIPPED: private bounded tmpfs mount unavailable"
    exit 0
fi
cat > "$CONFIG" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "disk-pressure"
cluster-name = "disk-pressure"
auto-provision = true
default-num-partitions = 1
default-replication = 1
base-dir = "$TMPFS_DIR/node"
listen-mode = "tcp"
bind-ip = "127.0.0.1"
bind-port = $NODE_PORT
advertised-address = "http://127.0.0.1:$NODE_PORT/"
shutdown-timeout = "10s"
disable-telemetry = true
experimental-enable-protocol-v7 = true
experimental-enable-vqueues = true
experimental-enable-scoped-virtual-objects = true
[bifrost]
default-provider = "replicated"
[worker]
durability-mode = "replica-set-only"
[admin]
bind-port = $ADMIN_PORT
advertised-address = "http://127.0.0.1:$ADMIN_PORT/"
[ingress]
bind-port = $INGRESS_PORT
EOF
start_node() {
    "$RESTATE_SERVER_BIN" --no-logo --config-file "$CONFIG" >> "$RESTATE_LOG" 2>&1 &
    SERVER_PID=$!
    for _ in $(seq 1 60); do
        if curl -fsS --max-time 2 "http://127.0.0.1:$ADMIN_PORT/deployments" >/dev/null 2>&1; then
            return 0
        fi
        kill -0 "$SERVER_PID" 2>/dev/null || return 1
        sleep 0.5
    done
    echo "FAIL: isolated Restate admin readiness deadline"
    return 1
}
query() {
    curl -fsS --max-time 10 -X POST "http://127.0.0.1:$ADMIN_PORT/query" \
        -H 'Content-Type: application/json' -H 'Accept: application/json' \
        -d "$(jq -nc --arg query "$1" '{query:$query}')"
}
run_workflow() {
    curl -fsS --max-time 30 -X POST "http://127.0.0.1:$INGRESS_PORT/Consolidate/$1/run" \
        -H 'Content-Type: application/json' -d '{"tables":[]}'
}
"$SERVE_BINARY" --listen "127.0.0.1:$SERVE_PORT" --data-dir "$FJALL_DIR" > "$ENDPOINT_LOG" 2>&1 &
ENDPOINT_PID=$!
start_node
registered=false
for _ in $(seq 1 30); do
    if curl -fsS --max-time 3 -X POST "http://127.0.0.1:$ADMIN_PORT/deployments" \
        -H 'Content-Type: application/json' \
        -d "{\"uri\":\"http://127.0.0.1:$SERVE_PORT/\"}" > "$TEST_DIR/deployment.json" 2>/dev/null; then
        registered=true
        break
    fi
    sleep 0.5
done
if [ "$registered" != true ]; then
    echo "FAIL: isolated endpoint registration deadline"
    exit 1
fi
run_workflow "$BASELINE_KEY" > "$TEST_DIR/baseline.json"
query "SELECT id,status FROM sys_invocation WHERE target_service_name='Consolidate'" > "$TEST_DIR/baseline-state.json"
jq -e '(.rows // .) as $r | ($r | length) == 1 and $r[0].status == "completed"' "$TEST_DIR/baseline-state.json"
BASELINE_ID="$(jq -er '(.rows // .)[0].id' "$TEST_DIR/baseline-state.json")"
echo "PROOF: acknowledged completed workflow $BASELINE_ID"
if dd if=/dev/zero of="$TMPFS_DIR/filler" bs=1M count=256 > "$TEST_DIR/fill.log" 2>&1; then
    echo "FAIL: bounded filler did not exhaust tmpfs"
    exit 1
fi
grep -q 'No space left on device' "$TEST_DIR/fill.log"
echo "PROOF: bounded tmpfs filler returned ENOSPC"
python3 -c 'import base64,json,os; print(json.dumps({"tables":[],"probe_padding":base64.b64encode(os.urandom(2*1024*1024)).decode()}))' > "$TEST_DIR/pressure-request.json"
for attempt in $(seq 1 128); do
    curl -sS --max-time 3 -X POST "http://127.0.0.1:$INGRESS_PORT/Consolidate/disk-pressure-$attempt/run" \
        -H 'Content-Type: application/json' --data-binary "@$TEST_DIR/pressure-request.json" \
        > "$TEST_DIR/pressure-response.json" 2> "$TEST_DIR/pressure-error.log" || true
    if grep -Eiq 'No space left on device|os error:? 28' "$RESTATE_LOG"; then
        break
    fi
done
if ! grep -Ei 'No space left on device|os error:? 28' "$RESTATE_LOG"; then
    echo "FAIL: no actual Restate storage ENOSPC observed"
    exit 1
fi
rm -- "$TMPFS_DIR/filler"
kill -KILL "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""
start_node
query "SELECT id,status FROM sys_invocation WHERE id='$BASELINE_ID'" > "$TEST_DIR/recovered-state.json"
jq -e --arg id "$BASELINE_ID" '(.rows // .) as $r | ($r | length) == 1 and $r[0].id == $id and $r[0].status == "completed"' "$TEST_DIR/recovered-state.json"
REPEAT_STATUS="$(curl -sS --max-time 10 -o "$TEST_DIR/repeated.json" -w '%{http_code}' \
    -X POST "http://127.0.0.1:$INGRESS_PORT/Consolidate/$BASELINE_KEY/run" \
    -H 'Content-Type: application/json' -d '{"tables":[]}')"
if [ "$REPEAT_STATUS" != 409 ]; then
    echo "FAIL: repeated workflow returned HTTP $REPEAT_STATUS instead of conflict"
    exit 1
fi
run_workflow disk-pressure-recovered > "$TEST_DIR/recovered.json"
jq -S . "$TEST_DIR/baseline.json" > "$TEST_DIR/baseline-normalized.json"
jq -S . "$TEST_DIR/recovered.json" > "$TEST_DIR/recovered-normalized.json"
cmp "$TEST_DIR/baseline-normalized.json" "$TEST_DIR/recovered-normalized.json"
echo "PROOF: $BASELINE_ID survived, duplicate refused, new workflow reproduced baseline output"
echo "PASS: acknowledged Restate workflow survives verified ENOSPC and restart"
