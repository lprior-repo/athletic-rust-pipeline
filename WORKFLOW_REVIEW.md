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

1. Does current code preserve all real rows from all source sheets without a cohort filter? Source contract: yes. Fixture execution preserves all 8 source rows through projection and matching (6 accepted, 1 no-match, 1 review, 0 pending) with source hash before/after match. Live-source volume remains pending.
2. Does optional ranking discovery remain separate from eligibility? The scope/type contracts require yes; both executed runs kept discovery separate from workbook membership, and the collection scopes were division-bound.
3. Are page requests, response bytes, row positions, roster joins, and checkpoint identities provenance-bound? The source/store interfaces provide these fields, and both executed runs' stopped-writer verification reconciled the retained raw evidence with the published XLSX/JSONL. Workbook-scale volume proof remains open.
4. Are genuine pause and explicit resume durable? Executed on the outdoor boys run: `rankings-pause` latched `{"Paused":"Manual"}` with `pause_reason: Manual` and froze source traffic for the full 25 s observation window (0 new requests); `rankings-resume` returned the run to collection, which completed and published.
5. Does failed ranking acquisition avoid stale failure replay? `SourceCache` explicitly avoids caching ranking failures; execution proof of a failure path remains open.
6. Does export preserve source fields and sidecar overflow? Projection/publication and verifier paths provide the contract; the executed indoor girls run published XLSX + JSONL + receipt, verified clean against the stopped store, and replayed to byte-identical artifacts with zero new source requests.

Executed evidence now includes the native retained-corpus comparison (95 queries, 4,256 receipts, 142,705 unique athletes), all 26 private storage scenarios, focused parser/storage/bundle and request-serialization regressions, green current-tree gates (formatter, `check`, strict Clippy, 15/15 test targets, 224 passed, with the library at 148 passed / 2 ignored), and live readiness-recovery cycles on the `lane-v14` cell (managed-browser fallback when the configured CDP endpoint is absent, bounded escalation to `human_required` at the 30 s window instead of a deadline-length wait, and an operator re-arm that re-runs the recovery navigation), and fixture-mode end-to-end qualification for the outdoor boys, indoor girls, and indoor boys divisions — including stopped-writer verification and exact cached replay. A live-source capture attempt on 2026-09-19 was blocked by Cloudflare (`403`, `Attention Required!`) before any real response could be retained. The allocation measurement remains 344,131 versus 13,131 across 1,000 synthetic iterations, not throughput evidence. Live-source collection, workbook-scale identity matching, and expanded roster coverage remain unproved.
