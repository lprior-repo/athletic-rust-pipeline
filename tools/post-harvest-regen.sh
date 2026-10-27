#!/bin/sh
# Post-harvest national regen. Run AFTER the detached milesplit_profiles
# worker exits (store must be free: pgrep 'census-service provider' empty).
# Rebuilds the binary first (picks up --address-corpus), then runs the
# blocked final sequence with the merged national-2 corpus and redelivers
# the Desktop workbook. Fails closed at the first error (set -e).
set -e
cd "$(dirname "$0")/.."
STORE=var/census-national-fresh
OUT=var/workbook-onesheet/national-final-1
CORPUS=var/school-address-national-2/current

pgrep -af 'census-service provider' | grep -v pgrep && {
  echo "harvest worker still running; store is locked"
  exit 1
} || true

env -u CI tools/moon-local run pipeline:build-native
BIN=$(ls -t ~/.cache/athletic-cohort-build-*/moon-native/x86_64-unknown-linux-gnu/release/census-service 2>/dev/null | head -1)
[ -n "$BIN" ] || { echo "no built census-service binary found"; exit 1; }
"$BIN" consolidate --store "$STORE"
"$BIN" bests --store "$STORE" --grad-year 2027
"$BIN" workbook --store "$STORE" --school-year 2026 --grad-year 2027 \
  --address-corpus "$CORPUS" --out "$OUT"
ls -la "$OUT"/workbook.xlsx "$OUT"/address-quarantine.csv
python3 -c "
import zipfile
names = zipfile.ZipFile('$OUT/workbook.xlsx').namelist()
print('sheets:', [n.split('/')[-1] for n in names if 'sharedStrings' in n or 'sheet' in n][:20])
"
cp "$OUT/workbook.xlsx" ~/Desktop/workbook.xlsx
ls -la ~/Desktop/workbook.xlsx
