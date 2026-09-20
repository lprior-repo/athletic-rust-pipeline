# Current native workflow review

## Reading scope

This packet is a current-source architecture map, not a historic alpha plan and not execution evidence. Read it with [README.md](README.md), [HANDOFF.md](HANDOFF.md), and [SCOPE.md](SCOPE.md). The five documents intentionally distinguish source-derived behavior, reported checks, retained historical evidence, and pending qualification.

The requested expansion is Grade 11/junior **boys and girls**, indoor/outdoor track **plus cross-country**, with complete available high-school performance histories, athlete URLs, competing-school history, and comparable PRs. [SCOPE.md](SCOPE.md#expanded-roster-target--requested-not-yet-complete) separates this target from the rankings collection, which is division-parameterized (gender x season kind, evidence-backed 2026 list ids) and fixture-qualified for the outdoor boys, indoor girls, and indoor boys divisions, with the other two divisions reachable but unexecuted. Rankings coverage alone cannot establish roster or career-history completeness.

The Rust module graph uses normal `mod` declarations, feature submodules, `#[path]` placement where a logical child is kept in a feature directory, `pub(crate)` implementation boundaries, and inline modules for tightly coupled tests/helpers. These are one compile-time graph; they are not duplicate collectors. Current source paths are authoritative over old line-number maps.

## Domain core versus effect shell

### Deterministic core

The core is responsible for values and decisions that can be recomputed from explicit inputs:

- Workbook decoding and row-identity calculations preserve every real source row and give each row a worksheet/physical-row key; filesystem reads and streaming ingestion are I/O adapters around those calculations.
- `domain::*` validates names, identity evidence, marks, performance context, and conservative decisions. Missing or conflicting evidence cannot become a positive match by model assertion.
- `runtime::rankings::{division, catalog, types, page::parse}` binds one evidence-backed division per scope (season kind x gender, 2026 list ids, per-division revision), validates navigation scope through the kind-specific `seasons` entry, maps the 46-family manifest to observed variants (current target 95), excludes walk, parses individual/relay rows, and exposes Grade 11 candidates only as discovery evidence.
- `runtime::export::projection` builds annotation fields, relocates oversized summaries to JSONL, and never mutates source fields.

### Runtime effect shell

Restate and worker modules own effects and durable ordering:

- `PipelineControl` prepares an immutable workbook/source snapshot and can submit `run-and-export`.
- `RunCoordinator` selects all/per-sheet rows, fans out bounded row work, seals immutable result pages, and publishes progress.
- `QueryWorker`/`ProfileWorker` call `SourceCache`; `SourceGateway` applies scoped admission, retry ownership, cooldown, and browser/source effects.
- `BrowserSession`/browser transport own headed Chromium, CDP observation, challenge readiness, physical gating, cancellation, drain, and shutdown.
- `RankingsCollectionState` schedules navigation/page steps, supports genuine pause and explicit resume/recovery, persists event checkpoints, and seals only a complete event plan.
- `ExportWorker` publishes immutable XLSX, JSONL detail, and receipt artifacts. `verify` reads an existing stopped store and independently checks source and evidence binding.
- `ArtifactStore` and `store::rankings` persist immutable evidence, page indexes, and lookup references with candidate kind, result ID, source row, checkpoint, and roster observations.

External HTTP is not transactional with Restate. Unacknowledged effects may repeat after crash recovery; evidence records distinguish SDK/workflow ownership, not an exactly-once guarantee.

## Main call chain

```text
main -> cli::run
  worker -> runtime::worker::serve -> Endpoint services
  deploy -> local admin capability/rule checks -> registration
  start -> PipelineControl.prepare -> RunCoordinator.run
  start --output -> PipelineControl.run_and_export -> RunCoordinator + ExportWorker
  status -> RunCoordinator.status
  browser-start/status -> BrowserSession await_ready/status
  rankings-status/pause/resume -> PipelineControl ranking controls
  export -> ExportWorker.publish -> durable output observation
  verify -> existing_stopped_store -> bundle_verify
```

Preparation identity includes the preparation revision, acquisition revision, source workbook digest, selection, concurrency, snapshot label, execution label, and optional rankings scope. Run identity separately binds the run revision, parser revision, prepared request, and immutable source snapshot. Reusing a digest does not prove artifacts exist; every store read verifies the referenced bytes.

## Ranking collection chain

```text
RankingsScope::requested
  -> PipelineControl / RankingsCollectionState
  -> SourceResource::Rankings(Navigation)
  -> SourceCache -> SourceGateway -> browser/source receipt
  -> EventCatalog::from_nav
  -> RankingsPlan (46 requested families, observed variants, absent families)
  -> SourceResource::Rankings(Results) per event/page
  -> parse_page_response
  -> RankingsPageCheckpoint + RankingPageIndex
  -> event terminality and lower-bound reconciliation
  -> immutable CollectionFinalSnapshot / sealed collection reference
  -> export coverage
```

The parser has a separate 8 MiB accounted ranking-capture bound; the general source-body cap is 32 MiB. Individual rows are accepted only for the expected Grade 11 context. Relay rows use roster joins and member Grade 11, never the relay row's placeholder grade. `SourceCache` caches retrieved outcomes and non-ranking failures; ranking failures are not cached, enabling a resumed source attempt to obtain fresh audit provenance.

## Storage, ownership, and publication

`ArtifactStore` is the worker-owned Fjall boundary. Content-addressed documents, source indexes, attempts, ranking page indexes, event statistics, and final seals are immutable or conflict-checked. `Restate` owns orchestration and checkpoints; Fjall is not a queue or a lease service. One writer owns a live store. Export is owner-online, then the writer is stopped before independent verification.

Publication is deliberately non-clobbering and multi-artifact: XLSX plus JSONL sidecar plus a final receipt. Original source columns and row positions remain present; annotation fields and long-form evidence are additive. A missing receipt or pending rows means publication is not complete.

## Review questions and status

1. Does current code preserve all real rows from all source sheets without a cohort filter? Source contract: yes. Fixture execution preserves all 8 source rows through projection and matching (6 accepted, 1 no-match, 1 review, 0 pending) with source hash before/after match. The live indoor boys run `57f88b34…` decided all 8 source rows against the real source (8 `review_required`, 0 pending — fail-closed, since the sample rows carry no source-identity evidence) and its publication passed stopped-writer `verify` with the source SHA-256 matching before and after. Workbook-scale volume remains pending.
2. Does optional ranking discovery remain separate from eligibility? The scope/type contracts require yes; both executed runs kept discovery separate from workbook membership, and the collection scopes were division-bound.
3. Are page requests, response bytes, row positions, roster joins, and checkpoint identities provenance-bound? The source/store interfaces provide these fields, and both executed runs' stopped-writer verification reconciled the retained raw evidence with the published XLSX/JSONL. Workbook-scale volume is now partly measured: a `scale-v15` owner-online export issued while a 5,000-row run was still collecting retained every one of the real workbook's 120,716 rows (462 completed, 120,254 explicitly pending) and reconciled source SHA-256 and all 1,569,308 source fields.
4. Are genuine pause and explicit resume durable? Executed on the outdoor boys run: `rankings-pause` latched `{"Paused":"Manual"}` with `pause_reason: Manual` and froze source traffic for the full 25 s observation window (0 new requests); `rankings-resume` returned the run to collection, which completed and published.
5. Does failed ranking acquisition avoid stale failure replay? `SourceCache` explicitly avoids caching ranking failures; execution proof of a failure path remains open.
6. Does export preserve source fields and sidecar overflow? Projection/publication and verifier paths provide the contract; the executed indoor girls run published XLSX + JSONL + receipt, verified clean against the stopped store, and replayed to byte-identical artifacts with zero new source requests.

Executed evidence now includes the native retained-corpus comparison (95 queries, 4,256 receipts, 142,705 unique athletes), all 26 private storage scenarios, focused parser/storage/bundle and request-serialization regressions, green current-tree gates (formatter, `check`, strict Clippy, 15/15 test targets, 234 passed re-verified 2026-09-20 on the CDP-patched tree, with the library at 156 passed / 2 ignored), and live readiness-recovery cycles on the `lane-v14` cell (managed-browser fallback when the configured CDP endpoint is absent, bounded escalation to `human_required` at the 30 s window instead of a deadline-length wait, and an operator re-arm that re-runs the recovery navigation), and fixture-mode end-to-end qualification for the outdoor boys, indoor girls, and indoor boys divisions — including stopped-writer verification and exact cached replay. A live-source capture attempt on 2026-09-19 was blocked by Cloudflare (`403`, `Attention Required!`) before any real response could be retained; the headed-profile capture on 2026-09-20 executed instead, retaining one real indoor navigation and boys/girls division pages (`200` responses, sha256 provenance in `lane-v14/live-capture/CAPTURE.md`) that confirm `seasons["12026"] = 173005` and `division.ID = 173005` on live data. The allocation measurement remains 344,131 versus 13,131 across 1,000 synthetic iterations, not throughput evidence. On the workbook-scale lane the source-operation rate is framed by the single-key exclusive `BrowserSession/await_ready` handler: it is entered exactly once per source operation, its 0.329 s mean duration equals the observed operation period (0.341 rows/s, 2.982 ops/s at `tabs = 8` and `row_concurrency = 8`), and neither the page pool nor coordinator concurrency moves it; a shared `capture_ready` fast path measured slower (1.549 ops/s) wherever the readiness bootstrap cannot answer and was reverted. A separate `scale-v15` lane then ran the **real 120,716-row two-sheet workbook** through the fixture transport rather than the 8-row synthetic copy: a 10-row smoke run published a verifier-matched XLSX plus an 83.7 MB JSONL sidecar, an owner-online export issued during an unfinished 5,000-row run retained all 120,716 rows (120,254 explicitly pending) with source SHA-256 and all 1,569,308 fields reconciled, and measured non-rankings throughput is ≈0.31 rows/s at `row_concurrency = 8` (≈50 durable invocations across ≈10 source operations per row, zero retries, zero model calls) — unchanged at ≈0.27 rows/s at `row_concurrency = 32`, with two workers together reaching ≈0.9 rows/s near-idle and no new fixture requests across a 30 s / 24-row window, so the ≈108 h full-workbook bound is per-worker waiting inside the row's source-operation chain rather than coordinator concurrency. That hop is now measured — and its cause corrected: sampling `/proc/<pid>/io` during collection showed ≈11 MB/s of device writes across the Restate node and worker (≈12–14 MB per completed row) with both processes near-idle, but standing queries over the retained journals show the *logical* durable cost is small (`sys_journal.raw_length`: 51.5 KB per decided row on `scale-v15`, 112.5 B/entry; `restate-data` 110 KB/row; store 36 KB/row; the collection path journals ≈800 KB per ranking page). Device write volume is therefore LSM compaction and fsync churn plus a debug-level worker log, not payload size — the levers are fewer durable steps per row (~46 invocations) and storage/log configuration. The live chain has since executed for one indoor boys division scope (run `57f88b34…`: 95/95 events terminal, 1,065 pages, `final_snapshot 40792b83…`, 8/8 rows decided, publication `a4eb1fe7…` / `87cfd651…` / `a4f0791b…` with `completeness: complete`, stopped-writer `verify` exit 0, byte-identical cached replay with zero new source requests) under the re-arm mitigation for the CDP frame drops (cause repaired in-tree; the mitigation stays until a live collection completes without it), and the current-tree gates re-ran green on that tree (232 passed / 0 failed, library 156 passed / 2 ignored). Workbook-scale identity-matched delivery and expanded-roster coverage remain unproved; the indoor girls live chain has since completed end to end (run `fe726b41…`: 94/94 events over 816 pages, 8/8 rows decided, publication `8322bae9…` / `44f9f773…` / `fc6c12fe…`, stopped-writer `verify` exit 0, byte-identical cached replay with zero new source requests).
