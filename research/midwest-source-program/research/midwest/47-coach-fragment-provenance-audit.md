# Coach-fragment provenance audit (every shipped row re-fetched)

Scope: the coach-contact artifact `data/coach-contacts.csv` (source fragments in
`out/coach-fragments/` and `out/coach-fragments-ad/`) that feeds `CanonicalCoach`. This audit answers
one question per shipped row: **can the row's values be re-derived from its own cited page, right now,
from a plain client?** A row ships only if the answer is yes *and* the role label is corroborated.

Method: `tools/verify-coach-fragment.sh` (single fragment) driven by
`tools/verify-all-coach-fragments.sh` (all 32 fragment files: 16 in `out/coach-fragments/`,
16 in `out/coach-fragments-ad/`; eight concurrent workers over one shared HTTP cache, per-row
`curl --max-time 25` so a dead host costs a bounded 25–50 s and never a hang);
`tools/provenance-audit-table.py` turns the logs into the table below;
`tools/merge-verified-coaches.sh` rebuilds the importable CSV from the verified rows only;
`tools/merged-vs-verified.py` reconciles the published artifact against the verified subset
(it reports `merged rows with no verified counterpart`, which is the gate that matters).
Those five scripts were retired when the gate became the `census-service verify-coaches`
subcommand; *Second pass*, below, records the same re-derivation on the pipeline's own fetcher.

## What "verified" means here

1. **Value** — at least one of `coach_name`, coach email, `ad_name`, AD email appears literally in one
   of the row's own `source_url` pages. Fetch passes escalate: (1) plain GET of *every* cited URL,
   (2) the same URLs with XHR headers + `Referer`, (3) for NSAA `direxportscreen.php`, a POST of the
   school name (that endpoint answers nothing to a GET).
2. **Role verdict** — the claimed role must be *stated* next to the matched value, and a different
   role must not be. Exact rules in `tools/verify-coach-fragment.sh`:
   * positive statement window: 200 chars before .. 500 after the matched value, tokens
     `director|ADName|ADEmail|ADCell|ADPhone|"AD"` for a Director claim and
     `coach|CoachName|CoachEmail|"Coach"` for a coach claim → `ok`;
   * contradiction window (deliberately tighter, because a title column sits immediately next to the
     value while an abbreviation legend or a neighbouring staff entry does not): 100 before .. 300
     after, tokens `athletic director|ADName|ADEmail|\bAD\b|principal|superintendent|secretary|
     counselor|administrative|official representative|nurse|trainer` against a coach claim (and
     `coach|principal|…` against a Director claim) → `role_contradicted`, dropped, never relabelled;
   * value present, neither window fires → `ok_role_context` (the page is a coach/AD directory that
     lists the sport next to the name) → ships, with the role labelled contextual in this audit.
3. **Consumer mailboxes** never ship (contract rule enforced in the store's read path; see the
   withholding audit below).

## Classification of every row

| fragment | rows | verified | role conveyed by page context | role contradicted | render-required | mismatch | empty | shipped share |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `coach-fragments/IA.csv` | 93 | 68 | 16 | 0 | 4 | 0 | 5 | 90.3 % |
| `coach-fragments-ad/IA.csv` | 63 | 53 | 0 | 7 | 3 | 0 | 0 | 84.1 % |
| `coach-fragments/IL.csv` | 197 | 170 | 0 | 27 | 0 | 0 | 0 | 86.3 % |
| `coach-fragments-ad/IL.csv` | 16 | 13 | 0 | 3 | 0 | 0 | 0 | 81.2 % |
| `coach-fragments/IN.csv` | 130 | 101 | 15 | 9 | 5 | 0 | 0 | 89.2 % |
| `coach-fragments/KS.csv` | 37 | 9 | 22 | 0 | 0 | 0 | 6 | 83.8 % |
| `coach-fragments-ad/KS.csv` | 523 | 523 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments/MI.csv` | 106 | 82 | 11 | 12 | 1 | 0 | 0 | 87.7 % |
| `coach-fragments-ad/MI.csv` | 14 | 14 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments/MN.csv` | 1176 | 1174 | 0 | 0 | 0 | 2 | 0 | 99.8 % |
| `coach-fragments-ad/MN.csv` | 7 | 7 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments/MO.csv` | 61 | 54 | 6 | 0 | 1 | 0 | 0 | 98.4 % |
| `coach-fragments/ND.csv` | 199 | 134 | 0 | 62 | 3 | 0 | 0 | 67.3 % |
| `coach-fragments-ad/ND.csv` | 39 | 39 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments/NE.csv` | 209 | 70 | 16 | 92 | 31 | 0 | 0 | 41.1 % |
| `coach-fragments-ad/NE.csv` | 16 | 2 | 1 | 13 | 0 | 0 | 0 | 18.8 % |
| `coach-fragments/OH.csv` | 190 | 92 | 76 | 5 | 11 | 0 | 6 | 88.4 % |
| `coach-fragments-ad/OH.csv` | 31 | 13 | 18 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments-ad/SD.csv` | 30 | 26 | 0 | 4 | 0 | 0 | 0 | 86.7 % |
| `coach-fragments/WI.csv` | 706 | 706 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments-ad/WI.csv` | 19 | 17 | 2 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments/AZ.csv` | 158 | 139 | 0 | 10 | 8 | 1 | 0 | 88.0 % |
| `coach-fragments/CA.csv` | 193 | 152 | 0 | 41 | 0 | 0 | 0 | 78.8 % |
| `coach-fragments/CO.csv` | 200 | 200 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments/DC.csv` | 50 | 36 | 4 | 0 | 10 | 0 | 0 | 80.0 % |
| `coach-fragments/FL.csv` | 4 | 1 | 3 | 0 | 0 | 0 | 0 | 100.0 % |
| `coach-fragments/GA.csv` | 188 | 0 | 41 | 147 | 0 | 0 | 0 | 21.8 % |
| `coach-fragments/NJ.csv` | 86 | 37 | 4 | 0 | 45 | 0 | 0 | 47.7 % |
| `coach-fragments/OR.csv` | 196 | 193 | 0 | 2 | 1 | 0 | 0 | 98.5 % |
| `coach-fragments/TN.csv` | 183 | 153 | 0 | 0 | 20 | 1 | 9 | 83.6 % |
| `coach-fragments/TX.csv` | 72 | 45 | 17 | 5 | 5 | 0 | 0 | 86.1 % |
| `coach-fragments/UT.csv` | 200 | 176 | 0 | 24 | 0 | 0 | 0 | 88.0 % |
| **TOTAL** | **5392** | **4499** | 252 | 463 | 148 | 4 | 26 | **88.1 %** |

## Case studies (each one changed the method)

| Case | Symptom | Resolution |
|---|---|---|
| `api.ihsa.org` (IL) | 16-row slice went from 16/16 verified to 0/16 | The host 403s plain curl clients but answers the XHR-header pass; restored to 16/16 |
| IHSA staff JSON (IL) | Row claimed `Head Coach`; the cited page's own `DefaultTitle` for that person is `Asst. Boys Athletic Director` | Motivation for the *role-proximity* rule; contradicted rows are dropped (27 of 197 IL rows) |
| KSHSAA school JSON (KS) | 523 AD rows looked "unverifiable" under a prose-only role test — the page says `"ADName":"Derek Berns"` and never prints the word *director* | Role-coded field names (`ADName`/`ADEmail`/`ADCell`) count as the page stating the role; all 523 rows verify |
| NDHSAA school pages (ND) | 62 rows claimed `Head Coach` while the only matching value on the page is the *Athletic Director's* name (`Athletic Director: Logan Midthun`) | Contradiction verdict: dropped (a lane-level attribution bug, not a fetch problem) |
| `cifsshome.org` (CA) | Row values absent from the plain GET | Widget endpoint: the XHR pass returns the data (193/193 verified) |
| GA association PDFs | Rows cite printable PDFs, not HTML | Verifier inflates PDFs with `pdftotext` before matching |
| NSAA `direxportscreen.php` (NE) | 0 rows verified from a GET | POST `school=<name>`; 178/209 rows verified with the POST pass enabled |
| TVCS `Administrative Assistant` (FL) | Row shipped `role=Athletic Director` for a Central Office administrative assistant, with a name sliced from a sentence (`ministrative Assistant`) | Dropped by the role rule (the page never says "director"); the stale merged copy of this row disappears on re-merge from current fragments |
| CRLF fragments | A first sweep version counted ~50 % phantom rows (`school=""`, `city="ok"`) and the merge then rejected 1,643 rows as `missing school` | Fragment files are CRLF; the verifier's `read` left a trailing `\r` on the 11th field, which CSV parsers read as a record break. Stripped in the verifier; the publish path was never affected (`data/coach-contacts.csv` had 0 stray CRs) |

## Consumer-mailbox withholding (measured)

Every value difference between the published artifact and its fragment inputs was checked
(`tools/merged-vs-verified.py` plus a value-level diff): **51 rows** differ, and in all 51 the only
difference is that one or two email values present in the inputs are absent from the published row.
All 90 withheld values are free-mail domains — gmail 58, hotmail 13, yahoo 8, aol 5, sbcglobal 3,
comcast 2, outlook 1 — i.e. the consumer-mailbox rule, not data loss. **Zero institutional addresses
were lost in de-duplication.**

The same diff surfaced **2 FL rows** in the published artifact that no current fragment contains
(`Dwyer HS` with `ad_name="dresses are public"`, `The Villages HS` with `ad_name="ventures Director"`):
they came from a fragment revision made after the merge. Re-merging from the current fragments removes
them, which is why the artifact is rebuilt from verified rows on every import.

## Store behaviour after the gate (measured)

`data/coach-contacts.csv` is what a consumer reads, and it is rebuilt from verified rows only. The
Fjall census store is separate and **append-only**: `census-service import-coaches` upserts entities
and their evidence, and no `retract`/`purge` subcommand exists (checked against
`census-service --help`), so a row that an earlier CSV revision imported stays observable in the
store after the gate excludes it. Measured from `out/coaches.jsonl`, the store's coach rows by
evidence source:

| source | rows |
|---|---|
| `ihsa` | 18,711 |
| `mshsl` | 12,439 |
| `coach_contacts_csv` | 5,846 |
| `wiaa_directory` | 2,550 |
| `nsaa` | 1,589 |
| `ndhsaa` | 922 |
| `ks` | 526 |
| **all sources (`coaches.jsonl`)** | **29,294** |

Consequences, stated plainly because they are easy to get wrong:

* `report.json` / the workbook's `Coaches` sheet (`coaches=30,244` in the current pass) are a
  **superset** of the published artifact. That number is *not* the count of verified research rows.
* The adapter-sourced rows (`ihsa`, `mshsl`, `wiaa_directory`, `nsaa`, `ndhsaa`, `ks`) are produced
  by the pipeline's own association adapters, not by the research fragments, and are outside this
  artifact audit.
* Re-importing the rebuilt CSV re-asserts the verified rows. It cannot delete the earlier revision's
  rows, so the store keeps a superset by construction.

## Aggregate result

One consistent pass, 2026-09-22 10:36–10:41, all 32 fragments, one verifier revision, one shared
cache, run by `tools/verify-all-coach-fragments.sh` (8 workers) against the frozen fragments whose
hashes are in `out/coach-freeze-manifest.txt` (the same file carries the per-fragment verdict lines
this table is generated from). Earlier mixed-revision passes are superseded; see *Revision history*.

| measure | value |
|---|---:|
| fragment files audited | 32 |
| fragment rows parsed by the verifier | 5,392 |
| `ok` (value + role stated next to it) | 4,499 |
| `ok_role_context` (value verified, role contextual) | 252 |
| **rows the verifier accepted (shipped share)** | **4,751 · 88.1 %** |
| `role_contradicted` (dropped, never relabelled) | 463 |
| `render_required` (JS shell, no row values in any fetched body) | 148 |
| `mismatch` (real content, values absent) | 4 |
| `empty` (no body at all) | 26 |
| distinct verified keys seen by the reconciler | 4,732 |
| published artifact rows after merge/dedupe (`data/coach-contacts.csv`) | 3,444 |
| rows rejected by the merge with a quoted reason | 40 |
| **published rows with no verified counterpart** | **0** |

Published artifact composition (recounted from the CSV, quote-safe `csv.DictReader`):

| measure | value |
|---|---:|
| rows | 3,444 |
| distinct schools | 1,180 |
| states | 23 |
| rows with a coach name | 2,348 |
| rows with a coach email (`public_professional_email`) | 1,407 |
| rows with an AD name | 2,929 |
| rows with an AD email | 2,317 |
| rows with any email | 2,758 |

The merge-before/after delta is the gate doing its job: the pre-gate merge read the same fragments
and published **4,063 rows**; verification removed the rows whose own cited page contradicts them
(463 contradicted + 148 unrenderable + 4 mismatched + 26 empty, minus rows that were already
duplicates of a verified key) and left 3,444.

## Second pass: the same gate on the pipeline's own fetcher

The table above gated the published artifact with `curl` — a browser user agent, no `robots.txt`. The
same re-derivation now runs inside the pipeline, so the audit is reproducible with the fetcher the
collectors themselves use. `census-service verify-coaches` (see *Reproduce*) writes, in one pass:

* one verified fragment per input, in the **input's own eleven-column shape** — `merge-coaches` reads
  that shape, so the per-row verdict moved to `--csv` instead of a twelfth column;
* `out/verified-union/<ST>.csv` — the per-state union `merge-coaches --fragments` consumes;
* `out/coach-freeze-rust.log` (one tally line per fragment), `out/coach-freeze-rust-manifest.txt`
  (timestamp, one `sha256` line per input fragment, then the verdict lines),
  `out/coach-gate-rust-table.md` (the table below), `out/coach-gate-rust-rows.csv` (one verdict per
  row) and the reconciliation against `data/coach-contacts.csv`.

### What the pipeline's fetcher changes

| | first pass (`curl`) | this pass (pipeline fetcher) |
|---|---|---|
| `robots.txt` | never read | fetched once per host, cached, honoured |
| patterns | — | RFC 9309: `*` and `$`, longest pattern decides, `Allow` wins ties |
| a host that refuses its `robots.txt` | walked anyway | closed for the run (a 401/403 is a refusal, not an absent file) |
| identity | a browser user agent | `census-service/0.1` |
| pacing | 8 workers, no per-host lane | one lane per host, its delay, 2 rps ceiling for authorized hosts |
| cache | a shared curl cache | the store's content-addressed HTTP cache (this pass: 15,945 cache hits, 14 requests) |

`net/robots.rs` had to be fixed before any of this meant anything: it matched literal prefixes, so
`Disallow: /*directory` (Bound) or `Disallow: /*/ctl/` (MHSAA) could only ever match a path beginning
with a literal `*` — nothing — while the host counted as unregulated. It now compiles each pattern to
an anchored regex (`*` → `.*`, an optional trailing `$` → end anchor) and takes the longest match;
wildcards, anchors, tie-breaking and all six patterns Bound publishes are covered by tests.

### Rows the pipeline's own fetcher will not re-derive

Measured over all **33** fragment files (15,072 rows): **14,275 shipped**, 9,086 distinct identities.
The 252 rows that could not be re-derived (1.7 %) each have a named, reproducible cause:

| host | rows | cause |
|---|---:|---|
| `officials.myohsaa.org` | 134 | answers the collector's user agent with 403 (curl's browser agent was served 200) |
| `www.gobound.com` | 93 | `robots.txt` is itself 403 to this client; when readable it carries `Disallow: /*directory` (IA 63, SD 29, +1) |
| `my.mhsaa.com` | 14 | `Disallow: /*/ctl/` in its `*` group — the wildcard the old matcher could not see |
| `senior.dbqschools.org`, `www.pleasval.org`, `hempstead.dbqschools.org` | 11 | 403 / transport failure for the collector's user agent |
| **total** | **252** | |

`--authorized-host <host>` exists for a host whose collection the operator commissioned (its refusals
are then recorded as `robots_authorized`, and requests proceed under the same ceilings);
`--user-agent` exists for a user-agent decision. Neither was used for this pass — the point was the
default policy's own verdict.

### Reconciliation against the published artifact

The published `data/coach-contacts.csv` (3,444 rows) has **241 rows with no verified counterpart**
in this pass: OH 128, IA 47, WI 34, SD 26, MI 4, IN 2. Only two of those groups are policy:

* **OH 128, SD 26, MI 4** — rows cite `officials.myohsaa.org` (user agent refused), Bound (closed)
  and `my.mhsaa.com` (wildcard rule) respectively.
* **IA 47** — 63 AD rows cite Bound directory paths; 11 more cite the three district hosts above.
* **WI 34** — not policy: the Wisconsin fragment was rebuilt from the WIAA's own coach table
  (`/Directory/School/CoachList`) *after* the publish, so 34 published identities (markup-split names
  such as `Donn<span>&nbsp;</span>Behnke`) no longer exist in the fragment the gate reads. The
  current WI fragment verifies 5,172 of 5,172 rows.

### What the gate's re-publish produced

`merge-coaches` over the union staged above keeps **6,215 rows from 22 states** (4,913 with any
email, 3,597 with a coach email, 1,058 AD rows), against 3,444 published. The difference is dominated
by Wisconsin (3,166 merged rows — the WIAA coach-table rebuild, 706 → 5,172 verified rows) and by
fragments added after the publish (TX). That merge was imported on 2026-09-22:
`census-service import-coaches out/coach-contacts.merged.csv` reported `rows=6261`, `with_email=4532`,
`schools=1642`, `rows_without_coach_role=7`, `errors=0`; `data/coach-contacts.csv` is now the merged
6,215-row file, and the runbook (`tools/run_pipeline.sh`) rebuilds the snapshots and data products on
top of it.

## Revision history of the published artifact

`data/coach-contacts.csv` is rebuilt on every import, so counts quoted in wave-stamped reports
(`research/midwest/29-…`, `30-…`, `synthesis/03-…`) describe the revision current at their wave:

| revision | rows | note |
|---|---:|---|
| a29 build (`tools/a29-coach/build_csv.py`) | 2,298 | original research CSV, 10 states |
| wave 6 merge (`tools/merge_coach_fragments.py`) | 2,480 | 16 lane fragments |
| wave 7 merge (`census-service merge-coaches`) | 4,063 | 23 states + AD slices |
| **wave 8 verified rebuild (this audit)** | **3,444** | every shipped row re-fetched, role corroborated, 0 unverified |
| wave 8 gate re-run on the pipeline fetcher (`census-service verify-coaches`) | 6,215 | re-derivation + re-publish: the union merge above, imported the same day |

Known open interaction (unchanged by this gate, tracked in `synthesis/03-adapter-ranking.md` Q6):
seven of the artifact's source hosts (`api.ihsa.org`, `kshsaa-api.kshsaa.org`, `www.mshsl.org`,
`ndhsaa.com`, `secure.nsaahome.org`, `myohsaa.org`, `schools.wiaawi.org`) are also covered by live
pipeline adapters, so those coaches are written into the store twice, from two evidence dates. The
gate makes the *research* rows trustworthy; it does not de-duplicate them against the adapters.

## Reproduce

```bash
cd ~/Downloads/midwest-tfxc-source-research

# 1. the gate: re-derive every fragment row from its own cited pages, on the pipeline's fetcher.
#    Writes one verified fragment per input, the per-state union, the freeze log + manifest, the
#    audit table, one verdict per row, and the reconciliation against the published artifact.
FRAGS=$( { ls out/coach-fragments/*.csv | grep -v WI.rebuilt.csv; ls out/coach-fragments-ad/*.csv; } | paste -sd, - )
~/src/ad-law-scrape/athletic-rust-pipeline/target/release/census-service verify-coaches \
  --fragments "$FRAGS" \
  --out out/verified --union out/verified-union \
  --log out/coach-freeze-rust.log --manifest out/coach-freeze-rust-manifest.txt \
  --report out/coach-gate-rust-table.md --csv out/coach-gate-rust-rows.csv \
  --reconcile data/coach-contacts.csv --jobs 32

# 2. the publish half. `merge-coaches` reads the union; nothing below writes the artifact until
#    `import-coaches` runs.
~/src/ad-law-scrape/athletic-rust-pipeline/target/release/census-service merge-coaches \
  --fragments out/verified-union --out out/coach-contacts.merged.csv \
  --report out/coach-contacts.merge-report.md
# then: import-coaches -> report / workbook / export-data
```

The gate is idempotent and safe to re-run: it recomputes every verdict from the fetched bytes (an
input `verify` column is ignored, never trusted) and shares one HTTP cache with the collectors, so a
re-run costs requests only where that cache is cold. The wave-8 shell/Python tooling
(`tools/verify-coach-fragment.sh`, `tools/verify-all-coach-fragments.sh`,
`tools/merge-verified-coaches.sh`, `tools/finalize-coach-wave.sh`, `tools/merged-vs-verified.py`,
`tools/provenance-audit-table.py`) is retired — the subcommand performs all five steps. The
first-pass log and manifest stay on disk as the record of the `curl` pass the table above came from:
`out/coach-freeze-manifest.log` and `out/coach-freeze-manifest.txt` (the Rust pass's own records are
the `-rust` files beside them).
