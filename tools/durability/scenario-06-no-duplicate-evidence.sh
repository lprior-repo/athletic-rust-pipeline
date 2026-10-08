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

for tool in curl python3 timeout ss jq sha256sum; do
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

SERVE="${S06_SERVE_BINARY:-${SERVE_BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve}}"
[ -x "$SERVE" ] || { echo "SKIPPED: endpoint missing at $SERVE"; exit 0; }
grep -aqF CENSUS_NATIVE_SOURCE_BOUNDARY "$SERVE" \
    || { echo "FAIL: selected endpoint lacks native source boundary seam: $SERVE"; exit 1; }
RECEIPT_ORACLE="${S06_RECEIPT_ORACLE_BINARY:-}"
if [ -z "$RECEIPT_ORACLE" ] || [ ! -x "$RECEIPT_ORACLE" ]; then
    echo "SKIPPED: exact physical receipt/readback Rust oracle missing (set S06_RECEIPT_ORACLE_BINARY)"
    exit 0
fi

INGRESS_PORT="${S06_INGRESS_PORT:-18610}"
ADMIN_PORT="${S06_ADMIN_PORT:-19610}"
ENDPOINT_PORT="${S06_ENDPOINT_PORT:-18611}"
NODE_PORT="${S06_NODE_PORT:-$((INGRESS_PORT + 2))}"
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
    case "$port" in 18095|19095|15192) echo "FAIL: shared port $port is forbidden"; exit 1 ;; esac
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
sha256sum "$CLIENT" "$SERVE" "$NODE" > "$WORK/binary-hashes.sha256"
cat "$WORK/binary-hashes.sha256"

write_node_config() {
cat > "$CASE/restate.toml" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "durability-scenario-06"
cluster-name = "durability-scenario-06"
auto-provision = true
default-num-partitions = 1
default-replication = 1
base-dir = "$RESTATE_DATA"
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
}

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
    local reply
    reply="$(curl -sS --fail-with-body --max-time 20 -X POST "http://127.0.0.1:$ADMIN_PORT/query" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -d "{\"query\":\"$1\"}")" || { printf '%s\n' "$reply" >&2; return 1; }
    printf '%s\n' "$reply" | jq -e 'has("rows") and (.rows | type == "array")' >/dev/null \
        || { printf 'FAIL: invalid admin query reply: %s\n' "$reply" >&2; return 1; }
    printf '%s\n' "$reply"
}

inventory() {
    sql "SELECT id, target_service_name, target_service_key, invoked_by_id, idempotency_key, status, journal_size, completion_result, completion_failure FROM sys_invocation" > "$1"
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

NODE_PID=""
SERVE_PID=""
stop_case() {
    local pid
    for pid in "$SERVE_PID" "$NODE_PID"; do
        [ -n "$pid" ] || continue
        kill -TERM "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    done
    SERVE_PID=""
    NODE_PID=""
}
trap stop_case EXIT
AS_OF="$(date -u +%F)"
LAST_YEAR="$(date -u +%Y)"
[ "$LAST_YEAR" -le 2027 ] || LAST_YEAR=2027
jq -n --arg date "$AS_OF" --argjson last "$LAST_YEAR" \
    '{jurisdiction:"VT",season:2026,revision:2,history:{first_calendar_year:2023,last_calendar_year:$last,as_of:$date},limit_per_state:1,concurrency:4,source_parallelism:1}' \
    > "$WORK/request.json"
KEY="jurisdiction:VT:2026-27:2"
KEY_ROUTE="$(printf '%s' "$KEY" | jq -sRr @uri)"

PROVEN=""
UNREACHED=""

for KIND in before_source_request response_received capture_committed page_chunk before_apply after_commit_before_ack; do
    echo ""
    echo "=== Scenario 06 / $KIND ==="
    
    CASE="$WORK/$KIND"
    DATA="$CASE/store"
    BOUNDARY="$CASE/boundary"
    RESTATE_DATA="$CASE/restate"
    CONFIG="$BOUNDARY/native-source-boundary-config.json"
    MARKER="$BOUNDARY/teams-source-reservation-reached.json"
    mkdir -p "$DATA" "$BOUNDARY"
    chmod 700 "$BOUNDARY"
    write_node_config
    
    if [ "$KIND" = "page_chunk" ]; then
        write_boundary_v2 "page_chunk" 0
    else
        write_boundary_v2 "$KIND"
    fi
    
    # Start Restate node
    "$NODE" --no-logo -c "$CASE/restate.toml" > "$WORK/node-$KIND.log" 2>&1 &
    NODE_PID=$!
    wait_admin || { echo "UNREACHED: $KIND — node did not answer on the admin port"; UNREACHED="$UNREACHED $KIND"; stop_case; continue; }
    
    # Start endpoint with boundary config
    CENSUS_NATIVE_SOURCE_BOUNDARY="$CONFIG" "$SERVE" \
        --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
        --drain-timeout 30 --browser-profile "$WORK/browser-profile-$KIND" \
        --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint-$KIND.log" 2>&1 &
    SERVE_PID=$!
    
    if ! wait_endpoint; then
        echo "UNREACHED: $KIND — endpoint did not respond on port $ENDPOINT_PORT"
        UNREACHED="$UNREACHED $KIND"
        stop_case
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
        stop_case
        continue
    fi
    
    curl -fsS --max-time 30 -X POST "http://127.0.0.1:$INGRESS_PORT/JurisdictionCensus/$KEY_ROUTE/run/send" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -H "idempotency-key: $KEY" --data-binary @"$WORK/request.json" \
        > "$WORK/submission-$KIND.json" 2> "$WORK/submission-$KIND.txt" \
        || { echo "FAIL: $KIND submission refused"; cat "$WORK/submission-$KIND.txt"; exit 1; }
    INVOCATION="$(jq -er 'select(.status == "Accepted") | .invocationId | select(test("^inv_[A-Za-z0-9]+$"))' "$WORK/submission-$KIND.json")"
    
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
        stop_case
        continue
    fi
    
    # Boundary reached — capture the marker and the pre-kill snapshot
    cp "$MARKER" "$WORK/marker-$KIND.json"
    jq -e --arg kind "$KIND" \
        '.phase == $kind and .attempt >= 1 and .attempt <= 3 and (.operation | type == "string" and length > 0) and (.request_digest | type == "string" and length > 0 and . != "fault_injection") and (.observed_on | type == "string" and . != "fault_injection") and (.acknowledged_effects | type == "object")' \
        "$WORK/marker-$KIND.json" >/dev/null || { echo "FAIL: $KIND marker lacks actual identity-bound checkpoint"; exit 1; }
    ACTUAL_OPERATION="$(jq -er '.operation' "$WORK/marker-$KIND.json")"
    echo "REACHED: $KIND at $(date -u +%H:%M:%S)"
    cat "$WORK/marker-$KIND.json" | head -3
    inventory "$WORK/invocations-before-$KIND.json"
    jq -e --arg id "$INVOCATION" 'any(.rows[]; .id == $id and .status != "completed")' \
        "$WORK/invocations-before-$KIND.json" >/dev/null || { echo "FAIL: $KIND original parent already settled before fault"; exit 1; }
    cp "$WORK/invocations-before-$KIND.json" "$CASE/invocations-before.json"
    cp "$WORK/submission-$KIND.json" "$CASE/submission.json"
    cp "$WORK/request.json" "$CASE/request.json"
    
    # Kill the endpoint at the reached boundary
    kill -9 "$SERVE_PID"
    KILL_EXIT=0
    wait "$SERVE_PID" || KILL_EXIT=$?
    [ "$KILL_EXIT" -eq 137 ] || { echo "FAIL: endpoint was not reaped as SIGKILL at $KIND (exit=$KILL_EXIT)"; exit 1; }
    KILLED_PID="$SERVE_PID"
    SERVE_PID=""
    echo "INJECTED: SIGKILL endpoint pid $KILLED_PID at $KIND; node pid $NODE_PID stayed up"
    
    # Restart endpoint against the same store and directories
    sha256sum -c "$WORK/binary-hashes.sha256" > "$WORK/restart-binary-hashes-$KIND.txt"
    env -u CENSUS_NATIVE_SOURCE_BOUNDARY "$SERVE" \
        --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
        --drain-timeout 30 --browser-profile "$WORK/browser-profile-$KIND" \
        --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint-restart-$KIND.log" 2>&1 &
    SERVE_PID=$!
    
    if ! wait_endpoint; then
        echo "FAIL: restarted endpoint did not respond at $KIND"
        stop_case
        exit 1
    fi
    echo "RECOVERED: endpoint restarted at $KIND as pid $SERVE_PID"
    
    curl -fsS --max-time 30 -X POST "http://127.0.0.1:$INGRESS_PORT/JurisdictionCensus/$KEY_ROUTE/run/send" \
        -H 'content-type: application/json' -H "idempotency-key: $KEY" --data-binary @"$WORK/request.json" \
        > "$WORK/resubmission-$KIND.json"
    jq -e --arg id "$INVOCATION" '.invocationId == $id and .status == "PreviouslyAccepted"' \
        "$WORK/resubmission-$KIND.json" >/dev/null || { echo "FAIL: $KIND original invocation changed on replay"; exit 1; }
    RESUMED=false
    for _ in $(seq 1 300); do
        inventory "$WORK/invocations-after-$KIND.json"
        jq -e --slurpfile before "$WORK/invocations-before-$KIND.json" \
            '([.rows[].id] as $after | all($before[0].rows[]; .id as $id | $after | index($id) != null))' \
            "$WORK/invocations-after-$KIND.json" >/dev/null || { echo "FAIL: $KIND pre-kill invocation lost"; exit 1; }
        if jq -e --slurpfile before "$WORK/invocations-before-$KIND.json" --arg id "$INVOCATION" \
            'any(.rows[]; select(.id == $id or .invoked_by_id == $id) | . as $after | any($before[0].rows[]; .id == $after.id and ($after.status != .status or $after.journal_size > .journal_size)))' \
            "$WORK/invocations-after-$KIND.json" >/dev/null; then
            RESUMED=true
            break
        fi
        sleep 2
    done
    
    if [ "$RESUMED" != true ]; then
        echo "UNREACHED: $KIND — no original root/owned-unit recovery progress was observed"
        UNREACHED="$UNREACHED $KIND"
        stop_case
        continue
    fi
    
    cp "$WORK/invocations-after-$KIND.json" "$CASE/invocations-after.json"
    kill -TERM "$SERVE_PID"
    wait "$SERVE_PID"
    SERVE_PID=""
    mkdir -p "$CASE/reconciliation"
    "$RECEIPT_ORACLE" --store "$DATA" --marker "$WORK/marker-$KIND.json" \
        --invocation "$INVOCATION" --operation "$ACTUAL_OPERATION" --kind "$KIND" \
        --evidence "$CASE/reconciliation" > "$CASE/receipt-oracle.log" 2>&1 \
        || { echo "FAIL: $KIND exact physical conservation/replay oracle rejected"; cat "$CASE/receipt-oracle.log"; exit 1; }
    cat "$CASE/receipt-oracle.log"
    echo "PASS: $KIND — actual boundary/root identity and exact physical receipt/readback oracle verified"
    PROVEN="$PROVEN $KIND"
    stop_case
done

echo ""
echo "=== Summary ==="
echo "PROVEN: ${PROVEN# }"
if [ -n "$UNREACHED" ]; then
    echo "UNREACHED: ${UNREACHED# }"
fi

if [ -n "$UNREACHED" ] || [ -z "$PROVEN" ]; then
    echo "FAIL: every named boundary must be reached and reconciled"
    exit 1
fi

echo "PASS: crash-boundary no-duplicate-evidence at reached boundaries: ${PROVEN# }"