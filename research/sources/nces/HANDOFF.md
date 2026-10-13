# nces-ccd / nces-pss address lanes - exact operator handoff

Bead `athletic-rust-pipeline-7lx.5`. Route landed: **(b) exact documented operator handoff**.
Written 2026-10-12. Reader contract, provenance digest and the acquisition-fetcher spec live in
[SOURCE_REPORT.md](SOURCE_REPORT.md); this file is the operator entrypoint for a fresh run.

Why route (b) and not a new fetcher: `downloads/_fetch.sh` already *is* the repo-owned acquisition
helper (per-file sha256 pins, aborts on any mismatch), but the `school-address` lanes consume
**operator-supplied** files by design - there is no code path that discovers a capture path on its
own - and this session has no shell or network-bytes access, so a fetcher could neither be executed
nor first-run-verified here. The binding deliverable is therefore this handoff plus
`downloads/verify.sh`, which makes the documented contract executable and the refusal loud.

## 1. One-command preflight (refuses loudly when anything is absent)

```bash
bash research/sources/nces/downloads/verify.sh            # default cycle dir
bash research/sources/nces/downloads/verify.sh <CYCLE_DIR>
```

It checks that both member CSVs exist at the documented placement, that their bytes hash to the
pins in `downloads/SHA256SUMS`, and that the required reader columns are present. On any failure it
prints `REFUSED: ...`, points back to this file and exits 1. It never downloads and never repairs a
mismatch - a drifted capture is re-fetched and re-pinned deliberately, with a dated note here.

## 2. Acquisition: exact URLs, names, pins

HTTP status/members were observed on 2026-10-12 through the session reader (marked "live today");
no digest was recomputed today (no shell), so every sha256 below is a repo-held capture digest
transcribed from `downloads/SHA256SUMS`, `crates/census-crawl/tests/fixtures/nces/SOURCE.md` and the
probe manifests (see [SOURCE_REPORT.md](SOURCE_REPORT.md) section 2 for the per-artifact citations).

| # | Artifact | URL | Observed | Bytes | sha256 |
|---|---|---|---|---|---|
| 1 | CCD school-directory ZIP, cycle 2025-26 (primary) | `https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip` | live today: served as `application/x-zip-compressed`; members `ccd_sch_029_2526_w_0a_050626.csv (39.2MB)` and `.sas7bdat (44.0MB)`; ZIP entry dated 2026-07-01 | nominal 12.4 MB (API `size`) | `b5d8dc341ecdc85549ffa9ba5accf4c4ed4f2a7c4a78a5115bbd281ef3e23bc8` (fixture `SOURCE.md`) |
| 2 | CCD member CSV (lane input `--ccd`) | extracted from item 1 | member of item 1 | 41,054,983 (header + 102,102 data rows, 65 columns) | `d1473136285b5994b73a1a8b640757811eb81e0ae770953bcf915ee8c422386e` |
| 3 | PSS public-use ZIP, cycle 2023-24 | `https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip` | live today: served as `application/x-zip-compressed`; member `pss2324_pu.csv (37.7MB)` | 3,970,317 (capture 2026-10-05T02:04:45Z) | `b9c85d08071b9e85bca5678e56fc9d78da0af97774703e4f332775f58283f01f` |
| 4 | PSS member CSV (lane input `--pss`) | extracted from item 3 | member of item 3 | 39,521,631 (22,511 lines, 359 columns) | `14a2f9e600a492940fd57646792b4b5163ea9d03b8015a7df1135066bcec3b8b` |
| 5 | CCD File API resolver (cycle discovery, not a lane input) | `https://nces.ed.gov/ccd/datatables/api/Lookup/` then `https://nces.ed.gov/ccd/datatables/api/File/2/0/40/0/0/0` (year id 40) | live today: JSON body; `{"schoolYearShort":"2025","id":40,"name":"2025 - 2026"}`; School fileId 6463 `fileURL` = item 1 | 64,262 for the 2024-25 (year-39) capture | year-39 capture `e8bc27f118fc3a1edab013b1cb5342d8669c92ce59899ff2809b9693f185cc80` (`probes/247-nces-ccd-selector/manifest.json`, `probes/249-nces-ccd-companion/manifest.json`); the year-40 body was not hashed (regenerated per fetch, informational) |
| 6 | PSS index page (documents the current cycle) | `https://nces.ed.gov/surveys/pss/pssdata.asp` | live today as a page; latest public-use cycle stated 2023-24, next stated spring 2027 | 58,640 (2026-10-05 capture) | `f40b12720589ef67fcea32a380db786c486bd54a664755473f2aa61adeb18748` |
| 7 | PSS record layout | `https://nces.ed.gov/surveys/pss/pdf/layout2023_24.pdf` | 200 on 2026-10-05; not re-fetched today | 234,597 | `c2c28b0bc969c08d7b807bb6c0b242060851f4e549e74056ab77c5aa8b3967b2` |
| 8 | PSS public-use readme | `https://nces.ed.gov/surveys/pss/pdf/Readme2023_24.pdf` | 200 on 2026-10-05; not re-fetched today | 268,439 | `cd5160aac77b991f1ccda0d7ba9a2e17c26d3c8bb19fbdcd0f82702cf5b27d1a` |
| 9 | PSS frame CSV | `https://nces.ed.gov/surveys/pss/xls/2023-24_PSS_Frame_Data.csv` | 200 on 2026-10-05; not re-fetched today | 849,644 | `760b5de8180bb6a635671223ae9ec098259ef651b7083017b867325a27dca443` |
| 10 | PSS frame dictionary | `https://nces.ed.gov/surveys/pss/pdf/2023-24_PSS_Frame_File_Data_Dictionary.pdf` | 200 on 2026-10-05; not re-fetched today | 157,766 | `2736e0961918a556e5a18f688359fc57622092f815a2cc8f25a3205d748ec4f3` |

Items 1-5 were re-observed live today (URL reachable, archive member names match the pins); items
6-10 were not re-read today and their bytes/pins are the last observed state. A live re-read that
contradicts a pin means the source rotated: re-pin deliberately and record it here. The page capture
`pssdata.html` (item 6) is documentation only - no reader consumes it - so page drift triggers a
re-pin decision, not a lane failure.

Fallback cycle if a run must pin the previous year: `https://nces.ed.gov/ccd/Data/zip/ccd_sch_029_2425_w_1a_073025.zip`
was live today (`application/x-zip-compressed`, member `ccd_sch_029_2425_w_1a_073025.csv (39.1MB)`);
the repo-held capture is 13,352,819 bytes, sha256
`39326da788aa322353d20ceaf8ad4baed26272502cd05b066cf6c594988b21ab` (`probes/248-nces-ccd-zip/manifest.json`).
PSS has no newer alternative: 2023-24 (`pss2324_pu`) is still the latest public-use file as of today.

## 3. Acquisition commands

```bash
# from the repository root; downloads every pinned artifact and verifies each against its pin,
# aborting on any mismatch (it has never been executed in an agent session - first run is
# unverified; read it before trusting it)
bash research/sources/nces/downloads/_fetch.sh

# if you only take the two member CSVs by hand (unzip the member, not the ZIP):
unzip -p ccd_sch_029_2526_w_0a_050626.zip ccd_sch_029_2526_w_0a_050626.csv > <CYCLE_DIR>/ccd_sch_029_2526_w_0a_050626.csv
unzip -p pss2324_pu_csv.zip pss2324_pu.csv > <CYCLE_DIR>/pss2324_pu.csv
bash research/sources/nces/downloads/verify.sh <CYCLE_DIR>
```

## 4. Placement (there is nothing to discover - hand the paths to the flags)

| File | Placement | Tracked? |
|---|---|---|
| `ccd_sch_029_2526_w_0a_050626.csv` | `research/sources/nces/downloads/2025-26/` (the `_fetch.sh` default `OUT`) | no - scratch, gitignored |
| `pss2324_pu.csv` | same cycle dir | no - scratch, gitignored |
| optional ZIPs / docs | same cycle dir | no - scratch, gitignored |
| `_fetch.sh`, `verify.sh`, `SHA256SUMS`, `HANDOFF.md`, `address-manifest.json` | `research/sources/nces/downloads/` | yes |

The lanes take the exact path you pass: `--ccd <path>` and `--pss <path>`. The generator records the
text it actually parsed together with its sha256 (in the generation manifest), so pointing at the
scratch path is sound; still pass `--evidence-url`/`--evidence-date` at join time (section 6)
because those are what a claim cites. For the CCD member the honest URL is the ZIP URL of item 1,
since the CSV has no standalone public URL.

`address-manifest.json` in this directory is machine-readable but **unconsumed**: a repo-wide grep
on 2026-10-12 found no reader of that filename under `crates/`, `xtask/` or `tools/`. Do not treat it
as a contract; the contract is `SHA256SUMS` plus this file.

## 5. Licensing / attribution (observed 2026-10-12)

* **NCES Data Usage Agreement** - `https://nces.ed.gov/ncesglobal/data_usage_agreement.asp`
  (200 `text/html` today). The PSS download links carry a `DataUseAgrmnt` onclick attribute
  (`probes/253-nces-pss-index/FINDINGS.md`), so this page is the term presented at download time;
  direct anonymous GET of the ZIP served the bytes without a cookie or challenge. The agreement
  text: "Under law, public use data collected and distributed by the National Center for Education
  Statistics (NCES) may be used only for statistical purposes. Any effort to determine the identity
  of any reported case by public-use data users is prohibited by law. Violations are subject to
  Class E felony charges of a fine up to $250,000 and/or a prison term up to 5 years. [...] Therefore,
  users shall: Use the data in any dataset for statistical purposes only. Make no use of the identity
  of any person or institution discovered inadvertently, and advise NCES of any such discovery. Not
  link any dataset with individually identifiable data from other NCES or non-NCES datasets."
* For this pipeline: the CCD/PSS files are public-use statistical data; joining a school's own
  identity to its public athletics material stays within statistical use, but do not derive or
  record any person-level identity from them and do not merge in individually identifiable sources.
* **Attribution** - the PSS readme (item 8) carries the source line to quote:
  "SOURCE: U.S. Department of Education, National Center for Education Statistics, Private School
  Universe Survey (PSS), 2023-24." Use the cycle in any published table, e.g. "NCES CCD 2025-26
  public school universe" and "NCES PSS 2023-24 public-use file".
* No copyright statement or reuse license was observed on the fetched NCES pages; the binding term
  observed is the Data Usage Agreement. `https://ies.ed.gov/help/website-policies` returned HTTP 404
  today, so there is nothing to cite from it.

## 6. Wiring a fresh run (and how it fails loudly when inputs are absent)

```bash
# 1. preflight the operator captures (refuses loudly)
bash research/sources/nces/downloads/verify.sh

# 2. generate the address directory (--out is required)
census-service school-address \
  --ccd research/sources/nces/downloads/2025-26/ccd_sch_029_2526_w_0a_050626.csv \
  --pss research/sources/nces/downloads/2025-26/pss2324_pu.csv \
  --out var/school-address-<cycle>

# 3. inspect a generation without re-running (no --store)
census-service school-address-preflight --generation var/school-address-<cycle>

# 4. join as owned postal claims; --store is global and required, --apply commits
census-service school-address-join --store <store-dir> --generation var/school-address-<cycle> \
  --evidence-url nces-ccd=https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip \
  --evidence-url nces-pss=https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip \
  --evidence-date nces-ccd=2026-10-12 --evidence-date nces-pss=2026-10-12
# add --apply to append claims; without it the join is a dry run
```

(`--baseline`/`--ledger` additionally require `--now YYYY-MM`; `--geocode`/`--validate-postal` are
separate consent gates needing their own secrets. See [SOURCE_REPORT.md](SOURCE_REPORT.md) section
5.4.)

| Condition | Exact loud failure |
|---|---|
| preflight: member absent or hash mismatch | `REFUSED: ...` + handoff pointer + exit 1 (`downloads/verify.sh`) |
| binary: `--ccd`/`--pss` not passed | `no input artifact was named: pass at least one of --ccd, --pss, --state-ed-index, --state-ed-profile, --state-ed-tabular, --associations or --association-directory` (`school_address/read.rs:63-66`) |
| binary: named file absent | `Error: opening directory capture <path>` with cause `No such file or directory (os error 2)`; `main` returns `Result`, so the process exits 1 (`read.rs:186-187`, anyhow `{:?}` rendering) [code-read, not executed today] |
| ZIP handed to a lane | `reading directory capture <path>` with cause `stream did not contain valid UTF-8` (read-to-string of the ZIP bytes, `read.rs:201-203`); unzip the member (section 3) |
| truncated capture | `directory capture <path> stopped at row <line>: <error>; no corpus is published` (`read.rs:147-151`) or `directory capture <path> has no completed parser frontier; no corpus is published` (`read.rs:153-156`) |
| file over 128 MiB | `directory capture <path> is <n> bytes, over the 134217728-byte limit` (`read.rs:192-197`) |
| join without URL/date for the provider | `provider nces-ccd needs a capture URL and an observation date; pass --evidence-url nces-ccd=URL --evidence-date nces-ccd=YYYY-MM-DD, or name one capture as nces-ccd@<path>=... when the generation holds several` (`join/link.rs:61-65`); the claim lands in `evidence_missing` instead of an owned claim (`join/generation.rs:39-41`) |
| join without `--store` | `school-address-join requires an explicit --store and a stopped store owner` (`cli/school_address_join.rs:49-51`) |

## 7. Every URL tried today (2026-10-12) with the observed result

| URL | Result today | What it proves |
|---|---|---|
| `https://nces.ed.gov/ccd/datatables/api/Lookup/` | served JSON: `{"schoolYearShort":"2025","id":40,"name":"2025 - 2026"}` | cycle 2025-26 is year id 40; 2024-25 is id 39 |
| `https://nces.ed.gov/ccd/datatables/api/File/2/0/40/0/0/0` | served JSON: School fileId 6463, `fileURL` = item 1 | the pinned primary ZIP is still what 2025-26 resolves to |
| `https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip` | served `application/x-zip-compressed`; members `ccd_sch_029_2526_w_0a_050626.csv (39.2MB)`, `.sas7bdat (44.0MB)` | the primary capture is live and its member name matches the pin |
| `https://nces.ed.gov/surveys/pss/pssdata.asp` | 200 `text/html`; 2023-24 latest; 2025-26 stated "available in spring 2027"; download links match the pins | PSS primary capture is current; no newer PSS file exists to fetch |
| `https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip` | served `application/x-zip-compressed`; member `pss2324_pu.csv (37.7MB)` | the PSS capture is live and its member name matches the pin |
| `https://nces.ed.gov/ccd/Data/zip/ccd_sch_029_2425_w_1a_073025.zip` | served `application/x-zip-compressed`; member `ccd_sch_029_2425_w_1a_073025.csv (39.1MB)` | the 2024-25 fallback is live and matches the pinned fallback capture |
| `https://nces.ed.gov/ncesglobal/data_usage_agreement.asp` | 200 `text/html`; agreement text quoted in section 5 | the licensing term the downloads invoke |
| `https://ies.ed.gov/help/website-policies` | HTTP 404 | no IES policy page to cite; nothing licensed there |

Status/type/member observations above come from the session reader (proxied fetch), not from `curl`
with a repo user agent; the member listing is the reader's archive rendering of the actual bytes.

## 8. What remains unverified

* No sha256 was recomputed today: this session has no shell (no `bash`, `curl`, `sha256sum`, no job
  tooling). Every digest here is transcribed from repo-held evidence (`downloads/SHA256SUMS`, the
  fixture `SOURCE.md`, `probes/247`, `probes/248`, `probes/249` and `probes/253` manifests) and
  cross-checked against `SOURCE_REPORT.md`; none was checked against freshly downloaded bytes.
* `verify.sh` and `_fetch.sh` were authored/updated but never executed. Main should run
  `bash research/sources/nces/downloads/verify.sh` once (today it must print `REFUSED` and exit 1,
  because no captures exist in the working tree) and again after acquisition.
* `_fetch.sh`'s own run behavior (pins, abort-on-mismatch, page drift) is read-from-source, not
  observed. Its `pssdata.html` pin may legitimately drift between captures (the 2023-24
  documentation link moved to `ies.ed.gov` since the 2026-10-05 read); the page is not a lane input.
* The CCD member-line count (102,103) and column count (65) are from fixture evidence, not today's
  bytes; `verify.sh` therefore gates on sha256 plus required column names, not on line counts.
* `address-manifest.json` remains unconsumed (section 4).
* The flags, error strings and exit behavior in section 6 are code-read from
  `crates/census-service/src/...` on 2026-10-12 and were not executed (another worker owns that
  tree; no shell was available).
