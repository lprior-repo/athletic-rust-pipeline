#!/usr/bin/env bash
# Preflight for the nces-ccd and nces-pss address lanes.
#
# Verifies that the operator-supplied captures exist at the documented placement and match the
# sha256 pins recorded in this directory's SHA256SUMS, then prints the two member paths to hand to
# `census-service school-address`. It never downloads, never edits capture bytes and never repairs a
# mismatch: acquisition is _fetch.sh plus a deliberate, dated re-pin.
#
# Operator handoff (exact URLs, placement, acquisition, licensing, fail-loud contract):
#   research/sources/nces/HANDOFF.md
# Reader contract and provenance:
#   research/sources/nces/SOURCE_REPORT.md
#
# Run from the repository root:
#   bash research/sources/nces/downloads/verify.sh [CYCLE_DIR]
set -uo pipefail

CYCLE=${1:-research/sources/nces/downloads/2025-26}
PINS=${PINS:-research/sources/nces/downloads/SHA256SUMS}
HANDOFF=${HANDOFF:-research/sources/nces/HANDOFF.md}
FETCH=${FETCH:-research/sources/nces/downloads/_fetch.sh}

CCD_MEMBER=ccd_sch_029_2526_w_0a_050626.csv
PSS_MEMBER=pss2324_pu.csv
CCD_REQUIRED="NCESSCH SCH_NAME MSTREET1 MCITY MSTATE MZIP MZIP4 PHONE GSLO GSHI CHARTER_TEXT"
PSS_REQUIRED="PPIN PINST PADDRS PCITY PSTABB PZIP PZIP4 PPHONE NUMSTUDS"

failed=0
refuse() { printf 'REFUSED: %s\n' "$*" >&2; failed=1; }

if [ ! -f "$PINS" ]; then
  printf 'REFUSED: pin table %s is missing; the lanes cannot be verified\n' "$PINS" >&2
  exit 1
fi

pin_of() {
  awk -v f="$1" '$2==f {print $1; exit}' "$PINS"
}

# Mirror the reader's header normalization: strip BOM and carriage returns, split on commas, trim
# surrounding whitespace, uppercase (crates/census-crawl/src/directory/artifact.rs).
header_of() {
  head -1 "$1" | tr -d '\357\273\277' | tr -d '\r' | tr ',' '\n' \
    | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | tr '[:lower:]' '[:upper:]'
}

verify_member() {
  local file=$1 expected observed bytes
  if [ ! -f "$CYCLE/$file" ]; then
    refuse "$CYCLE/$file is absent; no capture exists for this run (acquire: bash $FETCH)"
    return 0
  fi
  expected=$(pin_of "$file")
  observed=$(sha256sum "$CYCLE/$file" | cut -d' ' -f1)
  bytes=$(stat -c %s "$CYCLE/$file")
  if [ -z "$expected" ]; then
    refuse "no sha256 pin for $file in $PINS"
    return 0
  fi
  if [ "$expected" != "$observed" ]; then
    refuse "$CYCLE/$file sha256=$observed bytes=$bytes does not match pinned sha256=$expected"
    return 0
  fi
  printf 'pinned  %s bytes=%s sha256=%s\n' "$file" "$bytes" "$observed"
}

verify_columns() {
  local file=$1 columns=$2 column header
  [ -f "$CYCLE/$file" ] || return 0
  header=$(header_of "$CYCLE/$file")
  for column in $columns; do
    printf '%s\n' "$header" | grep -qx "$column" \
      || refuse "$file header lacks required column $column"
  done
}

verify_member "$CCD_MEMBER"
verify_member "$PSS_MEMBER"
verify_columns "$CCD_MEMBER" "$CCD_REQUIRED"
verify_columns "$PSS_MEMBER" "$PSS_REQUIRED"

if [ "$failed" -ne 0 ]; then
  printf '\nREFUSED: the nces address lanes cannot run from %s.\n' "$CYCLE" >&2
  printf 'Operator handoff (URLs, pins, placement, acquisition, licensing, fail-loud contract): %s\n' "$HANDOFF" >&2
  exit 1
fi

printf '\nVerified. Hand these two members to the run:\n'
printf '  census-service school-address \\\n'
printf '    --ccd %s/%s \\\n' "$CYCLE" "$CCD_MEMBER"
printf '    --pss %s/%s \\\n' "$CYCLE" "$PSS_MEMBER"
printf '    --out var/school-address-<cycle> ...\n'
printf 'Bind provenance at join time with --evidence-url/--evidence-date (see %s).\n' "$HANDOFF"
