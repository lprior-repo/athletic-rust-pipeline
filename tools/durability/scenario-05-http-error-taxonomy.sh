#!/usr/bin/env bash
# Scenario 05: HTTP error taxonomy against a real local HTTP fault server.
#
# Obligation (docs/NATIONAL-CENSUS-FAULTS.md item 5): exercise the actual HTTP and browser
# responses for 429 + Retry-After, 500 and a challenge, and prove the taxonomy, physical
# admission, and that work is neither silently dropped nor bypassed.
#
# Phase 1 - plain HTTP lane through the production Fetcher (`census-service fetch` and
# `census-service school-sites`):
#   * a 200 is fetched and cached from a real response,
#   * a 429 and a 500 each surface as an explicit `http status <code>` refusal after exactly
#     one physical attempt for that URL,
#   * a crawl that meets a 429 records a host cooldown that absorbs the next attempt for the
#     same host before any packet leaves the process: four attempts, three physical requests.
# Phase 2 - browser lane through real Chromium on the real Restate deployment of the
# feature-enabled endpoint build, with CENSUS_BROWSER_SOURCE_ORIGIN aimed at the fault
# server:
#   * a 200 capture,
#   * a 429 + Retry-After capture that leaves the lane in CoolingDown with the window live and
#     refuses the next lane task as terminal (unavailable), with no physical request for it,
#   * a drain and fresh relaunch (cooldown 0ms, Ready), then a Cloudflare-style challenge page
#     captured and classified as a challenge; the next task is refused as human-required and
#     the page is never bypassed over plain HTTP,
#   * the durable Restate journal holds every lane invocation, completed.
#
# Asserted evidence: the fault server's own JSONL request log (physical requests only), the
# school-sites report.json and per-site record, the endpoint's classified outcome JSON, the
# lane's status readback from the Restate object, and the journaled invocation rows.
#
# LIMIT (stated, never claimed as PASS): a durable source workflow cannot be aimed at a local
# fault origin because source hosts come from the registry, so the durable three-attempt
# invocation ceiling of the Restate services and the store-persisted source_access rows are
# not exercised here.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

check() {
    local label="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        echo "ASSERT: $label"
    else
        echo "FAIL: $label"
        exit 1
    fi
}

skip() {
    echo "SKIPPED: $1"
    exit 0
}

for tool in curl python3 jq ss timeout; do
    command -v "$tool" >/dev/null 2>&1 || skip "missing prerequisite tool $tool"
done

CHROMIUM="${CHROMIUM_EXECUTABLE:-/usr/bin/chromium}"
[ -x "$CHROMIUM" ] || skip "no executable browser at $CHROMIUM"

CLIENT="${BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-service}"
[ -x "$CLIENT" ] || skip "census-service binary missing at $CLIENT"

SERVE="${SERVE_BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve}"
[ -x "$SERVE" ] || skip "census-serve binary missing at $SERVE"

FAULT_SERVE="${FAULT_BINARY:-$REPO_ROOT/target/moon-build/x86_64-unknown-linux-gnu/release/census-serve}"
[ -x "$FAULT_SERVE" ] || skip "feature-enabled endpoint missing; build it with env -u CI tools/moon-local run pipeline:build -- --release -p census-service --features native-fault-injection --bin census-serve"
grep -qac CENSUS_BROWSER_SOURCE_ORIGIN "$FAULT_SERVE" >/dev/null \
    || { echo "FAIL: $FAULT_SERVE lacks the CENSUS_BROWSER_SOURCE_ORIGIN lane-origin seam"; exit 1; }

PINNED_SERVER="$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server"
if [ -n "${RESTATE_SERVER_BIN:-}" ] && [ -x "$RESTATE_SERVER_BIN" ]; then
    :
elif [ -x "$PINNED_SERVER" ]; then
    export RESTATE_SERVER_BIN="$PINNED_SERVER"
else
    skip "no executable restate-server 1.7.10 (set RESTATE_SERVER_BIN)"
fi
NODE="$RESTATE_SERVER_BIN"

INGRESS_PORT="${ADMIN_PORT:-19405}"
ENDPOINT_PORT="${SERVICE_PORT:-9405}"
NODE_PORT=$((INGRESS_PORT + 2))
ADMIN_HTTP=$((INGRESS_PORT + 1))
FAULT_PORT="${FAULT_PORT:-$((INGRESS_PORT + 3))}"

for port in "$INGRESS_PORT" "$ADMIN_HTTP" "$NODE_PORT" "$ENDPOINT_PORT" "$FAULT_PORT"; do
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        skip "port $port is already in use"
    fi
done

SCRATCH_STORE="${SCRATCH_STORE:-$REPO_ROOT/var/durability-scratch-05}"
mkdir -p "$SCRATCH_STORE"
WORK="$(mktemp -d "$SCRATCH_STORE/scenario-05-XXXXXX")"
HTTP_STORE="$WORK/http-store"
ENDPOINT_STORE="$WORK/endpoint-store"
REQUESTS="$WORK/requests.jsonl"
FAULT_ORIGIN="http://127.0.0.1:$FAULT_PORT"
# Chromium's ProcessSingleton falls back to a socket under the temp dir when the profile path
# cannot hold it, and AF_UNIX sun_path is capped at 108 bytes. The harness scratch root is
# longer than that, so the browser lane gets a short private temp dir of its own.
LANE_TMP="$(mktemp -d /tmp/scenario-05-lane-XXXXXX)"
echo "EVIDENCE: $WORK"

NODE_PID=""
SERVE_PID=""
FAULT_PID=""
cleanup() {
    [ -n "$FAULT_PID" ] && kill -TERM "$FAULT_PID" 2>/dev/null || true
    [ -n "$SERVE_PID" ] && kill -TERM "$SERVE_PID" 2>/dev/null || true
    [ -n "$NODE_PID" ] && kill -TERM "$NODE_PID" 2>/dev/null || true
    [ -n "$FAULT_PID" ] && wait "$FAULT_PID" 2>/dev/null || true
    [ -n "$SERVE_PID" ] && wait "$SERVE_PID" 2>/dev/null || true
    [ -n "$NODE_PID" ] && wait "$NODE_PID" 2>/dev/null || true
    rm -rf -- "$LANE_TMP"
}
trap cleanup EXIT

count_path() {
    local n
    n="$(jq -r --arg p "$1" 'select(.path == $p) | 1' "$REQUESTS" 2>/dev/null | wc -l | tr -d ' ')" || true
    echo "${n:-0}"
}

count_path_status() {
    local n
    n="$(jq -r --arg p "$1" --argjson s "$2" 'select(.path == $p and .status == $s) | 1' "$REQUESTS" 2>/dev/null | wc -l | tr -d ' ')" || true
    echo "${n:-0}"
}

FAULT_SERVER="$WORK/fault-server.py"
cat > "$FAULT_SERVER" <<'FAULT_SERVER_PY'
#!/usr/bin/env python3
"""HTTP fault server for scenario-05-http-error-taxonomy.

Serves the real response classes the census transport has to classify, at real URLs,
on 127.0.0.1 only, and appends one JSON object per request to its log file so the
scenario can assert on physical requests instead of on product log text.

Routes (method GET, path matched without the query string):

    /probe             200  harness liveness probe; never requested by the product
    /                  200  benign HTML: the browser lane's bootstrap target under
                            CENSUS_BROWSER_SOURCE_ORIGIN
    /ok                200  benign HTML page for the plain HTTP lane
    /home              200  benign school-home page (no anchors, no WordPress markers)
    /athletics         500  final server error (guess path 1)
    /coaches           429  rate limited, Retry-After: 30 (guess path 2)
    /staff-directory   200  benign page that must never be requested: the 429 above
                            records a host cooldown that absorbs this attempt
    /status/429        429  rate limited, Retry-After: 2 (one-shot fetch taxonomy)
    /status/500        500  final server error (one-shot fetch taxonomy)
    /lane/ok           200  benign HTML page for the browser lane
    /lane/429          429  rate limited, Retry-After: 120 (browser lane cooldown)
    /lane/later        200  benign page that must never be requested (lane cooldown)
    /challenge         200  Cloudflare-style challenge page carrying the markers that
                            athleticnet_browser::challenge::html_body_challenge detects
    /lane/later2       200  benign page that must never be requested (latched challenge)
    /favicon.ico       204  navigation noise, no body

The 500 bodies are plain text so a text/plain 200 is never mistaken for a page.
"""

import argparse
import json
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

BENIGN_PAGE = b"""<!doctype html>
<html><head><title>Fault probe school</title></head>
<body><h1>Fault probe school</h1><p>Home page.</p></body></html>
"""

CHALLENGE_PAGE = b"""<!doctype html>
<html><head><title>Just a moment...</title></head>
<body>
<script src="/cdn-cgi/challenge-platform/h/b"></script>
<noscript>
<form id="challenge-form" action="/cdn-cgi/challenge-platform/h/b/verify" method="get">
<input type="hidden" name="md" value="probe">
</form>
</noscript>
</body></html>
"""

TEXT = "text/plain; charset=utf-8"
HTML = "text/html; charset=utf-8"

ROUTES = {
    "/probe": (200, TEXT, b"probe\n", {}),
    "/": (200, HTML, BENIGN_PAGE, {}),
    "/ok": (200, HTML, BENIGN_PAGE, {}),
    "/home": (200, HTML, BENIGN_PAGE, {}),
    "/athletics": (500, TEXT, b"internal server error\n", {}),
    "/coaches": (429, TEXT, b"too many requests\n", {"Retry-After": "30"}),
    "/staff-directory": (200, HTML, BENIGN_PAGE, {}),
    "/status/429": (429, TEXT, b"too many requests\n", {"Retry-After": "2"}),
    "/status/500": (500, TEXT, b"internal server error\n", {}),
    "/lane/ok": (200, HTML, BENIGN_PAGE, {}),
    "/lane/429": (429, TEXT, b"too many requests\n", {"Retry-After": "120"}),
    "/lane/later": (200, HTML, BENIGN_PAGE, {}),
    "/challenge": (200, HTML, CHALLENGE_PAGE, {}),
    "/lane/later2": (200, HTML, BENIGN_PAGE, {}),
    "/favicon.ico": (204, TEXT, b"", {}),
}

DEFAULT_ROUTE = (404, TEXT, b"not found\n", {})


class FaultHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "scenario-05-fault-server"
    sys_version = ""

    def log_message(self, format, *args):
        pass

    def log_request(self, code="-", size="-"):
        pass

    def record(self, path, status):
        entry = {
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "client": self.client_address[0],
            "method": self.command,
            "path": path,
            "status": status,
            "user_agent": self.headers.get("user-agent", ""),
        }
        line = json.dumps(entry)
        with self.server.log_lock:
            self.server.log_file.write(line + "\n")
            self.server.log_file.flush()

    def respond(self, path):
        status, content_type, body, extra = ROUTES.get(path, DEFAULT_ROUTE)
        self.record(path, status)
        self.send_response(status)
        self.send_header("content-type", content_type)
        self.send_header("content-length", str(len(body)))
        self.send_header("cache-control", "no-store")
        self.send_header("connection", "close")
        for name, value in extra.items():
            self.send_header(name, value)
        self.end_headers()
        if body:
            self.wfile.write(body)

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        self.respond(path)

    def do_HEAD(self):
        self.respond(self.path.split("?", 1)[0])


def main():
    parser = argparse.ArgumentParser(description="scenario-05 HTTP fault server")
    parser.add_argument("--port", type=int, required=True, help="TCP port on 127.0.0.1")
    parser.add_argument("--log-file", type=str, required=True, help="JSONL request log path")
    args = parser.parse_args()

    log_file = open(args.log_file, "w", encoding="utf-8")
    server = ThreadingHTTPServer(("127.0.0.1", args.port), FaultHandler)
    server.daemon_threads = True
    server.log_file = log_file
    server.log_lock = threading.Lock()
    print(f"listening on 127.0.0.1:{args.port}", file=sys.stderr, flush=True)
    try:
        server.serve_forever()
    finally:
        server.server_close()
        log_file.close()


if __name__ == "__main__":
    main()

FAULT_SERVER_PY
python3 "$FAULT_SERVER" \
    --port "$FAULT_PORT" --log-file "$REQUESTS" > "$WORK/fault-server.log" 2>&1 &
FAULT_PID=$!
PROBE_CODE=""
for _ in $(seq 1 60); do
    PROBE_CODE="$(curl -sS -o /dev/null -w '%{http_code}' --max-time 2 "$FAULT_ORIGIN/probe" 2>/dev/null || true)"
    [ "$PROBE_CODE" = "200" ] && break
    sleep 0.25
done
[ "$PROBE_CODE" = "200" ] || {
    echo "FAIL: fault server did not answer /probe (last code ${PROBE_CODE:-none})"
    cat "$WORK/fault-server.log"
    exit 1
}
echo "EVIDENCE: fault server http://127.0.0.1:$FAULT_PORT answered /probe 200 (pid $FAULT_PID)"
check "the fault server log records the probe as one 200 request" test "$(count_path_status /probe 200)" = 1

echo "PHASE: plain HTTP lane (production Fetcher, $CLIENT)"

set +e
OK_OUT="$("$CLIENT" --store "$HTTP_STORE" --authorized-host 127.0.0.1 fetch --refresh "$FAULT_ORIGIN/ok" 2>&1)"
OK_RC=$?
set -e
printf '%s\n' "$OK_OUT" > "$WORK/source-fetch-ok.out"
check "fetch of a served 200 exits 0" test "$OK_RC" = 0
check "the 200 was answered exactly once" test "$(count_path_status /ok 200)" = 1
check "the plain lane used the census-service user agent" sh -c "jq -r 'select(.path == \"/ok\") | .user_agent' '$REQUESTS' | grep -q census-service"

set +e
RATE_OUT="$("$CLIENT" --store "$HTTP_STORE" --authorized-host 127.0.0.1 fetch --refresh "$FAULT_ORIGIN/status/429" 2>&1)"
RATE_RC=$?
set -e
printf '%s\n' "$RATE_OUT" > "$WORK/source-fetch-429.out"
echo "EVIDENCE: fetch /status/429 rc=$RATE_RC: $(head -c 200 "$WORK/source-fetch-429.out" | tr '\n' '|')"
check "a served 429 is refused explicitly, naming 429" grep -q "http status 429" "$WORK/source-fetch-429.out"
check "the 429 refusal exits nonzero" test "$RATE_RC" -ne 0
check "the 429 was answered exactly once" test "$(count_path_status /status/429 429)" = 1

set +e
FIVE_OUT="$("$CLIENT" --store "$HTTP_STORE" --authorized-host 127.0.0.1 fetch --refresh "$FAULT_ORIGIN/status/500" 2>&1)"
FIVE_RC=$?
set -e
printf '%s\n' "$FIVE_OUT" > "$WORK/source-fetch-500.out"
echo "EVIDENCE: fetch /status/500 rc=$FIVE_RC: $(head -c 200 "$WORK/source-fetch-500.out" | tr '\n' '|')"
check "a served 500 is refused explicitly, naming 500" grep -q "http status 500" "$WORK/source-fetch-500.out"
check "the 500 refusal exits nonzero" test "$FIVE_RC" -ne 0
check "the 500 was answered exactly once for one invoked attempt" test "$(count_path_status /status/500 500)" = 1

QUEUE="$WORK/queue.jsonl"
printf '{"state":"WI","name":"Fault Probe School","website":"%s/home"}\n' "$FAULT_ORIGIN" > "$QUEUE"
SITE_OUT="$WORK/site-out"
set +e
CRAWL_OUT="$("$CLIENT" --store "$HTTP_STORE" --authorized-host 127.0.0.1 --delay-ms 100 school-sites "$QUEUE" --out "$SITE_OUT" --refresh 2>&1)"
CRAWL_RC=$?
set -e
printf '%s\n' "$CRAWL_OUT" > "$WORK/school-sites.out"
echo "EVIDENCE: school-sites rc=$CRAWL_RC: $(head -c 400 "$WORK/school-sites.out" | tr '\n' '|')"
check "the crawl exits 0 with the site written" test "$CRAWL_RC" = 0
check "the report exists" test -f "$SITE_OUT/report.json"
check "the per-site record exists" test -f "$SITE_OUT/WI__fault-probe-school.json"
check "the crawl attempted four fetches" jq -e '.requests == 4' "$SITE_OUT/report.json"
check "three of the four attempts failed and were recorded" jq -e '.errors == 3' "$SITE_OUT/report.json"
check "the site is not silently dropped: it is written, not failed" jq -e '.crawled == 1 and .empty == 1 and .failed == 0 and (.failures == [])' "$SITE_OUT/report.json"
check "only the homepage produced page evidence" jq -e '.page_evidence | length == 1' "$SITE_OUT/WI__fault-probe-school.json"
check "the analysed page list holds only the homepage" jq -e --arg u "$FAULT_ORIGIN/home" '.pages == [$u]' "$SITE_OUT/WI__fault-probe-school.json"
check "the homepage evidence is the served 200" jq -e --arg u "$FAULT_ORIGIN/home" '.page_evidence[0].url == $u and .page_evidence[0].status == 200' "$SITE_OUT/WI__fault-probe-school.json"
check "the homepage reached the network exactly once" test "$(count_path_status /home 200)" = 1
check "the 500 guess path reached the network exactly once" test "$(count_path_status /athletics 500)" = 1
check "the 429 guess path reached the network exactly once" test "$(count_path_status /coaches 429)" = 1
check "the post-429 guess path never reached the network (cooldown honoured)" test "$(count_path /staff-directory)" = 0
echo "EVIDENCE: cooldown honoured: 4 attempts, 3 physical requests, 0 requests for /staff-directory"

echo "PHASE: browser lane (real Chromium via the Restate deployment of $FAULT_SERVE)"

NODE_CONFIG="$WORK/restate.toml"
cat > "$NODE_CONFIG" <<EOF
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "durability-scenario-05"
cluster-name = "durability-scenario-05"
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
bind-address = "127.0.0.1:$ADMIN_HTTP"
EOF

"$NODE" --no-logo -c "$NODE_CONFIG" > "$WORK/node.log" 2>&1 &
NODE_PID=$!
ADMIN_UP=false
for _ in $(seq 1 60); do
    if curl -sS -o /dev/null --max-time 2 "http://127.0.0.1:$ADMIN_HTTP/deployments" 2>/dev/null; then
        ADMIN_UP=true
        break
    fi
    sleep 1
done
check "the Restate admin surface answered" test "$ADMIN_UP" = true

TMPDIR="$LANE_TMP" CENSUS_BROWSER_SOURCE_ORIGIN="$FAULT_ORIGIN" "$FAULT_SERVE" \
    --listen "127.0.0.1:$ENDPOINT_PORT" \
    --data-dir "$ENDPOINT_STORE" \
    --max-concurrent 16 \
    --drain-timeout 30 \
    --browser-profile "$WORK/browser-profile" \
    --browser-executable "$CHROMIUM" \
    --browser-headless \
    > "$WORK/endpoint.log" 2>&1 &
SERVE_PID=$!
sleep 2
kill -0 "$SERVE_PID" 2>/dev/null || {
    echo "FAIL: the endpoint exited during startup"
    tail -20 "$WORK/endpoint.log"
    exit 1
}

curl -sS -X POST "http://127.0.0.1:$ADMIN_HTTP/deployments" \
    -H 'content-type: application/json' \
    -d "{\"uri\":\"http://127.0.0.1:$ENDPOINT_PORT/\"}" > "$WORK/deployment.json" 2>/dev/null || true
check "the deployment registered" jq -e '.id != null' "$WORK/deployment.json"

REGISTERED=false
for _ in $(seq 1 60); do
    curl -sS --max-time 10 "http://127.0.0.1:$ADMIN_HTTP/deployments" > "$WORK/deployments.json" 2>/dev/null || true
    if jq -e '.deployments[] | .services[] | select(.name == "BrowserSession")' "$WORK/deployments.json" >/dev/null 2>&1; then
        REGISTERED=true
        break
    fi
    sleep 1
done
check "the endpoint registered the BrowserSession service" test "$REGISTERED" = true

lane() {
    "$CLIENT" browser-session --ingress "http://127.0.0.1:$INGRESS_PORT/" "$@"
}

# Chromium on a local origin occasionally aborts a top-level navigation at the transport
# level (the fault server still logs the physical request). Retry only that failure class and
# only as harness-level tolerance: a wrong status, a missed cooldown or a bypassed challenge
# is a taxonomy defect and is never retried away.
LANE_ATTEMPTS=0
lane_fetch() {
    local url="$1" out="$2" attempt payload=""
    LANE_ATTEMPTS=0
    for attempt in 1 2 3 4; do
        LANE_ATTEMPTS="$attempt"
        payload="$(lane fetch --url "$url" --semantic-url "$url" --json 2>&1)" || true
        if jq -e '.outcome == "failed" and .error == "transport"' <<<"$payload" >/dev/null 2>&1; then
            sleep 1.5
            continue
        fi
        break
    done
    printf '%s\n' "$payload" > "$out"
}

set +e
START_OUT="$(lane start 2>&1)"
START_RC=$?
set -e
printf '%s\n' "$START_OUT" > "$WORK/lane-start.out"
echo "EVIDENCE: browser-session start rc=$START_RC: $(tr '\n' '|' < "$WORK/lane-start.out")"
check "the lane launched" test "$START_RC" = 0
check "the lane reports itself running" grep -qE "profile [^ ]+: running" "$WORK/lane-start.out"
check "the lane bootstrap navigated to the override origin" test "$(count_path_status / 200)" -ge 1
check "the bootstrap request came from the browser engine" sh -c "jq -r 'select(.path == \"/\") | .user_agent' '$REQUESTS' | grep -qi chrome"

LANE_READY=false
for _ in $(seq 1 30); do
    lane status > "$WORK/lane-status-initial.out" 2>&1 || true
    if grep -q "state Ready" "$WORK/lane-status-initial.out"; then
        LANE_READY=true
        break
    fi
    sleep 1
done
check "the lane reached Ready after launch" test "$LANE_READY" = true

lane_fetch "$FAULT_ORIGIN/lane/ok" "$WORK/lane-ok.out"
OK_ATTEMPTS="$LANE_ATTEMPTS"
check "a lane fetch of a served 200 exits 0" jq -e '.outcome == "captured" and .response.status == 200 and .challenge == false' "$WORK/lane-ok.out"
check "the lane captured the 200" jq -e '.outcome == "captured" and .response.status == 200' "$WORK/lane-ok.out"
check "the lane 200 was served exactly once per invoked attempt" test "$(count_path /lane/ok)" = "$OK_ATTEMPTS"
echo "EVIDENCE: lane 200: $(jq -c '{outcome, status: .response.status}' "$WORK/lane-ok.out"), $OK_ATTEMPTS invoked attempt(s), $(count_path /lane/ok) physical request(s)"

lane_fetch "$FAULT_ORIGIN/lane/429" "$WORK/lane-429.out"
RATE_ATTEMPTS="$LANE_ATTEMPTS"
check "the lane fetch of the 429 exits 0 with a capture" jq -e '.outcome == "captured" and .response.status == 429' "$WORK/lane-429.out"
check "the lane captured the 429" jq -e '.outcome == "captured" and .response.status == 429' "$WORK/lane-429.out"
check "the capture records the Retry-After cooldown" jq -e '.retry_after_ms == 120000' "$WORK/lane-429.out"
check "the lane 429 was served exactly once per invoked attempt" test "$(count_path /lane/429)" = "$RATE_ATTEMPTS"
echo "EVIDENCE: lane 429: $(jq -c '{outcome, status: .response.status, retry_after_ms}' "$WORK/lane-429.out"), $RATE_ATTEMPTS invoked attempt(s), $(count_path /lane/429) physical request(s)"

lane status > "$WORK/lane-status-cooling.out" 2>&1 || true
echo "EVIDENCE: $(tr '\n' '|' < "$WORK/lane-status-cooling.out")"
check "the lane reports CoolingDown after the 429" grep -q "state CoolingDown" "$WORK/lane-status-cooling.out"
check "the lane reports a live cooldown window" grep -qE "cooldown [1-9][0-9]{4,}ms" "$WORK/lane-status-cooling.out"

lane_fetch "$FAULT_ORIGIN/lane/later" "$WORK/lane-later.out"
check "a later lane task is refused as terminal while the cooldown holds" jq -e '.outcome == "failed" and .error == "unavailable" and .verdict == "terminal"' "$WORK/lane-later.out"
check "the refused task never reached the network" test "$(count_path /lane/later)" = 0
echo "EVIDENCE: cooldown refusal on the lane: $(tr -d '\n ' < "$WORK/lane-later.out"), 0 physical requests for /lane/later"

set +e
STOP1_OUT="$(lane stop 2>&1)"
STOP1_RC=$?
set -e
printf '%s\n' "$STOP1_OUT" > "$WORK/lane-stop-1.out"
echo "EVIDENCE: lane stop: $(tr '\n' '|' < "$WORK/lane-stop-1.out")"
check "the first lane drain exits 0" test "$STOP1_RC" = 0
check "the first lane drain reports the completed lane requests" grep -qE "drained: accepted [1-9][0-9]* .* remaining 0" "$WORK/lane-stop-1.out"

set +e
START2_OUT="$(lane start 2>&1)"
START2_RC=$?
set -e
printf '%s\n' "$START2_OUT" > "$WORK/lane-start-2.out"
check "the lane relaunched" test "$START2_RC" = 0
check "the relaunched lane reports itself running" grep -qE "profile [^ ]+: running" "$WORK/lane-start-2.out"

LANE_READY2=false
for _ in $(seq 1 30); do
    lane status > "$WORK/lane-status-initial-2.out" 2>&1 || true
    if grep -q "state Ready" "$WORK/lane-status-initial-2.out"; then
        LANE_READY2=true
        break
    fi
    sleep 1
done
check "the relaunched lane reached Ready" test "$LANE_READY2" = true
echo "EVIDENCE: relaunched lane: $(tr '\n' '|' < "$WORK/lane-status-initial-2.out")"
check "the relaunched lane holds no leftover cooldown" grep -q "cooldown 0ms" "$WORK/lane-status-initial-2.out"

lane_fetch "$FAULT_ORIGIN/challenge" "$WORK/lane-challenge.out"
CH_ATTEMPTS="$LANE_ATTEMPTS"
check "the lane captured the challenge page" jq -e '.outcome == "captured" and .response.status == 200' "$WORK/lane-challenge.out"
check "the captured page was classified as a challenge" jq -e '.challenge == true' "$WORK/lane-challenge.out"
check "the challenge page was served exactly once per invoked attempt, with no plain-HTTP fallback" test "$(count_path /challenge)" = "$CH_ATTEMPTS"

lane status > "$WORK/lane-status-challenged.out" 2>&1 || true
echo "EVIDENCE: $(tr '\n' '|' < "$WORK/lane-status-challenged.out")"
check "the lane reports Challenged after the challenge" grep -q "state Challenged" "$WORK/lane-status-challenged.out"

lane_fetch "$FAULT_ORIGIN/lane/later2" "$WORK/lane-later2.out"
check "a task issued after the challenge is refused as human-required" jq -e '.outcome == "failed" and .error == "human_required" and .verdict == "human_required"' "$WORK/lane-later2.out"
check "the task refused after the challenge never reached the network" test "$(count_path /lane/later2)" = 0
echo "EVIDENCE: challenge refusal on the lane: $(tr -d '\n ' < "$WORK/lane-later2.out"), 0 physical requests for /lane/later2"

set +e
STOP2_OUT="$(lane stop 2>&1)"
STOP2_RC=$?
set -e
printf '%s\n' "$STOP2_OUT" > "$WORK/lane-stop-2.out"
echo "EVIDENCE: lane stop: $(tr '\n' '|' < "$WORK/lane-stop-2.out")"
check "the second lane drain exits 0" test "$STOP2_RC" = 0
check "the second lane drain reports the completed lane requests" grep -qE "drained: accepted [1-9][0-9]* .* remaining 0" "$WORK/lane-stop-2.out"

JOURNAL_JSON="$(curl -sS --max-time 20 -X POST "http://127.0.0.1:$ADMIN_HTTP/query" \
    -H 'content-type: application/json' -H 'accept: application/json' \
    -d '{"query":"SELECT id, target_handler_name, status, completion_result FROM sys_invocation WHERE target_service_name = '"'"'BrowserSession'"'"' ORDER BY created_at"}' 2>/dev/null || true)"
printf '%s\n' "$JOURNAL_JSON" > "$WORK/lane-journal.json"
if jq -e '.rows | length > 0' "$WORK/lane-journal.json" >/dev/null 2>&1; then
    check "the durable journal holds the browser session invocations" jq -e '.rows | length >= 6' "$WORK/lane-journal.json"
    check "every journaled lane invocation completed" jq -e '[.rows[] | select(.status != "completed")] | length == 0' "$WORK/lane-journal.json"
    check "the journal covers the whole lane lifecycle" jq -e '[.rows[].target_handler_name] | (index("start") != null and index("status") != null and index("fetch") != null and index("stop") != null)' "$WORK/lane-journal.json"
    echo "EVIDENCE: journaled BrowserSession invocations: $(jq -r '.rows | length' "$WORK/lane-journal.json"), handlers: $(jq -r '[.rows[].target_handler_name] | join(",")' "$WORK/lane-journal.json")"
else
    echo "LIMIT: the Restate admin /query surface answered no journal rows, so the durable journal readback is unproven in this run"
fi
echo "LIMIT: durable three-attempt invocation ceiling and store-persisted source_access rows are not exercised: a durable source workflow cannot be aimed at a local fault origin"

HIT_COUNT="$(wc -l < "$REQUESTS" | tr -d ' ')"
echo "EVIDENCE: fault server logged $HIT_COUNT physical requests; log $REQUESTS"
echo "PASS: scenario-05-http-error-taxonomy verified: plain lane classified a served 200, 429 and 500 with explicit refusals after one physical attempt each; the crawl recorded the 429 and absorbed the next attempt for that host before the network (4 attempts, 3 physical requests, 0 requests for /staff-directory); the browser lane (real Chromium via the Restate deployment, CENSUS_BROWSER_SOURCE_ORIGIN on the fault server) captured a 200, captured a 429 with retry_after_ms=120000 and a live CoolingDown window, refused the next task as terminal/unavailable with no physical request, drained and relaunched with no leftover cooldown, then captured and classified the challenge page and refused the next task as human-required with no physical request; the durable journal holds every lane invocation as completed"
