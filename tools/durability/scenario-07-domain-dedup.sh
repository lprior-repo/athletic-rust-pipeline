#!/usr/bin/env bash
# Scenario 07: two concurrent, identical jurisdiction submissions produce ONE
# durable effect (one invocation, one physical acquisition, one settled
# outcome), not duplicated rows hidden by export deduplication.
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/07}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
for tool in curl python3 ss timeout; do
    command -v "$tool" >/dev/null 2>&1 || { echo "SKIPPED: missing prerequisite tool $tool"; exit 0; }
done

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

SERVE="${SERVE_BINARY:-$(dirname "$CLIENT")/census-serve}"
[ -x "$SERVE" ] || { echo "SKIPPED: census-serve binary missing at $SERVE"; exit 0; }

CHROMIUM="${CHROMIUM_EXECUTABLE:-/usr/bin/chromium}"
[ -x "$CHROMIUM" ] || { echo "SKIPPED: no executable browser at $CHROMIUM"; exit 0; }

INGRESS_PORT="${SERVICE_PORT:-9107}"
ADMIN_PORT="${ADMIN_PORT:-19107}"
ENDPOINT_PORT=9108
NODE_PORT=9109
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        echo "SKIPPED: port $port is already in use"
        exit 0
    fi
done

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
WORK="$(mktemp -d "$SCRATCH_STORE/scenario-07-XXXXXX")"
DATA="$WORK/store"
mkdir -p "$DATA"
echo "EVIDENCE: $WORK"

cat > "$WORK/restate.toml" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "durability-scenario-07"
cluster-name = "durability-scenario-07"
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

STATE="${S07_STATE:-}"
if [ -z "$STATE" ]; then
    STATE="VT"
fi
JURISDICTION="$STATE"
SEASON=2026
REVISION=2
KEY="jurisdiction:${JURISDICTION}:2026-27:${REVISION}"
TEAM_KEY="jurisdiction:${JURISDICTION}:2026-27:${REVISION}/teams/milesplit"
KEY_ENC="$(printf '%s' "$TEAM_KEY" | sed 's#/#%2F#g')"

sql() {
    curl -sS --max-time 20 -X POST "http://127.0.0.1:$ADMIN_PORT/query" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -d "{\"query\":\"$1\"}"
}

wait_admin() {
    for _ in $(seq 1 60); do
        curl -sS -o /dev/null --max-time 2 "http://127.0.0.1:$ADMIN_PORT/deployments" 2>/dev/null && return 0
        sleep 1
    done
    return 1
}

# Start Restate
"$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node.log" 2>&1 &
NODE_PID=$!
wait_admin || { echo "FAIL: node did not answer on the admin port"; exit 1; }
echo "NODE: restate-server pid $NODE_PID admin $ADMIN_PORT ingress $INGRESS_PORT"

# Start endpoint
"$SERVE" \
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

# Register deployment
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
[ "$REGISTERED" = true ] || { echo "FAIL: endpoint did not register the expected services"; exit 1; }

echo "SUBMITTING: first jurisdiction census for $JURISDICTION 2026-27 revision $REVISION"
"$CLIENT" jurisdiction "$JURISDICTION" --ingress "http://127.0.0.1:$INGRESS_PORT/" \
    --season "$SEASON" --revision "$REVISION" --limit-per-state 1 --timeout-seconds 600 --json \
    > "$WORK/submit-1.json" 2> "$WORK/submit-1.txt" &
SUBMIT1_PID=$!

# Wait for first submission to report its invocation
for _ in $(seq 1 60); do
    if grep -q 'submitted as invocation' "$WORK/submit-1.txt" 2>/dev/null; then break; fi
    sleep 1
done
if ! grep -q 'submitted as invocation' "$WORK/submit-1.txt"; then
    echo "FAIL: first submission did not report an invocation identity"
    cat "$WORK/submit-1.txt" 2>/dev/null || true
    exit 1
fi
echo "FIRST SUBMISSION: $(grep -m1 'submitted as invocation' "$WORK/submit-1.txt")"

echo "SUBMITTING: second (identical) jurisdiction census — should be deduplicated by Restate"
"$CLIENT" jurisdiction "$JURISDICTION" --ingress "http://127.0.0.1:$INGRESS_PORT/" \
    --season "$SEASON" --revision "$REVISION" --limit-per-state 1 --timeout-seconds 600 --json \
    > "$WORK/submit-2.json" 2> "$WORK/submit-2.txt" &
SUBMIT2_PID=$!

# Both submissions must exit 0 (the run's handler returned its report)
wait "$SUBMIT1_PID" || { echo "FAIL: first submission exited non-zero"; cat "$WORK/submit-1.txt"; exit 1; }
wait "$SUBMIT2_PID" || { echo "FAIL: second submission exited non-zero"; cat "$WORK/submit-2.txt"; exit 1; }
echo "BOTH SUBMISSIONS COMPLETED: rc 0 for each"

# Assert: same invocation id in both
INV1=$(grep -m1 -o 'invocation [^ ]*' "$WORK/submit-1.txt" | awk '{print $2}')
INV2=$(grep -m1 -o 'invocation [^ ]*' "$WORK/submit-2.txt" | awk '{print $2}')
echo "INVOCATION 1: $INV1"
echo "INVOCATION 2: $INV2"
[ "$INV1" != "" ] || { echo "FAIL: could not extract invocation id from first submission"; exit 1; }
[ "$INV1" = "$INV2" ] || { echo "FAIL: different invocation ids — dedup not observed ($INV1 vs $INV2)"; exit 1; }
echo "ASSERTION: identical invocation ids across both submissions — PASS"

# Assert: second submission contains the dedup note
if grep -q 'already had a run — restate deduplicated this submission' "$WORK/submit-2.txt"; then
    echo "ASSERTION: second submission contains the dedup note — PASS"
else
    echo "FAIL: second submission did not contain the dedup note"
    cat "$WORK/submit-2.txt"
    exit 1
fi

# Assert: exactly one JurisdictionCensus invocation
sql "SELECT count(*) FROM sys_invocation WHERE target_service_name = 'JurisdictionCensus' AND target_service_key = '$KEY'" \
    > "$WORK/juris-count.json"
JURIS_COUNT=$(python3 -c "import json,sys; print(json.load(sys.stdin).get('rows',[[0]])[0][0])" < "$WORK/juris-count.json")
echo "JurisdictionCensus invocations for $KEY: $JURIS_COUNT"
[ "$JURIS_COUNT" = "1" ] || { echo "FAIL: expected exactly 1 JurisdictionCensus invocation, found $JURIS_COUNT"; exit 1; }
echo "ASSERTION: exactly 1 JurisdictionCensus invocation — PASS"

# Assert: exactly one TeamsSource invocation
sql "SELECT count(*) FROM sys_invocation WHERE target_service_name = 'TeamsSource' AND target_service_key = '$TEAM_KEY'" \
    > "$WORK/teams-count.json"
TEAMS_COUNT=$(python3 -c "import json,sys; print(json.load(sys.stdin).get('rows',[[0]])[0][0])" < "$WORK/teams-count.json")
echo "TeamsSource invocations for $TEAM_KEY: $TEAMS_COUNT"
[ "$TEAMS_COUNT" = "1" ] || { echo "FAIL: expected exactly 1 TeamsSource invocation, found $TEAMS_COUNT"; exit 1; }
echo "ASSERTION: exactly 1 TeamsSource invocation — PASS"

# Assert: TeamsSource inspection shows settled, one completed attempt, records >= 1
curl -sS --max-time 10 -X POST "http://127.0.0.1:$INGRESS_PORT/TeamsSource/${KEY_ENC}/inspection" \
    -H 'content-type: application/json' -H 'accept: application/json' \
    > "$WORK/teams-inspection.json" 2>&1 || true
echo "TEAMS SOURCE INSPECTION: $(cat "$WORK/teams-inspection.json" | head -c 400)"
python3 - "$WORK/teams-inspection.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d.get('status') == 'settled', f"not settled: {d.get('status')}"
outcome = d.get('outcome', {})
assert outcome.get('status') == 'completed', f"outcome not completed: {outcome.get('status')}"
progress = outcome.get('progress', [])
completed = [p for p in progress if p.get('status') == 'completed']
assert len(completed) == 1, f"expected exactly 1 completed attempt, got {len(completed)}: {progress}"
inner = completed[0].get('outcome', {})
records = inner.get('records', 0)
assert isinstance(records, int) and records >= 1, f"records not a positive int: {records}"
print(f"ASSERTION: inspection settled, 1 completed attempt, {records} record(s) — PASS")
PY
if [ $? -ne 0 ]; then
    echo "FAIL: TeamsSource inspection assertions failed"
    cat "$WORK/teams-inspection.json"
    exit 1
fi

# Drain and shutdown
kill -TERM "$SERVE_PID"
wait "$SERVE_PID" 2>/dev/null || true
DRAIN=$(grep -a 'drained:' "$WORK/endpoint.log" | tail -1 || true)
[ -n "$DRAIN" ] || { echo "FAIL: endpoint log contains no 'drained:' line"; exit 1; }
echo "DRAIN: $DRAIN"
kill -TERM "$NODE_PID"
wait "$NODE_PID" 2>/dev/null || true

# After drain: fjall-stats
"$CLIENT" --store "$DATA" fjall-stats > "$WORK/fjall-stats.txt" 2>&1
if [ $? -ne 0 ]; then
    echo "FAIL: fjall-stats exited non-zero"
    cat "$WORK/fjall-stats.txt"
    exit 1
fi
echo "FJALL STATS:"
cat "$WORK/fjall-stats.txt"

# Assert observations > 0
OBS=$(grep '^observations' "$WORK/fjall-stats.txt" | awk '{print $2}')
echo "observations: $OBS"
[ "$OBS" != "" ] || { echo "FAIL: no observations line in fjall-stats output"; exit 1; }
[ "$OBS" -gt 0 ] || { echo "FAIL: observations not positive ($OBS)"; exit 1; }
echo "ASSERTION: observations > 0 — PASS"

# Assert at least one table with positive count
TABLES_WITH_ROWS=$(awk '$1 != "store" && $1 != "evidence_generation" && $1 != "derived_generation" && $1 != "observations" && $1 != "bytes_on_disk" && $1 != "store_bytes" && $2 > 0' "$WORK/fjall-stats.txt" | wc -l)
echo "tables with rows: $TABLES_WITH_ROWS"
[ "$TABLES_WITH_ROWS" -gt 0 ] || { echo "FAIL: no table has a positive row count"; exit 1; }
echo "ASSERTION: at least one table has positive row count — PASS"

echo "PASS: two concurrent identical jurisdiction submissions produced one durable effect; invocation ids identical; restated dedup note present; exactly one JurisdictionCensus and one TeamsSource invocation; TeamsSource inspection shows one settled completed attempt with records; drain observed; store has observations and populated tables"