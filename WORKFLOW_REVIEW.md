# Current native workflow review

## Reading scope

This packet is a current-source architecture map, not a historic alpha plan and not execution evidence. Read it with [README.md](README.md), [HANDOFF.md](HANDOFF.md), and [SCOPE.md](SCOPE.md). The five documents intentionally distinguish source-derived behavior, reported checks, retained historical evidence, and pending qualification.

The Rust module graph uses normal `mod` declarations, feature submodules, `#[path]` placement where a logical child is kept in a feature directory, `pub(crate)` implementation boundaries, and inline modules for tightly coupled tests/helpers. These are one compile-time graph; they are not duplicate collectors. Current source paths are authoritative over old line-number maps.

## Domain core versus effect shell

### Deterministic core

The core is responsible for values and decisions that can be recomputed from explicit inputs:

- Workbook decoding and row-identity calculations preserve every real source row and give each row a worksheet/physical-row key; filesystem reads and streaming ingestion are I/O adapters around those calculations.
- `domain::*` validates names, identity evidence, marks, performance context, and conservative decisions. Missing or conflicting evidence cannot become a positive match by model assertion.
- `runtime::rankings::{catalog, types, page::parse}` validates navigation scope, maps the 46-family manifest to observed variants (current target 95), excludes walk, parses individual/relay rows, and exposes Grade 11 candidates only as discovery evidence.
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

1. Does current code preserve all real rows from all source sheets without a cohort filter? Source contract: yes; full native execution remains pending.
2. Does optional ranking discovery remain separate from eligibility? The scope/type contracts require yes; native qualification remains pending.
3. Are page requests, response bytes, row positions, roster joins, and checkpoint identities provenance-bound? The source/store interfaces provide these fields; current repaired-binary execution must still prove the graph.
4. Are genuine pause and explicit resume durable? CLI and state interfaces exist; native pause/resume/recovery scenarios remain open.
5. Does failed ranking acquisition avoid stale failure replay? `SourceCache` explicitly avoids caching ranking failures; execution proof remains open.
6. Does export preserve source fields and sidecar overflow? Projection/publication and verifier paths provide the contract; current end-to-end proof remains open.

Current evidence is limited to a fresh production/test-library `cargo check`, a non-throughput allocation measurement (344,131 versus 13,131 across 1,000 synthetic iterations), and a private 26-case qualification that exposed bugs under repair. Strict Clippy still has one known trivial conversion. No live-collection readiness, passing 26-case qualification, or completed full collection/matching/export/replay claim is supported.
