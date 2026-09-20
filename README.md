# Native athlete evidence pipeline

This repository contains the native Rust/Restate athlete-evidence pipeline. It ingests an XLSX workbook, acquires bounded source evidence through the headed Chromium transport, performs deterministic parsing and identity assessment, optionally sends genuine ambiguity to one assigned local model lane, and publishes an immutable XLSX plus JSONL evidence sidecar.

This README describes the **current design**. It is not a claim that workbook-scale matching or expanded-roster delivery has completed. The live rankings collection has executed against the real source (one 95-event indoor boys division scope, recorded in the qualification status below), and fixture-mode end-to-end qualification (collection -> matching -> owner-online export -> stopped-writer verification -> cached replay) is executed for the outdoor boys, indoor girls, and indoor boys divisions; historical experiments are labelled as historical in [HANDOFF.md](HANDOFF.md).

The clarified expansion target is **USA Grade 11/junior boys and girls, indoor track, outdoor track, and cross-country**, with athlete URLs, competing-school history, all available high-school performances, and evidence-backed event-specific PRs. See [the expanded roster contract](SCOPE.md#expanded-roster-target--requested-not-yet-complete). The rankings collector is division-parameterized (gender x season kind) and evidence-backs both 2026 division lists, but this target remains broader than anything qualified end-to-end and is not a completed-delivery claim.

## Product contract

- Workbook membership is the population contract. Every real source row is retained, including duplicate and incomplete identity rows. There is no cohort, grade, graduation-year, or junior eligibility filter.
- For the supplied two-sheet source shape, both source worksheets are included; no sheet is dropped because of sport or grade. The completed consolidated rankings workbook contains 142,705 unique athletes on one `AllAthletes` sheet. It is a separate rankings delivery, not an identity-matched copy of the original workbook.
- Original source fields remain immutable in the export. Added annotations never replace source columns. Oversized summaries move to the JSONL sidecar rather than being truncated.
- Expanded-roster annotations come only from accepted profile evidence: `native.competing_school` for the newest evidenced season, `native.school_history` for every season/school pair (a transfer does not rewrite the school behind earlier seasons), `native.junior_evidence` for the grade rows exactly as observed, and `native.junior_status` for the newest grade row's season-qualified verdict, left empty when no standard US high-school grade was observed. Name, age, and graduation year are never consulted, and the profile evidence plus JSONL sidecar remain the lossless record behind each value.
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

`start --rankings` creates a scope for one division: fixed list, gender, season, event-family manifest, projection grade, and page cap. Division selection is explicit (`--rankings-gender {m,f}`, `--rankings-season {outdoor,indoor}`); the evidence-backed 2026 table maps outdoor to list `168416` and indoor to `173005`, and each division carries its own revision (`2026-usa-hs-grade11-<kind>-<boys|girls>-v2`). `EventCatalog::from_nav` selects the configured list through the kind-specific `seasons` entry (`"2026"` outdoor, `"12026"` indoor), requires the navigation's own division to be that list or its level, removes walk events, rejects malformed or conflicting event IDs, records absent families, and expands each observed variant into a `RankingsPlan`. Individual pages accept Grade 11 candidates. Relay pages carry system placeholder grade metadata; only a roster member whose relay-team ID joins the row and whose member grade is 11 becomes a projected candidate.

Each successful source response has a content digest, request URL/method/body, status/media type, byte count, timing, and ranking capture metadata. Page publication stores the raw receipt, parsed observation, immutable checkpoint, and `RankingPageIndex` with source row positions, candidates, and roster observations. A collection seals only when every planned event page is terminal and the final snapshot binds the same scope and source snapshot.

The source body bound is 32 MiB. Ranking page parsing has a separate 8 MiB accounted capture bound; neither is a total-process RSS guarantee. `SourceCache` caches successful outcomes. Failed rankings outcomes are intentionally not cached, so a resumed gateway operation receives a fresh audit identity and can retry the source rather than replaying a cached failure.

## Browser and source safety

Chromium runs normally, headed, with one private persistent profile and a bounded tab pool (two by default, configurable within the validated limit). Chrome owns cookies and storage. The pipeline does not extract/replay cookies, spoof identity, patch webdriver, rotate profiles, use proxies/CAPTCHA services, or use a direct source-HTTP fallback. `reqwest` is restricted to local model/control traffic.

A definitive challenge closes new profile admission while already-issued requests drain. Automatic readiness is bounded; unresolved challenges enter durable `human_required`. Only an actual successful, non-challenged source document allows resumption. Real Cloudflare interaction is manual human work in the same headed profile; no live challenge was automated for this documentation refresh.

Exhaustion ends that wait instead of looping it: once a challenge or a stalled relaunch escalates, `await_ready` returns the durable status rather than polling to the readiness deadline. An explicit operator re-run of `browser-start` re-arms exactly one bounded attempt (a real recovery navigation) instead of reporting the same stalled state; internal waiters never clear a stall. A configured CDP endpoint that is not listening falls back to launching a managed browser on the configured profile instead of failing outright.

## CLI surface

The current executable exposes:

```text
worker, deploy, start, status,
browser-start, browser-status, export, verify,
rankings-status, rankings-pause, rankings-resume
```

`start` requires `--per-sheet N` or `--all`, an input path, and its pre-recorded SHA-256. `--rankings` enables the optional discovery collection, with `--rankings-gender` and `--rankings-season` selecting the division. `--output PATH` submits `run-and-export`, which queues the durable run and export publication automatically; the returned `submitted` state is not completion. Inspect status and run independent verification after the owner has published and the writer is stopped.

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

- Current-tree gates are green: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo build --locked` are clean, and the 16-target test command passes in full (re-verified 2026-09-20 on the current working tree; library 155 passed / 2 ignored, every named target green, `result_verify` 14/14).
- The restored fixture is repaired: `result_verify` passes 14/14, and the workbook targets that the earlier failing command blocked now run.
- A serialization measurement compared 344,131 with 13,131 allocations over 1,000 synthetic iterations, 331 fewer allocations per iteration. This is an allocation observation, not a throughput claim.
- The native parser passed the retained corpus: 95 queries, 4,256 receipt digest checks, 340,238 individual results, 57,629 relay-member results, and 142,705 unique athletes. The report retains 15,724 unresolved roster results rather than inventing members.
- All 26 private storage scenarios passed after repairing replay, counters, and multiple references per athlete/checkpoint. The distinct-position oracle was corrected to count positions, not result/position pairs.
- Focused parser/storage/bundle regressions and the durable request-serialization regression passed.
- The private native fixture deployment is Restate admin 21041, ingress 21042, fixture 21046, worker 21140, with the worker deployed and serving the fixture transport.
- End-to-end fixture qualification has executed for the indoor girls, indoor boys, and outdoor boys divisions: collection -> deterministic matching -> owner-online export -> stopped-writer verification -> cached replay. Indoor girls run `6f3c0c59…` bound list `173005` through revision `2026-usa-hs-grade11-indoor-girls-v2`, completed 95 of 95 events, published its XLSX/JSONL/receipt, passed `verify` against the stopped store (source hash before/after match; 6 accepted, 1 no-match, 1 review, 0 pending), and replayed with zero new source requests and byte-identical output. Outdoor boys run `3c726c6d…` bound list `168416`, froze source traffic for a full 25 s pause window, and completed after explicit resume. Indoor boys run `f0ef3948…` ran the same lane and store: published XLSX `6403fb0a…` and JSONL `86323db6…`, passed `verify` with the writer stopped (8 rows -> 6 accepted, 1 no-match, 1 review, 0 pending), and replayed to byte-identical artifacts with zero new source requests.
- A separate `scale-v15` lane (own Restate identity, fixture transport, worker binary sha256 `0f044e59…`) ran the pipeline against the **real 120,716-row two-sheet workbook** rather than the 8-row synthetic copy: a 10-row smoke run published a verifier-matched XLSX (111,939 + 8,777 rows, 1,569,308/1,569,308 source fields, 26/26 headers) with an 83.7 MB JSONL sidecar; an owner-online export issued while a 5,000-row run was still collecting published all 120,716 rows with 462 completed and 120,254 explicitly pending, matching source SHA-256 before and after. Measured non-rankings throughput is ≈0.31 rows/s at `row_concurrency = 8`, framed by the single-key exclusive `BrowserSession/await_ready` gate that every source operation enters exactly once (≈0.34 s per entry; `tabs` and `row_concurrency` changes do not move it; ≈50 durable invocations and ≈10 source operations per row, zero retries, zero model calls); a second identical lane at `row_concurrency = 32` measured ≈0.27 rows/s, so coordinator concurrency is not the bound, while two workers together sustained ≈0.9 rows/s with every process near-idle and no new fixture requests across a 30 s / 24-row window. At ≈3.2 s per row a full workbook pass is ≈108 h. Sampling `/proc/<pid>/io` during collection isolated the bound: ≈11 MB/s of durable writes across the Restate node (≈8.1 MB/s) and worker (≈2.9 MB/s), i.e. ≈12–14 MB per completed row, with both processes near-idle — journal/Fjall write volume, not source I/O, sets the rate. Stopped-writer verification for this lane is executed: the frozen `out-v16` publication committed a complete `native-export-worker-v4` report (120,716/120,716 aggregate rows, 1,569,308/1,569,308 source fields, 26/26 matched headers plus 11 appended, output sha256 `dee3ed6b…`, detail sha256 `1aead772…`) while the writer was stopped, with 117,887 rows explicitly pending because the source run was interrupted; cached replay for this lane remains pending.
- A live-source capture attempt on 2026-09-19 (dedicated headless Chromium, fresh profile, production request shapes issued in-page from the source origin) was blocked by Cloudflare: the first navigation and both indoor ranking requests returned `403` / `Attention Required!`. Raw block-page bodies, sizes, and sha256 are retained under `lane-v14/live-capture/`. The 2026-09-20 capture through the authorized **headed** profile executed instead: the indoor `GetNavInfo` (69,343 bytes boys / 69,339 girls) and the boys/girls `GetRankings` POST (48,087 / 52,936 bytes) returned `200` with retained sha256 provenance, and the live pages report `seasons["12026"] = 173005` with `division.ID = 173005` over real grade-11 100m results. A live collection run through the pipeline has since executed (run `57f88b34…`, 95/95 events terminal, 1,065 pages, `final_snapshot 40792b83…`), under the re-arm mitigation for the unsolved CDP client drift recorded in [HANDOFF.md](HANDOFF.md).
- The 2026-09-20 headed capture was extended to the page-2 and per-event payload shape: the source's own `page=2` requests returned byte-identical page-1 payloads (`settings.page = 1`, ranks 1..101) for `100m` and `200m`, so the live list report serves one page per event; responses declare `settings.depth = 100` with `blurAfterDepth = 5`, leaving five rows per event unblurred and the remainder masked for this session. The pagination rule now requires a page to fill its declared depth before requesting a successor, so a complete short page ends the chain instead of asking for a page the source answers with page-1 content.
- The live chain is executed for the indoor boys scope, not just its collection: run `57f88b34…` acquired 95/95 events (1,065 pages, `final_snapshot 40792b83…`), decided 8/8 workbook rows, published `lane-v14/out-live-full-3/result.xlsx` (`a4eb1fe7…`) with its JSONL sidecar (`87cfd651…`) and receipt (`a4f0791b…`, `native-export-worker-v4`, `completeness: complete`), passed `verify --store live-store` against the stopped writer (exit 0, source SHA-256 `0a1d53f1…` matching before and after, 8 rows → 8 review / 0 pending), and replayed byte-identically on the re-arm build (`508dc094…`) with zero new source requests (488 → 488). Workbook-scale identity matching and the expanded roster (girls indoor/outdoor, XC, athlete URLs, school history, all high-school performances, PRs) are not qualified; the girls indoor live collection is in progress (run `fe726b41…`, list `173005`, 94 events). Every live decision in that sample is `review_required` because the sample rows carry no source-identity evidence. A completed live chain is not a broad readiness claim while the CDP client drift recorded in [HANDOFF.md](HANDOFF.md) remains unresolved.

Quality commands are plans until their result is recorded for the current tree. Do not infer current status from historical build, formatter, Clippy, unit, fuzz, or benchmark records. Removed alpha/exhaustive/restate-native commands, removed tools, stale benchmark targets, and campaign documents are not part of the interface.
