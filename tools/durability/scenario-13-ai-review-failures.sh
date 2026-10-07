#!/usr/bin/env bash
# Scenario 13: the two-lane model review under modelled HTTP faults and around
# advice persistence.
#
# Runs the review CLI against a private copy of the retained dual-lane review
# store (brought to the current schema with the sanctioned `store-migrate`
# command, never opening the retained original) through two fault-injecting
# proxies that front the real approved local model servers.  Phases:
#   0  transport unit lane: census-review model::transport_tests
#   1  refusal taxonomy (dry-run, nothing persists): an injected 503, malformed
#      assistant content and a timed-out lane are every time not accepted, and
#      each lane is charged exactly one request per ask (no nested retries)
#   2  a failed second lane is retried in the next run while the first valid
#      advice is reused (lane A charged nothing); the standing advice then
#      replays from a fresh process without re-charging either lane
#   3  SIGKILL before the first advice write: nothing durable is left, and a
#      later run completes the same case
#   4  SIGKILL as soon as the advice write is observed in the store journal:
#      the row survives the kill and is replayed without re-asking
# The proxies never stop, restart or reconfigure the model servers, and every
# phase output, both proxy JSONL logs and both stats dumps stay in EVIDENCE.
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/13}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
CLIENT="${BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-service}"
SEED_STORE="${S13_SEED_STORE:-$REPO_ROOT/var/review-dual-gpu-20261005/store}"
PROXY_A_PORT="${S13_PROXY_A_PORT:-11210}"
PROXY_B_PORT="${S13_PROXY_B_PORT:-11211}"
TARGET_A="${S13_TARGET_A:-http://127.0.0.1:11000}"
TARGET_B="${S13_TARGET_B:-http://127.0.0.1:11001}"
MODEL="${S13_MODEL:-qwen3.8-27b-uncensored}"
LANE_TIMEOUT="${S13_TIMEOUT_SECS:-240}"
MAX_TOKENS="${S13_MAX_TOKENS:-8192}"
FAULT_TIMEOUT="${S13_FAULT_TIMEOUT_SECS:-15}"
STALL_SECONDS="${S13_STALL_SECONDS:-30}"
OBSERVED_ON="${S13_OBSERVED_ON:-2026-10-06}"

for tool in python3 curl ss timeout stat awk; do
    command -v "$tool" >/dev/null 2>&1 || { echo "SKIPPED: missing prerequisite tool $tool"; exit 0; }
done
[ -x "$CLIENT" ] || { echo "SKIPPED: no census-service client at $CLIENT (set BINARY)"; exit 0; }
[ -d "$SEED_STORE" ] || { echo "SKIPPED: no dual-lane review seed store at $SEED_STORE (set S13_SEED_STORE)"; exit 0; }
for port in "$PROXY_A_PORT" "$PROXY_B_PORT"; do
    if ss -ltn 2>/dev/null | grep -q ":$port "; then
        echo "SKIPPED: proxy port $port is already in use"
        exit 0
    fi
done

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
EVIDENCE_DIR="$(mktemp -d "$SCRATCH_STORE/scenario-13-XXXXXX")"
echo "EVIDENCE: $EVIDENCE_DIR"
WORK="$EVIDENCE_DIR"
HELPER="$WORK/model-proxy.py"
cat > "$HELPER" <<'MODEL_PROXY_PY'
#!/usr/bin/env python3
"""Owned model-HTTP fault proxy for durability scenario-13.

Fronts exactly one real approved local model server (a foreign service: the
proxy never stops or reconfigures it) and either forwards the request or
injects the fault named by its mode file.  Every request and response is
appended to a JSONL log before the client is answered, so the log can be
asserted on while the review process is still running.

Modes, re-read from --mode-file for every request:
  forward    route the request to the target and return its response
  fail503    answer HTTP 503 with a JSON error body
  malformed  answer HTTP 200 with assistant content that is not a verdict batch
  stall      accept the request and answer only after --stall-seconds

Each request line records the decoded case ids and subject of the packet it
carries (the reviews send the packet text inside the chat message), so the
scenario can prove which case a lane was charged for.  Case ids contain
spaces ("School identity:<subject>:<revision>:<digest>"), so they are read to
the end of their packet line.
"""

import argparse
import hashlib
import http.server
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request

CASE_ID_RE = re.compile(r"case_id:\s*([^\n`]+)")
SUBJECT_ID_RE = re.compile(r"subject_id:\s*([^\n`]+)")
MODES = ("forward", "fail503", "malformed", "stall")


def parse_args(argv):
    parser = argparse.ArgumentParser()
    parser.add_argument("--listen", type=int, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--log", required=True)
    parser.add_argument("--mode-file", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--stall-seconds", type=float, default=20.0)
    parser.add_argument("--forward-timeout", type=float, default=600.0)
    return parser.parse_args(argv)


def extract_packet(text):
    """Decode the chat message content and pull the case ids and subject out."""
    content = text
    try:
        body = json.loads(text)
        messages = body.get("messages")
        if isinstance(messages, list):
            parts = [
                message.get("content", "")
                for message in messages
                if isinstance(message, dict)
            ]
            joined = "\n".join(part for part in parts if isinstance(part, str))
            if joined:
                content = joined
    except (ValueError, AttributeError):
        pass
    case_ids = []
    for match in CASE_ID_RE.finditer(content):
        case_id = match.group(1).strip()
        if case_id and case_id not in case_ids:
            case_ids.append(case_id)
    subject = SUBJECT_ID_RE.search(content)
    return case_ids, (subject.group(1).strip() if subject else None)


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "scenario13-proxy"

    def log_message(self, *_args):
        pass

    def record(self, event, **fields):
        row = {"event": event, "ts": time.time(), "label": self.server.label, **fields}
        line = json.dumps(row, sort_keys=True) + "\n"
        with open(self.server.log_path, "a", encoding="utf-8") as handle:
            handle.write(line)
            handle.flush()
            os.fsync(handle.fileno())

    def mode(self):
        try:
            with open(self.server.mode_file, encoding="utf-8") as handle:
                mode = handle.read().strip()
        except OSError:
            return "forward"
        return mode if mode in MODES else "forward"

    def fault_response(self, mode):
        if mode == "fail503":
            body = json.dumps(
                {"error": {"message": "scenario-13 injected 503", "type": "fault"}}
            ).encode()
            return 503, body
        if mode == "malformed":
            body = json.dumps(
                {
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "scenario-13 injected malformed content",
                            }
                        }
                    ]
                }
            ).encode()
            return 200, body
        body = json.dumps(
            {"error": {"message": "scenario-13 injected stall", "type": "fault"}}
        ).encode()
        return 504, body

    def forward(self, body):
        headers = {"content-type": self.headers.get("content-type", "application/json")}
        request = urllib.request.Request(
            self.server.target + self.path, data=body, headers=headers, method=self.command
        )
        try:
            with urllib.request.urlopen(
                request, timeout=self.server.forward_timeout
            ) as response:
                return response.status, response.read()
        except urllib.error.HTTPError as error:
            return error.code, error.read()
        except Exception as error:  # noqa: BLE001 - the fault is the evidence
            return 599, json.dumps({"proxy_error": repr(error)}).encode()

    def handle_request(self, body):
        started = time.time()
        mode = self.mode()
        text = body.decode("utf-8", "replace")
        case_ids, subject = extract_packet(text)
        self.record(
            "request",
            method=self.command,
            path=self.path,
            mode=mode,
            request_bytes=len(body or b""),
            request_sha256=hashlib.sha256(body or b"").hexdigest(),
            subject_id=subject,
            case_ids=case_ids,
            body=text,
        )
        if mode == "stall":
            deadline = started + self.server.stall_seconds
            while time.time() < deadline:
                time.sleep(0.2)
            code, payload = self.fault_response(mode)
        elif mode == "forward":
            code, payload = self.forward(body)
        else:
            code, payload = self.fault_response(mode)
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(payload)))
        self.end_headers()
        try:
            self.wfile.write(payload)
        except (BrokenPipeError, ConnectionResetError):
            pass
        self.record(
            "response",
            method=self.command,
            path=self.path,
            mode=mode,
            status=code,
            response_bytes=len(payload),
            duration_ms=round((time.time() - started) * 1000.0, 3),
            case_ids=case_ids,
            payload=payload.decode("utf-8", "replace"),
        )

    def do_POST(self):
        length = int(self.headers.get("content-length") or 0)
        body = self.rfile.read(length) if length else b""
        self.handle_request(body)

    def do_GET(self):
        self.handle_request(b"")


def main(argv):
    args = parse_args(argv)
    with open(args.mode_file, "w", encoding="utf-8") as handle:
        handle.write("forward\n")
    server = http.server.ThreadingHTTPServer(("127.0.0.1", args.listen), Handler)
    server.daemon_threads = True
    server.label = args.label
    server.log_path = args.log
    server.mode_file = args.mode_file
    server.target = args.target.rstrip("/")
    server.stall_seconds = args.stall_seconds
    server.forward_timeout = args.forward_timeout
    print(
        f"scenario-13 proxy {args.label} listening on 127.0.0.1:{args.listen} -> {args.target}",
        flush=True,
    )
    server.serve_forever()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

MODEL_PROXY_PY
cd "$REPO_ROOT"

PROXY_A_PID=""
PROXY_B_PID=""
REVIEW_PID=""
cleanup() {
    if [ -n "$REVIEW_PID" ]; then kill -9 "$REVIEW_PID" 2>/dev/null || true; fi
    if [ -n "$PROXY_A_PID" ]; then kill "$PROXY_A_PID" 2>/dev/null || true; fi
    if [ -n "$PROXY_B_PID" ]; then kill "$PROXY_B_PID" 2>/dev/null || true; fi
    wait 2>/dev/null || true
}
trap cleanup EXIT

fail() { echo "FAIL: $*"; exit 1; }

# ---- assertions -----------------------------------------------------------
# Read one whitespace-separated counter field from a phase output, anchored so
# that `answered=` never matches inside `unanswered=`.
counter() {
    awk -v key="$1" '
        { for (i = 1; i <= NF; i++) if ($i ~ ("^" key "=[0-9]+$")) value = substr($i, length(key) + 2) }
        END { if (value != "") print value }
    ' "$2"
}
expect() {
    local key="$1" want="$2" file="$3" got
    got="$(counter "$key" "$file" || true)"
    [ "$got" = "$want" ] || fail "expected $key=$want, observed ${got:-<missing>} in $(basename "$file")"
}

# Count log lines carrying an event name and, unless the needle is empty, the
# needle (a case id) as a literal substring.
awkc() {
    awk -v needle="$2" -v event="$3" '
        BEGIN { marker = "\"event\": \"" event "\"" }
        index($0, marker) && (needle == "" || index($0, needle)) { n += 1 }
        END { print n + 0 }
    ' "$1"
}
has_request() { [ "$(awkc "$1" "$2" request)" -gt 0 ]; }
has_more_requests() { [ "$(awkc "$1" "" request)" -gt "$2" ]; }
has_response() { [ "$(awkc "$1" "$2" response)" -gt 0 ]; }
newest_case() {
    python3 -c '
import json, sys
rows = [json.loads(line) for line in open(sys.argv[1])]
seen = [row for row in rows if row["event"] == "request" and row.get("case_ids")]
print(seen[-1]["case_ids"][0] if seen else "")
' "$1"
}
wait_until() {
    local what="$1" seconds="$2"
    shift 2
    local ticks=$((seconds * 20)) i=0
    while [ "$i" -lt "$ticks" ]; do
        if "$@"; then return 0; fi
        sleep 0.05
        i=$((i + 1))
    done
    echo "TIMEOUT: timed out after ${seconds}s waiting for $what"
    return 1
}
now_ms() { date +%s%3N; }

# ---- store statistics -----------------------------------------------------
stat_table() { "$CLIENT" --store "$STORE" fjall-stats 2>&1; }
stat_value() { stat_table | awk -v key="$1" '$1 == key { value = $2 } END { if (value != "") print value }'; }
journal_bytes() { stat -c '%s' "$STORE"/fjall/*.jnl 2>/dev/null | awk '{ total += $1 } END { print total + 0 }'; }

# ---- review invocation ----------------------------------------------------
build_argv() {
    ARGV=(--store "$STORE" review --family school-link --limit 1
        --endpoint "http://127.0.0.1:$PROXY_A_PORT" --endpoint "http://127.0.0.1:$PROXY_B_PORT"
        --model "$MODEL" --response-format prompt-json --response-format json-schema
        --timeout-secs "$LANE_TIMEOUT" --max-tokens "$MAX_TOKENS" --observed-on "$OBSERVED_ON" "$@")
}
set_modes() {
    printf '%s\n' "$1" > "$WORK/proxy-a.mode"
    printf '%s\n' "$2" > "$WORK/proxy-b.mode"
}
review() {
    local out="$1" mode_a="$2" mode_b="$3"
    shift 3
    build_argv "$@"
    set_modes "$mode_a" "$mode_b"
    REVIEW_RC=0
    {
        printf '$ census-service'; printf ' %q' "${ARGV[@]}"; printf '   # lane modes %s/%s\n' "$mode_a" "$mode_b"
        timeout $((LANE_TIMEOUT + 60)) "$CLIENT" "${ARGV[@]}"
    } > "$out" 2>&1 || REVIEW_RC=$?
}
# Background runs are started without a `timeout` wrapper so that the recorded
# pid is the review process itself and a SIGKILL lands on it, not on a wrapper.
review_bg() {
    local out="$1" mode_a="$2" mode_b="$3"
    shift 3
    build_argv "$@"
    set_modes "$mode_a" "$mode_b"
    {
        printf '$ census-service'; printf ' %q' "${ARGV[@]}"; printf '   # lane modes %s/%s\n' "$mode_a" "$mode_b"
    } > "$out"
    "$CLIENT" "${ARGV[@]}" >> "$out" 2>&1 &
    REVIEW_PID=$!
}

# ---- [0/5] transport lane -------------------------------------------------
echo "=== [0/5] transport lane: census-review model::transport_tests ==="
set +e
tools/moon-cargo test -p census-review --lib model::transport_tests -- --nocapture 2>&1 | tee "$WORK/transport.txt"
TRANSPORT_RC=${PIPESTATUS[0]}
set -e
grep -qE '^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;' "$WORK/transport.txt" \
    || fail "census-review model::transport_tests did not report a clean pass (exit $TRANSPORT_RC): $(grep -m1 'test result:' "$WORK/transport.txt" || true)"
echo "TRANSPORT: census-review model::transport_tests passed (exit $TRANSPORT_RC)"

# ---- private store --------------------------------------------------------
echo "=== seed store: private copy, migrated with store-migrate ==="
cp -a "$SEED_STORE" "$WORK/store"
STORE="$WORK/store"
"$CLIENT" --store "$STORE" store-migrate > "$WORK/store-migrate.json" 2>&1 \
    || fail "store-migrate refused the private copy: $(cat "$WORK/store-migrate.json")"
grep -q '"integrity_ok": true' "$WORK/store-migrate.json" \
    || fail "store-migrate reported integrity_ok != true: $(cat "$WORK/store-migrate.json")"
stat_table > "$WORK/stats-baseline.txt"
BASE_VERDICTS="$(stat_value identity_verdicts)"
BASE_CASES="$(stat_value review_cases)"
[ -n "$BASE_VERDICTS" ] || fail "could not read identity_verdicts from fjall-stats"
echo "STORE: review_cases=$BASE_CASES identity_verdicts=$BASE_VERDICTS"

# ---- fault proxies --------------------------------------------------------
echo "=== fault proxies in front of the real model servers ==="
python3 "$HELPER" --listen "$PROXY_A_PORT" --target "$TARGET_A" --log "$WORK/proxy-a.jsonl" \
    --mode-file "$WORK/proxy-a.mode" --label laneA --stall-seconds "$STALL_SECONDS" > "$WORK/proxy-a.log" 2>&1 &
PROXY_A_PID=$!
python3 "$HELPER" --listen "$PROXY_B_PORT" --target "$TARGET_B" --log "$WORK/proxy-b.jsonl" \
    --mode-file "$WORK/proxy-b.mode" --label laneB --stall-seconds "$STALL_SECONDS" > "$WORK/proxy-b.log" 2>&1 &
PROXY_B_PID=$!
sleep 0.5
kill -0 "$PROXY_A_PID" 2>/dev/null || fail "proxy lane A exited at startup: $(cat "$WORK/proxy-a.log")"
kill -0 "$PROXY_B_PID" 2>/dev/null || fail "proxy lane B exited at startup: $(cat "$WORK/proxy-b.log")"
set_modes forward forward
SERVER_OK=true
curl -sS --max-time 30 "http://127.0.0.1:$PROXY_A_PORT/v1/models" > "$WORK/models-lane-a.json" 2> "$WORK/models-lane-a.err" || SERVER_OK=false
curl -sS --max-time 30 "http://127.0.0.1:$PROXY_B_PORT/v1/models" > "$WORK/models-lane-b.json" 2> "$WORK/models-lane-b.err" || SERVER_OK=false
if [ "$SERVER_OK" = true ]; then
    echo "PROXIES: both lanes forward to reachable model servers"
else
    echo "PROXIES: model servers unreachable through the proxies (see models-lane-*.err); healthy-lane phases will not run"
fi

# ---- [1/5] refusal taxonomy (dry-run; nothing persists) -------------------
echo "=== [1/5] refusal taxonomy: 503, malformed content, timed-out lane ==="
if [ "$SERVER_OK" = true ]; then want_failed=1; else want_failed=2; fi
for fault in fail503 malformed stall; do
    extra=()
    [ "$fault" = stall ] && extra=(--timeout-secs "$FAULT_TIMEOUT")
    a0="$(awkc "$WORK/proxy-a.jsonl" "" request)"
    b0="$(awkc "$WORK/proxy-b.jsonl" "" request)"
    review "$WORK/phase1-$fault.txt" "$fault" forward --dry-run "${extra[@]}"
    [ "$REVIEW_RC" = 0 ] || fail "dry-run with lane A=$fault exited $REVIEW_RC: $(tail -3 "$WORK/phase1-$fault.txt")"
    expect requested 1 "$WORK/phase1-$fault.txt"
    expect unaskable 0 "$WORK/phase1-$fault.txt"
    expect answered 0 "$WORK/phase1-$fault.txt"
    expect accepted 0 "$WORK/phase1-$fault.txt"
    expect rejected 0 "$WORK/phase1-$fault.txt"
    expect insufficient 0 "$WORK/phase1-$fault.txt"
    expect failed "$want_failed" "$WORK/phase1-$fault.txt"
    a1="$(awkc "$WORK/proxy-a.jsonl" "" request)"
    b1="$(awkc "$WORK/proxy-b.jsonl" "" request)"
    [ $((a1 - a0)) -eq 1 ] || fail "$fault: lane A was charged $((a1 - a0)) requests, want exactly 1"
    [ $((b1 - b0)) -eq 1 ] || fail "$fault: lane B was charged $((b1 - b0)) requests, want exactly 1"
    echo "FAULT $fault: not accepted, one request per lane (failed=$want_failed)"
done
CASE1="$(newest_case "$WORK/proxy-a.jsonl")"
[ -n "$CASE1" ] || fail "no case id was logged during the refusal taxonomy"
stat_table > "$WORK/stats-after-phase1.txt"
[ "$(stat_value identity_verdicts)" = "$BASE_VERDICTS" ] \
    || fail "the dry-run faults wrote a verdict row (identity_verdicts $BASE_VERDICTS -> $(stat_value identity_verdicts))"
echo "REFUSALS: $CASE1 never accepted; dry-runs left identity_verdicts=$BASE_VERDICTS"

# ---- [2/5] failed second lane: bounded retry, first advice reused ---------
echo "=== [2/5] failed second lane: bounded retry and first-advice reuse ==="
a0="$(awkc "$WORK/proxy-a.jsonl" "" request)"
b0="$(awkc "$WORK/proxy-b.jsonl" "" request)"
review "$WORK/phase2a-lane-b-failed.txt" forward fail503
[ "$REVIEW_RC" = 0 ] || fail "persisting run with lane B failing exited $REVIEW_RC: $(tail -3 "$WORK/phase2a-lane-b-failed.txt")"
expect requested 1 "$WORK/phase2a-lane-b-failed.txt"
expect unaskable 0 "$WORK/phase2a-lane-b-failed.txt"
expect answered 0 "$WORK/phase2a-lane-b-failed.txt"
expect unanswered 1 "$WORK/phase2a-lane-b-failed.txt"
expect failed 1 "$WORK/phase2a-lane-b-failed.txt"
expect accepted 0 "$WORK/phase2a-lane-b-failed.txt"
verdicts_2a="$(stat_value identity_verdicts)"
[ "$verdicts_2a" = "$((BASE_VERDICTS + 1))" ] \
    || fail "a persisted lane failure should record exactly one standing row: identity_verdicts $BASE_VERDICTS -> $verdicts_2a"
a_2a="$(awkc "$WORK/proxy-a.jsonl" "" request)"
b_2a="$(awkc "$WORK/proxy-b.jsonl" "" request)"
[ $((a_2a - a0)) -eq 1 ] || fail "lane A charged $((a_2a - a0)) requests, want exactly 1"
[ $((b_2a - b0)) -eq 1 ] || fail "lane B charged $((b_2a - b0)) requests, want exactly 1"
RETRY_CASE="$(newest_case "$WORK/proxy-b.jsonl")"
[ "$RETRY_CASE" = "$CASE1" ] || fail "the failed-lane run asked $RETRY_CASE, expected the first actionable case $CASE1"
echo "PERSISTED-FAILURE: $CASE1 retained with lane A advice and lane B failed (identity_verdicts $BASE_VERDICTS -> $verdicts_2a)"

review "$WORK/phase2b-retry.txt" forward forward
[ "$REVIEW_RC" = 0 ] || fail "retry run exited $REVIEW_RC: $(tail -3 "$WORK/phase2b-retry.txt")"
expect requested 1 "$WORK/phase2b-retry.txt"
expect unaskable 0 "$WORK/phase2b-retry.txt"
expect answered 1 "$WORK/phase2b-retry.txt"
expect unanswered 0 "$WORK/phase2b-retry.txt"
expect failed 0 "$WORK/phase2b-retry.txt"
a_2b="$(awkc "$WORK/proxy-a.jsonl" "" request)"
b_2b="$(awkc "$WORK/proxy-b.jsonl" "" request)"
[ $((a_2b - a_2a)) -eq 0 ] \
    || fail "the first valid advice was re-asked: lane A charged $((a_2b - a_2a)) requests, want 0"
[ $((b_2b - b_2a)) -eq 1 ] \
    || fail "the failed lane was not retried exactly once: lane B charged $((b_2b - b_2a)) requests, want 1"
[ "$(awkc "$WORK/proxy-b.jsonl" "$RETRY_CASE" request)" = 2 ] \
    || fail "the bounded retry did not re-ask the same case $RETRY_CASE"
[ "$(stat_value identity_verdicts)" = "$verdicts_2a" ] \
    || fail "the retry added a second standing row for $RETRY_CASE"
echo "RETRY: $RETRY_CASE lane A +0 requests (advice reused), lane B +1 request (bounded retry), rows unchanged"

review "$WORK/phase2c-replay.txt" forward forward
verdicts_2c="$(stat_value identity_verdicts)"
[ "$verdicts_2c" = "$((BASE_VERDICTS + 2))" ] \
    || fail "the fresh process should record exactly one standing row for the advanced case: identity_verdicts $verdicts_2a -> $verdicts_2c"
[ "$REVIEW_RC" = 0 ] || fail "replay run exited $REVIEW_RC: $(tail -3 "$WORK/phase2c-replay.txt")"
expect requested 1 "$WORK/phase2c-replay.txt"
expect unaskable 0 "$WORK/phase2c-replay.txt"
expect answered 1 "$WORK/phase2c-replay.txt"
expect unanswered 0 "$WORK/phase2c-replay.txt"
expect failed 0 "$WORK/phase2c-replay.txt"
REPLAY_CASE="$(newest_case "$WORK/proxy-a.jsonl")"
[ -n "$REPLAY_CASE" ] && [ "$REPLAY_CASE" != "$RETRY_CASE" ] \
    || fail "the fresh process re-asked the standing case $RETRY_CASE instead of advancing"
[ "$(awkc "$WORK/proxy-a.jsonl" "$RETRY_CASE" request)" = 1 ] \
    || fail "the fresh process re-asked $RETRY_CASE on lane A"
[ "$(awkc "$WORK/proxy-b.jsonl" "$RETRY_CASE" request)" = 2 ] \
    || fail "the fresh process re-asked $RETRY_CASE on lane B"
echo "REPLAY: $RETRY_CASE advice replayed from a fresh process with zero new requests; advanced to $REPLAY_CASE"

if [ "$SERVER_OK" != true ]; then
    echo "SKIPPED: model servers $TARGET_A and $TARGET_B unreachable through the proxies: only the refusal taxonomy above was exercised; the healthy-lane retry, crash-before-write and crash-after-write phases did not run"
    exit 0
fi

# ---- [3/5] SIGKILL before the first advice write --------------------------
echo "=== [3/5] SIGKILL before the first advice write ==="
verdicts_before_3="$(stat_value identity_verdicts)"
b_before_3="$(awkc "$WORK/proxy-b.jsonl" "" request)"
review_bg "$WORK/phase3-killed.txt" forward stall
KILLED_PID="$REVIEW_PID"
wait_until "the killed run's lane B request" 60 has_more_requests "$WORK/proxy-b.jsonl" "$b_before_3" \
    || fail "the killed run never asked lane B: $(tail -3 "$WORK/phase3-killed.txt")"
CASE3="$(newest_case "$WORK/proxy-b.jsonl")"
wait_until "lane A's answer for $CASE3" 120 has_response "$WORK/proxy-a.jsonl" "$CASE3" \
    || fail "lane A never answered $CASE3: $(tail -3 "$WORK/phase3-killed.txt")"
sleep 0.5
kill -9 "$KILLED_PID" 2>/dev/null || fail "the killed run already exited before the SIGKILL: $(tail -3 "$WORK/phase3-killed.txt")"
set +e
wait "$KILLED_PID"
RC3=$?
set -e
REVIEW_PID=""
[ "$RC3" = 137 ] || fail "expected the review to die by SIGKILL (exit 137), observed $RC3"
[ "$(stat_value identity_verdicts)" = "$verdicts_before_3" ] \
    || fail "the killed run committed a verdict row for $CASE3 (identity_verdicts $verdicts_before_3 -> $(stat_value identity_verdicts))"
echo "CRASH-BEFORE: $CASE3 killed by SIGKILL with no verdict row (identity_verdicts unchanged at $verdicts_before_3)"

a_before_3b="$(awkc "$WORK/proxy-a.jsonl" "$CASE3" request)"
review "$WORK/phase3-resume.txt" forward forward
[ "$REVIEW_RC" = 0 ] || fail "resume run exited $REVIEW_RC: $(tail -3 "$WORK/phase3-resume.txt")"
expect requested 1 "$WORK/phase3-resume.txt"
expect unaskable 0 "$WORK/phase3-resume.txt"
expect answered 1 "$WORK/phase3-resume.txt"
expect failed 0 "$WORK/phase3-resume.txt"
[ "$(newest_case "$WORK/proxy-a.jsonl")" = "$CASE3" ] || fail "the resume run did not re-ask the killed case $CASE3"
a_after_3b="$(awkc "$WORK/proxy-a.jsonl" "$CASE3" request)"
[ $((a_after_3b - a_before_3b)) -eq 1 ] \
    || fail "the resume charged lane A $((a_after_3b - a_before_3b)) requests for $CASE3, want 1"
verdicts_3b="$(stat_value identity_verdicts)"
[ "$verdicts_3b" = "$((verdicts_before_3 + 1))" ] \
    || fail "the resume should record exactly one standing row: identity_verdicts $verdicts_before_3 -> $verdicts_3b"
echo "CRASH-BEFORE-RESUME: $CASE3 completed after the SIGKILL (identity_verdicts $verdicts_before_3 -> $verdicts_3b)"

# ---- [4/5] SIGKILL as soon as the advice write is observed ----------------
echo "=== [4/5] SIGKILL after the observed advice write ==="
CRASH_CASE=""
attempt=0
while [ "$attempt" -lt 3 ]; do
    attempt=$((attempt + 1))
    out="$WORK/phase4-attempt$attempt.txt"
    verdicts_before_4="$(stat_value identity_verdicts)"
    journal_before="$(journal_bytes)"
    a_before_4="$(awkc "$WORK/proxy-a.jsonl" "" request)"
    started_ms="$(now_ms)"
    review_bg "$out" forward forward
    KILLED_PID="$REVIEW_PID"
    if ! wait_until "the attempt's lane A request" 60 has_more_requests "$WORK/proxy-a.jsonl" "$a_before_4"; then
        kill -9 "$KILLED_PID" 2>/dev/null || true
        set +e; wait "$KILLED_PID"; set -e; REVIEW_PID=""
        echo "ATTEMPT $attempt: no lane A request; tail: $(tail -2 "$out")"
        continue
    fi
    ATTEMPT_CASE="$(newest_case "$WORK/proxy-a.jsonl")"
    # The review owns the store lock, so the journal file is the only witness
    # available from outside; kill as soon as it grows (that growth is the
    # commit), and record how long that took.
    grew_ms=""
    tick=0
    while [ "$tick" -lt 3000 ]; do
        if [ "$(journal_bytes)" != "$journal_before" ]; then grew_ms=$(( $(now_ms) - started_ms )); break; fi
        sleep 0.002
        tick=$((tick + 1))
    done
    kill_rc=0
    if [ -n "$grew_ms" ]; then kill -9 "$KILLED_PID" 2>/dev/null || kill_rc=$?; fi
    set +e
    wait "$KILLED_PID"
    RC4=$?
    set -e
    REVIEW_PID=""
    verdicts_after_4="$(stat_value identity_verdicts)"
    printf 'attempt %s: case=%s journal_grew_ms=%s kill_rc=%s rc=%s identity_verdicts=%s->%s\n' \
        "$attempt" "$ATTEMPT_CASE" "${grew_ms:-none}" "$kill_rc" "$RC4" "$verdicts_before_4" "$verdicts_after_4" \
        | tee -a "$WORK/phase4-attempts.txt"
    if [ -n "$grew_ms" ] && [ "$RC4" = 137 ] && [ "$verdicts_after_4" = "$((verdicts_before_4 + 1))" ]; then
        CRASH_CASE="$ATTEMPT_CASE"
        break
    fi
done
[ -n "$CRASH_CASE" ] \
    || fail "no attempt observed both the advice write and a SIGKILL death; see $WORK/phase4-attempts.txt"
echo "CRASH-AFTER: $CRASH_CASE killed by SIGKILL after its advice row landed"
a_at_crash="$(awkc "$WORK/proxy-a.jsonl" "$CRASH_CASE" request)"
b_at_crash="$(awkc "$WORK/proxy-b.jsonl" "$CRASH_CASE" request)"

review "$WORK/phase4-replay.txt" fail503 fail503
[ "$REVIEW_RC" = 0 ] || fail "post-crash replay exited $REVIEW_RC: $(tail -3 "$WORK/phase4-replay.txt")"
expect requested 1 "$WORK/phase4-replay.txt"
expect unaskable 0 "$WORK/phase4-replay.txt"
expect answered 0 "$WORK/phase4-replay.txt"
expect failed 2 "$WORK/phase4-replay.txt"
expect accepted 0 "$WORK/phase4-replay.txt"
AFTER_CASE="$(newest_case "$WORK/proxy-a.jsonl")"
[ -n "$AFTER_CASE" ] && [ "$AFTER_CASE" != "$CRASH_CASE" ] \
    || fail "the post-crash run re-asked the persisted case $CRASH_CASE"
[ "$(awkc "$WORK/proxy-a.jsonl" "$CRASH_CASE" request)" = "$a_at_crash" ] \
    || fail "the post-crash run re-asked $CRASH_CASE on lane A"
[ "$(awkc "$WORK/proxy-b.jsonl" "$CRASH_CASE" request)" = "$b_at_crash" ] \
    || fail "the post-crash run re-asked $CRASH_CASE on lane B"
echo "CRASH-AFTER-REPLAY: $CRASH_CASE advice survived and replayed with zero new requests; advanced to $AFTER_CASE"

# ---- [5/5] summary --------------------------------------------------------
stat_table > "$WORK/stats-final.txt"
echo "STORE-AFTER: review_cases=$BASE_CASES->$(stat_value review_cases) identity_verdicts=$BASE_VERDICTS->$(stat_value identity_verdicts)"
echo "PASS: model HTTP faults (503/malformed/timeout) never accepted with one bounded request per lane; a failed second lane retried once with the first valid advice reused; SIGKILL before the first advice write left nothing durable and the case resumed; SIGKILL after the observed advice write was survived and replayed without re-asking"
