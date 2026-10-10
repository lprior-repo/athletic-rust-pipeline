#!/usr/bin/env bash
# AthleticLIVE fixture capture for crates/census-crawl (meets/results/athletes).
#
# Run from the repository root:
#   bash crates/census-crawl/tests/fixtures/athleticlive/capture.sh
#
# 14 requests, >=2 s apart, sequential, no crawling. Writes raw response bodies,
# one meta json per capture, a JSONL ledger, a derived meet-harvest CSV and a
# ready-to-replay results manifest.
#
# robots.txt is captured as provenance only. This pipeline no longer gates on
# robots admission (owner directive 2026-10-10); the engineered rate limits stay.
set -euo pipefail

D="crates/census-crawl/tests/fixtures/athleticlive"
REL="crates/census-crawl/tests/fixtures/athleticlive"
UA="athletic-rust-pipeline-fixture-capture/1.0"
PAUSE=2
MEET=55746
SEARCH="https://search.athletic.live"
BLOB="https://athleticlive.blob.core.windows.net"
RTDB="https://s-gke-usc1-nssi3-33.firebaseio.com"

if [[ ! -d "$D" ]]; then
  echo "run from the repository root: $D is missing" >&2
  exit 1
fi
command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }

overwrite=0
if [[ "${1:-}" == "--overwrite" ]]; then overwrite=1; fi
if [[ $overwrite -eq 0 ]]; then
  shopt -s nullglob
  prior=( "$D"/*.meta.json "$D"/captures.log "$D"/PROVENANCE.json "$D"/harvest-wi-mn-2025.csv "$D"/manifest-meet-55746.json )
  shopt -u nullglob
  if (( ${#prior[@]} > 0 )); then
    echo "existing captures present; nothing written:" >&2
    printf '  %s\n' "${prior[@]}" >&2
    echo "move them aside or re-run with --overwrite" >&2
    exit 1
  fi
fi

: > "$D/captures.log"
STATUS=""; BYTES=""; CTYPE=""; SECS=""; EFFECTIVE=""; DIGEST=""; FETCHED=""; COUNT=0

write_meta() {
  jq -n \
    --arg file "$1" --arg method "$2" --arg url "$3" --arg effective "$4" \
    --argjson status "$STATUS" --argjson bytes "$BYTES" --arg ct "$CTYPE" \
    --arg sha "$DIGEST" --arg at "$FETCHED" --argjson secs "$SECS" \
    '{file:$file, url:$url, method:$method, status:$status, bytes:$bytes,
      content_digest:$sha, fetched_at:$at, content_type:$ct,
      sha256:$sha, fetched_at_utc:$at, time_seconds:$secs}
     + (if $effective == "" then {} else {response_url:$effective} end)' > "$D/$1.meta.json"
}

journal() {
  jq -c -n --arg file "$1" --arg method "$2" --arg url "$3" \
    --argjson status "$STATUS" --argjson bytes "$BYTES" --arg ct "$CTYPE" \
    --arg sha "$DIGEST" --arg at "$FETCHED" --argjson secs "$SECS" \
    '{file:$file, url:$url, method:$method, status:$status, bytes:$bytes,
      content_type:$ct, sha256:$sha, fetched_at_utc:$at, time_seconds:$secs}' >> "$D/captures.log"
}

journal_derived() {
  local digest bytes
  digest=$(sha256sum "$D/$1" | cut -d' ' -f1)
  bytes=$(wc -c < "$D/$1" | tr -d ' ')
  jq -c -n --arg file "$1" --arg source "$2" --arg sha "$digest" --argjson bytes "$bytes" \
    '{file:$file, derived:true, source:$source, bytes:$bytes, sha256:$sha}' >> "$D/captures.log"
}

capture() {
  local method="$1" file="$2" url="$3"; shift 3
  sleep "$PAUSE"
  local out
  if ! out=$(curl -sS --max-time 60 -L --max-redirs 3 -A "$UA" -X "$method" \
      -o "$D/$file" -w '%{http_code}\n%{size_download}\n%{content_type}\n%{time_total}\n%{url_effective}' \
      "$@" "$url"); then
    echo "curl failed: $method $url" >&2
    exit 1
  fi
  STATUS=$(printf '%s\n' "$out" | sed -n 1p)
  BYTES=$(printf '%s\n' "$out" | sed -n 2p)
  CTYPE=$(printf '%s\n' "$out" | sed -n 3p)
  SECS=$(printf '%s\n' "$out" | sed -n 4p)
  EFFECTIVE=$(printf '%s\n' "$out" | sed -n 5p)
  DIGEST=$(sha256sum "$D/$file" | cut -d' ' -f1)
  FETCHED=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  if [[ "$EFFECTIVE" == "$url" ]]; then EFFECTIVE=""; fi
  COUNT=$((COUNT + 1))
  write_meta "$file" "$method" "$url" "$EFFECTIVE"
  journal "$file" "$method" "$url"
  printf '%s %s %s %s\n' "$STATUS" "$BYTES" "$CTYPE" "$file"
}

expect200() {
  if [[ "$STATUS" != "200" ]]; then
    echo "expected HTTP 200, got $STATUS for $1" >&2
    exit 1
  fi
}

echo "== robots.txt (provenance only; any status is recorded, not gated)"
capture GET robots-search-athletic-live.txt "$SEARCH/robots.txt"
capture GET robots-www-athletic-live.txt "https://www.athletic.live/robots.txt"
capture GET robots-blob-athletic-live.txt "$BLOB/robots.txt"
capture GET robots-rtdb-firebaseio.txt "$RTDB/robots.txt"

echo "== two-state meet listing (Wisconsin + Minnesota, 2025-08-01 .. 2025-12-01)"
capture POST meet-list-wi-mn-2025.json "$SEARCH/*_meet_list/_search" \
  -H 'Content-Type: application/json' --data-binary "@$D/meet-list-wi-mn-2025.request.json"
if [[ "$STATUS" != "200" ]]; then
  jq -c -n --arg note "POST listing refused ($STATUS); captured the URI dialect that was verified 2026-10-09" \
    '{note:$note}' >> "$D/captures.log"
  capture GET meet-list-wi-mn-2025.json \
    "$SEARCH/*_meet_list/_search?q=lsa.keyword:(Wisconsin%20OR%20Minnesota)%20AND%20sd:%5B2025-08-01%20TO%202025-12-01%5D&size=50&track_total_hits=true"
fi
expect200 "the WI+MN meet listing"

echo "== the fixture meet's own meet document (tenant, name, state, date)"
capture POST meet-doc-55746.json "$SEARCH/*_meet_list/_search" \
  -H 'Content-Type: application/json' --data-binary "@$D/meet-doc-55746.request.json"
expect200 "meet $MEET"
jq -e --argjson id "$MEET" \
  '(.hits.hits | length) >= 1 and ((.hits.hits[0]._source.i|tostring) == ($id|tostring))' \
  "$D/meet-doc-55746.json" >/dev/null || { echo "meet-doc-55746.json does not carry meet $MEET" >&2; exit 1; }
TENANT=$(jq -r '.hits.hits[0]._index | sub("_meet_list$";"")' "$D/meet-doc-55746.json")
NAME=$(jq -r '.hits.hits[0]._source.n' "$D/meet-doc-55746.json")
STATE=$(jq -r '.hits.hits[0]._source.lsa' "$D/meet-doc-55746.json")
DATE=$(jq -r '.hits.hits[0]._source | (.sdy // (.sd[0:10]))' "$D/meet-doc-55746.json")

echo "== event summary (RTDB, the collector's canonical summary URL)"
capture GET event-summary-meet-55746.json "$RTDB/meet_$MEET/event_summary.json?ns=trackmeet-io"
expect200 "the event summary for meet $MEET"
jq -e 'type == "object"' "$D/event-summary-meet-55746.json" >/dev/null \
  || { echo "the summary is not a meet object (RTDB purge or null body)" >&2; exit 1; }
mapfile -t EVENTS < <(jq -r 'to_entries | map(.value) | map(select(.ec == "Individual") | .i)
  | map(select(type == "number")) | unique | sort | .[]' "$D/event-summary-meet-55746.json")
if (( ${#EVENTS[@]} == 0 )); then
  echo "the summary lists no individual events" >&2
  exit 1
fi
echo "== one blob document per listed event (${#EVENTS[@]} events)"
DOCS=()
for id in "${EVENTS[@]}"; do
  file="ind-res-list-$id.json"
  capture GET "$file" "$BLOB/\$web/ind_res_list/_doc/$id"
  expect200 "event document $id"
  jq -e --arg id "$id" --argjson mi "$MEET" \
    '((._source.i|tostring) == $id) and ((._source.mi|tostring) == ($mi|tostring))' \
    "$D/$file" >/dev/null || { echo "$file does not carry event $id of meet $MEET" >&2; exit 1; }
  DOCS+=("$file")
done

echo "== athlete seed page (the exact batch_query the athletes collector issues)"
capture POST athlete-list-meet-55746.json "$SEARCH/athlete_list/_search" \
  -H 'Content-Type: application/json' --data-binary "@$D/athlete-list-meet-55746.request.json"
expect200 "the athlete list for meet $MEET"
jq -e '(.hits.total.value // 0) > 0' "$D/athlete-list-meet-55746.json" >/dev/null \
  || { echo "the athlete list is empty; the grade filter matches nothing here" >&2; exit 1; }
ATHLETES=$(jq -r '.hits.total.value' "$D/athlete-list-meet-55746.json")
LISTED=$(jq -r '.hits.total.value' "$D/meet-list-wi-mn-2025.json")
SUMMARY_EVENTS=${#EVENTS[@]}

echo "== derived harvest CSV in the published-meet CSV shape the meets arm parses"
jq -r '
  (["tenant","athleticlive_meet_id","athleticnet_meet_id","name","city_state","state","start","end","has_results"] | @csv),
  ( .hits.hits[]
    | (._index | sub("_meet_list$";"")) as $tenant
    | ._source
    | [ $tenant,
        (.i|tostring),
        (if (.ani|type) == "number" then (.ani|tostring) else "" end),
        .n,
        (if (.ls|type) == "string" then .ls else "" end),
        .lsa,
        (if (.sdy|type) == "string" then .sdy else (.sd[0:10]) end),
        (if (.ed|type) == "string" then (.ed[0:10]) else "" end),
        ""
      ] | @csv )' "$D/meet-list-wi-mn-2025.json" > "$D/harvest-wi-mn-2025.csv"
journal_derived harvest-wi-mn-2025.csv meet-list-wi-mn-2025.json
HARVEST_ROWS=$(($(wc -l < "$D/harvest-wi-mn-2025.csv" | tr -d ' ') - 1))

echo "== derived replay manifest for the fixture meet"
summary_meta=$(cat "$D/event-summary-meet-55746.json.meta.json")
doc_paths=$(printf '%s\n' "${DOCS[@]}" | jq -R . | jq -s --arg rel "$REL" 'map($rel + "/" + .)')
doc_captures=$(for f in "${DOCS[@]}"; do jq -c --arg p "$REL/$f" '{($p): .}' "$D/$f.meta.json"; done | jq -s 'add')
jq -n --argjson id "$MEET" --arg tenant "$TENANT" --arg name "$NAME" --arg state "$STATE" --arg date "$DATE" \
  --arg sp "$REL/event-summary-meet-55746.json" --argjson sm "$summary_meta" \
  --argjson dp "$doc_paths" --argjson dc "$doc_captures" \
  '{meets:[{athleticlive_meet_id:$id, tenant:$tenant, name:$name, state:$state, date:$date,
            summary:$sp, documents:$dp, captures:($dc + {($sp):$sm})}]}' \
  > "$D/manifest-meet-55746.json"
journal_derived manifest-meet-55746.json event-summary-meet-55746.json

jq -s --arg generator "bash $REL/capture.sh" '{generator:$generator, captures:.}' \
  "$D/captures.log" > "$D/PROVENANCE.json"

echo
echo "requests issued: $COUNT"
echo "meet: $MEET $NAME ($STATE, $DATE) tenant=$TENANT"
echo "listing hits.total: $LISTED; harvested CSV rows: $HARVEST_ROWS"
echo "summary individual events: $SUMMARY_EVENTS; event documents: ${#DOCS[@]}"
echo "athlete seed hits.total: $ATHLETES"
echo
echo "== digests"
sha256sum "$D"/captures.log "$D"/PROVENANCE.json "$D"/harvest-wi-mn-2025.csv \
  "$D"/manifest-meet-55746.json "$D"/meet-list-wi-mn-2025.json "$D"/meet-doc-55746.json \
  "$D"/event-summary-meet-55746.json "$D"/athlete-list-meet-55746.json \
  "$D"/robots-search-athletic-live.txt "$D"/robots-www-athletic-live.txt \
  "$D"/robots-blob-athletic-live.txt "$D"/robots-rtdb-firebaseio.txt \
  "$D"/*.meta.json | sort -k2
