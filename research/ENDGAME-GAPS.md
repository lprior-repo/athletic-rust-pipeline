# Endgame gaps: what still blocks an exported national Class-of-2027 census

Read-only investigation, 2026-09-21. Every `file:line` below was read in this session; every
number was measured with the command shown beside it. Nothing here edits or builds the tree.

**Provenance.** `git rev-parse HEAD` = `3af713ba2a9d9422cbec2788e75a4009370c534c`, working tree dirty
with sibling edits in flight (`git status --porcelain` = 12 entries, including new
`crates/midwest-census/src/workbook/performances.rs`, `.../workbook/recruiting/`,
`.../report/coverage.rs`). Findings about those files describe the tree at the minute I read them;
line numbers in `src/cli/{gather,mod}.rs` and `src/report/mod.rs` are moving under sibling edits,
so re-`grep` the symbol before relying on a line.

**What is proven today (raw evidence).**

| Fact | Evidence |
|---|---|
| Fixture-transport runs accept rows: 6 accepted / 1 no-match / 1 review, per 8-row run | `jq -r '.report.resolution.status' ~/.local/share/athletic-rust-pipeline/lane-v14/out*/result.jsonl` in `out`, `out-girls`, `out-indoor`, `out-indoor-boys`, `out-pause` |
| Every live-source run accepts nothing: 8/8 `review_required` | same command in `out-live-boys`, `out-live-full-3`, `out-live-girls`, `out-live-outdoor-boys`, `out-live-outdoor-girls` |
| Live pages are rejected wholesale by the search envelope | issue tallies (`jq -r '.report.issues[]?' … \| sort \| uniq -c`) identical across the three newest live runs: 38 sport-mismatch, 17 non-canonical URL, 16 malformed page, 6 short page |
| The 120,716-row real workbook has only ever been answered by the synthetic search fixture: 0 accepted | `scale-v15/FINAL.json` → `coverage {selected: 5000, completed: 5000, deterministic: 0, local_review: 0, no_match: 1595, review_required: 3405}`; `jq` over `scale-2500.jsonl` (120,716 rows) → 115,716 `report: null`, 1,595 no-match, 3,405 review |
| Real identities appear in exactly one retained run (2026-09-16 pilot): 0 accepted of 33 assessed | `golden-pilot-100-v1-partial-c.jsonl` (mtime 2026-09-16 22:49): 32 review, 1 no-match, every assessed row `candidates: []` |
| Live runs since 2026-09-20 use synthetic people | `unzip -p lane-v14/live-input/athletic-grade11-boys-2026-indoor.xlsx xl/sharedStrings.xml` → "Avery Example", "100 Synthetic Avenue", "Exampleville", "North Example High School", `Fixture Extra` = "Preserve exact source cell 0/0: α & < > "" |
| The census walk works: 12 states, 6,737 rosters, 35,622 cached responses, 2.2 GB | `wc -l var/midwest-census/journal/milesplit_rosters_*.jsonl` = 6,737; `ls var/midwest-census/http \| wc -l` = 35,622; `du -sh` |
| Six live run directories are empty (submitted, no artifact) | `ls -A lane-v14/out-live-{entitled,full,full-2,rankings,rankings3,girls-rankings}` → 0 files |

---

## G1. The live identity search rejects every page it parses, so no row can be accepted from the real source

**Evidence.** Live runs emit, per row, `query issue invalid_athlete_row: result row belongs to
another or unspecified sport` (38 across 8 rows), `athlete URL is not canonical` (17),
`query failure MalformedResponse: search page contains malformed or unrelated result rows` (16),
`ended before the advertised result count` (6) — identical in
`lane-v14/out-live-{outdoor-boys,girls,full-3}/result.jsonl`. Code: a row that names an athlete but
carries no sport-matching profile link becomes an `invalid_athlete_row` issue
(`src/search/parser/stream/row.rs:97`), the profile link must end `/track-and-field` or
`/cross-country` (`src/search/parser/markup.rs:70-81`), a non-canonical URL is an error
(`markup.rs:67`), and **any** page issue rejects the whole page as malformed
(`src/search/progress.rs:161,185`; `:178` for a short page; `:95` for an over-bound advertised
count). Acceptance then needs a `2..=64` hard-eligible candidate set
(`src/runtime/row_worker/review.rs:43`), which the rejected page can never supply.
The team's own live probe already assumes the other behaviour: it POSTs
`{q, fq: "t:a a:tf", start: 0}` and skips any `<tr>` whose athlete link does not match
`href="/athlete/(\d+)/track-and-field"` (`~/.local/share/athletic-rust-pipeline/lane-v14/
live-capture/probe-candidates2.mjs`, written 2026-09-21 07:49; `probe-search-shape.mjs` prints the
raw body). So the live site does return rows the pipeline's rule turns into page-fatal issues —
the two disagree about the same bytes.
**Objective.** §46 D–F (bulk acquisition → discovery → cohort verification), §48 ("no
Acquiring → Complete"), §70 item 9 (identity-matched rows). **Smallest change.** Save one live
`POST https://www.athletic.net/Search.aspx/runSearch` response body (`config.toml:10` names the
endpoint; the probes above already issue the call) as a fixture and reconcile
`stream/row.rs`'s link rule with the live markup; separately decide whether a page that
legitimately contains other-sport rows should fail closed (today's `progress.rs` rule) or treat
them as skips — an owner contract decision, not a bug fix. **Proof.** `cargo run -p athletic-rust-pipeline -- start --input <real-8-row>.xlsx
--sha256 <digest> --per-sheet 8 --concurrency 8 --snapshot real-slice-v1 --output <out>.xlsx`
followed by `… status --run <digest>` → expect `coverage.deterministic >= 1` (today 0 on every
live run) and at least one `"status":"accepted"` in the JSONL sidecar. **Risk.** If the live pages
are challenge/login interstitials rather than real results, no parser change helps and the block
is session/browser-level; that distinction is exactly what the capture settles.

## G2. Real identities and the real source have never met at scale on the current revision

**Evidence.** Real identities appear only in the 2026-09-16 pilot (33 assessed rows of 120,716,
older parser revision, `golden-pilot-100-v1-partial-c.jsonl`); real scale appears only through the
fixture transport (`scale-v15/FINAL.json`; worker `0f044e59…` built from tree `9d101d9`,
HANDOFF.md:161). The newest live evidence (2026-09-20 15:46–20:41) is 8 synthetic rows per run.
The owner's own line agrees: "no live *collection* run — the pipeline's own `start` against the
real source — has executed, so every end-to-end lane run remains fixture-mode" (HANDOFF.md:182).
Measured counter-detail: those runs' rows *do* carry real Athletic.net candidate documents
(`out-live-outdoor-boys` row 2 lists 40 candidates with per-document evidence digests, e.g.
athlete 1269571 "Britany Axman" for `/athlete/FirstName` and `/athlete/LastName`), so real bytes
did reach the parser; whether by live transport or by replaying live captures is not determinable
from the artifacts (the lane also keeps `replay-body-out-live-outdoor-boys.json`, and HANDOFF.md:171
records the 5,000-row run `a8328b7f…` still at 1,631 decided rows when that note was written).
**Objective.** §46 F/I/J, §70 items 4 and 9, §47 (gap-driven second pass needs a measured
distribution). **Smallest change.** None in code: run 100–500 rows of `original-slate.xlsx`
(digest `5969df23…`) through the live chain once G1 lands, or replay one captured live search body
as the fixture's `/Search.aspx/runSearch` answer to get scale without live traffic. **Proof.**
Same `start`/`status` pair as G1 on a 100-row slice → `coverage.deterministic`, `no_match` and
`review_required` all non-zero and summing to the selection. **Risk.** Every published
"workbook-scale" number today describes the fixture's canned corpus, not Athletic.net: the
1,595/3,405 split says nothing about real matching yield, and any capacity plan built on it is
fiction.

## G3. No command and no durable owner runs the national census

**Evidence.** `NationalCensus` fans out per jurisdiction and folds reports
(`crates/midwest-census/src/restate_services/national.rs:158-200`), each jurisdiction runs exactly
three stages — `teams`, `rosters`, `consolidate`
(`restate_services/jurisdiction.rs:27-37`, runners at `jurisdiction/stages.rs:115,133,149`) — and
nothing invokes it: the CLI has no `national` command (`src/cli/mod.rs:55-112`), the service tests
cover only request/plan helpers (`restate_services/tests.rs:146-248`), and the e2e test asserts
service *advertisement*, not invocation (`tests/fjall_restate_e2e.rs:493`). The walk itself is
reachable only as a batch CLI (`src/cli/gather.rs:66-71,108-113`, `collect --all-states`), calling
the same code the workflow's stages wrap (`restate_services/jobs.rs:136,153`). The publishing
command `run` gathers an Athletic.net registry only (`src/cli/cycle.rs:15-66`). Measured cost of
the national walk: 12 states = 6,737 roster pages and 35,622 cached responses ⇒ ≈28.6k roster
pages for 51 jurisdictions (linear extrapolation, not measured). **Objective.** §7 (every unit of
work durably owned), §46 A–M, §48. **Smallest change.** One CLI subcommand that submits
`NationalCensus/run` through the ingress and reads back `NationalReport/run`, plus
`--all-states` on `run` so walk and publish are one operation. **Proof.**
`cargo run -p midwest-census -- national --season 2026 --revision national-v1` → prints a
`NationalReport` with 51 jurisdiction rows and any failures; today's nearest runnable probe is
`cargo run -p midwest-census -- collect --all-states --limit-per-state 1 --state-concurrency 8`
(51 live requests) → 51 lines, one per USPS code. **Risk.** A national walk that is not
durably owned cannot be resumed, journaled, or reported per state, and a per-state failure has no
home (§69).

## G4. §47 gap sweep and §48 completion lattice do not exist

**Evidence.** `grep -rn "CensusState\|SealedCensus"` over the whole repository returns nothing;
`grep -rn "seal\|gap"` over `crates/midwest-census/src/{census,report}` returns nothing (both run
in this session). `NationalCensus::run` completes when its fan-out drains
(`restate_services/national.rs:143-157`) — i.e. exactly the "HTTP queue empty" completion §48
forbids. **Objective.** §47 (athletes without performances / without graduation evidence, schools
without coach data, unnormalized events, conflicts, source failures), §48 (state lattice; no
direct `Acquiring → Complete`). **Smallest change.** A `CensusState` value on the jurisdiction
object's durable state with the gap classes as a re-plan stage; note a sibling lane is landing
§47 gap classes for the coverage report right now
(`crates/midwest-census/src/report/coverage.rs:1-30`, untracked, 1 minute old when read), so the
missing half is the *sealing* state, not the classification. **Proof.** After the change:
`cargo run -p midwest-census -- national …` yields per-jurisdiction `state` that is not
`Complete` while its `gaps` list is non-empty; today the observable is the grep above returning
zero matches. **Risk.** Without a lattice, any workbook built from the census crate is sealed on
"queue empty" — the exact failure §70's 14 acceptance items exist to prevent.

## G5. `review_required` is a dead end and the ambiguity review stage has never fired

**Evidence.** Every coverage summary in every retained run shows `local_review: 0`
(`scale-v15/FINAL.json`; live rows all `review_required`). The review path requires a `2..=64`
hard-eligible candidate set (`src/runtime/row_worker/review.rs:43`) and the accepted branch is
`map_or(RowResolution::ReviewRequired, …)` (`review.rs:82-86`). Once a row is `review_required`,
no artefact in the tree assigns it to a human or model, and the row is not retried by `status`
(its page is sealed). **Objective.** §46 J (review only unresolved candidates), §49
("review required" denominator). **Smallest change.** A review queue surface: emit the
`review_required` set (row identity + candidate ids + probe digests) as a sheet/CSV the operator
can adjudicate, and accept a decided verdict as an additive observation. **Proof.** After G1,
`cargo run -p athletic-rust-pipeline -- status --run <digest>` → `coverage.local_review > 0` and a
`Review` sheet listing case ids; today the same command parses to `local_review: 0` on every run
measured. **Risk.** If review never runs, the owner's second-best outcome (a canonical
"needs-adjudication" list with evidence) is unavailable and rows silently stay unresolved.

## G6. Throughput ceiling: ≈0.31 rows/s per lane ⇒ ≈4.5 days for one 120,716-row workbook

**Evidence.** HANDOFF.md:165 — 0.31 rows/s at `row_concurrency = 8` over 500+ rows, 0.27 rows/s at
32 (no gain from quadrupling concurrency), 0.9 rows/s for two lanes together; ≈50 durable
invocations and ≈10 source operations per row. HANDOFF.md:168 gives the instrumented window
(120.1 s, `tabs = 8`): 41 rows / 358 source operations = 0.341 rows/s, 8.73 ops per row, and
`BrowserSession/await_ready` entered exactly once per operation (358 gate calls to 358 operations;
14,267 lifetime calls against 14,265 fetches), with the handler's 0.329 s mean duration equal to
the observed operation period — the single-key exclusive gate *is* the rate. HANDOFF.md:169 records
the attempted fix: a shared-probe fast path (worker `fe95a735…`) stayed 1:1 (280/280) and *cut* the
lane to 0.167 rows/s, was reverted, and left its constraint in `src/runtime/source/dispatch.rs`.
HANDOFF.md:166-167 measure the rest of the cost as storage-engine churn — ≈12–14 MB of durable
writes per row, then corrected to ≈51.5 KB journal per decided row, 110 KB `restate-data` and
36 KB store per row. **Objective.** §46 (national: 120,716 source rows), §48 completion, §68
(effort and request cost in the ranking). **Smallest change.** Not a code guess: the exclusive gate
must answer without a physical act (the reverted probe's own conclusion) or the row's ~10 source
operations must be cut, which is a design change in `src/runtime/source/`. Measure first.
**Proof.** `cargo run -p athletic-rust-pipeline -- status --run <digest>` twice, 10 minutes apart,
at `--concurrency 8` and `--concurrency 32` → rows/min; today's expectation from the record is
"no gain from 8 → 32", and the fix's expectation is strictly higher rows/min at one setting.
**Risk.** At 0.31 rows/s the deliverable needs days of wall clock per pass, and §47's second pass
doubles it; a national census on this path is not reproducible in a working day.

## G7. Publication is not part of any run's durable status

**Evidence.** `RunProgress` carries `coverage`, `pages`, `pending_rows`, `complete`, `summary`,
`collection_ref` — and no export/publication field (`src/runtime/run_protocol.rs:129-139`);
`status` prints exactly that object (`src/cli/status.rs:18-28`). Publication is a separate
control chain (`PipelineControl::run_and_export`, `src/runtime/control.rs:100-124`) that returns
`PublishedExport` to its *caller* and records nothing into the run. Six live run directories are
empty while their submissions were accepted (`lane-v14/out-live-{entitled,full,full-2,rankings,
rankings3,girls-rankings}`; `out-live-roster-slice/start.log` shows `state: "submitted"`).
**Objective.** §70 item 13 (export verified before sealing), §44 (observability). **Smallest
change.** Have `run_and_export` (or the export worker) write the publication result — destination
path, output digest, verification verdict — into the run's durable state, and surface it from
`status`. **Proof.** `cargo run -p athletic-rust-pipeline -- status --run <digest>` on an
export-failed run → prints a non-`Complete` publication field naming the failure; today the output
has no such key (compare its key set to `run_protocol.rs:129-139`). **Risk.** An operator cannot
tell "delivered" from "still running but nothing will be published", which is precisely the state
the six empty directories are in.

## G8. The census report omits jurisdictions it has no data for, instead of reporting zeros

**Evidence.** `Census::by_state` (`src/report/mod.rs:143`) is built only from observed rows:
athlete states come from `state_entry(&mut rollup.by_state, &state)` per athlete
(`src/report/projection.rs:33-34`), coach states from `apply_coach_states` (`:68-76`), and school
counts are attached only to states that already have a row (`:105-110`). A jurisdiction with
schools but no athletes or coaches therefore publishes nothing, and a state with no observations at
all is silently absent from `report.json`, the "By state" sheets and the per-state CSVs.
**Objective.** §49 ("Report exact denominators where known"; emptiness is a finding), §46 B
(school universe), §70 item 11. **Smallest change.** Seed `by_state` with a default row for every
`UsJurisdiction::ALL` entry (plus the existing `UNKNOWN` bucket) before the rollups run. **Proof.**
`cargo run -p midwest-census -- --data-dir <store> report --print` over a one-state store → expect
51 USPS keys plus `UNKNOWN` (today: only the states present). **Risk.** An omitted state reads as
"not covered" when it may mean "covered, empty"; §49's whole purpose is distinguishing those.

## G9. The census workbook has no athlete/PR/coach surface yet

**Evidence.** The workbook's asserted sheet set is nine aggregate sheets — "Goal & method",
"Summary", "By state - core", "By state - all sources", "Athletic.net marginal", "Best results",
"Meets", "Evidence mix", "Method notes" (`crates/midwest-census/src/workbook/tests.rs:97-107`).
There is no `Athletes`, `Performances`, `PRs`, `Coaches` or per-jurisdiction sheet, i.e. none of
the objective's §50–§53 recruiter-facing tables; the in-flight frozen signatures
(`workbook/performances.rs`, `workbook/meta.rs`, wired by peer `WbRecruiting`) are the first of
them. **Objective.** §49–§54 columns and workbook surfaces, §22 (performances), §50 (athlete
roster). **Smallest change.** Land the in-flight sheet modules and extend the crate's sheet-list
test with the new names (the test is the contract; it fails until the sheets exist). **Proof.**
`cargo test -p midwest-census workbook::tests` → the sheet-name assertion list contains the new
names, and `unzip -p <out>.xlsx xl/workbook.xml \| grep -o 'name="[^"]*"'` lists them in the built
file. **Risk.** Without them the census crate's export cannot serve the stated product (a
recruiter-facing nationwide roster), only its own aggregate summary.

## G10. Capability coverage is national for discovery only: 1 state of bulk results, 7 of coach directories

**Evidence.** The registry (`crates/midwest-census/src/sources/registry/table.rs:26-267`) declares
`bulk_results` for one adapter (`wiaa_results`, Wisconsin only), `meet_discovery` for three
(`athleticlive` — an import artifact — `wiaa_results`, `wayzata`), `coach_directory` for seven
jurisdictions (IHSA IL, KSHSAA KS, MSHSL MN, OHSAA OH, WIAA WI, NDHSAA ND / NSAA NE; the last two
names-only), `athlete_profile` for Athletic.net with operator-supplied ids because its discovery
endpoint is robots-disallowed (same file, athleticnet entry), and `athlete_discovery` for MileSplit
(all states) plus AthleticLIVE. So §46 C/D/H and §49's meet and coach denominators are bounded by
one to seven jurisdictions outside MileSplit, while §46 E is national only through roster parses.
**Objective.** §46 C/D/H, §5 (coach contacts), §49 (meets/coaches denominators), §68 (priority by
new coverage per request). **Smallest change.** Adapters, in the measured order the sibling
ranking lane is already executing; the missing *census-side* piece is a per-jurisdiction plan that
runs the applicable registry rows per state (see G3). **Proof.** `cargo test -p midwest-census
sources::registry` → 13 slugs with their capability declarations; per-adapter live probe
`cargo run -p midwest-census -- provider mshsl --states MN` → report with `with_email > 0`.
**Risk.** A "nationwide" claim rests on MileSplit alone: no meet inventory and no coach coverage
for 44 jurisdictions, which is exactly the §49 column set the owner reads first.

---

## Shortest path, and the one gap that blocks the most

The census crate can already be driven to a workbook for every state it has walked — `collect
--all-states` (≈28.6k roster pages extrapolated from the measured 12-state walk) then `run` — so
a *census* workbook is days, not weeks, away once G3 (one national command), G8 (zero rows for
empty states) and G9 (athlete/PR/coach sheets) land. The owner's actual product is the
**identity-matched** workbook, and that chain cannot produce an accepted row at all today: G1
makes every live search page fail its own envelope, so the `2..=64` hard-eligible precondition is
never met, `review_required` absorbs 100% of live rows (8/8 in five runs, 32/33 in the only
real-identity run), and nothing downstream — profiles for accepted identities, §46 G/H enrichment,
§46 J review, §47 second pass, §48 sealing, §70 acceptance — has a live client. So the shortest
path is: capture one live search body and settle the page contract (G1) → run 8 real rows live
(G2's smallest form) → if rows accept, scale the selection under the measured ~0.31 rows/s ceiling
(G6) while the census-side national command (G3) proceeds in parallel to deliver the non-identity
half. **G1 blocks the most other work**: it is upstream of acceptance, of the expanded roster's
profile stage (SCOPE.md:32-35 states the same conclusion independently), of every §49/§70
denominator that counts matched athletes, and of any meaningful throughput measurement.

## Unverified (and what would settle it)

- **Why the live pages fail their envelope.** I read the rejection reasons and the code path, not
  the live HTML: the retained artifacts hold *candidate bio* documents, and the rows' search
  evidence is retained as digests (`report.query_evidence` is `[]`). One captured `runSearch`
  response body settles mapper-vs-markup, and whether the pages are interstitials.
- **Whether any national workflow run ever executed against a live Restate server.** No CLI, test
  or script invokes it and the retained journals under `var/midwest-census/journal/` are produced
  by the CLI walks (`src/cli/gather.rs:108-113`), but both paths share the same store, so the
  journals cannot separate them. An ingress-call log or the object's durable state would.
- **SCOPE.md:32 says `profile_artifacts`/`performance_evidence` "have stayed empty"; the retained
  pilot contradicts that for its own run** — 2,263 profile artifacts and 46,524 performance
  evidence entries across 33 rows (`jq -s` over `golden-pilot-100-v1-partial-c.jsonl`), and one
  row of `out-live-girls` carries 1 profile with 21 performance evidences. The doc sentence is
  probably about post-split runs; as written it is false of retained evidence.
- **The 51-jurisdiction walk cost is an extrapolation** (12 states measured, linear scaling
  assumed); per-state host pacing, robots `Crawl-delay` and page counts differ.
