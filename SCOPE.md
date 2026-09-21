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

**Current gap:** the native rankings scope is division-parameterized and **all four 2026 divisions have now executed live end to end**: indoor boys (run `57f88b34…`, 95/95 events over 1,065 pages), indoor girls (`fe726b41…`, 94/94 events over 816 pages), outdoor boys (`7be7e9dd…`, 95/95 events over 385 pages), and outdoor girls (`3b7c07fa…`, 94/94 events over 134 pages). Each decided its 8-row sample, published `result.xlsx` with its JSONL sidecar and receipt (`completeness: complete`), passed stopped-writer `verify` with the source digest matching before and after, and replayed byte-identically with zero new source requests. The two outdoor runs collected on a **signed-out** profile, so every event closed below its declared `minCount`; those acquisitions are complete as chains but not complete as per-event evidence. `RankingsScope::for_division` builds one seasonal division at a time from an evidence-backed table with a per-division revision, and `start --rankings-gender {m,f} --rankings-season {outdoor,indoor}` selects it. The retained 142,705-athlete delivery still covers the outdoor boys rankings dataset only, the 8-row sample decisions are all `review_required` by design so workbook-scale identity matching remains unproved, and the expanded roster below remains unimplemented: the existing profile/domain code supports track and cross-country evidence types, but [the discovery findings](#source-surfaces-for-the-expanded-roster--discovery-findings) show that the qualified rankings API has no cross-country surface and that the profile surface is not anonymously addressable. Those are new acceptance criteria, not completed features.

## Source surfaces for the expanded roster — discovery findings

Read-only probes through the lane's own headed transport (scripts and digests retained under `~/.local/share/athletic-rust-pipeline/lane-v14/live-capture/probe-2026-09-21/`) established the following against the live source on 2026-09-21. These are capability findings about the source, not collection evidence.

- **The Track & Field rankings API carries no cross-country season.** `GetNavInfo`'s `seasons` map holds only indoor keys (`12004`–`12027`, where `12026` maps to indoor list `173005`) and outdoor keys (`2004`–`2027`, where `2026` maps to outdoor list `168416`); its `events` list is track-only and the payload contains no cross-country season, event, or division. The two qualified rankings endpoints (`/api/v1/tfRankings/GetNavInfo`, `/api/v1/tfRankings/GetRankings`) are therefore not a cross-country source.
- **The cross-country rankings URL is a landing surface.** `/CrossCountry/rankings` returns 200 and renders a video landing page; it issues no rankings request.
- **There is no cross-country rankings API namespace.** `/api/v1/xcRankings`, `/api/v1/xcRankings/GetNavInfo`, and `/api/v1/crossCountry/GetNavInfo` all return 404.
- **Athlete profile URLs do not resolve anonymously.** A profile ID taken from live rankings candidate output resolves as follows: `/TrackAndField/athlete/{id}` redirects to the site home page, `/CrossCountry/athlete/{id}` redirects to the site home page, and `/athlete/{id}` returns 404. The probes ran on the same signed-out profile the outdoor runs used, so this finding is inconclusive about an entitled session; it is conclusive that the profile surface is not anonymously addressable at those shapes.

**Consequence:** the expanded roster needs a new source surface for cross-country results and for athlete profile history (athlete URLs, competing school, all available performances). Neither is reachable through the qualified rankings collector, and the profile question cannot be settled until an entitled session is available.

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

**Executed evidence:** green current-tree gates (`cargo fmt --check`, `cargo check --all-targets`, strict Clippy, `cargo build --locked`, and the 15-target test command re-verified 2026-09-20 on the CDP-patched tree, including the repaired `result_verify` at 14/14, the CLI division-flag guards, the new `cdp_frame_compat` guard, and a library at 156 passed / 2 ignored); completed live **chains** for two indoor division scopes: indoor boys (`57f88b34…`: 95/95 events terminal, 1,065 pages, `final_snapshot 40792b83…`, 8/8 rows decided, publication `a4eb1fe7…` / `87cfd651…` / `a4f0791b…` with `completeness: complete`, stopped-writer `verify` exit 0, byte-identical cached replay with zero new source requests) and indoor girls (`fe726b41…`: 94/94 events terminal over 816 pages, 8/8 rows decided, publication `8322bae9…` / `44f9f773…` / `fc6c12fe…` with `completeness: complete`, stopped-writer `verify` exit 0, byte-identical cached replay with zero new source requests) under the re-arm mitigation for the CDP frame drops (cause repaired in-tree; the mitigation stays until a live collection completes without it); allocation measurement 344,131 versus 13,131 over 1,000 synthetic iterations (not throughput); native retained-corpus comparison covering 95 queries, 4,256 receipts, and 142,705 unique athletes; all 26 private storage scenarios; focused parser/storage/bundle and request-serialization regressions. Native fixture deployment: Restate admin 21041, ingress 21042, fixture 21046, worker 21140 with the worker deployed. End-to-end fixture qualification executed for three divisions: indoor girls run `6f3c0c59…` (list 173005, 95/95 events, published export, stopped-writer `verify` exit 0 with source hash match and 6 accepted / 1 no-match / 1 review / 0 pending, cached replay with zero new source requests and byte-identical artifacts) and outdoor boys run `3c726c6d…` (list 168416, pause froze source traffic for a 25 s window, explicit resume completed the run); indoor boys run `f0ef3948…` published XLSX `6403fb0a…` / JSONL `86323db6…`, passed stopped-writer `verify` (6 accepted / 1 no-match / 1 review / 0 pending), and replayed with zero new source requests.

 A separate `scale-v15` lane (own Restate identity and store, fixture transport, worker binary sha256 `0f044e59…` built from the working tree at `9d101d9`) exercised the **real 120,716-row two-sheet workbook** rather than the synthetic fixture copy: a 10-row smoke run published a verifier-matched XLSX (111,939 + 8,777 rows, 1,569,308 source fields matched, 26/26 source headers) with an 83.7 MB JSONL sidecar, and an owner-online export during an unfinished 5,000-row run retained all 120,716 rows with 462 completed and 120,254 explicitly pending while matching source SHA-256 before and after. Measured non-rankings throughput: ≈0.31 rows/s at `row_concurrency = 8` with ≈50 durable invocations and ≈10 source operations per row (zero retries, zero model calls), unchanged at ≈0.27 rows/s when a second identical lane ran `row_concurrency = 32`, while two workers together sustained ≈0.9 rows/s with every process near-idle and no new fixture requests across a 30 s / 24-row window — so the ≈108 h full-workbook bound is per-worker waiting inside the row's source-operation chain, not coordinator concurrency or CPU, and locating that hop is the work that precedes raising admission. The hop is now isolated and its cause corrected: sampling `/proc/<pid>/io` during collection shows ≈11 MB/s of device writes across the Restate node and worker (≈12–14 MB per completed row) with both processes near-idle, but the *logical* durable cost is small — `sys_journal.raw_length` totals 51.5 KB per decided row (112.5 B/entry), `restate-data` grows 110 KB/row, and the store 36 KB/row, while the collection path journals ≈800 KB per ranking page. The device volume is therefore LSM compaction and fsync churn plus a debug-level worker log rather than payload size, so the levers toward workbook-scale delivery are fewer durable steps per row (~46 invocations) and storage/log configuration, not smaller journaled payloads.

**Pending:** both cross-country scopes and the athlete profile surface, neither of which is reachable through the qualified rankings API ([discovery findings](#source-surfaces-for-the-expanded-roster--discovery-findings)); an entitled session, which the two live outdoor runs lacked and which blocks settling whether profiles resolve when signed in; a live run on the CDP-patched binary rather than the re-arm mitigation (the frame-drop cause is repaired in-tree: `vendor/chromiumoxide_cdp` makes `ClientSecurityState.privateNetworkRequestPolicy` optional and `tests/cdp_frame_compat.rs` pins it; the re-arm supervision stays until a live collection completes without it) (the 2026-09-20 headed capture retained one real indoor navigation and boys/girls division pages with `200` responses, confirming `seasons["12026"] = 173005`, see [HANDOFF.md](HANDOFF.md); the 2026-09-19 headless attempt was Cloudflare-blocked with raw block-page bodies at `lane-v14/live-capture/`); workbook-scale identity-matched delivery (the real 120,716-row workbook is bound to ≈0.31 rows/s per lane, ≈108 h for `--all`, and the `scale-v15` 5,000-row run's cached replay is pending); the broader boys/girls indoor/outdoor/XC roster, full available histories, school history, and PR publication. No live-readiness or expanded-roster claim is supported beyond the four executed 2026 division chains (indoor and outdoor, boys and girls).

Removed alpha/exhaustive/run-restate commands, deleted `tools/restate-native.sh`, old benchmark targets, and removed campaign documents must not reappear in operational instructions. Retained historical evidence remains useful only when labelled with its frozen binary, store, and source contract; it cannot certify this current tree.

## Module graph note

The production graph is the graph compiled from `src/main.rs`/`src/lib.rs` and their declared submodules. `#[path]` places implementations such as ranking parser children in feature directories without creating alternate roots; `pub(crate)` limits internal helpers; inline modules hold local support/tests. No claim should be based on naive filesystem reachability or historic dead-file counts; inspect declarations and actual callers.
