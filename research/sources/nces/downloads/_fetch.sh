#!/usr/bin/env bash
# NCES CCD/PSS fetcher: resolves the current school-directory ZIP through the CCD File API,
# downloads both national universes, extracts the CSV members, and verifies pinned digests.
# Evidence + spec live in ../SOURCE_REPORT.md. Authored 2026-10-09 and not executed in that session
# (no shell there); run it from the repository root on a machine with curl/unzip.
# The pinned table this asserts against is ../SHA256SUMS; a successful run rewrites SHA256SUMS
# inside the cycle directory ($OUT) from the bytes it actually downloaded.
set -euo pipefail

UA='census-service/0.1 (independent HS track & field research collector; polite; contact: repo owner)'
SPACING=${SPACING:-1.1}
YEAR_ID=${YEAR_ID:-40}
OUT=${OUT:-research/sources/nces/downloads/2025-26}
PSS_URL=${PSS_URL:-https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip}
CCD_URL=${CCD_URL:-https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip}
API_URL=${API_URL:-https://nces.ed.gov/ccd/datatables/api/File/2/0/${YEAR_ID}/0/0/0}
LOG="$OUT/CAPTURE_LOG.txt"

EXPECT_ROBOTS=${EXPECT_ROBOTS:-365e79b35eb96b9faea53b33dfa56996865622d43271482edf9b8928b8f6daf1}
EXPECT_PSS_HTML=${EXPECT_PSS_HTML:-f40b12720589ef67fcea32a380db786c486bd54a664755473f2aa61adeb18748}
EXPECT_PSS_ZIP=${EXPECT_PSS_ZIP:-b9c85d08071b9e85bca5678e56fc9d78da0af97774703e4f332775f58283f01f}
EXPECT_PSS_CSV=${EXPECT_PSS_CSV:-14a2f9e600a492940fd57646792b4b5163ea9d03b8015a7df1135066bcec3b8b}
EXPECT_CCD_ZIP=${EXPECT_CCD_ZIP:-b5d8dc341ecdc85549ffa9ba5accf4c4ed4f2a7c4a78a5115bbd281ef3e23bc8}
EXPECT_CCD_CSV=${EXPECT_CCD_CSV:-d1473136285b5994b73a1a8b640757811eb81e0ae770953bcf915ee8c422386e}
EXPECT_LAYOUT=${EXPECT_LAYOUT:-c2c28b0bc969c08d7b807bb6c0b242060851f4e549e74056ab77c5aa8b3967b2}
EXPECT_README=${EXPECT_README:-cd5160aac77b991f1ccda0d7ba9a2e17c26d3c8bb19fbdcd0f82702cf5b27d1a}
EXPECT_FRAME=${EXPECT_FRAME:-760b5de8180bb6a635671223ae9ec098259ef651b7083017b867325a27dca443}
EXPECT_FRAME_DICT=${EXPECT_FRAME_DICT:-2736e0961918a556e5a18f688359fc57622092f815a2cc8f25a3205d748ec4f3}

mkdir -p "$OUT"

note() { printf '%s\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >> "$LOG"; }

fetch() {
  local url=$1 file=$2 meta status ct effective bytes digest
  sleep "$SPACING"
  meta=$(curl --fail --silent --show-error --location --max-time 300 --user-agent "$UA" \
    --output "$OUT/$file" --write-out '%{http_code}\t%{content_type}\t%{url_effective}' "$url")
  IFS=$'\t' read -r status ct effective <<<"$meta"
  bytes=$(stat -c %s "$OUT/$file")
  digest=$(sha256sum "$OUT/$file" | cut -d' ' -f1)
  note "$file url=$url effective=$effective status=$status content_type=$ct bytes=$bytes sha256=$digest"
  printf '%s\t%s\t%s\t%s\t%s\n' "$file" "$status" "$bytes" "$digest" "$ct"
}

digest_of() { sha256sum "$OUT/$1" | cut -d' ' -f1; }

check() {
  local file=$1 expected=$2 actual
  actual=$(digest_of "$file")
  if [ "$actual" != "$expected" ]; then
    echo "digest mismatch: $file expected=$expected observed=$actual" >&2
    echo "the cycle may have rotated; re-pin deliberately in ../SOURCE_REPORT.md, never overwrite pins" >&2
    exit 1
  fi
  echo "pinned  $file $actual"
}

line_count() { wc -l < "$OUT/$1" | tr -d ' '; }
column_count() { head -1 "$OUT/$1" | tr ',' '\n' | wc -l | tr -d ' '; }

echo "== NCES national universes -> $OUT"
fetch https://nces.ed.gov/robots.txt robots.txt
fetch "$API_URL" ccd-file-api.json

echo "-- resolve the school-directory ZIP from the File API (advisory; a new cycle must be re-pinned)"
resolved=$( { grep -o 'https://nces\.ed\.gov/ccd/data/zip/[^"]*\.zip' "$OUT/ccd-file-api.json" || true; } | head -1 )
if [ -z "$resolved" ]; then
  echo "WARNING: no zip URL found in the File API body; falling back to the pinned CCD_URL" >&2
elif [ "$resolved" != "$CCD_URL" ]; then
  echo "WARNING: resolved ZIP differs from the pinned CCD_URL ($CCD_URL); pass CCD_URL=$resolved to fetch the new cycle" >&2
else
  echo "resolved $resolved (matches the pinned URL)"
fi

fetch "$CCD_URL" "$(basename "$CCD_URL")"
fetch https://nces.ed.gov/surveys/pss/pssdata.asp pssdata.html
fetch "$PSS_URL" "$(basename "$PSS_URL")"
fetch https://nces.ed.gov/surveys/pss/pdf/layout2023_24.pdf layout2023_24.pdf
fetch https://nces.ed.gov/surveys/pss/pdf/Readme2023_24.pdf Readme2023_24.pdf
fetch https://nces.ed.gov/surveys/pss/xls/2023-24_PSS_Frame_Data.csv 2023-24_PSS_Frame_Data.csv
fetch https://nces.ed.gov/surveys/pss/pdf/2023-24_PSS_Frame_File_Data_Dictionary.pdf 2023-24_PSS_Frame_File_Data_Dictionary.pdf

echo "-- extract members"
unzip -p "$OUT/$(basename "$CCD_URL")" "$(basename "$CCD_URL" .zip).csv" > "$OUT/$(basename "$CCD_URL" .zip).csv"
unzip -p "$OUT/$(basename "$PSS_URL")" pss2324_pu.csv > "$OUT/pss2324_pu.csv"

echo "-- verify pins"
check robots.txt "$EXPECT_ROBOTS"
check pssdata.html "$EXPECT_PSS_HTML"
check "$(basename "$PSS_URL")" "$EXPECT_PSS_ZIP"
check pss2324_pu.csv "$EXPECT_PSS_CSV"
check "$(basename "$CCD_URL")" "$EXPECT_CCD_ZIP"
check "$(basename "$CCD_URL" .zip).csv" "$EXPECT_CCD_CSV"
check layout2023_24.pdf "$EXPECT_LAYOUT"
check Readme2023_24.pdf "$EXPECT_README"
check 2023-24_PSS_Frame_Data.csv "$EXPECT_FRAME"
check 2023-24_PSS_Frame_File_Data_Dictionary.pdf "$EXPECT_FRAME_DICT"

echo "-- structure"
printf 'ccd csv lines=%s header_columns=%s\n' \
  "$(line_count "$(basename "$CCD_URL" .zip).csv")" "$(column_count "$(basename "$CCD_URL" .zip).csv")"
printf 'pss csv lines=%s header_columns=%s\n' "$(line_count pss2324_pu.csv)" "$(column_count pss2324_pu.csv)"

echo "-- required columns"
for column in NCESSCH SCH_NAME MSTREET1 MCITY MSTATE MZIP MZIP4 PHONE GSLO GSHI CHARTER_TEXT; do
  head -1 "$OUT/$(basename "$CCD_URL" .zip).csv" | tr ',' '\n' | grep -qx "$column" \
    || { echo "ccd csv lacks $column" >&2; exit 1; }
done
for column in PPIN PINST PADDRS PCITY PSTABB PZIP PZIP4 PPHONE NUMSTUDS; do
  head -1 "$OUT/pss2324_pu.csv" | tr ',' '\n' | grep -qx "$column" \
    || { echo "pss csv lacks $column" >&2; exit 1; }
done

( cd "$OUT" && sha256sum robots.txt pssdata.html "$(basename "$PSS_URL")" pss2324_pu.csv \
    "$(basename "$CCD_URL")" "$(basename "$CCD_URL" .zip).csv" layout2023_24.pdf Readme2023_24.pdf \
    2023-24_PSS_Frame_Data.csv 2023-24_PSS_Frame_File_Data_Dictionary.pdf > SHA256SUMS )

echo "-- done; captured rows, digests and provenance in $LOG"
