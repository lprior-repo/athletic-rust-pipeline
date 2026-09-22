#!/bin/sh
# Anonymous, sequential, <=1 req/s per host probe harness for state-assoc-mountain
# (AZ, CO, ID, MT, NM, UT, WY).
# usage: probe.sh <sample-name> <url> [referer]
set -u
LANE=$(cd "$(dirname "$0")" && pwd)
UA='ad-law-scrape-source-research/0.1 (anonymous; respectful 1rps; contact: repo owner)'
name=$1; url=$2; ref=${3:-}
out="$LANE/samples/$name"
ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
if [ -n "$ref" ]; then
  code=$(curl -sSL --compressed -o "$out" -w '%{http_code} %{url_effective}' --max-time 40 -A "$UA" -e "$ref" "$url" 2>"$out.curlerr")
  cmd="curl -sSL --compressed -A '$UA' -e '$ref' '$url' > samples/$name"
else
  code=$(curl -sSL --compressed -o "$out" -w '%{http_code} %{url_effective}' --max-time 40 -A "$UA" "$url" 2>"$out.curlerr")
  cmd="curl -sSL --compressed -A '$UA' '$url' > samples/$name"
fi
bytes=$(wc -c < "$out" | tr -d ' ')
printf '%s\t%s\t%s\t%s\t%s\n' "$ts" "$code" "$bytes" "$url" "$cmd" >> "$LANE/samples/.captures.tsv"
printf '%s %s bytes req=%s  %s\n' "$code" "$bytes" "$url" "$name"
sleep 1.2
