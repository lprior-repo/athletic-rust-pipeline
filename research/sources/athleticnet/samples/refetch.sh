#!/usr/bin/env bash
# Exact commands used to capture every live sample in this directory.
# Run: bash refetch.sh [outdir]   (default outdir: /tmp/atn-refetch)
# Self-imposed discipline: >=1.2 s between requests; <=1 req/s/host.
# Tokens/cookies are never written to disk (see redact-samples.py).
set -uo pipefail
OUT="${1:-/tmp/atn-refetch}"
mkdir -p "$OUT"
UA='Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36'
BASE='https://www.athletic.net'
API="$BASE/api/v1"

hdr=(-H "Accept: application/json, text/plain, */*" -H "Accept-Language: en-US,en;q=0.9")
ua=(-A "$UA")
slp() { sleep 1.2; }

# --- A. robots.txt ------------------------------------------------------------------
curl -sS "${ua[@]}" -D "$OUT/atn-robots.headers" -o "$OUT/atn-robots.txt" "$BASE/robots.txt"; slp

# --- B. anonymous JSON probes -------------------------------------------------------
curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings" \
  -D "$OUT/atn-probe-statescountries-anon.headers" \
  -o "$OUT/atn-probe-statescountries-anon.json" \
  "$API/public/GetStatesCountries2"; slp

curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings/list/170770" \
  -D "$OUT/atn-probe-divchildren-170770.headers" \
  -o "$OUT/atn-probe-divchildren-170770.json" \
  "$API/SiteHeader/GetDivChildren?sport=tf&divId=170770"; slp

# selected division level (WI=170770 -> its "Boys" level node), state level (170771=boys),
# country level (168416=California state node probe) - see CAPTURES.md for the id map
curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings/list/170770" \
  -D "$OUT/atn-probe-divchildren-170771.headers" -o "$OUT/atn-probe-divchildren-170771.json" \
  "$API/SiteHeader/GetDivChildren?sport=tf&divId=170771"; slp
curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings/list/170770" \
  -D "$OUT/atn-probe-divchildren-168416.headers" -o "$OUT/atn-probe-divchildren-168416.json" \
  "$API/SiteHeader/GetDivChildren?sport=tf&divId=168416"; slp
curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings/list/170770" \
  -D "$OUT/atn-probe-divchildren-170305.headers" -o "$OUT/atn-probe-divchildren-170305.json" \
  "$API/SiteHeader/GetDivChildren?sport=tf&divId=170305"; slp
curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings/list/170770" \
  -D "$OUT/atn-probe-divchildren-168546.headers" -o "$OUT/atn-probe-divchildren-168546.json" \
  "$API/SiteHeader/GetDivChildren?sport=tf&divId=168546"; slp
curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings/list/170770" \
  -D "$OUT/atn-probe-divchildren-170772.headers" -o "$OUT/atn-probe-divchildren-170772.json" \
  "$API/SiteHeader/GetDivChildren?sport=tf&divId=170772"; slp

# anonymous rankings (proves the page param is ignored + blurAfterDepth masking)
curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings/list/170770" \
  -H "Content-Type: application/json" \
  -D "$OUT/atn-probe-getrankings-anon.headers" -o "$OUT/atn-probe-getrankings-anon.json" \
  --data '{"reportType":"div","mode":"list","divListId":170770,"indoor":null,"eventShort":"100m","gender":"m","qParams":{"grades":[],"page":1},"qualifyingListKey":"","version":2}' \
  "$API/tfRankings/GetRankings"; slp
curl -sS "${ua[@]}" "${hdr[@]}" -H "Referer: $BASE/TrackAndField/rankings/list/170770" \
  -H "Content-Type: application/json" \
  -D "$OUT/atn-probe-getrankings-anon-g11-p2.headers" -o "$OUT/atn-probe-getrankings-anon-g11-p2.json" \
  --data '{"reportType":"div","mode":"list","divListId":170770,"indoor":null,"eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":2},"qualifyingListKey":"","version":2}' \
  "$API/tfRankings/GetRankings"; slp

# SPA shell for the state events calendar (0 server-rendered meet links)
curl -sS "${ua[@]}" -H "Accept: text/html,application/xhtml+xml" \
  -D "$OUT/atn-probe-events-wisconsin-2026-5-26.headers" \
  -o "$OUT/atn-probe-events-wisconsin-2026-5-26.html" \
  "$BASE/events/usa/wisconsin/2026-5-26"; slp

# --- C. whole-meet lane (3 requests, browser-free; jwtMeet minted by step 1) ----------
python3 "$(dirname "$0")/probe-anon-meet.py"

echo "done -> $OUT"
