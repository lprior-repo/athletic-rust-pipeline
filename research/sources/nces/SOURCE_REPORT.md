# NCES CCD/PSS source report — provenance, reader mapping, fetcher spec

Scope: make the two national school universes the corpus is addressed from **repo-regenerable** and
bind their bytes into the `school-address` generation / `school-address-join` pipeline
(bead `athletic-rust-pipeline-7lx.5`). This report is evidence + spec; no production code was edited.

Written 2026-10-09 by agent `NcesStateEdEvidence`. Everything marked **[live 2026-10-09]** below was
fetched today with the read tool; everything else is a digest recorded by earlier captures and is
cited to the file that holds it. No local `sha256sum` was run (no shell in this session): digests are
quoted from repo-held evidence and cross-checked across independent documents.

---

## 1. Live reachability check (2026-10-09)

| # | URL fetched today | Content-Type observed | What the body showed |
|---|---|---|---|
| 1 | `https://nces.ed.gov/ccd/files.asp` | `text/html` | Angular shell only; no file rows in the delivered HTML (matches SRC-247) |
| 2 | `https://nces.ed.gov/ccd/datatables/api/Lookup/` | `text/html` (body is JSON) | `{"schoolYearShort":"2025","id":40,"name":"2025 - 2026"}` — **SchoolYearId 40 = 2025-26**; id 39 = 2024-25 |
| 3 | `https://nces.ed.gov/ccd/datatables/api/File/2/0/40/0/0/0` | `text/html` (body is JSON) | 2025-26 nonfiscal list: School-level Directory, versionId 52, **fileId 6463**, `fileURL = https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip`, description `Flat and Sas Files (12.4 MB)` |
| 4 | `https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip` | `application/x-zip-compressed` | archive members: `ccd_sch_029_2526_w_0a_050626.csv (39.2MB)`, `ccd_sch_029_2526_w_0a_050626.sas7bdat (44.0MB)` |
| 5 | `https://nces.ed.gov/surveys/pss/pssdata.asp` | `text/html` | latest downloadable cycle is **2023-24**; links `zip/pss2324_pu_csv.zip (3.8 MB Text File)`, `xls/2023-24_PSS_Frame_Data.csv (829 KB)`, `pdf/layout2023_24.pdf (229 KB)`, `pdf/codebook2023_24.pdf (1.5 MB)`, `pdf/Readme2023_24.pdf (262 KB)`; page says 2025-26 data "will be available in spring 2027" |
| 6 | `https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip` | `application/x-zip-compressed` | archive member: `pss2324_pu.csv (37.7MB)` |

The file list is client-rendered (1); the JSON API (2,3) is the deterministic resolver. Both ZIP URLs
named in the codebase README resolve today with the expected member names. `nces.ed.gov/robots.txt`
served 200 / 348 bytes / `sha256=365e79b35eb96b9faea53b33dfa56996865622d43271482edf9b8928b8f6daf1`
(SRC-253 manifest, 2026-10-05); per the owner directive the fetcher ignores the policy but keeps the
engineered spacing (see §5.1).

---

## 2. Provenance tables

### 2.1 CCD public-school universe, 2025-26 — the primary `nces-ccd` lane

| Item | Value | Source of record |
|---|---|---|
| Resolver API | `https://nces.ed.gov/ccd/datatables/api/File/2/0/40/0/0/0` (Fiscal=2 nonfiscal, year id 40) | [live 2026-10-09]; SRC-247 REPORT.md (used id 39 for 2024-25) |
| ZIP URL (pinned) | `https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip` | [live 2026-10-09]; README of `crates/census-crawl/src/nces/` |
| ZIP `sha256` | `b5d8dc341ecdc85549ffa9ba5accf4c4ed4f2a7c4a78a5115bbd281ef3e23bc8` | `crates/census-crawl/tests/fixtures/nces/SOURCE.md` |
| ZIP size | nominal 12.4 MB (API `size`); member decompresses to 41,054,983 B; ZIP entry dated 2026-07-01 | API [live]; SOURCE.md |
| Member | `ccd_sch_029_2526_w_0a_050626.csv` | [live archive listing]; SOURCE.md |
| Member `sha256` | `d1473136285b5994b73a1a8b640757811eb81e0ae770953bcf915ee8c422386e` | SOURCE.md; SRC-248 extract; `var/school-address-join-20261004/corpus-assoc/generations/c42d7d54f2c8cee6/manifest.json`; `TAP-REGISTRY.json:9930-9935` |
| Member bytes / lines / columns | 41,054,983 B; 102,103 lines (header + 102,102 data rows); 65 columns, no quoted commas (10 lines carry a literal `"` inside an unquoted field) | SOURCE.md; README (`crates/census-crawl/src/nces/`) |
| Companion workbook (optional) | `https://nces.ed.gov/ccd/xls/SY_2025-26_School_Directory_Companion_2026-022.xlsx` (73 KB, fileId 6464) | API [live] |
| Release notes / state notes / SPSS (optional) | `.../doc/SY_2025-26_Preliminary_Data_Release_CCD_Nonfiscal_Release_Notes.docx`; `.../xls/SY_2025-26_0a_Preliminary_Directory_Data_Notes.xlsx`; `.../data/txt/Code_to_create_SCH_Directory_File_2025-26_0a.txt` | API [live] |

### 2.2 PSS public-use file, 2023-24 — the primary `nces-pss` lane

| Item | Value | Source of record |
|---|---|---|
| Index page | `https://nces.ed.gov/surveys/pss/pssdata.asp` — 58,640 B, `sha256=f40b12720589ef67fcea32a380db786c486bd54a664755473f2aa61adeb18748` | SRC-253 manifest; [live 2026-10-09] |
| ZIP URL (pinned) | `https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip` | README; SRC-253 manifest; [live 2026-10-09] |
| ZIP bytes / `sha256` | 3,970,317 B / `b9c85d08071b9e85bca5678e56fc9d78da0af97774703e4f332775f58283f01f` | SRC-253 manifest (fetched 2026-10-05T02:04:45Z) |
| Member | `pss2324_pu.csv` — 39,521,631 B, `sha256=14a2f9e600a492940fd57646792b4b5163ea9d03b8015a7df1135066bcec3b8b`, 22,511 lines, 359 columns | SRC-253 derivation (asserted `file_size`, printed `header_mapping_exact True`); SOURCE.md; `HELD-INVENTORY.md:83`; generation lane |
| Record layout (defines all 359 columns) | `https://nces.ed.gov/surveys/pss/pdf/layout2023_24.pdf` — 234,597 B, `sha256=c2c28b0bc969c08d7b807bb6c0b242060851f4e549e74056ab77c5aa8b3967b2` | SRC-253 manifest |
| Public-use README | `https://nces.ed.gov/surveys/pss/pdf/Readme2023_24.pdf` — 268,439 B, `sha256=cd5160aac77b991f1ccda0d7ba9a2e17c26d3c8bb19fbdcd0f82702cf5b27d1a` | SRC-253 manifest |
| Frame file + dictionary (optional, ISR gating) | `https://nces.ed.gov/surveys/pss/xls/2023-24_PSS_Frame_Data.csv` 849,644 B `sha256=760b5de8180bb6a635671223ae9ec098259ef651b7083017b867325a27dca443`; `.../pdf/2023-24_PSS_Frame_File_Data_Dictionary.pdf` 157,766 B `sha256=2736e0961918a556e5a18f688359fc57622092f815a2cc8f25a3205d748ec4f3` | SRC-253 manifest |
| Derived, reusable | `probes/253-nces-pss-index/derived/layout.json` (95,126 B, `sha256=d276704c…6f665`) is the machine-readable 359-field layout; `derived/seed-slice.json` (80,465 B, `sha256=b3a1beba…3aa95`) is an RI seed slice. PSS pins exactly 22,510 interviewed cases (`ISR=1`), 6,809 noninterviews, 27,946 out-of-scope | SRC-253 FINDINGS.md |
| 2025-26 cycle | not yet published; page states spring 2027 | [live 2026-10-09]; SRC-253 FINDINGS.md |

### 2.3 Previous-cycle fallback (2024-25 school directory + supplemental ZIPs)

Held under SRC-248 with full URL/status/bytes/`sha256` in
`probes/248-nces-ccd-zip/manifest.json`; the school directory ZIP is
`https://nces.ed.gov/ccd/Data/zip/ccd_sch_029_2425_w_1a_073025.zip` (13,352,819 B,
`sha256=39326da788aa322353d20ceaf8ad4baed26272502cd05b066cf6c594988b21ab`). Use only if the 2025-26
URL rotates and the replacement has not been resolved yet; the lane must record which cycle it read.

### 2.4 Where the bytes currently sit (scratch, not repo)

| Artifact | Path on this workstation | Note |
|---|---|---|
| CCD 2025-26 CSV (decompressed) | `var/school-address-join-20261004/ccd/ccd_sch_029_2526_w_0a_050626.csv` | the lane path recorded in the preserved generation |
| CCD 2025-26 sas7bdat | `var/school-address-join-20261004/ccd/ccd_sch_029_2526_w_0a_050626.sas7bdat` | unused by the reader |
| PSS 2023-24 CSV | `/home/lewis/src/ad-law-scrape/data/nces/pss/pss2324_pu.csv` | lane path recorded as an absolute path outside the repo |
| PSS frame CSV | `/home/lewis/src/ad-law-scrape/data/nces/pss/2023-24_PSS_Frame_Data.csv` | optional |
| PSS data dictionary / docs | `/home/lewis/src/ad-law-scrape/data/nces/pss/2023-24_PSS_Data_Dictionary.pdf`, `2021-22_PSS_Documentation.pdf` | documentation only |

A fresh run should store its downloads under `research/sources/nces/downloads/<cycle>/` (scratch,
hash-named) and stage the decompressed member under the generation root rather than the home tree —
the join permits an absolute lane path, but in-repo staging keeps `--evidence-url`/capture selectors
readable. This session downloaded nothing to that tree (no shell): `downloads/` holds `_fetch.sh`
(the runnable fetcher, unexecuted here) and the pinned `SHA256SUMS` table.

---

## 3. What each reader expects (file:line)

CSV engine shared by both: comma-delimited, RFC 4180 quoting, strict UTF-8, header names trimmed,
BOM-stripped and ASCII-uppercased (`crates/census-crawl/src/directory/artifact.rs:18-40`); missing
required column → `CrawlError::Invariant { "the <source> file has no <NAME> column" }`
(`artifact.rs:46-54`), which `read.rs` re-wraps as `DirectoryArtifact` and aborts the lane
(`crates/census-service/src/school_address/read.rs:138-142`). Hard limits: 512 columns and 1 MiB per
record (`crates/census-crawl/src/directory/artifact/decoded.rs:4-5`).

### 3.1 `nces::parse_ccd` — `--ccd <decompressed csv>`

Required headers (`crates/census-crawl/src/nces/parse.rs:21-33`, `CCD_REQUIRED`):

```text
NCESSCH  SCH_NAME  MSTREET1  MCITY  MSTATE  MZIP  MZIP4  PHONE  GSLO  GSHI  CHARTER_TEXT
```

Consumed values beyond the required list:

| Value | Column(s) read | Cite |
|---|---|---|
| jurisdiction / state gate | `LSTATE`, falling back to `ST`, `MSTATE` | `parse.rs:86` + `:270-285` |
| kind | `CHARTER_TEXT == "yes"` (case-insensitive) → charter, else public | `parse.rs:103-107` |
| grades | `GSLO`, `GSHI` (`N`/`M`/empty = absent) | `parse.rs:111-115`; `directory.rs:180-198` |
| website (optional) | `WEBSITE` | `parse.rs:140` |
| enrollment (optional) | `ENROLLMENT` | `parse.rs:146` |
| address | physical `LSTREET1/LSTREET2/LSTREET3/LCITY/LSTATE/LZIP/LZIP4` preferred, mailing `M*` retained with a note when unselected | `crates/census-crawl/src/nces/address.rs` (`ccd`) |
| budget | NationalFile: ≤128 MiB text, ≤200,000 rows, 65,536→200,000 source-row cap | `crates/census-crawl/src/directory/budget.rs:9-28` |

Header of the real member (first line of `crates/census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv`)
contains all required names, plus `ST`, `LSTATE`, `WEBSITE`, `ENROLLMENT`, `LEVEL`, `SCH_TYPE*`.

### 3.2 `nces::parse_pss` — `--pss <csv>`

Required headers (`parse.rs:35-37`, `PSS_REQUIRED`):

```text
PPIN  PINST  PADDRS  PCITY  PSTABB  PZIP  PZIP4  PPHONE  NUMSTUDS
```

Consumed values beyond the required list: `LATITUDE24`/`LONGITUDE24` (optional coordinates,
`parse.rs:256-263`); state gate on `PSTABB` (`parse.rs:205-208`); address is `PADDRS/PCITY/PZIP/PZIP4`
as *mailing*, kind fixed `private` (`parse.rs:212-238`). The 359-column file is well under the 512
limit; quoted fields containing commas are handled by the engine.

### 3.3 Row accounting the fetcher must reproduce (proof the parse consumed everything)

| Lane | File rows | entries | skipped | Sum check |
|---|---|---|---|---|
| `nces-ccd` | 102,102 | 100,307 | 1,795 | 100,307 + 1,795 = 102,102 |
| `nces-pss` | 22,510 | 22,385 | 125 | 22,385 + 125 = 22,510 |

From the preserved generation `var/school-address-join-20261004/corpus-assoc/generations/c42d7d54f2c8cee6/manifest.json`
(same figures in `corpus-web` and `corpus` generations). A fresh run whose lane counts do not add up
to the published line/row counts has a truncated or reformatted artifact and must be rejected.

---

## 4. How the join consumes the lanes

* `school-address --out <root>` reads each artifact, hashes the **text it parsed**, and records
  `LaneReport {source, path, sha256, entries, skipped, notes, skipped_rows, note_rows, captured}`
  (`crates/census-service/src/school_address/read.rs:165-181`; struct at
  `crates/census-service/src/school_address/report.rs:35-46`). `captured` is the deduplicated,
  sorted set of `IdentifiedKey`s the lane actually carried (`read.rs:158-164`).
* Source tokens come from `ScheduleSource::label()`: `Ccd → "nces-ccd"`, `Pss → "nces-pss"`,
  `StateEducationAgency → "state-ed"` (`crates/census-domain/src/school_directory/ledger.rs:24-31`);
  the join admits exactly those plus `association:<slug>` (`join/lanes.rs:8-10`).
* Generation manifest: `run_id = digest(created_at, inputs)` and
  `generation_digest = digest(schema_revision, run, artifacts)` over canonical JSON
  (`manifest.rs:55` and `manifest.rs:78-84`, body at `:42-47`); the generation directory name must be
  the first 16 hex chars of the digest (`manifest.rs:163-169`).
* `school-address-join <root> --store <dir> [--out DIR] [--apply]` binds per-provider provenance:
  every lane needs `--evidence-url nces-ccd=… --evidence-date nces-ccd=YYYY-MM-DD` (same pair for
  `nces-pss`), or a per-capture selector `nces-ccd@<path>=…` when the generation holds several
  captures of a provider (`join/lanes.rs:104-157`; CLI help `cli/school_address_join.rs:30-41`).
  Defaults: report dir `<store>/out/school-address-join`; dry-run unless `--apply`.
* Claim evidence carries the lane's digest: `Evidence.note = "<token> lane <path> sha256=<capture>
  generation <digest>"` (`join/support.rs:126-142`); postal claims carry the capture sha as their
  attribution (`join/support.rs:163-179`).

### 4.1 Why the preserved generations fail today's binary (digest mismatch)

All three preserved roots (`corpus-assoc/…/c42d7d54f2c8cee6`, `corpus-web/…/5578b3f3670fc9b6`,
`corpus/…/57edd0b2c3c88b25`) store lanes with exactly these keys:

```text
entries  note_rows  notes  path  sha256  skipped  skipped_rows  source
```

Today's `LaneReport` has a ninth field, `captured` (`report.rs:44-45`), added with
`#[serde(default)]` and *no* `skip_serializing_if`. Decoding an old lane therefore yields
`captured: []`, and re-serializing it emits `"captured":[]` — bytes the stored digest was never
computed over. `Manifest::validate` recomputes `digest()` and compares run_id, and fails with
`GenerationError::Digest` ("generation manifest digest mismatch")
(`manifest.rs:86-94`); `verify_report` independently compares `digest(report.lanes)` against
`digest(manifest.run.inputs.lanes)` and fails with `GenerationError::Report` (`manifest.rs:202-220`).
`verification` of a preserved generation is therefore impossible with today's binary; a fresh run
must rebuild the generation (both lanes re-read from the same pinned bytes), after which
`school-address-join` can bind the same observations. [INFERENCE: the mismatch mechanism is read
from the code above; it was not executed in this session — reproduce with
`school-address-preflight <root>` or a dry-run `school-address-join`.] The preserved 2026-10-04
join report `var/…/corpus-assoc/out/school-address-join/report.json` already recorded
`manifest_digest=c42d7d54…` plus per-lane `capture_sha256` and the full counter block
(`scanned 207`, `already_linked 158`, `no_match 48`, `ambiguous 1`).

### 4.2 Counters a fresh run must be able to reproduce

`school-address-join` prints/records: `scanned`, `linked`, `already_linked`, `backfilled`,
`websites`, `review`, `no_match`, `refused`, `evidence_missing`, `missing_state`
(`cli/school_address_join.rs:69-80`), plus per-rule counters (`exact_name`, `core_name`,
`parenthetical`, `parenthetical_inner`, `alias`), `ambiguous`, `review_filed`, `review_present`
and the co-op counters (`join/apply.rs`, `join/support.rs`). `evidence_missing` is the counter that
fires when a school's directory key cannot be attributed to a lane with URL+date
(`join/link.rs:28-65`: `provider {} has no lane in this generation` /
`no capture of {} carries {} (…)` / `provider {} needs a capture URL and an observation date …`).
A fresh run with the two national lanes and real `--evidence-url/--evidence-date` pairs is the
prerequisite for those counters to mean anything; without it the join refuses to invent provenance.

---

## 5. Fetcher spec

### 5.1 Acquisition (all commands are what the operator/fetcher runs; nothing here was executed in this session)

```bash
UA='census-service/0.1 (independent HS track & field research collector; polite; contact: repo owner)'
ROOT=research/sources/nces/downloads/2025-26
mkdir -p "$ROOT"

# 1 — NCES policy: recorded, not enforced (owner directive), but never hammered.
sleep 1.1; curl -fsSL --max-time 120 -A "$UA" -o "$ROOT/robots.txt" https://nces.ed.gov/robots.txt

# 2 — resolve the current school-directory ZIP (do not trust the pinned name once a new cycle ships)
sleep 1.1; curl -fsSL --max-time 120 -A "$UA" \
  -o "$ROOT/ccd-file-api.json" https://nces.ed.gov/ccd/datatables/api/File/2/0/40/0/0/0
# expected: level=School, component=Directory, fileURL=.../ccd_sch_029_2526_w_0a_050626.zip

# 3 — the ZIP itself (spacing >= 1s between every request)
sleep 1.1; curl -fL --max-time 300 -A "$UA" \
  -o "$ROOT/ccd_sch_029_2526_w_0a_050626.zip" \
  https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip

# 4 — extract, never feed the ZIP to the verb
unzip -p "$ROOT/ccd_sch_029_2526_w_0a_050626.zip" ccd_sch_029_2526_w_0a_050626.csv \
  > "$ROOT/ccd_sch_029_2526_w_0a_050626.csv"

# 5 — PSS (latest cycle is 2023-24 until spring 2027)
sleep 1.1; curl -fsSL --max-time 120 -A "$UA" -o "$ROOT/pssdata.html" \
  https://nces.ed.gov/surveys/pss/pssdata.asp
sleep 1.1; curl -fL --max-time 300 -A "$UA" -o "$ROOT/pss2324_pu_csv.zip" \
  https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip
unzip -p "$ROOT/pss2324_pu_csv.zip" pss2324_pu.csv > "$ROOT/pss2324_pu.csv"
```

`research/sources/nces/downloads/_fetch.sh` in this tree implements exactly this order with the
pins below wired in as assertions (authored here, not executed in this session).

### 5.2 Verification rules (fail loudly; never "fix" a mismatch)

1. `sha256sum -c SHA256SUMS` — see `research/sources/nces/downloads/SHA256SUMS`. Expected pins:
   * CCD zip `b5d8dc34…e23bc8`; member `d1473136…22386e`
   * PSS zip `b9c85d08…83f01f`; member `14a2f9e6…ec3b8b`
   * `pssdata.html f40b1272…b18748`; layout `c2c28b0b…967b2` (optional docs likewise).
2. Structural checks on the members (cheap, catches reformatting even when a digest is refreshed):
   * CCD: 65 comma-separated fields in the header, 102,103 physical lines, header identical to the
     fixture head's first line; count "102,102 data rows".
   * PSS: 359 header fields, 22,511 physical lines, "22,510 data rows".
3. Column gates: every name in §3.1 / §3.2 present in the header after trim + uppercase.
   One-line check: `head -1 file.csv | tr ',' '\n' | tr -d '\r' | grep -x -c 'NCESSCH'` → 1.
4. Row-accounting gate after the generation build: `entries + skipped + notes` relationship per §3.3
   (entries+skipped must equal the published row count).
5. Sizes to expect: CCD member 41,054,983 B (±0 unless NCES republishes the cycle — a new cycle gets
   a new name and new pins, never an in-place overwrite); PSS member 39,521,631 B; PSS zip 3,970,317 B;
   CCD zip ~12.4 MB (API) / 11–13 MB observed.
6. On failure: stop, keep the downloaded bytes, and record the observed digest/bytes in this tree —
   a rotated artifact must be re-pinned with a *new* file name cycle, not silently accepted.

### 5.3 Storage paths

| Artifact | Path |
|---|---|
| downloads + hashes (scratch; gitignore-sized) | `research/sources/nces/downloads/<cycle>/…` + `SHA256SUMS` |
| decompressed member handed to `--ccd` / `--pss` | anywhere stable; the generation records the path it read |
| generation | `--out <root>`; lane report keeps `path` + `sha256` of the exact text |

### 5.4 Wiring into the pipeline

```bash
census-service school-address \
  --ccd  research/sources/nces/downloads/2025-26/ccd_sch_029_2526_w_0a_050626.csv \
  --pss  research/sources/nces/downloads/2025-26/pss2324_pu.csv \
  --out  var/school-address-<cycle> \
  [--baseline var/…/current/baseline.json --ledger var/…/current/update_ledger.json --now YYYY-MM]

census-service school-address-join var/school-address-<cycle> \
  --store <store dir> [--apply] [--out DIR] \
  --evidence-url  nces-ccd=https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip \
  --evidence-date nces-ccd=2026-10-09 \
  --evidence-url  nces-pss=https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip \
  --evidence-date nces-pss=2026-10-09
```

Rules the spec must respect (all enforced in code):

* `--now` is mandatory with `--baseline`/`--ledger` (`school_address/mod.rs:154-169`); one input
  artifact minimum, else `no input artifact was named …` (`read.rs:62-67`).
* `--out` must be fresh: existing `school_directory.*`/`manifest.json`… at the root, a non-directory
  `generations`, or a non-symlink `current` are refused (`preflight.rs:25-64`), and `current` is
  re-verified on every run.
* The lane file must be UTF-8 text ≤128 MiB (`read.rs:183-208`); a ZIP/XLSX fails read-to-string, and
  a truncated file fails the "completed parser frontier" check (`read.rs:152-156`).
* Evidence pairs are validated: scheme http/https (`lanes.rs:32-38`), date `YYYY-MM-DD` or RFC3339
  (`lanes.rs:87-102`); a selector naming a capture that does not exist errors with
  `no capture in this generation matches …` (`lanes.rs:148-155`).
* Every admitted lane needs URL **and** date or the join will not claim it
  (`join/generation.rs:39-41`; `join/link.rs:61-71`), and missing attribution lands in
  `evidence_missing` rather than in an owned claim.

### 5.5 Rotation policy

* **CCD**: a new cycle changes both the `SchoolYearId` and the ZIP name (`..._<yy><yy>_w_0a_<mmddyy>.zip`).
  Re-resolve through the File API (`Fiscal=2`, the new year id from `Lookup/`), then re-pin: new file
  name, new digests, new line counts. The 2024-25 pins in §2.3 are the fallback until then.
* **PSS**: the next public-use file (2025-26) is stated to arrive spring 2027; until then the 2023-24
  CSV is the latest cycle and `PSTABB`/`PPIN` semantics stay as documented by `layout2023_24.pdf`.

---

## 6. Limits of this report

* No shell here: the six fetches above prove reachability, content type and member names; they do not
  recompute digests. Every digest is quoted from repo-held evidence (`fixtures/nces/SOURCE.md`,
  `probes/253-nces-pss-index/manifest.json`, `probes/248-nces-ccd-zip/manifest.json`,
  `HELD-INVENTORY.md:81-85`, the generation manifest) and is consistent across those independent
  records. A first `_fetch.sh` run with network access closes that gap in one command.
* The PSS column list is quoted from the SRC-253 derivation's assertion
  (`variables 359 header_mapping_exact True`), not re-derived here.
* The digest-mismatch mechanism in §4.1 is read from code (`[INFERENCE]`, reproducible by running the
  binary against a preserved root); the *observed* failure is not re-executed in this session.
