#!/usr/bin/env bash
# Preflight for the state-ed address lanes: NYSED index/profile HTML captures and the
# operator-converted tabular shards that enrich strong CCD/PSS entries.
#
# Checks the capture against the reader contract documented in
# research/sources/state-ed/SOURCE_REPORT.md (section 3) and research/sources/state-ed/HANDOFF.md.
# Read-only: it never fetches and never rewrites a capture; every failure is loud (non-zero exit).
#
# Run from the repository root:
#   bash research/sources/state-ed/downloads/verify.sh index   <page.html>...
#   bash research/sources/state-ed/downloads/verify.sh profile <page.html>...
#   bash research/sources/state-ed/downloads/verify.sh tabular <shard.csv>...
set -uo pipefail

MAX_BYTES=8388608
MAX_ROWS=20000
MAX_COLUMNS=512
HANDOFF=${HANDOFF:-research/sources/state-ed/HANDOFF.md}

MODE=${1:-}
shift 2>/dev/null || true

case "$MODE" in
  index|profile|tabular) ;;
  *)
    printf 'usage: bash %s <index|profile|tabular> <file>...\n' "$0" >&2
    exit 2
    ;;
esac

if [ "$#" -eq 0 ]; then
  printf 'usage: bash %s <index|profile|tabular> <file>...\n' "$0" >&2
  exit 2
fi

failed=0
refuse() { printf 'REFUSED: %s\n' "$*" >&2; failed=1; }

check_index() {
  local file=$1 bytes=$2 ids
  ids=$(grep -o 'profile\.php?instid=[0-9]\{12\}' "$file" | sort -u | wc -l | tr -d ' ')
  if [ "$ids" -eq 0 ]; then
    refuse "$file has no profile.php?instid=<12 digits> link; the reader refuses an empty index as 'NYSED index has no published school rows'"
  else
    printf 'index   %s bytes=%s distinct_profile_ids=%s\n' "$file" "$bytes" "$ids"
  fi
}

check_profile() {
  local file=$1 bytes=$2
  if ! grep -q 'NYSED Data Site' "$file"; then
    refuse "$file lacks the 'NYSED Data Site' title marker; the reader refuses it as 'not a school profile'"
  elif ! grep -q 'INSTITUTION ID:' "$file"; then
    refuse "$file lacks 'INSTITUTION ID:'; the reader refuses it as 'not a school profile'"
  else
    printf 'profile %s bytes=%s markers=present\n' "$file" "$bytes"
  fi
}

check_tabular() {
  local file=$1 bytes=$2 header column columns rows
  # Mirror the reader's header normalization: strip BOM and carriage returns, split on commas,
  # trim surrounding whitespace, uppercase (crates/census-crawl/src/directory/artifact.rs).
  header=$(head -1 "$file" | tr -d '\357\273\277' | tr -d '\r' | tr ',' '\n' \
    | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | tr '[:lower:]' '[:upper:]')
  for column in NAME CITY STATE; do
    printf '%s\n' "$header" | grep -qx "$column" \
      || refuse "$file header lacks literal column $column (required; the reader uppercases trimmed header names)"
  done
  columns=$(printf '%s\n' "$header" | wc -l | tr -d ' ')
  if [ "$columns" -gt "$MAX_COLUMNS" ]; then
    refuse "$file has $columns columns, over the $MAX_COLUMNS cap"
  fi
  rows=$(wc -l < "$file" | tr -d ' ')
  if [ "$rows" -gt $((MAX_ROWS + 1)) ]; then
    refuse "$file has $rows physical lines, over the $MAX_ROWS data-row Page budget heading + 1"
  fi
  printf 'tabular %s bytes=%s columns=%s lines=%s\n' "$file" "$bytes" "$columns" "$rows"
}

for file in "$@"; do
  if [ ! -f "$file" ]; then
    refuse "$file is absent; capture set and placement: $HANDOFF"
    continue
  fi
  bytes=$(stat -c %s "$file")
  if [ "$bytes" -gt "$MAX_BYTES" ]; then
    refuse "$file is $bytes bytes, over the $MAX_BYTES-byte reader cap"
    continue
  fi
  case "$MODE" in
    index) check_index "$file" "$bytes" ;;
    profile) check_profile "$file" "$bytes" ;;
    tabular) check_tabular "$file" "$bytes" ;;
  esac
done

if [ "$failed" -ne 0 ]; then
  printf '\nREFUSED: at least one state-ed capture fails the documented handoff: %s\n' "$HANDOFF" >&2
  exit 1
fi
printf '\nVerified %s capture(s); wire them with the matching --state-ed-* flags (see %s).\n' "$MODE" "$HANDOFF"
