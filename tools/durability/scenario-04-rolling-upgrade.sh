#!/usr/bin/env bash
# Scenario 04: upgrade actual V1 (native-fault-injection build) to a different V2 (plain build) with
# in-flight work, prove single-writer store handoff, the schema gate, immutable pinned replay and
# single settlement of the retained child, then retire the superseded deployment.
#
# V1 binary: the fault-injection build. Documented build (see scenario-02's own SKIP text):
#   env -u CI tools/moon-local run pipeline:build -- --release -p census-service \
#       --features native-fault-injection --bin census-serve
# Optional inputs (all real, never simulated):
#   S04_LEGACY_STORE   a store written by a different store-schema revision, for the negative side of
#                      the schema binding. Defaults to the retained legacy store from the S04
#                      mechanism investigation when present; the sub-check is excluded by name when
#                      no such store is available.
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/04}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
for tool in curl python3 ss timeout sha256sum; do
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
V2SRC="${S04_V2_BINARY:-${SERVE_BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve}}"
V1SRC="${S04_V1_BINARY:-$REPO_ROOT/target/moon-build/x86_64-unknown-linux-gnu/release/census-serve}"
[ -x "$CLIENT" ] || { echo "SKIPPED: census-service binary missing at $CLIENT"; exit 0; }
[ -x "$V2SRC" ] || { echo "SKIPPED: census-serve (plain build) missing at $V2SRC"; exit 0; }
if [ ! -x "$V1SRC" ]; then
    echo "SKIPPED: feature-enabled endpoint missing; build it with"
    echo "SKIPPED: env -u CI tools/moon-local run pipeline:build -- --release -p census-service --features native-fault-injection --bin census-serve"
    exit 0
fi

ADMIN_PORT="${ADMIN_PORT:-19405}"
V1_PORT="${SERVICE_PORT:-9405}"
V2_PORT="${S04_V2_PORT:-$((V1_PORT + 1))}"
PROBE_PORT="${S04_PROBE_PORT:-$((V1_PORT + 2))}"
INGRESS_PORT="${S04_INGRESS_PORT:-$((V1_PORT + 100))}"
NODE_PORT="${S04_NODE_PORT:-$((V1_PORT + 101))}"
for port in "$ADMIN_PORT" "$V1_PORT" "$V2_PORT" "$PROBE_PORT" "$INGRESS_PORT" "$NODE_PORT"; do
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        echo "SKIPPED: port $port is already in use"
        exit 0
    fi
done

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
WORK="$(mktemp -d "$SCRATCH_STORE/scenario-04-XXXXXX")"
DATA="$WORK/store"
BOUNDARY="$WORK/boundary"
mkdir -p "$DATA" "$BOUNDARY" "$WORK/bin"
chmod 700 "$BOUNDARY"
echo "EVIDENCE: $WORK"

OPERATION="jurisdiction:VT:2026-27:4/teams/milesplit"
KEY_ENC="$(printf '%s' "$OPERATION" | sed 's#/#%2F#g')"
CONFIG="$BOUNDARY/native-source-boundary-config.json"
MARKER="$BOUNDARY/teams-source-reservation-reached.json"

# Freeze the two build identities; a scenario that upgrades V1 to the same binary proves nothing.
V1="$WORK/bin/census-serve-v1"
V2="$WORK/bin/census-serve-v2"
cp "$V1SRC" "$V1"
cp "$V2SRC" "$V2"
sha256sum "$V1" "$V2" "$CLIENT" | tee "$WORK/artifact-identities.sha256"
V1_SHA="$(sha256sum "$V1" | cut -d' ' -f1)"
V2_SHA="$(sha256sum "$V2" | cut -d' ' -f1)"
[ "$V1_SHA" != "$V2_SHA" ] || { echo "FAIL: V1 and V2 are the same binary (sha256 $V1_SHA)"; exit 1; }
grep -qac CENSUS_NATIVE_SOURCE_BOUNDARY "$V1" >/dev/null || { echo "FAIL: V1 lacks the native-fault-injection seam"; exit 1; }

# Optional legacy store for the negative side of the schema binding.
LEGACY_STORE="${S04_LEGACY_STORE:-}"
if [ -z "$LEGACY_STORE" ] && [ -d "$REPO_ROOT/var/scratch-s04-restate/store-b" ]; then
    LEGACY_STORE="$REPO_ROOT/var/scratch-s04-restate/store-b"
fi

cat > "$WORK/restate.toml" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "durability-scenario-04"
cluster-name = "durability-scenario-04"
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

printf '{\n  "schema": 1,\n  "operation": "%s",\n  "attempt": 1,\n  "timeout_seconds": 60\n}\n' "$OPERATION" > "$CONFIG"

sql() {
    curl -sS --max-time 20 -X POST "http://127.0.0.1:$ADMIN_PORT/query" \
        -H 'content-type: application/json' -H 'accept: application/json' \
        -d "{\"query\":\"$1\"}"
}
inv() {
    sql "SELECT id, target_service_name, target_handler_name, target_service_key, status, completion_result, completion_failure, invoked_by, pinned_deployment_id, journal_size FROM sys_invocation WHERE target_service_key LIKE 'jurisdiction:VT:2026-27:4%' ORDER BY created_at LIMIT 12"
}
pid_by_port() { ss -ltnpH "sport = :$1" 2>/dev/null | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2; }
wait_admin() {
    for _ in $(seq 1 60); do
        curl -sS -o /dev/null --max-time 2 "http://127.0.0.1:$ADMIN_PORT/deployments" 2>/dev/null && return 0
        sleep 1
    done
    return 1
}
row_field() { inv | python3 -c "
import json,sys
rows=json.load(sys.stdin).get('rows',[])
m=[r for r in rows if r.get('id')=='$1']
print(m[0].get('$2') if m else '')
"; }

NODE_PID=""
V1_PID=""
V2_PID=""
cleanup() {
    for pid in "$V2_PID" "$V1_PID" "$NODE_PID"; do
        [ -n "$pid" ] && kill -TERM "$pid" 2>/dev/null || true
    done
    for pid in "$V2_PID" "$V1_PID" "$NODE_PID"; do
        [ -n "$pid" ] && wait "$pid" 2>/dev/null || true
    done
}
trap cleanup EXIT

# ---- 1. own Restate node ---------------------------------------------------
"$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node.log" 2>&1 &
NODE_PID=$!
wait_admin || { echo "FAIL: node did not answer on the admin port"; exit 1; }
echo "NODE: $("$NODE" --version 2>&1 | head -1) pid $NODE_PID admin $ADMIN_PORT ingress $INGRESS_PORT"

start_endpoint() { # $1=binary $2=port $3=log $4=with-seam
    if [ "$4" = true ]; then
        ( cd "$WORK"; exec env CENSUS_NATIVE_SOURCE_BOUNDARY="$CONFIG" "$1" --listen "127.0.0.1:$2" \
            --data-dir "$DATA" --max-concurrent 64 --drain-timeout 30 ) > "$3" 2>&1 &
    else
        ( cd "$WORK"; exec "$1" --listen "127.0.0.1:$2" \
            --data-dir "$DATA" --max-concurrent 64 --drain-timeout 30 ) > "$3" 2>&1 &
    fi
    echo $!
}
register() { # $1=port ; echoes deployment id
    curl -sS -X POST "http://127.0.0.1:$ADMIN_PORT/deployments" -H 'content-type: application/json' \
        -d "{\"uri\":\"http://127.0.0.1:$1/\"}" | tee "$2" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("id",""))'
}

# ---- 2. V1 owns a fresh store and is the newest deployment ----------------
V1_PID="$(start_endpoint "$V1" "$V1_PORT" "$WORK/v1.log" true)"
sleep 3
[ -n "$(pid_by_port "$V1_PORT")" ] || { echo "FAIL: V1 did not bind $V1_PORT"; tail -5 "$WORK/v1.log"; exit 1; }
DP_V1="$(register "$V1_PORT" "$WORK/register-v1.json")"
[ -n "$DP_V1" ] || { echo "FAIL: V1 registration returned no deployment id"; cat "$WORK/register-v1.json"; exit 1; }
echo "V1: pid $V1_PID port $V1_PORT deployment $DP_V1 sha256 ${V1_SHA:0:16}"

# ---- 3. in-flight work reaches the real reservation boundary --------------
"$CLIENT" jurisdiction --ingress "http://127.0.0.1:$INGRESS_PORT/" --revision 4 --limit-per-state 1 --detach VT \
    > "$WORK/submission.txt" 2>&1 || { echo "FAIL: submission refused"; cat "$WORK/submission.txt"; exit 1; }
grep -q 'submitted as invocation' "$WORK/submission.txt" || { echo "FAIL: submission reported no invocation identity"; cat "$WORK/submission.txt"; exit 1; }
REACHED=false
for i in $(seq 1 120); do
    if [ -f "$MARKER" ]; then REACHED=true; break; fi
    [ $((i % 30)) -eq 0 ] && echo "WAITING: reservation boundary not reached after ${i}s"
    sleep 1
done
[ "$REACHED" = true ] || { echo "FAIL: the teams reservation boundary for $OPERATION was never reached"; exit 1; }
cat "$MARKER"
inv > "$WORK/rows-pinned.json"
PARENT="$(python3 -c "
import json
rows=json.load(open('$WORK/rows-pinned.json')).get('rows',[])
print([r['id'] for r in rows if r.get('target_service_name')=='JurisdictionCensus'][0])
")"
CHILD="$(python3 -c "
import json
rows=json.load(open('$WORK/rows-pinned.json')).get('rows',[])
print([r['id'] for r in rows if r.get('target_service_name')=='TeamsSource' and r.get('target_handler_name')=='run'][0])
")"
echo "IN-FLIGHT: parent $PARENT child $CHILD"
[ "$(row_field "$CHILD" pinned_deployment_id)" = "$DP_V1" ] || { echo "FAIL: child is not pinned to $DP_V1"; exit 1; }
LEDGER_BEFORE="$(curl -sS --max-time 20 "http://127.0.0.1:$INGRESS_PORT/TeamsSource/$KEY_ENC/inspection")"
echo "LEDGER-BEFORE: $LEDGER_BEFORE"
echo "$LEDGER_BEFORE" | grep -q '"status":"unsettled"' || { echo "FAIL: pre-kill ledger is not an honest unsettled reservation"; exit 1; }

# ---- 4. pause the in-flight units, then SIGKILL the endpoint --------------
for id in "$PARENT" "$CHILD"; do
    code="$(curl -sS -o /dev/null -w '%{http_code}' --max-time 30 -X PATCH "http://127.0.0.1:$ADMIN_PORT/invocations/$id/pause")"
    [ "$code" = "202" ] || { echo "FAIL: pause of $id returned HTTP $code"; exit 1; }
    echo "PAUSED: $id HTTP:$code"
done
sleep 2
inv > "$WORK/rows-paused.json"
[ "$(row_field "$CHILD" status)" = "paused" ] || { echo "FAIL: child did not reach paused"; exit 1; }
[ "$(row_field "$PARENT" status)" = "paused" ] || { echo "FAIL: parent did not reach paused"; exit 1; }
KILL_PID="$(pid_by_port "$V1_PORT")"
kill -9 "$KILL_PID"
wait "$V1_PID" 2>/dev/null || true
echo "INJECTED: SIGKILL V1 pid $KILL_PID (deployment $DP_V1)"
sleep 3
inv > "$WORK/rows-after-kill.json"
[ "$(row_field "$CHILD" status)" = "paused" ] || { echo "FAIL: in-flight child lost at the fault"; exit 1; }
[ "$(row_field "$CHILD" pinned_deployment_id)" = "$DP_V1" ] || { echo "FAIL: child pin changed at the fault"; exit 1; }
echo "RETAINED: child $CHILD paused on $DP_V1 after the kill"

# ---- 5. the upgrade: V2 takes the SAME store, new URI ---------------------
V2_PID="$(start_endpoint "$V2" "$V2_PORT" "$WORK/v2.log" false)"
sleep 4
[ -n "$(pid_by_port "$V2_PORT")" ] || { echo "FAIL: V2 could not take the store the fault build wrote (schema mismatch)"; tail -5 "$WORK/v2.log"; exit 1; }
DP_V2="$(register "$V2_PORT" "$WORK/register-v2.json")"
[ -n "$DP_V2" ] || { echo "FAIL: V2 registration returned no deployment id"; cat "$WORK/register-v2.json"; exit 1; }
[ "$DP_V2" != "$DP_V1" ] || { echo "FAIL: the upgrade did not create a distinct deployment"; exit 1; }
echo "UPGRADED: V2 pid $V2_PID port $V2_PORT deployment $DP_V2 sha256 ${V2_SHA:0:16} (distinct from $DP_V1)"
python3 - "$WORK/register-v1.json" "$WORK/register-v2.json" <<'PY' | tee "$WORK/wire-compat.txt"
import json, sys
a, b = (json.load(open(p)) for p in sys.argv[1:3])
na = sorted(s["name"] for s in a.get("services", []))
nb = sorted(s.name if hasattr(s, "name") else s["name"] for s in b.get("services", []))
print(f"WIRE: v1 services={len(na)} v2 services={len(nb)} same set={na == nb}")
assert na == nb, "service sets differ between the two builds"
PY

# single-writer handoff: a second owner of the same store directory must be refused
set +e
timeout 8 "$V1" --listen "127.0.0.1:$PROBE_PORT" --data-dir "$DATA" --max-concurrent 4 --drain-timeout 5 \
    > "$WORK/second-owner.txt" 2>&1
set -e
grep -q 'FjallError: Locked' "$WORK/second-owner.txt" || { echo "FAIL: a second store owner was not refused with the Fjall lock"; cat "$WORK/second-owner.txt"; exit 1; }
echo "HANDOFF: second owner refused: $(grep -m1 -o 'store open failed: FjallError: Locked.*' "$WORK/second-owner.txt" | cut -c1-120)"

# schema binding, negative side: a store from a different schema revision must be refused by name
if [ -n "$LEGACY_STORE" ] && [ -d "$LEGACY_STORE" ]; then
    cp -a "$LEGACY_STORE" "$WORK/legacy-store"
    set +e
    timeout 8 "$V2" --listen "127.0.0.1:$PROBE_PORT" --data-dir "$WORK/legacy-store" --max-concurrent 4 --drain-timeout 5 \
        > "$WORK/legacy-refusal.txt" 2>&1
    set -e
    grep -q 'needs an explicit migration before opening' "$WORK/legacy-refusal.txt" \
        || { echo "FAIL: a different-revision store was not refused with the migration text"; cat "$WORK/legacy-refusal.txt"; exit 1; }
    echo "SCHEMA-BINDING: different-revision store refused: $(grep -m1 -o 'needs an explicit migration before opening[^:]*' "$WORK/legacy-refusal.txt" | cut -c1-140)"
else
    echo "SCHEMA-BINDING: EXCLUDED different-revision store refusal — no legacy store available (set S04_LEGACY_STORE); the same-revision binding is asserted above by V2 taking V1's store"
fi

# ---- 6. drain the new owner out, bring V1 back and resume the pin ---------
V2_PID_NOW="$(pid_by_port "$V2_PORT")"
kill -TERM "$V2_PID_NOW"
wait "$V2_PID" 2>/dev/null || true
sleep 2
DRAINED=false
for _ in $(seq 1 15); do [ -z "$(pid_by_port "$V2_PORT")" ] && { DRAINED=true; break; }; sleep 1; done
[ "$DRAINED" = true ] || { echo "FAIL: V2 did not drain and release the store"; exit 1; }
echo "DRAINED: V2 pid $V2_PID_NOW exited and released the store"

V1_PID="$(start_endpoint "$V1" "$V1_PORT" "$WORK/v1-restart.log" true)"
sleep 3
NEW_PID="$(pid_by_port "$V1_PORT")"
[ -n "$NEW_PID" ] || { echo "FAIL: V1 did not come back on $V1_PORT"; tail -5 "$WORK/v1-restart.log"; exit 1; }
[ "$NEW_PID" != "$KILL_PID" ] || { echo "FAIL: V1 restart reused the killed pid — recovery not distinguishable"; exit 1; }
echo "RECOVERED: V1 back at its registered URI, pid $NEW_PID"
for id in "$CHILD" "$PARENT"; do
    code="$(curl -sS -o /dev/null -w '%{http_code}' --max-time 30 -X PATCH "http://127.0.0.1:$ADMIN_PORT/invocations/$id/resume")"
    [ "$code" = "200" ] || { echo "FAIL: resume of $id returned HTTP $code"; exit 1; }
    echo "RESUMED: $id HTTP:$code"
done

SETTLED=false
for i in $(seq 1 24); do
    sleep 5
    inv > "$WORK/rows-poll-$i.json"
    s="$(row_field "$CHILD" status)"
    echo "POLL $i: child status=$s result=$(row_field "$CHILD" completion_result) journal=$(row_field "$CHILD" journal_size)"
    if [ "$s" = "completed" ]; then SETTLED=true; break; fi
done
[ "$SETTLED" = true ] || { echo "FAIL: the resumed child never completed"; exit 1; }
[ "$(row_field "$CHILD" completion_result)" = "success" ] || { echo "FAIL: the resumed child did not succeed"; exit 1; }
[ "$(row_field "$CHILD" pinned_deployment_id)" = "$DP_V1" ] || { echo "FAIL: the replay did not run on the pinned deployment"; exit 1; }
sql "SELECT id, index, entry_type FROM sys_journal WHERE id = '$CHILD' ORDER BY index" > "$WORK/journal-after.json"
python3 - "$WORK/journal-after.json" <<'PY' | tee "$WORK/journal-shape.txt"
import json, sys
rows = json.load(open(sys.argv[1])).get("rows", [])
types = [r["entry_type"] for r in rows]
want = ["Command: Input", "Command: Run", "Notification: Run", "Command: Output"]
print(f"JOURNAL: {types}")
assert types == want, f"unexpected journal shape {types}"
PY
echo "PARENT-OBSERVED: parent $PARENT status=$(row_field "$PARENT" status) result=$(row_field "$PARENT" completion_result) journal=$(row_field "$PARENT" journal_size)"

# ---- 7. ledger oracle: retire the superseded deployment first (F5) --------
code="$(curl -sS -o /dev/null -w '%{http_code}' --max-time 20 -X DELETE "http://127.0.0.1:$ADMIN_PORT/deployments/$DP_V2?force=true")"
[ "$code" = "202" ] || { echo "FAIL: retiring $DP_V2 returned HTTP $code"; exit 1; }
echo "RETIRED: deployment $DP_V2 HTTP:$code (retire-before-oracle covered: the ingress routes to the latest registration)"
LEDGER_AFTER="$(curl -sS --max-time 30 "http://127.0.0.1:$INGRESS_PORT/TeamsSource/$KEY_ENC/inspection")"
echo "LEDGER-AFTER: $LEDGER_AFTER"
echo "$LEDGER_AFTER" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert d.get('status')=='settled', f\"ledger not settled: {d}\"
prog=d['outcome'].get('progress',[])
done=[p for p in prog if p.get('status')=='completed']
assert len(done)==1, f'expected exactly one settled attempt, got {prog}'
assert prog[0].get('status')=='unknown', f'pre-kill attempt lost its honest accounting: {prog}'
o=done[0].get('outcome',{})
assert done[0].get('attempt')==2, f'settled attempt identity changed: {done[0]}'
assert isinstance(o.get('records'),int) and o['records']>0, f'settled attempt recorded no acquisition evidence: {o}'
assert o.get('errors')==[], f'settled attempt carries errors: {o}'
print('ORACLE: settled exactly once; pre-kill attempt unknown; records=' + str(o.get('records')))
" || { echo "FAIL: ledger oracle did not hold"; exit 1; }

echo "PASS: V1(${V1_SHA:0:12}, $DP_V1) -> V2(${V2_SHA:0:12}, $DP_V2) upgrade with paused in-flight parent $PARENT / child $CHILD; child retained as paused through the SIGKILL, resumed on its pinned deployment and completed ($(row_field "$CHILD" journal_size) journal entries), single-writer handoff asserted (second owner refused: Fjall lock), schema binding asserted, superseded deployment retired, ledger settled exactly once"
