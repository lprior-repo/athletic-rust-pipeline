#!/usr/bin/env bash
# Scenario 07: two concurrent, identical jurisdiction submissions produce ONE
# durable effect (one invocation, one physical acquisition, one settled
# outcome), not duplicated rows hidden by export deduplication.
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/07}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
for tool in curl python3 jq ss timeout sha256sum; do
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

SERVE="${S07_SERVE_BINARY:-${SERVE_BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve}}"
[ -x "$SERVE" ] || { echo "SKIPPED: census-serve binary missing at $SERVE"; exit 0; }

CHROMIUM="${CHROMIUM_EXECUTABLE:-/usr/bin/chromium}"
[ -x "$CHROMIUM" ] || { echo "SKIPPED: no executable browser at $CHROMIUM"; exit 0; }

INGRESS_PORT="${S07_INGRESS_PORT:-18520}"
ADMIN_PORT="${S07_ADMIN_PORT:-19520}"
ENDPOINT_PORT="${S07_ENDPOINT_PORT:-18521}"
NODE_PORT="${S07_NODE_PORT:-18522}"
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
    case "$port" in 18095|19095|15192) echo "FAIL: shared port $port is forbidden"; exit 1 ;; esac
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
sha256sum "$CLIENT" "$SERVE" "$NODE" > "$WORK/binary-hashes.sha256"
cat "$WORK/binary-hashes.sha256"

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
KEY_ENC="$(printf '%s' "$TEAM_KEY" | jq -sRr @uri)"

sql() {
    local reply
    reply="$(curl -sS --fail-with-body --max-time 20 -X POST "http://127.0.0.1:$ADMIN_PORT/query" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -d "{\"query\":\"$1\"}")" || { printf '%s\n' "$reply" >&2; return 1; }
    printf '%s\n' "$reply" | jq -e 'has("rows") and (.rows | type == "array")' >/dev/null \
        || { printf 'FAIL: invalid admin query reply: %s\n' "$reply" >&2; return 1; }
    printf '%s\n' "$reply"
}

wait_admin() {
    for _ in $(seq 1 60); do
        curl -sS -o /dev/null --max-time 2 "http://127.0.0.1:$ADMIN_PORT/deployments" 2>/dev/null && return 0
        sleep 1
    done
    return 1
}

NODE_PID=""
SERVE_PID=""
SUBMIT1_PID=""
SUBMIT2_PID=""
cleanup() {
    local pid
    for pid in "$SUBMIT1_PID" "$SUBMIT2_PID" "$SERVE_PID" "$NODE_PID"; do
        [ -n "$pid" ] || continue
        kill -TERM "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    done
}
trap cleanup EXIT
"$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node.log" 2>&1 &
NODE_PID=$!
wait_admin || { echo "FAIL: node did not answer on the admin port"; exit 1; }
echo "NODE: restate-server pid $NODE_PID admin $ADMIN_PORT ingress $INGRESS_PORT"
"$SERVE" \
    --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 64 \
    --drain-timeout 30 --browser-profile "$WORK/browser-profile" \
    --browser-executable "$CHROMIUM" --browser-headless > "$WORK/endpoint.log" 2>&1 &
SERVE_PID=$!
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

AS_OF="$(date -u +%F)"
LAST_YEAR="$(date -u +%Y)"
[ "$LAST_YEAR" -le 2027 ] || LAST_YEAR=2027
jq -n --arg state "$JURISDICTION" --arg date "$AS_OF" --argjson last "$LAST_YEAR" \
    '{jurisdiction:$state,season:2026,revision:2,history:{first_calendar_year:2023,last_calendar_year:$last,as_of:$date},limit_per_state:1,concurrency:4,source_parallelism:1}' \
    > "$WORK/request.json"
KEY_ROUTE="$(printf '%s' "$KEY" | jq -sRr @uri)"
submit() {
    curl -fsS --max-time 30 -X POST "http://127.0.0.1:$INGRESS_PORT/JurisdictionCensus/$KEY_ROUTE/run/send" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -H "idempotency-key: $KEY" --data-binary @"$WORK/request.json"
}
echo "SUBMITTING: concurrent identical requests for $KEY with explicit idempotency key"
submit > "$WORK/submit-1.json" 2> "$WORK/submit-1.txt" &
SUBMIT1_PID=$!
submit > "$WORK/submit-2.json" 2> "$WORK/submit-2.txt" &
SUBMIT2_PID=$!
wait "$SUBMIT1_PID" || { echo "FAIL: first submission refused"; cat "$WORK/submit-1.txt"; exit 1; }
SUBMIT1_PID=""
wait "$SUBMIT2_PID" || { echo "FAIL: second submission refused"; cat "$WORK/submit-2.txt"; exit 1; }
SUBMIT2_PID=""
INV1="$(jq -er '.invocationId | select(type == "string") | select(test("^inv_[A-Za-z0-9]+$"))' "$WORK/submit-1.json")"
INV2="$(jq -er '.invocationId | select(type == "string") | select(test("^inv_[A-Za-z0-9]+$"))' "$WORK/submit-2.json")"
[ "$INV1" = "$INV2" ] || { echo "FAIL: different invocation ids ($INV1 vs $INV2)"; exit 1; }
jq -es '[.[].status] | sort == ["Accepted","PreviouslyAccepted"]' "$WORK/submit-1.json" "$WORK/submit-2.json" >/dev/null \
    || { echo "FAIL: expected one admitted and one deduplicated structured reply"; exit 1; }
echo "IDENTITY: both submissions admitted as $INV1 for $KEY"
SETTLED=false
for _ in $(seq 1 300); do
    sql "SELECT id, target_service_key, idempotency_key, status, completion_result, completion_failure FROM sys_invocation WHERE id = '$INV1'" \
        > "$WORK/parent-status.json"
    jq -e --arg id "$INV1" --arg key "$KEY" \
        '.rows | length == 1 and .[0].id == $id and .[0].target_service_key == $key and .[0].idempotency_key == $key' \
        "$WORK/parent-status.json" >/dev/null || { echo "FAIL: admitted invocation identity missing"; exit 1; }
    if jq -e '.rows[0].status == "completed"' "$WORK/parent-status.json" >/dev/null; then
        jq -e '.rows[0].completion_result == "success"' "$WORK/parent-status.json" >/dev/null \
            || { echo "FAIL: original invocation failed"; cat "$WORK/parent-status.json"; exit 1; }
        SETTLED=true
        break
    fi
    sleep 2
done
[ "$SETTLED" = true ] || { echo "FAIL: original invocation did not settle"; exit 1; }

# Assert: exactly one JurisdictionCensus invocation
sql "SELECT count(*) AS n FROM sys_invocation WHERE target_service_name = 'JurisdictionCensus' AND target_service_key = '$KEY'" \
    > "$WORK/juris-count.json"
JURIS_COUNT="$(jq -er '.rows[0].n' "$WORK/juris-count.json")"
echo "JurisdictionCensus invocations for $KEY: $JURIS_COUNT"
[ "$JURIS_COUNT" = "1" ] || { echo "FAIL: expected exactly 1 JurisdictionCensus invocation, found $JURIS_COUNT"; exit 1; }
echo "ASSERTION: exactly 1 JurisdictionCensus invocation — PASS"

# Assert: exactly one TeamsSource invocation
sql "SELECT count(*) AS n FROM sys_invocation WHERE target_service_name = 'TeamsSource' AND target_service_key = '$TEAM_KEY'" \
    > "$WORK/teams-count.json"
TEAMS_COUNT="$(jq -er '.rows[0].n' "$WORK/teams-count.json")"
echo "TeamsSource invocations for $TEAM_KEY: $TEAMS_COUNT"
[ "$TEAMS_COUNT" = "1" ] || { echo "FAIL: expected exactly 1 TeamsSource invocation, found $TEAMS_COUNT"; exit 1; }
echo "ASSERTION: exactly 1 TeamsSource invocation — PASS"

# Assert: TeamsSource inspection shows settled, one completed attempt, records >= 1
curl -fsS --max-time 10 -X POST "http://127.0.0.1:$INGRESS_PORT/TeamsSource/${KEY_ENC}/inspection" \
    -H 'content-type: application/json' -H 'accept: application/json' \
    > "$WORK/teams-inspection.json"
cat "$WORK/teams-inspection.json"
jq -e '.outcome.progress | length == 1' "$WORK/teams-inspection.json" >/dev/null \
    || { echo "FAIL: more than one physical acquisition reservation"; exit 1; }
jq -e '.outcome.outcome.disposition == "complete" and .outcome.outcome.errors == [] and .outcome.outcome.unfinished == [] and .outcome.outcome == .outcome.progress[0].outcome' \
    "$WORK/teams-inspection.json" >/dev/null || { echo "FAIL: source completion is unknown, partial or contradicts its physical attempt"; exit 1; }
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
wait "$SERVE_PID"
SERVE_PID=""
DRAIN=$(grep -a 'drained:' "$WORK/endpoint.log" | tail -1 || true)
[ -n "$DRAIN" ] || { echo "FAIL: endpoint log contains no 'drained:' line"; exit 1; }
echo "DRAIN: $DRAIN"
kill -TERM "$NODE_PID"
wait "$NODE_PID"
NODE_PID=""

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

echo "PASS: concurrent identical structured submissions reused one invocation and idempotency key; exactly one jurisdiction and source invocation; source settled with one completed physical reservation and positive records; endpoint drained and store observations retained"