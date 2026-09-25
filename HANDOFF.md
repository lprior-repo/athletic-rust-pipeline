# Current design handoff

**Superseded 2026-09-25:** `docs/OPERATIONS.md` and `docs/VERIFICATION-EVIDENCE.md` own this subject now. Kept as the dated record of the 2026-09-21 live-chain handoff for the pre-census pipeline.

**Status: the live end-to-end chain is executed for the indoor boys, outdoor boys, and outdoor girls scopes; workbook-scale identity-matched delivery and the expanded roster are pending.** This handoff records the current source contracts and the exact boundary between available evidence and unexecuted work. Automated collection -> deterministic matching -> owner-online export -> stopped-writer verification -> cached replay has completed against the private fixture transport for the outdoor boys, indoor girls, and indoor boys divisions, and against the **real source** for indoor boys: run `57f88b34…` acquired 95/95 events terminal / 1,065 pages / `final_snapshot 40792b83…`, decided 8/8 workbook rows, published `lane-v14/out-live-full-3/result.xlsx` (`a4eb1fe7…`), verified against the stopped writer (`exit 0`, source SHA-256 matching before and after), and replayed byte-identically with zero new source requests. It still does not claim identity-matched delivery at workbook scale or expanded-roster coverage: the 8-row sample decisions are all `review_required` by design, and the girls indoor half of the expanded roster has since completed its own live chain (run `fe726b41…`: 94/94 events, 8/8 rows decided, publication, stopped-writer verification exit 0, byte-identical cached replay) without delivering expanded-roster coverage.

The **outdoor boys live chain completed 2026-09-21** on run `7be7e9dd68cd5dc51c0c03edf05b5503297eec772756350870a04a1ebd95e0b3` (list `168416`, workbook `0a1d53f1…`, `live-store`), the run left pending in the previous section: collection `phase: Complete` at 95/95 events / 385 pages / `final_snapshot ee97f6f13f6317f705dffb703e19115dd7eb7b72dc728877b6e9ff060b7138ea`; row phase decided 8/8 rows (all 8 `review_required`, 0 pending) and `run_and_export` published `lane-v14/out-live-outdoor-boys/result.xlsx` (`3ee370e1…`, 8,342 B), `result.jsonl` (`9060e7c0…`, 6,548,556 B), and `result.commit.json` (`6b4011ac…`); the stopped-writer `verify` exited 0 with the source digest `0a1d53f1…` matching before and after, 2/2 sheets, 8/8 source rows, 120/120 fields and 60/60 output headers; and re-submitting the identical request body returned `HTTP 202`, left the run's `updated_at` unchanged, added **zero** source fetches (5,474 before and after), and reproduced byte-identical artifacts. Unlike the entitled indoor run above, this outdoor collection ran on a **signed-out** profile: every event closed below its declared `minCount` (101 ranked rows against `minCount` 300–1,288, five named rows and the rest masked), so the acquisition is live and complete as a *chain* but not complete as *per-event evidence*.

The **outdoor girls live chain completed 2026-09-21** on run `3b7c07fa4e1f80d95f15f24564c4a18925220f3fa313bee5e779e12fd3ba3323` (workbook `0a1d53f1…`, `live-store`): collection terminalized 94/94 events over 134 pages; the row phase decided 8/8 rows (all 8 `review_required`, 0 pending) and published `lane-v14/out-live-outdoor-girls/result.xlsx` (`8656f9f0…`, 8,342 B), `result.jsonl` (`b64359d7…`, 6,545,309 B), and `result.commit.json` (`7ea80cd5…`); the stopped-writer `verify` exited 0 with the source digest `0a1d53f1…` matching before and after, 2/2 sheets and 8/8 rows; and re-submitting the identical request body returned `HTTP 202`, left the run's `updated_at` unchanged, added **zero** source fetches (6,103 before and after), and reproduced byte-identical artifacts. Two failures were repaired on the way: the row phase needed the frozen-coordinator kill + re-submit twice before the rows terminalized, and the first export attempt panicked in `rust_xlsxwriter` (`packager.rs:460`) on `Os { code: 122, kind: QuotaExceeded }` because the `/tmp` tmpfs was 81% full of oversized terminal logs. The export succeeded immediately after that space was freed and the identical request body was re-posted.

## Authorized Athletic.net source and the one-command cycle (2026-09-21)

The owner authorized Athletic.net, and `crates/census-service` now carries both the authorization
mechanism and its own Athletic.net adapter. Neither existed before this session.

**Fetch authorization (`crates/census-crawl/src/net/`).** `Fetcher` records operator-authorized hosts
(`--authorized-host`, repeatable, a bare domain covering its subdomains) and counts their
robots-blocked requests as `robots_authorized` instead of blocking them, still under the 2 rps
per-host ceiling. An entry authorizes exactly the host it names plus anything below it — never the
parent domain — so naming a narrow host cannot widen into a whole site. Evidence:
`cargo test -p census-service --lib net::` → 9 passed.

**`crates/census-crawl/src/athleticnet/`** reads
`GET /api/v1/AthleteBio/GetAthleteBioData?athleteId=<id>&sport=<tf|xc>&level=4` into canonical
schools, meets, teams, athletes, events and performances. Verified contract (live responses,
2026-09-21): the two `sport` calls are **not** interchangeable — an athlete with 40 track results
and 22 cross-country results returns only the first under `sport=tf` and only the second under
`sport=xc` — and the indoor/outdoor split is published only in `allSeasons[].Display`. The adapter
refuses to guess: rows whose season, school, meet or state the payload does not publish are skipped
and **counted in the run report** rather than minted on an assumption. Access is honest: robots.txt
(40 lines, whole file) allows `/api/` and disallows `/Search.aspx`, and the host answers the
collector's own user agent with `200`, so no browser headers are spoofed and athlete ids come from an
operator registry (`--input`, one `athlete_id[,ST]` per line).

**`run`** chains the whole cycle in one command — gather → consolidate → report (both scopes) →
bests → workbook — with each stage using the same code path as its own subcommand and every stage
resumable. `--all-sources` selects the evidence scope for the best-mark reduction (core stays
Athletic.net-free by design; see below).

**Executed evidence** (`cargo run --bin census-service -- --store /tmp/an-run2 --delay-ms 700 run
--input /tmp/an-registry.txt --all-sources`, two Alaska athletes):

- gather: 2 athletes, 4 requests, 0 errors; 97 result rows seen, 89 absorbed, 8 no-mark tokens;
  0 rows skipped for an unpublished season, school, meet or state.
- consolidate: schools 2, teams 11, athletes 2, meets 62, events 89, performances 89.
- report: `all_sources` → athletes 2 (profile_url 1); `core` → athletes 0, i.e. the core scope is
  exactly as independent of Athletic.net as before.
- bests `co2027`: 5 rows for the class-of-2027 athlete — cross country 15:40.12 (2023-09-09),
  1600m 5:37.67, 3200m 12:20.30, 400m 1:09.90, 800m 2:35.79 — each with meet, date, place, timing,
  marks-in-event count and the profile URL.
- workbook: 9 sheets including `Best results` and `Athletic.net marginal`.

**Defects this delivery fixed, with the evidence that found them:** the shared ontology mapper
missed the relay spellings Athletic.net publishes (`"4x400 Relay"` normalizes to `4x400relay`), which
had filed 4 live performances as `Unmapped`; after the fix the same run stores `relay4x400` 3 /
`relay4x100` 1 and **0** unmapped rows. `workbook::build` hard-coded `Scope::Core` for the
best-results reduction and overwrote `bests::write`'s sidecars, so an `--all-sources` reduction was
silently replaced by an empty core-scoped one; `workbook::Options` now carries the scope and the
`workbook`/`run` commands take `--all-sources`. Four `deny(unused_qualifications)` failures in
`hytek/`, `ohsaa/`, `wayzata/` and `wiaa_results/` blocked a clean build and were repaired.
Finally, `bests::write`'s CSV sidecar carried the header **twice** for any non-empty cohort:
`csv::Writer`'s default `has_headers(true)` makes the first `serialize` emit a derived header *after*
the explicit `write_record`, so a 5-row cohort shipped 7 lines (two identical headers). The writer
is now built with `has_headers(false)`, and `bests::tests::the_csv_carries_exactly_one_header_row`
fails on the old writer and passes on the new one.

**Gate state after this delivery:** `cargo test -p census-service` → 219 passed;
`cargo test --workspace` → all suites green, including `result_verify` 14/14 (the obsolete-enum
fixture recorded as failing in the previous handoff now passes);
`cargo clippy -p census-service --all-targets` → clean.

**Remaining, not claimed as done:**

- `cargo clippy --workspace --all-targets` still reports 74 pre-existing errors in the root crate
  (`tests/result_verify.rs` 26, `src/profile/bio/events.rs` 14, `src/xlsx_stream_tests.rs` 12,
  `src/xlsx_scope_tests.rs` 12, `tests/workbook_verify.rs` 10, …): assertions inside test functions
  that return `Result`. They are untouched by this delivery and unrelated to the adapter.
- `README.md`, `SCOPE.md` and `WORKFLOW_REVIEW.md` still describe the adapter list without
  `athleticnet` and without the `run` command.
- Broad qualification (a full multi-state registry, and matching the Athletic.net source against the
  core scope on shared athletes) is not attempted; two athletes is a smoke run, not coverage.

## Row-phase failure modes and their supervision (2026-09-21)

Three distinct defects stall a live row phase. They were separated on the outdoor boys and outdoor girls runs and are now handled without operator input by [row-supervisor.sh](../../.local/share/athletic-rust-pipeline/lane-v14/row-supervisor.sh), which every scope chain invokes:

- **Frozen coordinator.** The `RunCoordinator` invocation reads `paused` with **no** `last_failure` and its journal stops growing (observed frozen at 31,520 entries and then at 3,163). `resume` returns `200` and updates `modified_at`, but the journal stays put, so the run never dispatches another row. The only repair is `PATCH /invocations/{id}/kill` followed by re-posting the identical `PipelineControl/run_and_export` body: that creates a fresh coordinator (`journal 33` and climbing) and rows resume. The supervisor detects it by comparing `journal_size` before and after a resume and applies this repair at every twelfth stalled round.
- **CDP client hang.** A `RowWorker/process` invocation stays `running` for many minutes while the CDP client loses the fetch response. Re-arming the browser session does not clear it; terminating the live worker does, and the next row completes within seconds. The supervisor restarts the worker every eighth stalled round.
- **Worker down.** With no worker listening on `21140` the status call fails outright and no re-arm can help, because every page fetch and row decision is served through it. The supervisor counts consecutive status failures and restarts the worker from `live-worker.toml` after three. Both the worker restart and the kill/re-submit repairs are no-ops when the destination argument is absent, which is why the scope chain now forwards `$DEST` into `row-supervisor.sh`.

**Environment defect: `/tmp` exhaustion blocks export.** The export writer is the only stage that fails on system pressure rather than source behaviour. With the `/tmp` tmpfs at 81% (62 GB of which ~42 GB was three oversized terminal logs), the worker's export thread panicked in `rust_xlsxwriter`'s packager and the run stayed `complete: true` with no artifacts on disk. Neither `verify` nor the replay can proceed from that state; the fix is to free `/tmp` and re-post the run's own request body, which publishes immediately. A stalled export is therefore a host condition, not a source contract failure, and the `PipelineControl` invocation reaching `completed` is the signal to inspect host pressure rather than to re-drive the collection.

## Freeze and ownership

This documentation refresh covers these existing files:

- `README.md`
- `HANDOFF.md`
- `CHROMIUM_DESIGN.md`
- `WORKFLOW_REVIEW.md`
- `SCOPE.md`

Two companion documents at the repository root were imported verbatim at the user's request and are reference material, not current state. They have since been removed from the tree (2026-09-25): `athletic-pipeline-project-pack.md`, a synthesis pinned to tree `33e2adb`, and `athletic-pipeline-handoff.md`, written for the earlier `rankings-lane` harness. Their recorded numbers and next-step lists are superseded by the ledgers below; where they disagreed with these five documents or the current source, the repository won.

The prior GPU development lanes are frozen. Main owns integration, executable verification, native qualification, and subsequent publication. The local 3090 drafted the documentation; local 5090/3090 repair recommendations remain inputs requiring source review and execution, not proof.

The raw local drafting response is retained outside the repository at `native-rankings-1789772180206/final-development/3090/docs-draft-01.json`. It contains no workbook rows, identities, credentials, or hosted-model traffic. A local model draft is design input, not evidence of pipeline execution.
The drafting request used the local 3090 endpoint `http://127.0.0.1:11001/v1/chat/completions`, model `Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf`, with `chat_template_kwargs.enable_thinking=false`. Additional storage/fixture recommendations were requested from that endpoint and the local 5090 at port `11000`, model `Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf`; responses are retained as `final-development/{5090,3090}/integration-repair-01.json` under the same private evidence root.

The user pushed the accumulated changes. Subsequent documentation updates must continue to distinguish implementation, executed qualification, and remaining work; a model response or successful compilation is not end-to-end proof.

Preserve original workbooks, profiles, active stores, journals, retained binaries, and evidence. Never replace a changed binary under its old journals. Never open a live Fjall store from a second process. For offline verification: owner-online export first, drain, stop the sole writer, then open the stopped store.

## Current source design (historical: root package deleted 2026-09-23)

The following describes the design that lived in the deleted root package. Its concepts are now distributed across the nine census crates:

- `crates/census-domain/src/` — the pure canonical model (`SchoolId`, `AthleteId`, `MeetId`, `GradYear`, `ObservedGrade`, `SourceNamespace`) and the `DomainError`/`StoreError` type taxonomy.
- `crates/census-crawl/src/` — all source adapters (athleticnet, hytek, milesplit, ohsaa, mshsl, ihsa, tfrrs, athleticlive, plain_names, ks, wiaa, wiaa_results) that crawl, parse, and absorb into canonical entities.
- `crates/census-store/src/` — the Fjall-backed observation store with twelve tables, merge, snapshot, and index write paths.
- `crates/census-service/src/` — Restate services (`restate_services/`), the CLI, workbook/report/bests/projection, and the `BrowserSession` for headed Chromium transport.
- `crates/census-report/src/`, `crates/census-review/src/` — report and review surfaces.
- `xtask/src/` — developer tooling: quality gates (`scan`, `contract`, `seams`, `integrity`), census reporting (`CensusStatus`, `Coverage`, `Export`), and the `g1` Grade-1 audit pipeline.

The deleted root package's `SourceCache`, rankings parser, rankings discovery (46-family manifest, `RankingsScope`), `ArtifactStore`, and Restate runtime with `RunCoordinator`/`PipelineControl` are no longer present as a monolith. Rankings collection for the expanded roster is now exercised through the census `gather` command (athleticnet adapter) or the `xtask g1` pipeline. Browser transport is owned by `census-service::cli::browser_session`.

## Current CLI and operations

### `census-service` (production binary)

```text
Fetch, Sites, Teams, Meets, Collect, ImportCoaches, Provider,
Consolidate, Index, Report, Bests, Workbook, Seal, Verify, Run,
FjallStats, ImportLegacy, StoreBackup, StoreRestore, StoreIntegrity,
Serve, National, Jurisdiction, NationalReport, Review, MergeCoaches,
VerifyCoaches, QaReports, ExportData, SchoolNames, CensusDoc,
OpenWork, BrowserSession(start|status|stop|fetch)
```

### `xtask` (developer binary)

```text
Gate, Scan, Contract, Seams, Integrity, QualityBaseline, Ratchet,
DomainPurity, SourceTest, SourceTests, SourceFixture, Replay,
CensusStatus, Coverage, Bench, Export, NewSource, DumpSheet
```

`census-service Run` is the main one-command cycle: gather → consolidate → report (both scopes) → bests → workbook. It takes `--input REGISTRY` (optional; without it the cycle publishes what the store already holds), plus `--states`, `--limit`, `--grad-year`, `--all-sources`, `--refresh`, `--observed-on`, `--meets`, `--event-metadata`, `--meet-limit`, `--out`, and `--ingress`. `census-service Verify` requires the existing stopped Fjall store and must not open a live writer. `census-service BrowserSession start` launches the headed browser profile; `BrowserSession status` reads it; `stop` drains it; `fetch` posts one request through it. `xtask Export` builds the census workbook from the running deployment or offline.

Real Cloudflare handling is manual human interaction only in the existing headed profile. There is no evasion, cookie extraction/replay, webdriver patch, proxy/CAPTCHA service, or direct source-HTTP fallback. A challenge closes new admission profile-wide while issued requests drain. Resumption requires successful non-challenged document evidence.

Readiness recovery is bounded by operator action, not by elapsed time: an escalated session (`human_required`) ends the wait with that durable status, and only an explicit `census-service BrowserSession start` re-runs the readiness workflow with operator intent, which re-arms one recovery navigation before the session escalates again. The `BrowserSession` Restate object owns the browser lifecycle with four handlers: `start` launches the persistent profile (idempotent — if the manager is already alive it returns the current reading without relaunching), `status` reads it, `stop` drains it, and `fetch` posts one request through it. The profile is launched via `BrowserManager::connect` when `cdp_endpoint` is set on the bootstrap options, or `BrowserManager::launch` (managed browser) when it is `None`; a failed connect or launch becomes `TerminalError("the browser lane could not start: …")` — there is no fallback between the two paths, and the current CLI leaves `cdp_endpoint` unset, so the connect path is currently unreachable.

## Evidence and remaining work

The current evidence record says:

- Current-tree gates are green: `cargo fmt --check`, `cargo check --all-targets`, and `cargo clippy --all-targets -- -D warnings` are clean, and the test command below runs green (234 tests re-verified 2026-09-20 on the CDP-patched tree: library 156 passed / 2 ignored, main 3, 75 integration tests across 15 targets including the CLI division-flag guards, the fixture-driven rankings verifier cases, the operator ingress-failure surface, and the new `cdp_frame_compat` guard) with no target failures.
- The restored fixture is repaired in-tree: `result_verify` passes 14/14, including `rejects_indistinguishable_local_selection_with_rehashed_evidence`, and the workbook targets that the earlier failing command never reached now execute.
- Serialization measurement: 344,131 versus 13,131 allocations across 1,000 synthetic iterations, or 331 removed per iteration. This is not throughput evidence.
- Retained corpus qualification passed: 95 queries, 4,256 receipt digest checks, 340,238 individual results, 57,629 relay-member results, and 142,705 unique athletes. It explicitly retains 15,724 unresolved roster results. Private report: `native-rankings-1789772180206/native-corpus-cleanup-proof.json`.
- All 26 private public-API storage scenarios passed, including replay, conflicts, per-event/collection counts, multiple record references, roster reconciliation, bounds, and reopen persistence. One private oracle was corrected: source positions `1,2,1` contain two distinct positions, not three.
- Focused parser/storage/bundle regressions and the request-serialization regression passed.
- The operator error surface is covered: `tests/ingress_failure_surface.rs` points `status` at a loopback ingress that answers `500` with Restate's failure body. It failed before the repair (stderr showed only `ingress returned HTTP status 500 Internal Server Error`) and passes after it, printing `requested concurrency exceeds worker capacity`.
- Native fixture deployment: Restate admin `21041`, ingress `21042`, fixture `21046`, worker `21140`; the worker is deployed, registers 13 services on one endpoint, and serves the fixture transport (`source_origin = http://127.0.0.1:21046/`).

Executed end-to-end qualification (fixture transport, `lane-v14`, fresh store `indoor-girls-store`):

- **Indoor girls run `6f3c0c59…`** — scope revision `2026-usa-hs-grade11-indoor-girls-v2`, list `173005`, 46-family manifest, 95 scheduled events all terminal, phase `Complete`. Every source request the run issued used division `173005` (99 requests). The fixture's single non-`173005` request is the other half of a two-request pair written at fixture boot — one request per division, 28 ms apart, 7.5 minutes before the run's first request — so it is startup traffic, not run traffic. Export published as `out-indoor/result.xlsx` (`2e5f3ccc…`), `result.jsonl` (`7bb6496e…`), `result.commit.json` (`6c04cd61…`).
- **Stopped-writer verification** — `verify --store indoor-girls-store` after stopping the sole writer exits 0: source SHA-256 before/after match the recorded digest, 8 source rows project to 6 accepted, 1 no-match, 1 review, 0 pending. This also exercises the verifier's division binding end to end, since the retained snapshot is indoor-scoped: the records/verification path resolves the capture season from `source_season_id()` (`12026`) and builds its expected page context from the scope's kind and gender, so a reverting change to the outdoor season or a dropped gender would fail closed here.
- **Cached replay** — re-submitting the identical `start` command returned the same run identity, added zero source requests (fixture count unchanged), left the run at `Complete`, and reproduced byte-identical publication artifacts.
- **Pause/resume, outdoor boys run `3c726c6d…`** — scope revision `2026-usa-hs-grade11-outdoor-boys-v2`, list `168416`. `rankings-pause` reported `paused` with `phase: {"Paused":"Manual"}` and `pause_reason: Manual`; source traffic froze for the whole 25 s observation window (0 new fixture requests). `rankings-resume` reported `resumed`, after which the collection continued to completion (95/95 events) and published its export; that bundle also verifies clean against the same stopped store (8 rows → 6 accepted, 1 no-match, 1 review, 0 pending), so two runs publishing through one store did not interfere.
- **Indoor boys run `f0ef3948…`** — second division through the same lane and store (`indoor-girls-store`), executed without touching the indoor girls artifacts. Owner-online export published `out-indoor-boys/result.xlsx` (`6403fb0a…`), `result.jsonl` (`86323db6…`), and `result.commit.json`; `verify` then ran against the stopped store (0 writer processes running) and exited 0, with source SHA-256 `0a1d53f1…` matching before and after and 8 source rows projecting to 6 accepted, 1 no-match, 1 review, 0 pending. Restarting the writer and re-submitting the identical `start` returned the same run identity, added zero fixture requests (counter 300 -> 300), and reproduced byte-identical artifacts.
- **Live-source attempt (2026-09-19, blocked; superseded by the 2026-09-20 capture below)** — one real indoor navigation and one real indoor division page were attempted through a dedicated headless Chromium (fresh profile, CDP `127.0.0.1:9333`) with requests issued in-page from the source origin using the production request shapes. The first navigation (`https://www.athletic.net/robots.txt`) returned `403` with Cloudflare's `Attention Required!` page, and both `GetNavInfo` (`seasonId=12026`, `indoor=true`, gender `m`) and the `GetRankings` POST (list `173005`, `100m`, grade 11, page 1) returned the same `403` block page — 5,484 bytes each. Raw bodies, headers (`cf-mitigated`), sizes, and sha256 are retained at `lane-v14/live-capture/` (`capture-meta.json`; superseded by `CAPTURE.md`); no challenge handling was attempted, so clearing the challenge in the headed profile remains the required human step and the live-source derivations entered below stay unvalidated.

- **Live indoor-source capture executed (2026-09-20)** — the authorized headed profile reaches the site, so the 2026-09-19 block was a headless-fresh-profile artifact rather than a session-wide denial. Headed Chromium (`live-chromium-9333`, CDP `9333`, profile `lane-v14/chrome-profile-live`) took the lane's own route: navigate the rankings list UI (`/TrackAndField/rankings/list/173005/{m,f}/100m/?page=1&grades=11`) and record the responses the site itself issues. `GetNavInfo` (GET, `seasonId=12026&indoor=true`) answered `200` with 69,343 bytes (boys) and 69,339 bytes (girls); `GetRankings` (POST) answered `200` with 48,087 and 52,936 bytes. The live navigation carries `seasons["12026"] = 173005` beside `seasons["2026"] = 168416`, both pages report `division.ID = 173005` (`BaseDivID = 2`, `High School`), and the retained pages carry real grade-11 100m results (135 boys, 120 girls), so the indoor season/division identity the code derives is observed on live 200s instead of assumed. Bodies, response headers, byte counts, sha256, page titles, and the capture script are retained at `lane-v14/live-capture/` (`CAPTURE.md`, `live-*.json`, `capture-live.mjs`); no cookie extraction, proxy, direct-HTTP fallback, or challenge service was used. A live *collection* run through the pipeline is still unexecuted.
- **Readiness recovery (2026-09-20, live lane)** — after the bounded-escalation repair, three `browser-start`/`await_ready` cycles ran against the live cell (`lane-v14`, Restate admin `21041` / ingress `21042`, headed Chromium on CDP `9333`): (1) session already `ready` — invocation completed in ~1 s with `ready`; (2) headless-endpoint absent (the headed browser was stopped and the worker restarted) — the worker logged the unreachable endpoint and **launched a managed headed browser on the lane profile**, the session reached `challenged`, escalated at the configured 30 s window to `human_required`, and the invocation **returned** (`completed`, 13:34:09 → 13:34:40 = 31 s) instead of polling to the 24 h deadline; (3) an explicit operator `browser-start` from `human_required` re-armed one recovery navigation — observed state transitions `restarting` → `challenged` → `human_required` — and completed in 30 s. Restoring the headed browser on `9333` plus a fresh worker returned the session to `ready` (`stopped` → `challenged` → `ready` in 15 s). The physical browser launched by the fallback was terminated with its worker (no stray Chromium). Deployment note: Restate validates handler input from the newest registration that owns the service, so a changed handler signature requires re-registering the endpoint (`force=true`); a stale registration made `await_ready` reject its JSON body with an ingress 400 until it was replaced.

**Workbook-scale qualification (own Restate identity, `scale-v15`, fixture transport, 2026-09-20):** A separate lane with its own Restate admin `22041` / ingress `22042`, fixture `22043`, worker `22140`, and a private store and input root under `/home/lewis/.local/share/athletic-rust-pipeline/scale-v15/` exercised the pipeline against the **real two-sheet source workbook** (120,716 non-empty source rows, digest `5969df2379eab4eb22bc3f175329d97d06de5b4c2c8ced7172ccbd3af3a6383b`) instead of the 8-row synthetic fixture copy. The worker binary was sha256 `0f044e59e3c5bc5d0385c9ec607f8dffeb49bc4624d3648aed6f1a28a0295f99`, built from the working tree at `9d101d9`; the lane never passes `--rankings`, so the in-flight rankings-parser changes in that tree are outside this exercised path.

- **Smoke run `eab0d224…` (10 selected rows)** — `run-and-export` published `smoke.xlsx` (111,939 `Export` rows + 8,777 `Sheet1` rows), an 83.7 MB JSONL sidecar, and a receipt whose embedded verifier matched source SHA-256 before and after, 2/2 sheets, 120,716/120,716 rows, 1,569,308/1,569,308 source fields, and 26/26 source headers with 11 appended annotation headers.
- **Owner-online partial export against a live run (92 s)** — exporting the still-collecting 5,000-row selection while the writer stayed online published **all 120,716 source rows** with `completeness: partial`: 462 completed and **120,254 rows retained as explicitly pending**, not as guessed matches. The verifier again matched source SHA-256 before and after, both sheets, every row, and all 1,569,308 fields.
- **Measured throughput (non-rankings path, fixture transport, `row_concurrency = 8`)** — sustained ≈0.31 rows/s (≈3.2 s per row) over 500+ rows, with ≈50 durable invocations per row across ≈10 `QueryWorker` / `SourceCache` / `BrowserSession` source operations per row, 0 retries, 0 model calls, and 0 local-review decisions. A second identical lane (`scale-par`, own Restate identity, fresh store, same input and binary) measured ≈0.27 rows/s at `row_concurrency = 32` over 226 completed rows, so quadrupling coordinator concurrency produced no gain, while running both lanes together sustained ≈0.9 rows/s (≈1.8× one lane) with every process near-idle (~7% CPU for each worker and its Restate node). The wait is therefore per-worker and inside the row's source-operation chain, not coordinator concurrency, CPU, or journal volume; over a 30 s window in which 24 rows completed, the fixture's counters recorded no new source requests — the hop itself is isolated in the next bullet. At the measured single-lane rate a full `--all` pass over the 120,716-row workbook is ≈108 h of wall clock, which is the concrete capacity bound for workbook-scale delivery.
- **Durable-write volume is the dominant per-row cost (isolated 2026-09-20)** — sampling `/proc/<pid>/io` over 30 s while both lanes collected shows the `scale-v15` Restate node writing **248,815,616 bytes (≈8.1 MB/s)** and its worker writing **88,444,928 bytes (≈2.9 MB/s)** — ≈11 MB/s combined against ≈0.9 completed rows/s, i.e. **≈12–14 MB of durable writes per row**. Both processes stayed near-idle (~7% CPU), no `Sleep` journal entries exist anywhere, and 24 rows completed in a sampled 30 s window with no new fixture requests, so the per-row wait is durable-write latency, not coordinator concurrency, CPU, or source I/O. Write volume tracks operation count rather than saturating on its own, so it is the cost ceiling on any rate increase; the observed rate itself is framed by the operation-level serialization measured in the next bullet. Independent clusters do scale partially (two clusters reached ≈0.9 rows/s), but `row_concurrency` within a cluster does not. What the ceiling is *made of* is corrected in the next bullet.
- **Correction (2026-09-20, measured over the retained journals): the cost is storage-engine churn, not journal payload.** Standing queries show `sys_journal.raw_length` averaging **112.5 B/entry** on `scale-v15`: **51.5 KB of journal per decided row** (258,153,689 bytes over 2,295,223 entries across 5,010 rows), with one `SourceCache/fetch` journal holding 8 entries / 2,099 bytes — beside a `restate-data` root of 551 MB (**110 KB/row**) and a store of 181 MB (**36 KB/row**). The live lane holds 855,439,148 journal bytes over 398,666 entries (2,146 B/entry, ≈800 KB per ranking page: the collection path's receipt/parse chain, not the row path). The ≈11 MB/s / ≈12–14 MB-per-row figure above is therefore device-level write volume dominated by LSM compaction and fsync churn — plus a 785 MB debug-level worker log on that lane — rather than by payload size, so the levers are **fewer durable steps per row** (≈46 invocations) and storage/log configuration, not smaller journaled payloads: the lane's published artifacts are ≈0.7 KB per source row (83.7 MB of JSONL for 120,716 rows) and its logical journal is ≈51 KB per decided row. Measurement note: `sys_invocation.journal_size` is an **entry count**; `sys_journal.raw_length` is the byte measure, and `sys_journal` is queryable read-only, which is why this attribution needs no running lane. The concrete levers, in order: (1) fewer durable steps per row — the gate is a *safety* serializer, so parallelizing it is not free (`BrowserSession/await_ready` is single-key because a challenge must close admission profile-wide), which is why the measured scaling path is **independent clusters** (own Restate identity and `BrowserSession`; two clusters reached ≈0.9 rows/s against ≈0.31–0.34 rows/s single-lane, so a full `--all` pass is double-digit lane-hours, not 108 h of one lane); (2) log volume — the binary's default filter is already `info`, and the 785 MB lane log is `chromiumoxide`'s per-frame `WS Invalid message` warning, so long lanes want `RUST_LOG=info,chromiumoxide=error` or rotation; (3) storage configuration, not payload size.
- **The exclusive readiness handler frames the source-operation rate (scale-v15, 2026-09-20, `tabs = 8`, `row_concurrency = 8`)** — a 120.1 s window recorded 41 rows and 358 source operations: **0.341 rows/s and 2.982 ops/s**, 8.73 operations per row, and `BrowserSession/await_ready` entered **exactly once per source operation** (358 gate calls against 358 operations in the window; 14,267 lifetime calls against 14,265 lifetime fetches). The handler's 0.329 s mean duration *is* the observed operation period (1 ÷ 0.329 s = 3.04/s), so the single-key exclusive gate paces the lane; that identity is why `tabs` 2→8 moved throughput only ≈14% and `row_concurrency` 8→32 did not move it. In the same window `/proc/<pid>/io` shows the worker writing 2.96 MB/s and its Restate node 10.97 MB/s (13.93 MB/s combined, 36.4 MB per row, 4.6 MB per operation) — block-layer accounting that includes Fjall compaction, so it is a cost signal, not a per-row byte ledger.
- **A shared-probe fast path was built, measured, and reverted** — worker binary `fe95a735…` called the shared `capture_ready` first and entered `await_ready` only when the probe was not ready. The probe never short-circuited: `capture_ready` answers `500 browser bootstrap failed` whenever the worker cannot bootstrap its managed browser, so probe and gate calls stayed 1:1 (280/280) and the added durable step per operation cut the lane to **1.549 ops/s and 0.167 rows/s** (180.1 s window). It is not shipped (the lane runs the reverting build, sha256 `8e1587ed…`); `src/runtime/source/dispatch.rs` records the measurement and the constraint on any replacement — answer without a physical act, and leave `await_ready` the only writer of readiness state.
- **The lane's missing browser was a stale fixture build (resolved 2026-09-20)** — the managed launch was failing its bootstrap navigation because the running `native_fixture` binary (built 2026-09-17) served `/` as `404`, which Chromium commits as an error document, so no page ever reached a ready classification. The deleted example source was restored from `aeb2116^`, where `/` is routed to `browser_home` (an HTML landing document that sets a fixture cookie and `document.body.dataset.ready`); the rebuilt fixture (hub process `native-fixture-18081`, output dir `fixture-native-v2-run2` so other lanes keep their `native-fixture.xlsx` digest) answers `200`, `browser-start` drove `BrowserSession` to `state: "ready"` with 8 tabs and a live Chromium on the lane profile, and resuming the parked driver (`RunCoordinator/run` and `PipelineControl/run_and_export`, both paused by the outage with `on_max_attempts = "pause"`) restarted collection from 1,631 decided rows at ≈0.6 rows/s — 140 fixture source requests in the first 30 s with the fixture's own `max_active_requests` pinned at 1, an independent view of the exclusive readiness gate.
- **Pending in this lane:** the 5,000-row end-to-end run `a8328b7f…` holds **1,631+ decided rows** and is now advancing again after the browser repair above; stopped-writer verification and cached replay for this lane have not executed, so this lane is not yet part of the completed end-to-end qualification list above.

Four operator boundaries are worth carrying forward:

- **A pre-change store is not drivable by the current tree, and the failure is fail-closed.** Re-registering the `scale-v15` endpoint (`worker --config …/scale-v15/worker.toml --bind 127.0.0.1:22140`) on the current build answers `13 services` and then `status --run a8328b7f…` refuses with `evidence_digest has an invalid format`; the lane also holds **zero non-terminal invocations**, so its 5,000-row run is abandoned mid-selection rather than parked. That is the retained-binary rule in action: drive that lane with the binary registered at lane start (`0f044e59…` / `8e1587ed…`, tree `9d101d9`), never with this tree, and re-register with `force=true` if its endpoint is later pointed at a changed handler signature. The lane's published artifacts and its frozen `out-v16` verification remain valid; only its unexecuted cached replay is still open.
- The durable workbook-import identity is `(original path, workbook digest)`, and its retained journal rebinds the manifest digest against the store that imported it. Pointing a **fresh store** at the same original path fails closed at `PipelineControl/prepare` with `requested artifact is absent`; give each store its own byte-identical source copy (used here: `indoor-input/original-synthetic.xlsx`) or a fresh Restate identity. Re-importing the same path into the *same* store is served from the cached manifest.
- `start --concurrency N` is validated against the worker's `row_concurrency`; a larger value fails with `requested concurrency exceeds worker capacity`. Collection throughput is paced by the single-key exclusive readiness handler every source operation passes through (`BrowserSession/await_ready`, ≈0.34 s per entry, one entry per operation), so neither a larger requested concurrency nor a wider page pool by itself raises observed parallelism.
- A writer stop mid-run leaves the run **paused, not failed**: the `on_max_attempts = "pause"` retry policy parked 37 invocations during the `scale-v15` interruption (four `RunCoordinator` global calls, one `PipelineControl`, and the in-flight `SourceGateway` / `SourceCache` / `QueryWorker` / `RowWorker` handles), and `status` froze at 955/5,000 across repeated polls. Re-submitting the identical `start` does not clear that state on its own; recovery is `PATCH /invocations/{id}/resume` for each paused row (`POST` answers 405), enumerated from the admin's `sys_invocation` table. Resuming all 37 advanced the same run past 1,100 completions with the same snapshot digest and no new run identity. Admin SQL requires `accept: application/json`; the bare request returns an opaque binary body that JSON tooling cannot parse.

- A fixture-mode lane that reports `500 browser bootstrap failed` — or whose `browser-status` stays `stopped` while `browser-start` itself completes — is usually pointing at an origin with no landing document: the managed launch navigates every tab to `source_origin`, and a `GET /` that answers `404` makes Chromium commit an error document that never classifies as ready, so no page is ever handed out. Rebuild the fixture (`cargo build --example native_fixture`), start it with a fresh `--output-dir` (`fixture-native-v2-run2` here, so other lanes keep their `native-fixture.xlsx` digest), confirm `GET /` answers `200` with the `browser_home` document, re-arm with `browser-start`, and `PATCH /invocations/{id}/resume` the parked `RunCoordinator` / `PipelineControl` invocations — the browser is repaired before the run can move again.

Remaining gaps are live-source qualification (real endpoints and manual challenge handling), workbook-scale identity-matched delivery, and the expanded roster (indoor/outdoor/XC completeness, athlete URLs, school history, PRs). Inside live-source qualification, the indoor season identity (`seasons["12026"] = 173005`, `division.ID = 173005`) is now observed on live `200` responses (2026-09-20 capture above) rather than derived, but no live *collection* run — the pipeline's own `start` against the real source — has executed, so every end-to-end lane run remains fixture-mode. Fixture-mode evidence must not be represented as live readiness, and expanded-roster coverage remains a set of acceptance criteria, not a completed feature.

Historical synthetic and legacy-run material may remain in retained evidence, but must be labelled historical and must not be used as current-tree proof. Old live profiles/stores/journals and changed binaries remain preserved; they are not replaced or replayed. The same rule governs revisions: the pre-split rankings revision `2026-usa-boys-grade11-v1` is **retired** — a scope carrying it still deserializes (its absent `season_kind` defaults to outdoor) but `validate` rejects it with the retired revision named, so those stores are verified and exported by their retained pre-change binary rather than reinterpreted by this tree.

## Live acquisition findings and pagination repair (2026-09-20)

Captured through the lane's headed Chromium using the site's own requests; raw bodies are
retained outside the repository at
`~/.local/share/athletic-rust-pipeline/lane-v14/live-capture`.

- **The live list report serves one page per event.** Requesting `page=2` for `100m` and `200m`
  (boys grade 11, USA indoor division 173005) returned byte-identical page-1 payloads with
  `settings.page = 1`: `site-page2-2026-09-20T1840Z-0.json` (`2ced3016…`) repeats
  `site-page2-2026-09-20T1830200mZ-0.json` (`3b702d16…`) row for row, ranks 1..101. The source
  ignores the page parameter, so a second page is not retrievable for this report.
- **Rows are entitlement-limited for this session.** Every response declares `settings.depth = 100`
  and `blurAfterDepth = 5`: five rows carry real athlete names and the remainder are masked
  (`Xxxxx Xxxxx`) with a blurred tail row (`live-page-indoor-173005-m.2026-09-20T1730Z.json`,
  `00796851…`). Complete per-event evidence therefore needs an entitled session; the lane's
  refusals below are fail-closed restatements of that limit, not parse failures.
- **Pagination rule repaired.** `next_page_after` used to request a successor for any non-empty
  page, so a complete short page (`100m`: 71 rows against `depth: 100`) always asked for a page
  the source answers with page-1 content, and strict publication then paused with
  `page mismatch: expected 2, got 1`. The rule now requires the page to fill its declared depth
  (`settings.depth`, falling back to `defaultSettings.depth`); a short page ends the chain.
  Unit tests cover the live short and full shapes plus the fixture's declared depth.
- **The blocking event is full, and its declared list is far larger than what the session sees.** `55m`
  returns 101 rows against `depth: 100` (a full page, so a successor is implied) while declaring
  `minCount: 749`; only five of those rows are unblurred. The running collection therefore accepted
  `55m` page 1 and then paused on `page mismatch: expected 2, got 1` when the source answered the
  `page=2` request with page-1 content. Both the source refusal and the terminal `minCount` guard
  agree that this session cannot assemble the event's evidence, so completing a live event with more
  than one page of ranked rows requires an entitled browser profile — not a code change. Capture:
  `site-page2-2026-09-20T1905Z-0.json` (`13f89ec2…`); the short-page path is separately exercised by
  `100m` (71 rows against `depth: 100`, terminal after one page).
- **The live re-run remains unexecuted.** A fresh collection (`source_snapshot 839b9cad…`) was
  created for run `fb564d0c…`; it acquired `55m` page 1, paused on the source's page-2 refusal as
  described above, and its earlier attempts failed with `browser transport failed` until the
  readiness workflow was restarted (`browser-start`; `rankings-resume` intermittently answers
  `409 Conflict: browser is not ready`). The repaired pagination therefore rests on unit and
  regression evidence, not on a completed live collection, and the live collection gate stays open.

### Entitled profile: pagination and transport repair (2026-09-20, later)

The operator signed in to the lane's headed profile (`/dashboard/960929`) and the session serves
complete rank lists again: `55m` accepted pages 1 through 9 (`next_page: 10`) and `100m` page 2
returned ranks 100..201, so the earlier "the source ignores the page parameter" limit belongs to the
anonymous session, not to the report. Entitlement also restored unblurred names for the rows that
were masked before (101/101 named where five were).

Two defects surfaced while collecting under that session; both are repaired in source:

- **The source canonicalises navigation URLs.** A live four-shape probe through CDP
  `Page.navigate` shows `…/list/173005/m/55m/?page=1&grades=11` is served as
  `…/list/173005/m/55m?grades=11` — the trailing slash and the `page=1` query are stripped, while
  `grades` is kept. A canonicalising navigation makes Chromium report `net::ERR_ABORTED` for the
  original document, which `classify_observation` read as `Transport`; the run then paused with
  `browser transport failed` and the readiness workflow escalated to `Restarting`/`human_required`.
  `build_ui_url` now emits the canonical URL the source serves unchanged (page depth travels only in
  the API payload) and both navigation paths ignore browser-aborted (`net::ERR_ABORTED` or
  `canceled`) documents instead of latching a failure. Unit test:
  `navigation_url_is_already_canonical`.
- **The CDP client desynchronises on this site's heavy pages.** `chromiumoxide::handler` logs
  hundreds of `WS Invalid message: data did not match any variant of untagged enum Message` per page
  load. The fetch path used to enable `Network.enable(maxTotalBufferSize 32 MiB,
  maxResourceBufferSize 8 MiB, enableDurableMessages true)`; a durable buffer makes Chromium replay
  buffered events the client cannot parse, and lost responses surface as `Transport`. The fetch path
  now enables default `Network` — the ranking payload arrives through the injected binding, so no
  buffered replay is required — and transport failures log their stage
  (`browser transport failure: {stage}: {error}`) instead of discarding the cause.

Collection advances again under the entitled session; the transport failures above are gone, and the
remaining stop is a configuration boundary rather than a source limit. Run `bbad8a7c…` was submitted
with `--max-pages-per-event 12` as an early qualification bound; under the entitled profile it
accepted twelve distinct full pages for `55m` (`next_page: 13`) and then paused with
`pause_reason: PageLimit`, so multi-page acquisition is now observed end to end and the cap — not the
source and not the transport — ended the chain. `start` defaults the cap to 10000 with a short page
ending the chain, so the live gate was re-submitted with that default as run `57f88b34…` (`--all`,
`--rankings --rankings-gender m --rankings-season indoor`, automatic export to
`lane-v14/out-live-full-3/result.xlsx`). The submission before it reused run identity `c25150c2…` at the
old cap and never started; see the serialization note below.

Because the frame drops are repaired but a full live collection has not yet confirmed that the residual stalls shared that cause, a bounded
supervisor (`~/.local/share/athletic-rust-pipeline/lane-v14/resume-supervisor.sh`, hub process
`live-resume-supervisor`) re-arms `browser-start` and `rankings-resume` and stops on completion or
after twelve rounds without new pages. This is an operational mitigation for a client defect, not a
completed live collection; the two capped chains were canceled 2026-09-20 and the default-cap run
`57f88b34…` was the live gate, acquiring across the full 95-event scope; it has since completed with publication, verification, and replay.

`start` derives the run identity from the plan and source, and submissions are **serialized rather than
replaced**: `PipelineControl/run_and_export` calls `RunCoordinator/run` at the `global` key, an exclusive
handler, so every later submission queues behind the first invocation of that key that is still open.
The first entitled live submission paused at `PageLimit`, and that paused invocation kept the key; three
later submissions each answered `submitted` with their own invocation id and then never started (the
CLI's `state: submitted` is acceptance, not progress; `status`/`rankings-status` answer
`404 run not found` until the run's collection registers). They also carried the 12-page cap, because
the first of them reused the identity of the earlier capped run. Clearing the queue needs the lock
holder canceled — `PATCH /invocations/{id}/cancel`, or `PATCH /invocations/{id}/kill` for a paused
invocation, since cancel is graceful and a paused invocation reaches no cancellation point. With the
paused coordinator killed and the two stale queued submissions canceled, the next queued run started
immediately. The diagnostic is the admin `sys_invocation` table (`status <> 'completed'`), not the
CLI. Automatic export is separate: each `--output` destination gets its own `run_and_export`
invocation, which waits for run completion before publishing.

### Rankings transport retry and session re-arm (2026-09-20, later)

The residual CDP desync surfaced as `browser transport failed` with `pause_reason: SourceFailure`:
the client loses the command response while the source's pages replay network events, so
`capture_body` (`Network.getResponseBody`) answers `{"code":-32000,"message":"No data found for
resource with given identifier"}` and the attempt is classified `FailureCode::Transport` with **no
receipt**. Its origin is now identified: the worker log shows the CDP client discarding
WebSocket frames it cannot deserialize, continuously, every 1–3 s while acquisition runs —

```
WARN chromiumoxide::handler: WS Invalid message: data did not match any variant of untagged enum Message
```

The lane is pinned to `chromiumoxide = "=0.9.1"` (the newest release, published 2026-02-25) and drives
Chromium 151.0.7922.173, so this is protocol drift inside the CDP client, not pipeline logic. The dropped
frames are now identified exactly: the crate's own debug target
`chromiumoxide::conn::raw_ws::parse_errors` retains the raw payload, and every retained frame is
`Network.requestWillBeSentExtraInfo` whose `clientSecurityState` carries
`localNetworkAccessRequestPolicy` (`"PermissionBlock"`) and no `privateNetworkRequestPolicy`. Chromium 151
replaced that field; the pinned generated model still requires it, so the frame fails to parse as an
untagged `Message` and is dropped. What is lost is an extra-info event, not the completion of a command
the lane awaits.

The repair is local and one field wide: `vendor/chromiumoxide_cdp` is a copy of the published crate with
`ClientSecurityState.private_network_request_policy` made optional, wired through `[patch.crates-io]` in
`Cargo.toml`, and `tests/cdp_frame_compat.rs` pins the behaviour against a sanitized copy of a real
captured frame, so the guard fails if the patch stops applying or the model drifts again. Live evidence on
the patched binary (`0ed78cc33bf90020863e55485ecdbe5c2ada8c7f03c142fd05f1656ebad7fd46`): the same headed
dashboard navigation that dropped a frame every 1-3 s on the previous build produced none, and every
retained drop in the worker log predates the patched worker's start. The re-arm supervision stays in place
until a full live collection runs clean end to end. The residual stalls are **independent of the dropped
frames**: on the patched binary the frame loss is gone (no `WS Invalid message` or `parse_errors` line
after the patched worker started), yet `SourceFailure` pauses continue at roughly one per two to four
pages, each cleared by the supervisor's re-arm.
That stall is a second, distinct defect and it is identified: a launched Chromium restores the profile's
previous tabs, so every recovery relaunch left another restored dashboard page beside the two-page pool.
The extra pages kept loading and competed for the same renderer capacity, so each stall made the next one
likelier (observed: seven leaked pages, ~33-40 s/page late in the run; ~6-10 s/page once pruned mid-run,
and the stalled run resumed advancing). `Actor::bootstrap` now closes pages the pool does not track when
this process launched the browser; attached loopback sessions keep their pages. The code change passes fmt,
strict Clippy, and the full test suite, and its live effect is verified on the next relaunch. While a
patched worker runs, the lane supervisor also prunes restored tabs after every recovery.

**Live outdoor run on the patched client (2026-09-20, in progress at this writing).** Run
`7be7e9dd68cd5dc51c0c03edf05b5503297eec772756350870a04a1ebd95e0b3` (outdoor boys, list `168416`,
workbook `0a1d53f1…`, `live-store`) is the first live collection on the patched binary. Two
`SourceFailure` pauses appeared in its first minute, before the patched worker was serving it, and no
`WS Invalid message` or `parse_errors` line has appeared since — the dropped-frame repair holds. The stalls
nevertheless continued at roughly one per two to four pages, and the rate decayed to ~33-40 s/page as
leaked browser pages accumulated; closing the extras mid-run restored ~6-10 s/page. See the tab
accumulation finding above for the cause and the committed fix. The
progress supervisor was restarted detached so the lane stays polled past this session. Completion,
publication, stopped-writer verification, and replay all executed 2026-09-21 and are recorded with
their digests in the status section above; the run is qualification evidence for the live chain.

**Unattended outdoor tail (2026-09-20, running).** `lane-v14/finish-live-outdoor.sh` (detached as
`live-outdoor-tail`) waits for the boys publish, takes the stopped-writer verification, restarts the
worker, and replays the identical request body built from the run's own `status` request plus its
destination; then it starts the outdoor girls scope (`start --rankings --rankings-gender f
--rankings-season outdoor --output out-live-outdoor-girls/result.xlsx`), supervises its collection and row
phase, and takes the same tail. A publication appearing outside the expected out directory fails the
scope rather than being reported as a replay. The writer stop/restart inside the script mirrors the
scale-v15 finisher (`pgrep` on `live-worker.toml`, SIGTERM, `nohup` restart of the same build), so the
hub's `lane-live-worker` entry reads `exited` after the stop and the restarted worker is unmanaged.

**Row-phase stalls and their repairs (2026-09-21).** The outdoor boys row phase sat at `0/8` for hours with
`pages: 0` while its coordinator invocation read `paused`. Three distinct failures were separated:

- **A retry-policy pause is permanent and `resume` is a no-op on the frozen invocation.** The
  coordinator's journal had grown to 31,520 entries / 6,302 `snapshot_ref` calls before it stopped; after
  that, `PATCH …/resume` returned 200 with an unchanged journal and an instantly re-paused invocation, and
  `DELETE /invocations/{id}` (cancel) also answered 202 without effect. `PATCH /invocations/{id}/kill` is the
  route that works — the handlers are `patch`-only, since `POST` answers 405 — and the paused
  `PipelineControl/run_and_export` must be killed with it too, because an exclusive `RunCoordinator/global`
  invocation holds that key and every later submission queues behind it.
- **The stuck invocation holds the key; the run has to be re-submitted, not resumed.** After the kill, the
  identical request body posted to
  `POST {ingress}restate/send/PipelineControl/run_and_export` immediately produced a fresh coordinator that
  read the already-sealed collection, passed the seal wait, and drove the rows.
- **The worker's HTTP/2 link to its Restate node wedged.** The worker process kept its listen socket and
  still answered Restate's deployment discovery, but `hub logs lane-live-worker` showed continuous
  `hyper::Error(Http2, GoAway(… ENHANCE_YOUR_CALM/PROTOCOL_ERROR))` stream resets, and no invocation
  advanced. Restarting the **Restate node** (not the worker) cleared it: the first `snapshot_ref` call after
  the restart returned the sealed digest instead of `null`, and the coordinator's journal started advancing
  again. `lane-live-restate` is the hub-managed process for that node; its readiness pattern must be one the
  node actually prints (`Admin:`/`HTTP Ingress:`), because a port-only check reports ready against the
  previous instance.
- **The row fetches themselves still stall on the CDP client.** A `RowWorker` sat `running` for fifteen
  minutes on a single row; killing and restarting the worker cleared it and the next row completed within
  seconds. `row-supervisor.sh` now restarts the live worker every 8 stalled rounds (SIGTERM on
  `live-worker.toml` plus a `nohup` restart of the same build) and, every 12, kills a paused invocation whose
  journal does not move after a resume and re-submits the identical request body from the run's own
  `status`; `scope-chain.sh` forwards the destination it needs. Both scripts are `bash -n` clean.

**The outdoor collection ran signed out.** The lane profile lost its entitled session between the indoor
run and this one, and the source answers a signed-out session with a capped list: every event closed at 101
ranked rows against a declared `minCount` of 300–1,288, with five named rows and the remainder masked
(`Xxxxx Xxxxx`). The chain therefore completed against live endpoints but not against complete per-event
evidence, and all 8 workbook rows landed `review_required` rather than matched. Re-running after signing the
lane profile back in is what closes the per-event gate; no code change addresses it.

**Stall diagnosis and supervision hardening (2026-09-20).** The outdoor boys collection stopped advancing at
217 pages for an hour while supervision kept failing: each `BrowserSession/recover` reported the cached manager
as not running, rebuilt it, and the rebuild's shutdown then aborted after its timeout, in a ~16 s loop (visible
in `lane-live-worker`). The tab target itself was gone — `browser-status` read `stopped` and CDP `9333` held a
single `about:blank` — while the browser process stayed up, so re-arms could not rebuild the pool. A plain
`browser-start` returned the session to `ready` with two tabs, and the collection resumed six seconds later
(217 → 248 pages). Three changes follow: the collection supervisor now exits 2 instead of 0 when it exhausts
no-progress, logs the browser state every round, and after eight fruitless re-arms closes **every** page
(`prune-tabs.py 0 --all`) so the next `browser-start` rebuilds the pool from scratch; and `row-keeper.sh`
covers the row phases without needing the run id, because the row supervisor's resume is global over parked
invocations and the completed-invocation count is a liveness signal. The keeper re-arms only after four
minutes of silence with parked work, since a re-arm navigates the lane and desyncs an in-flight page read.

These changes make the lane self-healing meanwhile:

- `receiptless_transport(code, has_receipt)` — only a transport fault that produced **no receipt** is
  retryable for rankings. Anything the source answered (403/429/challenge/parse) still retains its
  receipt and returns immediately, so no observation is discarded and the source is not hammered.
- `execute` re-arms the browser session (`BrowserSession/profile-0/recover`, the shared operator
  recovery that preserves cooldown) before each such retry. Retrying without the re-arm meets the same
  desynced client, which is why retries were disabled in the first place.

Evidence: with the resume supervisor stopped, a single `rankings-resume` advanced `55m` from 22 to 28 to
31 pages (`generation` 102→106) instead of stalling at one page per round. The supervisor's rounds also
gained the recovery call — 6→9→10→13→17→22 pages across five rounds — because a bare resume does not
clear the desync. Operator diagnosis uses the lane's own capture tooling
(`lane-v14/live-capture/capture-live.mjs`, which drives CDP 9333 directly) and the admin
`sys_invocation`/`sys_journal` tables.

### Live collection completed; paused-workflow recovery (2026-09-20, later)

**The live collection gate is executed.** Run `57f88b34…` (all events, `--rankings --rankings-gender m
--rankings-season indoor`, `--max-pages-per-event` at its default 10000) acquired the full scope through
the entitled headed profile: `rankings-status` reports `phase: Complete`, `terminal: 95/95`,
`pages: 1065`, `catalog_outcome: retrieved`,
`final_snapshot 40792b83cfb168de5de876773f1312e41af3c34fc49782a991bdd6d9d4389dd6`,
`plan_ref 24d3e3ae7c49cc0c`. The bounded supervisor (`live-resume-supervisor`) carried the closing rounds
(85 → 89 → 94 → 95 terminal, 1,011 → 1,065 pages) and exited 0 on `COLLECTION COMPLETE`. Every one of
those rounds re-arms through `browser-start` + `rankings-resume`, because the CDP client desync recorded
above persists — the completion is a live collection under the re-arm mitigation, not evidence that the
client drift is gone.

Two operational facts were established while bringing the run's own workflow forward; the worker was
replaced under the running lane (repaired build) while the coordinator invocation was in flight:

- **A paused workflow invocation does not resume onto a replaced worker.** `PATCH /invocations/{id}/resume`
  on the paused `RunCoordinator/run` returns `200`, and the invocation is `paused` again within seconds
  with **zero journal growth** (`journal_size` unchanged across repeated polls) — the continuation never
  reaches the handler. The worker logs the refusal at each attempt:
  `restate_sdk::http_server: Error serving connection 127.0.0.1:…: hyper::Error(Http2, Error { kind: GoAway(b"", ENHANCE_YOUR_CALM, Library) })`,
  i.e. the Restate → worker HTTP/2 stream is rejected before the invocation body runs. This is distinct
  from a missing pinned deployment: `GET /deployments` still lists `dp_15VGHjPnChtjVgUJzmtWTjb`
  (`http://127.0.0.1:21140/`, 13 services). The cause sits between the SDK and the server (transport),
  not in pipeline logic, and repeating the resume does not clear it.
- **`cancel` does not terminate a paused invocation; `kill` does.** `DELETE /invocations/{id}?mode=cancel`
  answers `202` for both the paused `RunCoordinator/run` and its live `run_and_export`; the export
  invocation reaches terminal `completed`, but the paused coordinator keeps `status = paused` (cancel is
  observed at a cancellation point, which a paused invocation never reaches). `?mode=kill` on the same id
  returns `202` and the row moves to terminal `completed`.

**Working recovery — re-submit the identical request.** `start` derives the run identity from the request
(`request.key()`) and the collection identity from the same plan, so re-sending the *identical*
`run_and_export` body attaches a **new** `RunCoordinator/run` invocation to the **already-`Complete`**
collection: nothing is re-acquired. The body was recovered from the paused invocation's journal entry 0 —
`{"request":{"manifest":"d2d564fc…","snapshot":"e8605d26…","selection":{"scope":"all"},"concurrency":1,"execution":"stage"},"destination":"…/lane-v14/out-live-full-3/result.xlsx"}`
— and posted to the ingress path the CLI itself uses,
`POST <ingress>/restate/send/PipelineControl/run_and_export` (`202 Accepted`, new invocation id). The
order matters: kill the paused coordinator first, because the queued successor stays `pending` until the
exclusive `global` key is free; canceling the stale `run_and_export` alone leaves the lock held. With the
kill applied, the run's `updated_at` moved off the frozen 13:36:07 within a minute and the row phase began
(`completed=1/8`, `review_required=1`, one row in flight) against the same collection and the lane's
two-sheet, 8-row sample workbook.

**The re-arm now covers the profile path too (source change, gates green).** `run_step` set
`rearm: is_rankings && retryable`, so a profile fetch that lost its command response retried against the
same desynced session until the row spent its attempt budget and parked. `rearm` is now plain
`retryable`; the retryability rule itself is unchanged — rankings still require a receipt-less transport
fault, non-rankings keep the earlier rule, and the re-arm still happens only before an actual retry.
Gates on the changed tree (`503587e`): `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D
warnings`, `cargo build --locked`, and the 16-target command — library 155 passed / 2 ignored,
`result_verify` 14/14, every named target green. No unit test asserts the flag itself: it composes two
already-tested predicates (`receiptless_transport` and the non-rankings retryable rule), and the change
is exercised live by the next run that fetches profiles — the worker serving this run still runs the
pre-change binary, since replacing it mid-run would park the in-flight rows.

### Live row phase, publication, stopped-writer verification, replay (2026-09-20, later)

**The live chain is now executed end to end for the indoor boys scope.** With the coordinator restored as
described above, run `57f88b34…` finished its own row phase against the live source and the lane's
two-sheet, 8-row sample workbook:

- **Row phase complete** — `status` reports `coverage {selected 8, completed 8, review_required 8,
  deterministic 0, local_review 0, no_match 0}`, `complete: true`, 0 pending (finished 20:46:26 UTC,
  ≈5 min for 8 rows). The decision is `review_required` for every row because the sample workbook carries
  no source-identity evidence: that is the fail-closed contract, not a parse failure. The rows ran on the
  **pre-re-arm binary**, and the retry storm is visible in the admin table — `SourceGateway/fetch`
  invocations went 212 → 488 across those 8 rows (≈35 attempts per row), each attempt paced by the
  exclusive readiness gate at ≈1/s, which is exactly the pathology `rearm: retryable` removes.
- **Owner-online export** — `result.xlsx` (8,241 B, sha256 `a4eb1fe7…`), `result.jsonl` (6,514,812 B,
  sha256 `87cfd651…`), `result.commit.json` (sha256 `a4f0791b…`, `protocol native-export-worker-v4`,
  `state: complete`, `completeness: complete`). The receipt's embedded verification block: source SHA-256
  `0a1d53f1…` matching before and after, 2/2 sheets, 8/8 source rows, 120 source fields, 30 headers.
- **Stopped-writer verification** — `hub stop lane-live-worker` (no lane-v14 writer process remained),
  then `verify --input … --output out-live-full-3/result.xlsx --sha256 0a1d53f1… --store live-store`
  returned **exit 0**: `output_sha256 a4eb1fe7…` matches the published workbook,
  `source_hash_before_matches`/`after` true, and the projection reports 8 review / 0 accepted /
  0 no-match / 0 pending. The same command is preserved as `lane-v14/post-run-chain.sh`.
- **Worker restart on the repaired binary** — `hub restart lane-live-worker` brought up pid `2094087`
  whose `/proc/<pid>/exe` sha256 is
  `508dc0949ca29c352855302dd1e117cc11848b95d873f39bad9f67c0c6516812` (the re-arm build), against the
  unchanged deployment `dp_15VGHjPnChtjVgUJzmtWTjb` (`http://127.0.0.1:21140/`, 13 services) — the frozen
  binary from the earlier collection was not reused.
- **Cached replay** — re-posting the identical `run_and_export` body answered `202` and
  `inv_1cu7U1qigWj13rYf3sEVBgVLOEzieYeuYF` completed in **0.5 s** (20:47:49.263Z → .798Z), routing through
  `ExportWorker/publish`. The three artifacts stayed **byte-identical**, `SourceGateway/fetch` stayed at
  **488 → 488** (zero new source requests), and the run stayed `complete: true` with its collection at
  `phase: Complete` / `final_snapshot 40792b83…`. A cached run therefore reproduces its publication
  without touching the source.

Operational scripts used (outside the repository, under `lane-v14/`):
`collection-supervisor.sh <run> [rounds]` (re-arms + resumes until every event is terminal) and
`row-supervisor.sh <run> [since-utc] [rounds]` (resumes parked invocations, re-arms on stall, stops on
`complete=true`); the run-specific `resume-supervisor.sh` / `run-supervisor.sh` and the step-by-step
`post-run-chain.sh` are retained beside them.

**Live expanded-roster run (2026-09-20):** girls indoor, run
`fe726b41…` — scope revision `2026-usa-hs-grade11-indoor-girls-v2`, list `173005`, 94 scheduled events,
`--rankings --rankings-gender f --rankings-season indoor`, auto-export to
`lane-v14/out-live-girls/result.xlsx`, supervised by hub process `live-girls-supervisor`. It ran live
under the same re-arm mitigation and completed the whole chain: 94/94 events over 816 pages, 8/8 rows
decided (8 `review_required`, 0 pending), publication `8322bae9…` / `44f9f773…` / `fc6c12fe…` with
`completeness: complete`, stopped-writer `verify` exit 0 against `live-store`, and a cached replay of its
own body with zero new source requests (3,616 → 3,616) and byte-identical artifacts. Its rows carry no
source-identity evidence, so the roster annotations are appended but empty; no other part of the expanded
roster (athlete URLs, school history, all high-school performances, PRs, XC) is delivered by it.

## Quality command ledger

**Executed (current tree):** `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and the 16-target test command — all green (226 tests re-verified 2026-09-20 on the current working tree), with the library at 149 passed / 2 ignored and every named target passing; the bounded-readiness escalation unit test (`can_escalate` over the state lattice); the live readiness-recovery cycles recorded above (fallback launch, bounded escalation, operator re-arm, restored `ready`); retained-corpus qualification; all 26 private storage scenarios; focused parser/storage/bundle regressions; the request-serialization regression that failed before its repair and passed afterward; the indoor girls and indoor boys runs with publication, stopped-writer verification, and cached replay; the outdoor boys pause/resume run; the verification-binding mutation check for the criterion tests (reverting `source_season_id()` fails all three rankings verifier tests; reverting the verifier's scope gender binding fails the gender rejection case — the restored tree passes); and the retained standalone `girls-store` verify re-run with the current binary ( exit 0, `output_sha256 b29af985…` matching the retained `out-girls/result.xlsx`, `detail_sha256 83524f6f…`, source SHA-256 `0a1d53f1…` matching before and after, 8 source rows -> 6 accepted / 1 no-match / 1 review / 0 pending).

**Latest complete command:** `cargo test --lib --bins --test rankings_parser --test rankings_catalog --test rankings_scope --test rankings_indoor --test rankings_storage --test profile_html_bounds --test profile_merge_bounds --test search_html_bounds --test workbook_verify --test workbook_zip_layout --test native_parser_properties --test result_verify --test bundle_verify --test ingress_failure_surface` — 16/16 targets green (re-run 2026-09-20 on the current working tree: 226 passed, 0 failed; library 149 passed / 2 ignored), including the previously failing restored fixture, the workbook targets it had blocked, the CLI guards that reject a division flag without `--rankings`, the fixture-driven rankings verifier cases, the bounded-readiness escalation test, and the operator ingress-failure surface above.

**Current-tree gate re-run (2026-09-20, canonical-URL and transport repair):** `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo build --locked` are clean, and the 16-target command above re-ran 16/16 green on the repaired tree (library 153 passed / 2 ignored, `result_verify` 14/14), including the new `navigation_url_is_already_canonical` unit test.

**scale-v15 stopped-writer verification (2026-09-20):** the frozen `out-v16` publication committed a complete `native-export-worker-v4` report — 120,716/120,716 aggregate rows across `Export` (111,939) and `Sheet1` (8,777), 1,569,308/1,569,308 source fields, 26/26 matched headers plus 11 appended, output sha256 `dee3ed6b…`, detail sha256 `1aead772…` — taken while the writer was stopped, with 117,887 rows explicitly pending while run `a8328b7f…` was still deciding rows.

**scale-v15 chain completed (2026-09-20):** run `a8328b7f…` reached `complete: true` with 5,000/5,000 selected rows decided (0 accepted, 3,405 review, 1,595 no-match) over the real 120,716-row two-sheet workbook (`per_sheet` 2,500, `concurrency` 8). The owner-online export then published `out-v17/scale-final.xlsx` (`7e0bc4b8…`), `scale-final.jsonl` (`d8ccfe00…`), and `scale-final.commit.json` (`cbead084…`): 120,716/120,716 output rows, 1,569,308/1,569,308 source fields preserved, 26/26 matched headers plus 15 appended, source digest `5969df23…`, and the 115,716 rows outside the selection explicitly pending. Stopped-writer `verify` exited 0 with the source SHA-256 matching before and after. The cached replay re-posted the identical request body: `202 Accepted`, invocations `PipelineControl/run_and_export` → `RunCoordinator/run` → `ExportWorker/publish` all completed within ~120 ms, zero new `SourceGateway/fetch` invocations (45,192 total, unchanged), the run left `complete` with its `updated_at` unchanged, and the three artifacts byte-identical. Workbook-scale identity-matched delivery for every row is still open: this run decided the bounded 5,000-row selection, not all 120,716.

**scale-v15 verify and replay re-confirmed (2026-09-21):** the pending item in that chain is closed. Stopped-writer `verify` exited 0 against `out-v17/scale-final.xlsx` on the current tree. The cached replay re-posted the run's own request body targeting `out/scale-2500.xlsx`: `HTTP 202`, `pages` unchanged at 79, `updated_at_unix_ms` unchanged at `1789927061965`, `pending_rows` empty, and all three artifact digests unchanged (`206d5653…` / `d8ccfe00…` / `016b28bd…`) — zero new source fetches and byte-identical artifacts. The three earlier `out/` publishes (`smoke`, `scale-partial`, `scale-2500`) fail the current verifier with `output worksheet header width differs from source plus supplied headers`; they came from an earlier exporter and, per the SCOPE qualification ledger, cannot certify this tree. `out-v17` is the publication that does.

**Current-tree gate re-run (2026-09-20, rankings transport retry):** `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo build --locked`, and the 16-target command above are green — library 155 passed / 2 ignored (the two new `receiptless_transport` tests), every integration target ok including `result_verify` 14/14. The live lane was rebuilt and its worker restarted on this tree; `browser-start` re-attached the headed session to `ready` within 12 s, and run `57f88b34…` resumed at `55m` page 31.

**Live-chain gate (2026-09-20, current tree at `3f78fd9`):** `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets --locked -- -D warnings`, and the 16-target command above re-ran green on the tree that served the live chain — **232 passed / 0 failed** (library 156 passed / 2 ignored, main 3, every integration target ok including `result_verify` 14/14). The chain executed end to end on that tree against the live source: collection `95/95` events / `1,065` pages / `final_snapshot 40792b83…` → row phase `8/8` (`complete: true`, 8 `review_required`, 0 pending) → owner-online export `result.xlsx` `a4eb1fe7…` / `result.jsonl` `87cfd651…` / `result.commit.json` `a4f0791b…` (`native-export-worker-v4`, `completeness: complete`, 2 sheets, 8 rows, 120 fields, 30 headers) → stopped-writer `verify` exit 0 with the same `output_sha256` and source digest `0a1d53f1…` → worker restarted on the re-arm build (`508dc0949ca29c352855302dd1e117cc11848b95d873f39bad9f67c0c6516812`) → cached replay `202` in 0.5 s, byte-identical artifacts, `SourceGateway/fetch` unchanged at 488.

**Next:** the live chain is executed for the indoor boys scope (run `57f88b34…`: 95/95 events terminal, 1,065 pages, `final_snapshot 40792b83…`, 8/8 rows decided, publication `a4eb1fe7…`, stopped-writer verification exit 0, byte-identical replay with zero new source requests on the re-arm binary `508dc094…`) and for the indoor girls scope (run `fe726b41…`: 94/94 events over 816 pages, 8/8 rows decided, publication `8322bae9…` / `44f9f773…` / `fc6c12fe…`, stopped-writer verification exit 0 against `live-store`, byte-identical cached replay with zero new source requests on build `cfdae271…`). Both outdoor scopes have since completed on the same tree: outdoor boys (run `7be7e9dd…`: 95/95 events over 385 pages, `final_snapshot ee97f6f1…`, 8/8 rows, publication `3ee370e1…` / `9060e7c0…` / `6b4011ac…`, stopped-writer verification exit 0, byte-identical cached replay with zero new source fetches) and outdoor girls (run `3b7c07fa…`: 94/94 events over 134 pages, 8/8 rows, publication `8656f9f0…` / `b64359d7…` / `7ea80cd5…`, stopped-writer verification exit 0, byte-identical cached replay with zero new source fetches); the row phase needed the frozen-coordinator repair twice on the girls run, and its first export died on `/tmp` exhaustion rather than source behaviour. What remains: both cross-country scopes and the athlete profile surface, which `SCOPE.md#source-surfaces-for-the-expanded-roster--discovery-findings` shows the qualified rankings API does not reach; workbook-scale identity-matched delivery (the real 120,716-row workbook is bound at ≈0.31 rows/s per lane, ≈108 h for `--all`; its bounded 5,000-row selection, stopped-writer verification, and cached replay are now qualified — see the scale-v15 chain entry above); and the expanded roster requirements in `SCOPE.md` (athlete URLs, school history, all available high-school performances, PRs, cross-country). Re-running the live lane needs the headed browser at `ready` on CDP `9333`: the profile is currently signed out (see the findings below), `browser-start` settles at `ready`, and acquisition still stalls on the residual transport failure recorded above — now attributed to profile-restored
tabs accumulating on every recovery relaunch. `Actor::bootstrap` closes those pages when the worker launches
its own browser; the live lane attaches to the persistent headed browser (`cdp_endpoint`), where foreign
pages are deliberately left alone, so the supervisor's prune is the operative control there. The verifier's rankings path now has its own frozen synthetic collection fixture (`src/result_verify/rankings/` driving `tests/fixtures/rankings/`) in addition to the executed indoor stopped-writer verification. No broad unit suite or fuzz campaign is required by this handoff.

## Live acquisition findings and parser repairs (2026-09-21)

The outdoor boys run `7be7e9dd…` (list `168416`, 95 events) exposed source and pipeline behavior the indoor runs never hit:

- **The lane's profile is signed out.** A read-only probe of the lane's headed Chromium (CDP `9333`) shows the source navigation offering `Log In` / `Sign Up` with no account node, so this session is anonymous. The earlier entitled runs of the same shape (indoor boys `57f88b34…`: 1,065 pages; indoor girls `fe726b41…`: 816 pages) paginated. This is the operative difference and the reason complete per-event evidence now needs the profile signed in again; the earlier statement that the profile is signed in is superseded.
- **The source answers an exhausted page request with an empty body.** Requesting page 246 of the live outdoor `100m` list returned `{}` (2 bytes), which the transport classified as a failure and retried indefinitely against a request the source will never serve. An empty body is now an empty terminal page.
- **The source restates the listing head for a refused page.** A page-2 request for `200m` returns a byte-identical page-1 body (`66498` bytes both times, `settings.page` 1). The parser ends the event's page chain there instead of raising a mismatch; only a genuinely different page identity remains an error.
- **The declared `minCount` is unreachable for this session.** Measured through the site's own UI in an independent browser, every page link after the first is `aria-disabled="true"` for `100m` and `200m` alike, while `minCount` reads 654 (`200m`), 701 (`100m`), and 1,288 (`4x100m`). The coverage guard now records a shortfall warning and seals what the source served; the terminal observation keeps `min_count`, so the gap stays visible in the evidence instead of wedging the run.
- **Relay pages arrive with masked rows.** The live `4x100m` page served 101 rows with 76 rosters, and 96 rows carry `AthleteID: 0` (only five are readable). A masked row whose roster cannot be joined is retained as `roster_present: Some(false)` and counted in `rows_missing_roster`, so relay events seal and the export reports `relay_membership_complete: false`; `4x100m` and `4x200m` sealed at pages 0/1 before this repair.
- **Acquisition pauses on `SourceFailure`** (the source refuses a fetch) and clears on `rankings-resume`. The run needed repeated resumes, which is the operator path the CLI already documents.

### Source-surface discovery for the expanded roster (2026-09-21)

Read-only probes through the same headed transport, run to scope the requested expansion before building against it, settled four questions (scripts and digests retained at `lane-v14/live-capture/probe-2026-09-21/`; summary in [SCOPE.md](SCOPE.md#source-surfaces-for-the-expanded-roster--discovery-findings)). An earlier revision of this block concluded the expansion needed a new source surface; that conclusion was wrong and is corrected here.

- **The rankings list API has no cross-country season.** `GetNavInfo`'s `seasons` map holds only indoor keys (`12004`–`12027`) and outdoor keys (`2004`–`2027`), its `events` list is track-only, and the payload contains no occurrence of cross-country at all.
- **`/CrossCountry/rankings` is a landing surface.** It returns `200` and renders a video page; it issues no rankings request. Cross-country *rankings* are meet-oriented: the site's own client picks `/api/v1/{tfRankings|xcRankings}/GetMeets` by sport, so `/api/v1/xcRankings/GetNavInfo` correctly `404`s without meaning the sport is unreachable.
- **The athlete bio surface is anonymously reachable and already answers the expanded roster.** The site builds profile URLs with its own `athleteBioUrl` pipe — `/athlete/{athleteId}/track-and-field` and `/athlete/{athleteId}/cross-country` — and `GET /api/v1/AthleteBio/GetAthleteBioData?athleteId={id}&sport={tf|xc}&level={0|4}` answered `200` while signed out, at 52,468 bytes for athlete `24416437` with `athlete`, `allSeasons`, `allTeams`, `grades`, `meets`, `resultsTF` (88 rows), `resultsXC`, `distancesXC`, `eventsTF`, and `relayTeamMembers`. `sport=xc` returned populated `resultsXC` for an athlete flagged `hasOtherSport`, and the XC bio route rendered a real bio page. The earlier "does not resolve anonymously" finding probed `/athlete/{id}` (missing the sport segment) and `/TrackAndField/athlete/{id}` (not the bio route), which is why it missed.
- **`level=4` scopes the bio to high school.** Five athletes from the live outdoor list: `resultsTF` 88 / 87 / 428 / 80 / 162 at `level=0` versus 73 / 74 / 148 / 71 / 68 at `level=4`. The in-tree builder asks for `level=0` and filters downstream.

The pipeline already implements this surface — `SourceResource::Bio` / `SourceResource::ProfileHtml` in `src/runtime/source/request/`, driven per athlete by `initial_resources` in `src/runtime/profile_worker/` for both sports plus the `/all` profile page — so the expansion needs an accepted identity to run, not a new collector. No collection was driven by these probes; they navigated and read only.

## Hardening wave 1 (2026-09-21, commit `cab6e5b`)

Per-file burndown of every tracked forbidden construct across both crates, plus recovery of the
work the 11:45 worktree reset had destroyed. Numbers, before/after tables and phase-2 notes live in
[`docs/HARDENING-PROGRAM.md`](docs/HARDENING-PROGRAM.md) § 8; the short version:

- **Gate PASS on all 11 lanes** on a clean target directory: fmt, check, doc, tests (465 run, 465
  passed, 2 skipped), strict clippy on source targets, production scan, domain type integrity, debt
  ratchet, deny, audit, machete, geiger, bench presence.
- Root strict clippy is **0** on every tracked lint; the census retains 68 arithmetic / 7
  as-conversion / 4 indexing diagnostics, all `[DOWN]` against the recorded baseline (Phase 1
  remainder: aggregation and adapter modules).
- Recovery verified by the scratch-store smoke run (`--store /tmp/census-smoke run --limit 3`)
  which produced both reports, both by-state CSVs, the best-results pair and a 6-sheet workbook,
  and by the census suite's 224 tests.
- Two unused dependencies dropped (`census` `unicode-normalization`, vendored
  `chromiumoxide_cdp` `chromiumoxide_pdl`). `cargo geiger` needs an unpolluted `target/`; stale
  `.d` files for deleted bench/bin targets make that lane fail with `Io(NotFound)` rather than an
  unsafe verdict — `cargo clean` before trusting it.
