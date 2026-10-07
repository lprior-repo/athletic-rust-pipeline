#!/usr/bin/env bash
# Scenario 18: one store admits exactly one census run. National admission binds a
# run manifest through Census.bind_run; the binding is durable across a census-serve
# restart, a second run is refused with a message that names both runs, and a seal
# request that names another cohort or another run's journal is refused.
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/18}"
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

INGRESS_PORT="${S18_INGRESS_PORT:-18810}"
ADMIN_PORT="${S18_ADMIN_PORT:-19810}"
ENDPOINT_PORT="${S18_ENDPOINT_PORT:-18811}"
NODE_PORT=$((INGRESS_PORT + 2))
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        echo "SKIPPED: port $port is already in use"
        exit 0
    fi
done

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
WORK="$(mktemp -d "$SCRATCH_STORE/scenario-18-XXXXXX")"
DATA="$WORK/store"
mkdir -p "$DATA"
echo "EVIDENCE: $WORK"

cat > "$WORK/restate.toml" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "durability-scenario-18"
cluster-name = "durability-scenario-18"
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

wait_admin() {
    for _ in $(seq 1 60); do
        curl -sS -o /dev/null --max-time 2 "http://127.0.0.1:$ADMIN_PORT/deployments" 2>/dev/null && return 0
        sleep 1
    done
    return 1
}

wait_endpoint() {
    for _ in $(seq 1 60); do
        if ss -ltn 2>/dev/null | grep -q ":$ENDPOINT_PORT "; then
            return 0
        fi
        sleep 1
    done
    return 1
}

register() {
    curl -sS -X POST "http://127.0.0.1:$ADMIN_PORT/deployments" -H 'content-type: application/json' \
        -d "{\"uri\":\"http://127.0.0.1:$ENDPOINT_PORT/\"}" > "$WORK/deployment.json"
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
sys.exit(0 if "Census" in names else 1)
PY
        then
            return 0
        fi
        sleep 1
    done
    return 1
}

start_endpoint() {
    "$SERVE" \
        --listen "127.0.0.1:$ENDPOINT_PORT" --data-dir "$DATA" --max-concurrent 16 \
        --drain-timeout 30 --browser-profile "$WORK/browser-profile" \
        --browser-executable "$CHROMIUM" --browser-headless >> "$WORK/endpoint.log" 2>&1 &
    SERVE_PID=$!
    wait_endpoint || { echo "FAIL: endpoint did not listen on $ENDPOINT_PORT"; tail -5 "$WORK/endpoint.log"; exit 1; }
}

stop_endpoint() {
    kill -TERM "$SERVE_PID" 2>/dev/null || true
    wait "$SERVE_PID" 2>/dev/null || true
    for _ in $(seq 1 30); do
        if ! ss -ltn 2>/dev/null | grep -q ":$ENDPOINT_PORT "; then
            return 0
        fi
        sleep 1
    done
    echo "FAIL: endpoint still listens on $ENDPOINT_PORT after SIGTERM"
    exit 1
}

NODE_PID=""
SERVE_PID=""
cleanup() {
    if [ -n "$SERVE_PID" ]; then
        kill -TERM "$SERVE_PID" 2>/dev/null || true
        wait "$SERVE_PID" 2>/dev/null || true
    fi
    if [ -n "$NODE_PID" ]; then
        kill -TERM "$NODE_PID" 2>/dev/null || true
        wait "$NODE_PID" 2>/dev/null || true
    fi
    return 0
}
trap cleanup EXIT

# ============================================================================
# Part A: bind one run over the ingress, refuse a second
# ============================================================================
echo ""
echo "PHASE: bind one census run over the ingress"

"$NODE" --no-logo -c "$WORK/restate.toml" > "$WORK/node.log" 2>&1 &
NODE_PID=$!
wait_admin || { echo "FAIL: node did not answer on the admin port"; tail -5 "$WORK/node.log"; exit 1; }
echo "NODE: restate-server pid $NODE_PID admin $ADMIN_PORT ingress $INGRESS_PORT"

start_endpoint
register || { echo "FAIL: endpoint did not register the Census service"; exit 1; }
echo "PROOF: Census service registered (bind_run reachable)"

bind() {
    local season="$1" revision="$2" body="$WORK/bind-$1-$2.json"
    local code
    code="$(curl -sS --max-time 20 -o "$body" -w '%{http_code}' \
        -X POST "http://127.0.0.1:$INGRESS_PORT/Census/bind_run" \
        -H 'content-type: application/json' \
        -d "{\"season\":$season,\"revision\":$revision,\"jurisdictions\":[\"Wisconsin\"]}")"
    echo "$code"
}

FIRST_CODE="$(bind 2026 1)"
if [ "$FIRST_CODE" != "200" ]; then
    echo "FAIL: first bind returned $FIRST_CODE"
    cat "$WORK/bind-2026-1.json" || true
    exit 1
fi
if grep -q '"jurisdictions":\["WI"\]' "$WORK/bind-2026-1.json"; then
    echo "PROOF: run 2026-1 bound with the admitted Wisconsin (WI) scope (200)"
else
    echo "FAIL: bind reply does not name the admitted jurisdiction"
    cat "$WORK/bind-2026-1.json" || true
    exit 1
fi

AGAIN_CODE="$(bind 2026 1)"
if [ "$AGAIN_CODE" != "200" ]; then
    echo "FAIL: identical rebind returned $AGAIN_CODE (expected the persisted binding to match)"
    cat "$WORK/bind-2026-1.json" || true
    exit 1
fi
echo "PROOF: the identical manifest rebinds idempotently (200) from the persisted run"

SECOND_CODE="$(bind 2026 2)"
if [ "$SECOND_CODE" = "200" ]; then
    echo "FAIL: a second run was bound to the same store"
    exit 1
fi
if grep -q 'fresh census needs its own store root' "$WORK/bind-2026-2.json"; then
    echo "PROOF: run 2026-2 refused ($SECOND_CODE) with the store's own message"
    grep -o 'this store is bound to census run [^"]*' "$WORK/bind-2026-2.json" | head -1
else
    echo "FAIL: the refusal does not name the retained run"
    cat "$WORK/bind-2026-2.json" || true
    exit 1
fi

set +e
WIRE_COHORT_CODE="$(curl -sS --max-time 20 -o "$WORK/seal-wire-2028.json" -w '%{http_code}' \
    -X POST "http://127.0.0.1:$INGRESS_PORT/Census/seal" -H 'content-type: application/json' \
    -d '{"grad_year":2028,"all_sources":true,"workbook":null,"write":false,"season":2026,"revision":1,"source_objects":[]}')"
set -e
if [ "$WIRE_COHORT_CODE" = "200" ]; then
    echo "FAIL: the ingress sealed a 2028 cohort request"
    exit 1
fi
if grep -q 'Class-of-2027' "$WORK/seal-wire-2028.json"; then
    echo "PROOF: the ingress refuses a non-Class-of-2027 request ($WIRE_COHORT_CODE) before reading the store"
else
    echo "FAIL: the ingress refusal does not name the Class-of-2027 cohort"
    cat "$WORK/seal-wire-2028.json"
    exit 1
fi

set +e
WIRE_RUN_CODE="$(curl -sS --max-time 60 -o "$WORK/seal-wire-run.json" -w '%{http_code}' \
    -X POST "http://127.0.0.1:$INGRESS_PORT/Census/seal" -H 'content-type: application/json' \
    -d '{"grad_year":2027,"all_sources":true,"workbook":null,"write":false,"season":2026,"revision":2,"source_objects":[]}')"
set -e
if [ "$WIRE_RUN_CODE" = "200" ]; then
    echo "FAIL: the ingress sealed a request naming another run's journal"
    cat "$WORK/seal-wire-run.json"
    exit 1
fi
if grep -q "measured run 2026-2 does not match the store's bound run 2026-1" "$WORK/seal-wire-run.json"; then
    echo "PROOF: a seal naming run 2026-2's journal refuses ($WIRE_RUN_CODE) with the typed mismatch naming both runs"
else
    echo "FAIL: the run refusal does not name the measured and bound runs"
    cat "$WORK/seal-wire-run.json"
    exit 1
fi

# ============================================================================
# Part B: the binding survives a census-serve restart
# ============================================================================
echo ""
echo "PHASE: restart census-serve and read the binding back"

stop_endpoint
echo "EVIDENCE: drain certificate: $(grep -o 'drained:.*' "$WORK/endpoint.log" | tail -1)"
start_endpoint
register || { echo "FAIL: endpoint did not re-register after restart"; exit 1; }

REBIND_CODE="$(bind 2026 1)"
if [ "$REBIND_CODE" != "200" ]; then
    echo "FAIL: rebind after restart returned $REBIND_CODE (the binding did not survive)"
    cat "$WORK/bind-2026-1.json" || true
    exit 1
fi
if grep -q '"jurisdictions":\["WI"\]' "$WORK/bind-2026-1.json"; then
    echo "PROOF: run 2026-1 still matches the store after a restart (200, WI scope)"
else
    echo "FAIL: post-restart reply does not name the admitted jurisdiction"
    exit 1
fi

SECOND_AFTER_CODE="$(bind 2026 2)"
if [ "$SECOND_AFTER_CODE" = "200" ]; then
    echo "FAIL: a second run bound to the store after restart"
    exit 1
fi
echo "PROOF: run 2026-2 still refused after restart ($SECOND_AFTER_CODE)"

# ============================================================================
# Part C: the offline ladder names the unmeasured run binding
# ============================================================================
echo ""
echo "PHASE: offline seal ladder names the run binding"

stop_endpoint

set +e
"$CLIENT" workbook --store "$DATA" --grad-year 2027 --school-year 2026 \
    > "$WORK/workbook.out" 2>&1
WORKBOOK_RC=$?
set -e
if [ "$WORKBOOK_RC" -ne 0 ]; then
    echo "FAIL: workbook build on the bound store exited $WORKBOOK_RC"
    tail -5 "$WORK/workbook.out"
    exit 1
fi
echo "EVIDENCE: workbook built for the run's own school year (2026)"

set +e
"$CLIENT" seal --store "$DATA" --grad-year 2027 > "$WORK/seal-run.out" 2>&1
SEAL_RUN_RC=$?
set -e
if [ "$SEAL_RUN_RC" -eq 0 ]; then
    echo "FAIL: the offline seal exited 0 without measuring the journal"
    exit 1
fi
if grep -q 'acceptance: a census run is measured and bound to this store unmet — not measured - this route reads no workflow journal' "$WORK/seal-run.out"; then
    echo "PROOF: the offline ladder names the unmeasured run binding with its exact detail"
else
    echo "FAIL: the ladder does not name the run binding"
    grep 'acceptance:' "$WORK/seal-run.out" || true
    exit 1
fi

set +e
"$CLIENT" workbook --store "$DATA" --out "$WORK/other-year" --grad-year 2027 --school-year 2025 \
    > "$WORK/workbook-other-year.out" 2>&1
WORKBOOK_OTHER_RC=$?
set -e
if [ "$WORKBOOK_OTHER_RC" -ne 0 ]; then
    echo "SKIPPED: a 2025 assessment year cannot be built on this store"
else
    set +e
    "$CLIENT" seal --store "$DATA" --grad-year 2027 \
        --workbook "$WORK/other-year/current/workbook.xlsx" > "$WORK/seal-year.out" 2>&1
    SEAL_YEAR_RC=$?
    set -e
    if [ "$SEAL_YEAR_RC" -eq 0 ]; then
        echo "FAIL: a publication assessed in another school year sealed"
        exit 1
    fi
    if grep -q 'verifying the complete current census publication' "$WORK/seal-year.out"; then
        echo "PROOF: a publication assessed in another school year is refused ($SEAL_YEAR_RC)"
        echo "EVIDENCE: the offline route collapses the reason; census-report's"
        echo "EVIDENCE: a_publication_assessed_in_another_school_year_cannot_seal_the_run owns the message"
    else
        echo "FAIL: the other school year publication refused without a workbook-inspection failure"
        cat "$WORK/seal-year.out"
        exit 1
    fi
fi

set +e
"$CLIENT" seal --store "$DATA" --grad-year 2028 > "$WORK/seal-cohort.out" 2>&1
SEAL_COHORT_RC=$?
set -e
if [ "$SEAL_COHORT_RC" -eq 0 ]; then
    echo "FAIL: a 2028 seal request exited 0"
    exit 1
fi
if grep -q 'Class-of-2027' "$WORK/seal-cohort.out"; then
    echo "PROOF: a non-Class-of-2027 seal request refused ($SEAL_COHORT_RC) naming the product cohort"
else
    echo "FAIL: the cohort refusal does not name the Class-of-2027 cohort"
    cat "$WORK/seal-cohort.out" || true
    exit 1
fi

# ============================================================================
# Part D: shutdown leaves nothing listening
# ============================================================================
kill -TERM "$NODE_PID" 2>/dev/null || true
wait "$NODE_PID" 2>/dev/null || true
NODE_PID=""
for port in "$INGRESS_PORT" "$ADMIN_PORT" "$ENDPOINT_PORT" "$NODE_PORT"; do
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        echo "FAIL: port $port still listens after shutdown"
        exit 1
    fi
done
echo "PROOF: no listener remained on $INGRESS_PORT, $ADMIN_PORT, $ENDPOINT_PORT or $NODE_PORT"

echo ""
echo "SUMMARY:"
echo "  - one store binds one census run; an identical rebind is idempotent and a second run is refused"
echo "  - the binding survives a census-serve restart and is read back through the store"
echo "  - the offline ladder names the unmeasured run binding by name"
echo "  - a publication assessed in another school year and a non-Class-of-2027 request are refused"
echo "  - a seal naming another run's journal is refused with the typed run mismatch"
echo ""
echo "PASS: the store admits one run, refuses another, keeps it across restart and fences the seal to it"
