# state-ed address lanes - exact operator handoff

Bead `athletic-rust-pipeline-7lx.5`. Route landed: **(b) exact documented operator handoff**.
Written 2026-10-12. Reader contract, state survey and the capture spec live in
[SOURCE_REPORT.md](SOURCE_REPORT.md); fixtures and their provenance live under
`crates/census-crawl/tests/fixtures/state_ed/` and in the `state_ed` report
(`research/sources/state_ed/SOURCE_REPORT.md`). This file is the operator entrypoint for a fresh run.

Why route (b) and not a new fetcher: NYSED publishes **no bulk export** in the reader's shape (the
eight-state survey in [SOURCE_REPORT.md](SOURCE_REPORT.md) section 2 found no static
`NAME,CITY,STATE` CSV anywhere), and the reader's only state-ed-owned identity path is per-page HTML:
27 index pages plus one profile per school, which is a deliberate operator capture, not a single
download. This session has no shell, so a fetcher could not be executed or first-run-verified; the
binding deliverable is this handoff plus `downloads/verify.sh`.

## 1. Canonical routes (all re-checked live 2026-10-12)

| Purpose | URL | Observed today |
|---|---|---|
| Index, letter page | `https://data.nysed.gov/lists.php?type=school` (defaults to letter A) | 200 `text/html`, `<title>A - Schools \| NYSED Data Site</title>`, `profile.php?instid=800000054526`-shaped links |
| Index, non-default page | `https://data.nysed.gov/lists.php?start=66&type=school` | 200, `<title>B - Schools \| NYSED Data Site</title>` - the `start` mapping is live |
| Profile | `https://data.nysed.gov/profile.php?instid=800000038718` | 200, `<title>A A KINGSTON MIDDLE SCHOOL \| NYSED Data Site</title>`, `INSTITUTION ID: 800000038718`, PHONE, WEBSITE, BEDS CODE, enrollment |
| Legacy/inferred index path | `https://data.nysed.gov/analysis/` | **HTTP 404** - not an index route (see section 7) |
| Disclaimer | `https://data.nysed.gov/datadisclaimer.php` | 200, text quoted in section 5 |

The index enumerates letters with its own pagination: `lists.php?start=65..91&type=school`, i.e.
`A`=65 ... `Z`=90 and `0-9`=91, **27 pages** (recorded in the fixture
`crates/census-crawl/tests/fixtures/state_ed/index_letter_a.html:132-159`; live spot check
`start=66` returns the `B - Schools` title). Capture one page per `--state-ed-index` flag; each
lane accepts up to 20,000 rows.

## 2. Capture naming and placement

| Capture | Placement | Naming |
|---|---|---|
| index pages | `research/sources/state-ed/downloads/nysed/` | `index-<letter>.html` (e.g. `index-a.html`, `index-0-9.html`); one file per page, the id/name comes from the page itself |
| profiles | same directory | `profile-<instid>.html` (12-digit NYSED institution id, e.g. `profile-800000038718.html`) |
| digests | same directory | `SHA256SUMS` (`sha256sum *.html > SHA256SUMS`) |
| converted tabular shards | `research/sources/state-ed/downloads/<state>/` | one CSV per shard; keep the original workbook and a conversion log (command + workbook digest) beside it |

Raw captures are scratch (gitignored, see `.gitignore` in this directory); the scripts and this
document are tracked. Capture with the identifying UA and >=1s spacing (SOURCE_REPORT.md section
3.4), and record URL + acquisition date per capture - the join refuses to attribute a capture
without both (section 4).

## 3. One-command preflight (refuses loudly)

```bash
# from the repository root
bash research/sources/state-ed/downloads/verify.sh index   <page.html>...
bash research/sources/state-ed/downloads/verify.sh profile <page.html>...
bash research/sources/state-ed/downloads/verify.sh tabular <shard.csv>...
```

It checks the reader's contract: index pages carry at least one
`profile.php?instid=<12 digits>` link; profiles carry `NYSED Data Site` and `INSTITUTION ID:`;
tabular shards carry the literal `NAME`/`CITY`/`STATE` header (uppercased like the reader does),
<=512 columns, <=20,000 data rows, <=8 MiB. Any violation prints `REFUSED: ...`, points back to this
file and exits 1. It never fetches and never rewrites a capture.

## 4. Wiring a fresh run (and how it fails loudly when inputs are absent)

```bash
# 1. generate: repeat the flags per captured page/shard
census-service school-address \
  --state-ed-index  research/sources/state-ed/downloads/nysed/index-a.html \
  --state-ed-index  research/sources/state-ed/downloads/nysed/index-b.html \
  --state-ed-profile research/sources/state-ed/downloads/nysed/profile-800000038718.html \
  --out var/school-address-<cycle>

# 2. join as owned state claims; selector form is required when the generation holds several captures
census-service school-address-join --store <store-dir> --generation var/school-address-<cycle> \
  --evidence-url  'state-ed@<capture path>=https://data.nysed.gov/lists.php?start=65&type=school' \
  --evidence-date 'state-ed@<capture path>=2026-10-12'
# or, when the generation holds a single state-ed capture:
#   --evidence-url state-ed=https://... --evidence-date state-ed=YYYY-MM-DD
```

| Situation | Exact loud failure |
|---|---|
| preflight: page/shard absent or off-contract | `REFUSED: ...` + handoff pointer + exit 1 (`downloads/verify.sh`) |
| binary: no artifact flags | `no input artifact was named: pass at least one of --ccd, --pss, --state-ed-index, --state-ed-profile, --state-ed-tabular, --associations or --association-directory` (`read.rs:63-66`) |
| index page with no rows | `NYSED index has no published school rows` (`parse.rs:80-92`) |
| page is not a profile | `not a school profile` refusal (`parse.rs:177-192`) |
| XLSX/PDF/ZIP handed to a lane | `reading directory capture <path>` with cause `stream did not contain valid UTF-8` (`read.rs:201-203`) |
| tabular header lacks a literal `NAME`/`CITY`/`STATE` | `the state education agency directory file has no NAME column` -> lane aborts (`artifact.rs:46-54`, `read.rs:138-142`) |
| shard over 8 MiB / 20,000 rows | decoder byte cap (`artifact.rs:113`) or `directory CSV source rows` capacity error (`artifact.rs:160-165`, `budget.rs:9-28`) -> lane aborts |
| capture >8 MiB | body cap refusal (`parse.rs:165-175`) |
| truncated capture | `directory capture <path> stopped at row <n>: ...; no corpus is published` or `has no completed parser frontier` (`read.rs:147-156`) |
| join without URL/date | `school-address lane state-ed@<path> requires its published URL and actual acquisition date` (`join/generation.rs:39-41`) / `provider state-ed needs a capture URL and an observation date; pass --evidence-url state-ed=URL --evidence-date state-ed=YYYY-MM-DD, or name one capture as state-ed@<path>=...` (`join/link.rs:61-71`) -> `evidence_missing`, never fabricated |
| selector names a missing capture | `no capture in this generation matches "state-ed@<path>" ...` (`lanes.rs:148-155`) |

**Enrichment-only:** `--state-ed-tabular` yields `WeakKey` entries. It can enrich strong CCD/PSS
rows when name+city+state match exactly one strong entry, but it can never own a state-ed claim.
If owned state records are required, the index/profile path is the only admitted shape today
(SOURCE_REPORT.md section 1.2/3.4).

## 5. Licensing / attribution (observed 2026-10-12)

* **Data Disclaimer** - `https://data.nysed.gov/datadisclaimer.php` (200, live today): "Users of the
  data agree not to misuse, add to without permission, or misrepresent the data provided in any
  way." and "No person, entity, or user shall use the information in a manner that is in violation
  of any federal, state, or local law or regulation, including, but not limited to FERPA and New
  York State Education Law §2-d." NYSED also disclaims liability for inaccurate data.
* **Copyright line** (site footer, fixture `index_letter_a.html:11565`):
  "© Copyright New York State Education Department, all rights reserved." No reuse license is
  granted on the pages fetched; no license URL is offered. Treat the captures as
  attribution-and-accuracy-bound public records.
* **Attribution** to use in any published table: "New York State Education Department,
  data.nysed.gov (<capture URL>, captured <date>)". Keep the capture URL and date that the join
  records as evidence; do not present modified values as NYSED's own.
* Accuracy duty: the disclaimer asks users not to "add to without permission"; the pipeline's own
  address/coach data must therefore stay distinguishable from NYSED-sourced fields (the
  `state-ed` lane attribution does this when URL+date are supplied).

## 6. Every URL tried today (2026-10-12) with the observed result

| URL | Result today | What it proves |
|---|---|---|
| `https://data.nysed.gov/lists.php?type=school` | 200 `text/html`; title `A - Schools \| NYSED Data Site`; letter-A profile links | canonical index route is live and is the reader's shape |
| `https://data.nysed.gov/lists.php?start=66&type=school` | 200; title `B - Schools \| NYSED Data Site` | the `start=65..91` letter mapping is live, not just fixture-era |
| `https://data.nysed.gov/profile.php?instid=800000038718` | 200; title `A A KINGSTON MIDDLE SCHOOL \| NYSED Data Site`; `INSTITUTION ID: 800000038718`, PHONE, WEBSITE, BEDS CODE, enrollment | profile route is live and carries the reader's markers |
| `https://data.nysed.gov/analysis/` | **HTTP 404** | it is not the index route (alias resolved, section 7) |
| `https://data.nysed.gov/datadisclaimer.php` | 200; disclaimer text quoted in section 5 | licensing/attribution term |
| `https://data.nysed.gov/` | 200 `text/html`, `NYSED Data Site` | host liveness and title marker |

(These observations come from the session reader, not from `curl` with the repo UA; no bytes were
hashed today.)

## 7. Alias resolution recorded here (this answers `state_ed` open question 1)

`crates/census-crawl/tests/fixtures/state_ed/SOURCE.md:9` inferred the letter-A index path as
`https://data.nysed.gov/analysis/`, while the applicability prose cites
`data.nysed.gov/lists.php?type=school`. Today's evidence: `/analysis/` is **404**, and
`lists.php?type=school` returns exactly the title and link shape of the committed fixture. The
canonical index route is therefore `https://data.nysed.gov/lists.php?type=school` (letter pages via
`?start=65..91`); the `/analysis/` label in the fixture provenance is an inference that does not
resolve. `research/sources/state_ed/SOURCE_REPORT.md` open question 1 can be closed with this
result; the fixture provenance line itself is outside this handoff's ownership and was not edited.

## 8. What remains unverified

* No sha256 was recomputed today (no shell); the fixture digests cited in the `state_ed` report
  stand as recorded, and nothing here was checked against freshly downloaded bytes.
* `downloads/verify.sh` was authored but never executed; Main should run it against the committed
  fixtures as a negative (no captures in the scratch dir -> `REFUSED`, exit 1) and, later, against a
  real capture set.
* The full index crawl (27 pages, ~220 ids per letter slice) was not executed or enumerated here;
  the count and pagination come from the committed fixture and today's spot check.
* Profile address extraction reads a percent-decoded `maps/embed...q=` value (`parse/details.rs`);
  today's reader-shaped fetch of the profile did not display that embed, so the address path is
  report/fixture-derived, not re-observed against live bytes.
* The reader contract strings in section 4 are code-read on 2026-10-12; the `state-ed` report cites
  the same lines and was the earlier derivation. Nothing was executed.
