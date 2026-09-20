# Current scope and architecture

This document is the current product and source scope. It replaces historic alpha/exhaustive/run-restate descriptions; those commands and tools are not part of the current CLI.

## Expanded roster target — requested, not yet complete

The clarified target is the existing USA/2026 scope with **source-verified Grade 11/junior boys and girls**, covering **indoor track, outdoor track, and cross-country**. It is not an all-grade or alumni census. Keep the previous race-walking exclusion. Junior status requires season-bound source evidence; do not infer it from a name, age, or assumed graduation year. This roster filter does not remove rows from an original workbook supplied for enrichment.

The required records are:

- **Athletes:** stable source athlete ID, name, athlete/profile URLs, evidenced junior status, source season, and competing school. Preserve conflicting or unknown observations explicitly.
- **Performances:** every available high-school result for the selected athletes, not just their ranking entries or junior-year bests. Retain raw mark, normalized value where supported, sport, event, indoor/outdoor context, timing, wind, implement/hurdle specification, date, meet, result URL, source result ID, and competing school/team at that performance.
- **Personal records:** event-specific comparable bests with links to their supporting results. Keep source-reported PR claims distinct from calculated bests among captured results. Missing history cannot establish a verified career PR. Cross-country comparisons retain distance and course context; a 5 km time and a three-mile time are not interchangeable.
- **School history:** retain school identity and the source season/result supporting it. A transfer must not rewrite the school attached to earlier performances.
- **Coverage:** separate discovered athletes, verified juniors, completed source requests, available performance histories, and unavailable/blocked evidence. A completed rankings plan is not proof of a complete roster or career history.

Full histories must remain tabular and lossless. Partition performance worksheets when Excel row limits require it, retain stable athlete/result links across sheets, and preserve the canonical JSONL evidence sidecar. Do not replace complete histories with a truncated summary cell.

Restate must own bounded scheduling, shared source admission, retry/cooldown state, durable checkpoints, and recovery. Prefer available authorized bulk responses, deduplicate athlete/profile work across events, and reuse immutable evidence. Bound network, CPU, storage, and local-model concurrency independently. Measure throughput, latency, requests per athlete, cache reuse, allocations, and throttle/challenge counts before increasing admission. Honor `Retry-After` and stop new profile admission on a challenge; never promise zero upstream 429s or treat evasion as optimization.

Implementation retains the agreed functional Rust/Holzman discipline: validated domain types, pure calculations separated from effects, explicit errors, bounded resources, owned cancellation/drain, and no avoidable hot-path cloning or allocation. The actual local 5090/3090 endpoints support development; model output requires source review and executable verification.

**Current gap:** the native rankings scope is division-parameterized and both 2026 divisions have now executed end-to-end fixture qualification (outdoor boys list `168416`, indoor girls list `173005`, each under its own revision). `RankingsScope::for_division` builds one seasonal division at a time from an evidence-backed table with a per-division revision, and `start --rankings-gender {m,f} --rankings-season {outdoor,indoor}` selects it. The retained 142,705-athlete delivery still covers the outdoor boys rankings dataset only, live-source capture is unqualified, and the expanded roster below remains unimplemented: existing profile/domain code supports track and cross-country evidence, but that does not prove comprehensive girls coverage, complete histories, school histories, or the expanded roster export. Those are new acceptance criteria, not completed features.

## End-to-end shape

```text
original XLSX (all real source rows, all source sheets)
  -> streaming workbook import + immutable source manifest
  -> stable worksheet/physical-row identities
  -> deterministic search/profile evidence and optional rankings discovery
  -> bounded Chromium/CDP source effects through Restate
  -> conservative identity assessment
  -> one assigned local model only for genuine ambiguity
  -> immutable row pages and ranking checkpoints
  -> optional Grade 11 ranking projection (discovery only)
  -> immutable XLSX + JSONL sidecar + publication receipt
  -> stopped-writer independent verification and exact replay
```

Workbook membership is eligibility. There is no cohort, grade, graduation-year, junior, or source-sport filter. Every real row remains accounted for, including duplicates and rows without identity fields. Generated sheets such as `Athletic Matches`, `Corrections`, and `Summary` are output artifacts rather than source population.

The completed consolidated rankings workbook contains 142,705 unique athletes on one `AllAthletes` sheet. It is a separate rankings delivery, not an identity-matched original workbook; matching its athletes to original source rows requires its own native run and verifier evidence.

## Ownership boundaries

| Area | Owns | Does not own |
|---|---|---|
| Workbook/domain/parser core | Streaming records, stable IDs, deterministic validation, ranking parsing, identity policy, mark arithmetic | Browser lifecycle, timers, model transport, eligibility shortcuts |
| Restate runtime | Durable calls, scheduling, retries, admission, cooldown, pause/resume, checkpoints, cancellation/drain | Exactly-once external HTTP, truth of upstream source |
| Chromium transport | Normal headed profile, tabs, CDP request/response identity, decoded bounded bytes, challenge gate | Cookie extraction/replay, evasion, source-rate authorization |
| SourceCache/Gateway | Snapshot-bound source outcomes, fresh ranking failures, bounded attempt evidence | Candidate decisions, stale failure success |
| Ranking collection | 46-family manifest, observed variant catalog, walk exclusion, page/roster joins, sealed indexes | Workbook eligibility or cohort selection |
| ArtifactStore | Immutable documents, row/source indexes, attempts, ranking indexes and seals | Live queue/lease, second writer, external truth |
| Export/verifier | Additive projections, JSONL overflow, receipts, original-field/evidence consistency | Mutating original input or certifying upstream authenticity |

## Ranking discovery contract

`RankingsScope` is optional. Its requested manifest has 46 event families. One scope binds exactly one seasonal division: season kind (`outdoor`/`indoor`), the division list id, season year, and the source request gender code (`m`/`f`), with a revision per division (`2026-usa-hs-grade11-<kind>-<boys|girls>-v2`). List ids and the season binding are evidence-backed from the captured `GetNavInfo`: `seasons["2026"] = 168416` (outdoor) and `seasons["12026"] = 173005` (indoor) for level div `168416`. `EventCatalog::from_nav` selects the configured list through that kind-specific season entry and requires the nav's own division to be either the selected list or its level, so a level-scoped navigation cannot be misread as another division. It excludes walk events, records absent families, and expands matching navigation events to observed variants; the current design target is 95 variants. Each results page is bound to collection, event, page number, request metadata, raw receipt, parsed observation, and prior checkpoint.

Individual rows use expected Grade 11 scope. Relay rows carry a system placeholder grade, so they become projected candidates only after `RelayTeamID == row AthleteID` and the joined roster member has Grade 11. This projection supplies discovery candidates and provenance; it never filters workbook membership or turns a missing ranking into an ineligible row.

Page parsing accounts for at most 8 MiB of ranking capture state. The general decoded source body bound is 32 MiB. `SourceCache` caches successes, but deliberately does not cache failed rankings outcomes; resumed rankings work must create fresh audit provenance. A collection is sealed only when all planned events have terminal pages and the final snapshot binds the same immutable scope/source snapshot.

## Privacy and source rules

The source profile is private and persistent. Chrome owns cookies and storage. Real Cloudflare challenges are manual human-only in the same headed profile. No challenge evasion, UA/fingerprint spoofing, webdriver patching, proxy/CAPTCHA service, cookie extraction/replay, or direct source-HTTP fallback is allowed. Local model endpoints remain local; no PII is sent to hosted models.

Source, request, response, attempt, candidate, row, page, and export identities are digest/provenance-bound. Public stable IDs are used for joins; physical worksheet coordinates are retained as source provenance, not as mutable identity. Original source fields are immutable and exported alongside additive annotations. Long summaries exceed Excel-cell limits by moving to the JSONL sidecar with row/report/athlete linkage.

## Current CLI contract

```text
worker, deploy, start, status,
browser-start, browser-status, export, verify,
rankings-status, rankings-pause, rankings-resume
```

`start` takes exactly one of `--per-sheet N` or `--all`, plus the original workbook digest. `--rankings` enables discovery collection; `--rankings-gender {m,f}` and `--rankings-season {outdoor,indoor}` select the division, and `--max-pages-per-event` bounds page traversal. `start --output PATH` queues `run-and-export` automatically; its response is submitted state, not completion. Ranking pause is durable; resume is explicit and performs the browser recovery path. `verify` requires the existing stopped ArtifactStore and must not open a live writer store.

## Qualification ledger

**Executed evidence:** green current-tree gates (`cargo fmt --check`, `cargo check --all-targets`, strict Clippy, and 15 test targets totalling 220 tests, including the repaired `result_verify` at 14/14, the CLI division-flag guards, and a library at 147 passed / 2 ignored); allocation measurement 344,131 versus 13,131 over 1,000 synthetic iterations (not throughput); native retained-corpus comparison covering 95 queries, 4,256 receipts, and 142,705 unique athletes; all 26 private storage scenarios; focused parser/storage/bundle and request-serialization regressions. Native fixture deployment: Restate admin 21041, ingress 21042, fixture 21046, worker 21140 with the worker deployed. End-to-end fixture qualification executed for three divisions: indoor girls run `6f3c0c59…` (list 173005, 95/95 events, published export, stopped-writer `verify` exit 0 with source hash match and 6 accepted / 1 no-match / 1 review / 0 pending, cached replay with zero new source requests and byte-identical artifacts) and outdoor boys run `3c726c6d…` (list 168416, pause froze source traffic for a 25 s window, explicit resume completed the run); indoor boys run `f0ef3948…` published XLSX `6403fb0a…` / JSONL `86323db6…`, passed stopped-writer `verify` (6 accepted / 1 no-match / 1 review / 0 pending), and replayed with zero new source requests.

**Pending:** live-source qualification against real endpoints with manual challenge handling; workbook-scale identity-matched delivery; the broader boys/girls indoor/outdoor/XC roster, full available histories, school history, and PR publication. No live-collection or expanded-roster readiness claim is supported.

Removed alpha/exhaustive/run-restate commands, deleted `tools/restate-native.sh`, old benchmark targets, and removed campaign documents must not reappear in operational instructions. Retained historical evidence remains useful only when labelled with its frozen binary, store, and source contract; it cannot certify this current tree.

## Module graph note

The production graph is the graph compiled from `src/main.rs`/`src/lib.rs` and their declared submodules. `#[path]` places implementations such as ranking parser children in feature directories without creating alternate roots; `pub(crate)` limits internal helpers; inline modules hold local support/tests. No claim should be based on naive filesystem reachability or historic dead-file counts; inspect declarations and actual callers.
