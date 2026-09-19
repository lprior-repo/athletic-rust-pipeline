# Native athlete evidence pipeline

This repository contains the native Rust/Restate athlete-evidence pipeline. It ingests an XLSX workbook, acquires bounded source evidence through the headed Chromium transport, performs deterministic parsing and identity assessment, optionally sends genuine ambiguity to one assigned local model lane, and publishes an immutable XLSX plus JSONL evidence sidecar.

This README describes the **current design**. It is not a claim that live collection, full-workbook matching, or end-to-end replay qualification has completed. Historical experiments are labelled as historical in [HANDOFF.md](HANDOFF.md).

The clarified expansion target is **USA Grade 11/junior boys and girls, indoor track, outdoor track, and cross-country**, with athlete URLs, competing-school history, all available high-school performances, and evidence-backed event-specific PRs. See [the expanded roster contract](SCOPE.md#expanded-roster-target--requested-not-yet-complete). This target is broader than the implemented boys rankings collector and is not a completed-delivery claim.

## Product contract

- Workbook membership is the population contract. Every real source row is retained, including duplicate and incomplete identity rows. There is no cohort, grade, graduation-year, or junior eligibility filter.
- For the supplied two-sheet source shape, both source worksheets are included; no sheet is dropped because of sport or grade. The completed consolidated rankings workbook contains 142,705 unique athletes on one `AllAthletes` sheet. It is a separate rankings delivery, not an identity-matched copy of the original workbook.
- Original source fields remain immutable in the export. Added annotations never replace source columns. Oversized summaries move to the JSONL sidecar rather than being truncated.
- Optional Grade 11 rankings are **discovery evidence only**, never eligibility. The requested manifest has 46 event families; navigation expands observed families to variants (the current design target is 95 variants), excludes walk events, and joins relay rows through the returned roster. A Grade 11 projection is a source projection, not a cohort gate.
- Candidate evidence is conservative: missing, conflicting, or incomplete evidence remains review/pending evidence rather than a guessed match. Email, street, and postal fields remain private context and are not sent to hosted services.

## Architecture

The **deterministic core** computes source-row identities, requests, parsed observations, event and mark comparisons, candidate assessments, provenance checks, and export projections from explicit inputs. Workbook reading, ranking-index persistence, and artifact publication are I/O adapters, not pure domain operations. Public IDs, digests, row references, page/checkpoint references, and request provenance are stable serialized contracts.

The **runtime effect shell** is Restate plus its native workers and stores. It owns durable invocation state, source admission, timers, retry/attempt ownership, Chromium lifecycle and challenge readiness, worker cancellation/drain, checkpoints, collection pause/resume, sealing, and publication. External HTTP is not exactly-once; an unacknowledged effect may repeat after recovery and is retained as uncertain-effect evidence.

The source path is:

```text
CLI -> PipelineControl / RunCoordinator
   -> WorkbookImport -> immutable source manifest and row index
   -> RowWorker -> QueryWorker/ProfileWorker -> SourceCache
   -> SourceGateway + Restate admission -> headed Chromium/CDP capture
   -> deterministic parsers and identity decision
   -> optional one-lane local review
   -> immutable result pages / ranking checkpoints
   -> ExportWorker -> XLSX + JSONL sidecar + receipt
```

`src/runtime/rankings/page/parse/` is a deliberately split parser module. `src/runtime/rankings/page/parse.rs` declares `main` and `relay` and re-exports `parse_page_response` and `PageParseError` from `main`. Elsewhere, `#[path]` declarations and inline modules place implementation details in feature directories without creating alternate collectors. `pub(crate)` boundaries keep internal helpers separate from explicit public serialized types.

## Rankings collection

`start --rankings` creates a scope for the fixed list, gender, season, event-family manifest, projection grade, and page cap. `EventCatalog::from_nav` validates list/season metadata, removes walk events, rejects malformed or conflicting event IDs, records absent families, and expands each observed variant into a `RankingsPlan`. Individual pages accept Grade 11 candidates. Relay pages carry system placeholder grade metadata; only a roster member whose relay-team ID joins the row and whose member grade is 11 becomes a projected candidate.

Each successful source response has a content digest, request URL/method/body, status/media type, byte count, timing, and ranking capture metadata. Page publication stores the raw receipt, parsed observation, immutable checkpoint, and `RankingPageIndex` with source row positions, candidates, and roster observations. A collection seals only when every planned event page is terminal and the final snapshot binds the same scope and source snapshot.

The source body bound is 32 MiB. Ranking page parsing has a separate 8 MiB accounted capture bound; neither is a total-process RSS guarantee. `SourceCache` caches successful outcomes. Failed rankings outcomes are intentionally not cached, so a resumed gateway operation receives a fresh audit identity and can retry the source rather than replaying a cached failure.

## Browser and source safety

Chromium runs normally, headed, with one private persistent profile and a bounded tab pool (two by default, configurable within the validated limit). Chrome owns cookies and storage. The pipeline does not extract/replay cookies, spoof identity, patch webdriver, rotate profiles, use proxies/CAPTCHA services, or use a direct source-HTTP fallback. `reqwest` is restricted to local model/control traffic.

A definitive challenge closes new profile admission while already-issued requests drain. Automatic readiness is bounded; unresolved challenges enter durable `human_required`. Only an actual successful, non-challenged source document allows resumption. Real Cloudflare interaction is manual human work in the same headed profile; no live challenge was automated for this documentation refresh.

## CLI surface

The current executable exposes:

```text
worker, deploy, start, status,
browser-start, browser-status, export, verify,
rankings-status, rankings-pause, rankings-resume
```

`start` requires `--per-sheet N` or `--all`, an input path, and its pre-recorded SHA-256. `--rankings` enables the optional discovery collection. `--output PATH` submits `run-and-export`, which queues the durable run and export publication automatically; the returned `submitted` state is not completion. Inspect status and run independent verification after the owner has published and the writer is stopped.

Examples (use private, already configured endpoints and new output paths):

```sh
cargo run --release -- worker --config config.native.toml --bind 127.0.0.1:19181
cargo run --release -- deploy --admin http://127.0.0.1:19070/ --endpoint http://127.0.0.1:19181/
cargo run --release -- start --input /path/input.xlsx --sha256 SHA256 --all --output /path/result.xlsx
cargo run --release -- rankings-status --run RUN_DIGEST
cargo run --release -- rankings-pause --run RUN_DIGEST
cargo run --release -- rankings-resume --run RUN_DIGEST
cargo run --release -- verify --input /path/input.xlsx --output /path/result.xlsx --sha256 SHA256 --store /path/stopped/store
```

`verify` requires the existing stopped writer-owned store. Never open a live Fjall directory from a second process, and never treat a missing publication receipt as an atomic multi-file export.

## Qualification status

The fresh evidence currently available is limited and explicitly scoped:

- Fresh production/test-library compilation succeeded. Strict Clippy passed at an earlier integration checkpoint; the subsequent storage/request repairs still require the final current-tree gate.
- A serialization measurement compared 344,131 with 13,131 allocations over 1,000 synthetic iterations, 331 fewer allocations per iteration. This is an allocation observation, not a throughput claim.
- The native parser passed the retained corpus: 95 queries, 4,256 receipt digest checks, 340,238 individual results, 57,629 relay-member results, and 142,705 unique athletes. The report retains 15,724 unresolved roster results rather than inventing members.
- All 26 private storage scenarios passed after repairing replay, counters, and multiple references per athlete/checkpoint. The distinct-position oracle was corrected to count positions, not result/position pairs.
- Focused parser/storage/bundle regressions and the durable request-serialization regression passed. The restored result verifier passed 13 of 14 scenarios; one fixture still uses an obsolete assessment enum spelling. The later workbook targets did not run because that command stopped at the failure.
- The private native fixture layout is Restate admin 21041, ingress 21042, fixture 21043, and worker 21140; the worker is not deployed. These are qualification details, not readiness evidence.
- Full native automated collection -> matching -> export -> replay remains pending execution. No live-collection readiness claim is made.

Quality commands are plans until their result is recorded for the current tree. Do not infer current status from historical build, formatter, Clippy, unit, fuzz, or benchmark records. Removed alpha/exhaustive/restate-native commands, removed tools, stale benchmark targets, and campaign documents are not part of the interface.
