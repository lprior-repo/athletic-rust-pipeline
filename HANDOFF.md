# Current design handoff

**Status: the live end-to-end chain is executed for the indoor boys and indoor girls scopes; workbook-scale identity-matched delivery and the expanded roster are pending.** This handoff records the current source contracts and the exact boundary between available evidence and unexecuted work. Automated collection -> deterministic matching -> owner-online export -> stopped-writer verification -> cached replay has completed against the private fixture transport for the outdoor boys, indoor girls, and indoor boys divisions, and against the **real source** for indoor boys: run `57f88b34…` acquired 95/95 events terminal / 1,065 pages / `final_snapshot 40792b83…`, decided 8/8 workbook rows, published `lane-v14/out-live-full-3/result.xlsx` (`a4eb1fe7…`), verified against the stopped writer (`exit 0`, source SHA-256 matching before and after), and replayed byte-identically with zero new source requests. It still does not claim identity-matched delivery at workbook scale or expanded-roster coverage: the 8-row sample decisions are all `review_required` by design, and the girls indoor half of the expanded roster has since completed its own live chain (run `fe726b41…`: 94/94 events, 8/8 rows decided, publication, stopped-writer verification exit 0, byte-identical cached replay) without delivering expanded-roster coverage.

## Freeze and ownership

This documentation refresh covers these existing files:

- `README.md`
- `HANDOFF.md`
- `CHROMIUM_DESIGN.md`
- `WORKFLOW_REVIEW.md`
- `SCOPE.md`

Two companion documents at the repository root are imported verbatim at the user's request and are reference material, not current state: `athletic-pipeline-project-pack.md` (sha256 `37699cdb8ee084b683dd6a036f4ffffd65f21ad52ed07b54a156adc82bf00695`, a synthesis pinned to tree `33e2adb`) and `athletic-pipeline-handoff.md` (sha256 `1c2b1adf60b189297d2a862bc9c5f258e8ff1e2d25a07a1cbdbc2b913ed26ab9`, written for the earlier `rankings-lane` harness). Their recorded numbers and next-step lists are superseded by the ledgers below; where they disagree with these five documents or the current source, the repository wins.

The prior GPU development lanes are frozen. Main owns integration, executable verification, native qualification, and subsequent publication. The local 3090 drafted the documentation; local 5090/3090 repair recommendations remain inputs requiring source review and execution, not proof.

The raw local drafting response is retained outside the repository at `native-rankings-1789772180206/final-development/3090/docs-draft-01.json`. It contains no workbook rows, identities, credentials, or hosted-model traffic. A local model draft is design input, not evidence of pipeline execution.
The drafting request used the local 3090 endpoint `http://127.0.0.1:11001/v1/chat/completions`, model `Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf`, with `chat_template_kwargs.enable_thinking=false`. Additional storage/fixture recommendations were requested from that endpoint and the local 5090 at port `11000`, model `Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf`; responses are retained as `final-development/{5090,3090}/integration-repair-01.json` under the same private evidence root.

The user pushed the accumulated changes. Subsequent documentation updates must continue to distinguish implementation, executed qualification, and remaining work; a model response or successful compilation is not end-to-end proof.

Preserve original workbooks, profiles, active stores, journals, retained binaries, and evidence. Never replace a changed binary under its old journals. Never open a live Fjall store from a second process. For offline verification: owner-online export first, drain, stop the sole writer, then open the stopped store.

## Current source design

For the supplied two-sheet source shape, both source worksheets remain in scope; sport and grade do not select or discard rows. The completed consolidated rankings delivery contains 142,705 unique athletes on `AllAthletes`. It is separate from original-workbook identity matching.

The expanded roster target is **source-verified Grade 11/junior boys and girls**, within the existing USA/2026 scope, covering **indoor track, outdoor track, and cross-country**. It requires athlete URLs, competing-school history, all available high-school performances, and evidence-backed comparable PRs. The exact contract and current implementation gaps are in [SCOPE.md](SCOPE.md#expanded-roster-target--requested-not-yet-complete). Do not describe the existing boys rankings collector as this completed expanded product.

The deterministic core covers stable source keys, request construction, HTML/JSON/rankings parsing, candidate evidence, identity policy, event/mark calculations, provenance checks, and export projection. The effect shell covers workbook I/O, ranking-index persistence, Restate calls/timers/admission, Chromium/CDP capture, browser challenge state, retries and uncertain-effect evidence, cancellation/drain, durable checkpoints, collection controls, seal, and publication.

Rankings are optional discovery. The requested manifest contains 46 event families. Cataloging expands matching navigation entries to observed variants (current target: 95), excludes walk events, and records absent families. Individual Grade 11 rows and relay members joined through the relay roster can enter the discovery projection. The Grade 11 projection does not decide workbook eligibility.

The rankings parser is bounded separately from source transport: source bodies are capped at 32 MiB while the parser accounts at most 8 MiB of ranking capture state. Pages retain source row numbers, result IDs, candidate kind, and checkpoint digest. Stable public IDs and provenance bind collection, event, page, request, raw receipt, parsed observation, and index. A collection seals only after every planned event has a terminal page.

`SourceCache` retains successful outcomes. Failed ranking outcomes are deliberately not cached; a resumed gateway call receives a new audit identity and can make a fresh source attempt. Restate owns durable attempt/retry scheduling and checkpoints; external HTTP remains an uncertain-effect boundary.

## Current CLI and operations

The actual CLI includes `worker`, `deploy`, `start`, `status`, `browser-start`, `browser-status`, `export`, `verify`, `rankings-status`, `rankings-pause`, and `rankings-resume`. The removed alpha/exhaustive/restate-native commands and deleted `tools/restate-native.sh` are not valid continuation instructions.

`start --output NEW_PATH` submits the durable `run-and-export` workflow, so it queues collection and export publication automatically. Its response is `state: submitted`; it is not completion. Without `--output`, submit the run, inspect `status`, then invoke `export` with a new destination. Ranking controls pause/resume the durable collection and its browser recovery path; a pause is genuine, and resumption is explicit rather than an implicit polling retry.

Real Cloudflare handling is manual human interaction only in the existing headed profile. There is no evasion, cookie extraction/replay, webdriver patch, proxy/CAPTCHA service, or direct source-HTTP fallback. A challenge closes new admission profile-wide while issued requests drain. Resumption requires successful non-challenged document evidence.

Readiness recovery is bounded by operator action, not by elapsed time: an escalated session (`human_required`) ends the wait with that durable status, and only an explicit `browser-start` re-runs the readiness workflow with operator intent, which re-arms one recovery navigation before the session escalates again. Fetch paths (`capture_ready`, the legacy `await_ready` waiter used by the source gateway) keep the strict policy and never clear a stall. A CDP endpoint that is not reachable at browser startup falls back to launching a managed browser on the configured profile; a connection that dies after startup still requires a worker restart, because the manager is cached per worker process.

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
publication, stopped-writer verification, and replay remain pending: this run is not qualification
evidence until they exist.

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

**Current-tree gate re-run (2026-09-20, rankings transport retry):** `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo build --locked`, and the 16-target command above are green — library 155 passed / 2 ignored (the two new `receiptless_transport` tests), every integration target ok including `result_verify` 14/14. The live lane was rebuilt and its worker restarted on this tree; `browser-start` re-attached the headed session to `ready` within 12 s, and run `57f88b34…` resumed at `55m` page 31.

**Live-chain gate (2026-09-20, current tree at `3f78fd9`):** `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets --locked -- -D warnings`, and the 16-target command above re-ran green on the tree that served the live chain — **232 passed / 0 failed** (library 156 passed / 2 ignored, main 3, every integration target ok including `result_verify` 14/14). The chain executed end to end on that tree against the live source: collection `95/95` events / `1,065` pages / `final_snapshot 40792b83…` → row phase `8/8` (`complete: true`, 8 `review_required`, 0 pending) → owner-online export `result.xlsx` `a4eb1fe7…` / `result.jsonl` `87cfd651…` / `result.commit.json` `a4f0791b…` (`native-export-worker-v4`, `completeness: complete`, 2 sheets, 8 rows, 120 fields, 30 headers) → stopped-writer `verify` exit 0 with the same `output_sha256` and source digest `0a1d53f1…` → worker restarted on the re-arm build (`508dc0949ca29c352855302dd1e117cc11848b95d873f39bad9f67c0c6516812`) → cached replay `202` in 0.5 s, byte-identical artifacts, `SourceGateway/fetch` unchanged at 488.

**Next:** the live chain is executed for the indoor boys scope (run `57f88b34…`: 95/95 events terminal, 1,065 pages, `final_snapshot 40792b83…`, 8/8 rows decided, publication `a4eb1fe7…`, stopped-writer verification exit 0, byte-identical replay with zero new source requests on the re-arm binary `508dc094…`) and for the indoor girls scope (run `fe726b41…`: 94/94 events over 816 pages, 8/8 rows decided, publication `8322bae9…` / `44f9f773…` / `fc6c12fe…`, stopped-writer verification exit 0 against `live-store`, byte-identical cached replay with zero new source requests on build `cfdae271…`). What remains: outdoor live collection (the boys run `7be7e9dd…` is in flight on the patched client; the
girls scope is the same `start` with `--rankings --rankings-gender f --rankings-season outdoor` and the
lane input `live-store/source-workbooks/0a1d53f1….xlsx` as both `--input` and `--sha256` — the lane serves
one run at a time) and both cross-country scopes (no `--rankings-season` value models cross-country yet, so
XC needs a source-kind addition, not a flag); workbook-scale identity-matched delivery (the real 120,716-row workbook is bound at ≈0.31 rows/s per lane, ≈108 h for `--all`; its bounded 5,000-row selection, stopped-writer verification, and cached replay are now qualified — see the scale-v15 chain entry above); and the expanded roster requirements in `SCOPE.md` (athlete URLs, school history, all available high-school performances, PRs, cross-country). Re-running the live lane needs the headed browser at `ready` on CDP `9333`: the profile is signed in, `browser-start` settles at `ready`, and acquisition still stalls on the residual transport failure recorded above — now attributed to profile-restored
tabs accumulating on every recovery relaunch, repaired in `Actor::bootstrap` and worked around by the lane
supervisor while a pre-repair worker runs. The verifier's rankings path now has its own frozen synthetic collection fixture (`src/result_verify/rankings.rs` driving `tests/fixtures/rankings/`) in addition to the executed indoor stopped-writer verification. No broad unit suite or fuzz campaign is required by this handoff.
