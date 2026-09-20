# Handoff — Athletic rankings durable verification (native Restate harness)

## Major goal
Close the **last open verification item** for the rankings acquisition lane in the Rust repo
`athletic-rust-pipeline`: prove **durable replay / pause / resume / export ownership** end to end,
on a **native Restate harness (NO DOCKER, ever)**. Everything runs through parallel subagents that
must apply `skill://holzman-rust`, `skill://async-rust-reviewer`, `skill://truth-serum`, plus
planner / test-writer / reviewer roles. Production code changes are already landed and pushed;
what remains is *executing and evidencing* the durable path.

## Where the repo stands (done, pushed, gates green)
- Repo `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline`, branch `main`, HEAD `3f78fd9`, clean, pushed.
- Lane cutover landed: `385f28b` (results served through the persistent in-page fetch lane via
  `fetch_results` → `request::rankings_spec` → `transport::fetch` → `RankingPageObservation`),
  `95a6996` (`next_page_after` pagination policy + 3 tests), `8ad47db` + `3f78fd9` (export coverage
  validation tests incl. an overflow guard).
- Gates green: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  **196 tests pass**, 0 failed; both ignored `lane_smoke` tests pass against the fixture.
- Do NOT re-litigate: an advisory to relax `src/result_verify/rankings/records.rs:~112` to accept
  `Some(page_count + 1)` on the sealed head page was **rejected as unsound** and independently
  re-confirmed by a reviewer agent with a full 3-page trace. `chain()` correctly demands `None`
  exactly at `page == page_count`.

## Harness (already built — do not rebuild from scratch)
- Restate server (native binary): `/home/lewis/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server`.
- Harness instance node `athletic-rankings-lane`: admin `http://127.0.0.1:19470`, ingress `http://127.0.0.1:19480`,
  config `.../rankings-lane-harness/restate.toml`, data `.../rankings-lane-harness/restate-data`.
  Protocol v7 / vqueues / scoped-virtual-objects flags are required.
- Worker: hub process **`rankings-lane-worker`**, bind `127.0.0.1:19282`, config
  `.../rankings-lane-harness/worker.toml` (mode `fixture`, `source_origin http://127.0.0.1:21045/`,
  `cpu_workers = 2`, `storage_dir .../rankings-lane-harness/artifacts` mode 0700, `[browser]` with
  `cdp_endpoint http://127.0.0.1:9223`, `headed = false`, `tabs = 1`). Last known pid 2398987.
- Deployment live: 13 services incl. `RankingsCollectionState` (VirtualObject) and `PipelineControl`.
- Rankings fixture: `.../lane-prototype-1789797782117/fixture-contract.cjs`, hub process
  **`athletic-lane-fixture-contract`**, port 21045.
- Headless chromium CDP: `http://127.0.0.1:9223`. Fixture workbooks:
  `.../fixture-native-v2/*.xlsx` (native-fixture.xlsx sha256
  `ee66f11d48a234bc4f21ca5b59bf779fd5ff85ba60b4d1e895e4b0ea88e41c24`).

## FOUR measured fixture gaps blocking the catalog/page flow
Derived from the Rust source and confirmed by live probes:
1. `GET /` returns **404 / 0 bytes** → browser bootstrap classifies failure → `ProfileGate` never opens →
   every fetch returns `HumanRequired`. Evidence: `browser-status` returned `restarting`/`challenged`,
   and fixture `/state` logged two `GET / → 404` at that moment. Fix: serve clean 200 HTML root.
2. Fixture has **no `GET /api/v1/tfRankings/GetNavInfo`** (only POST GetRankings). The catalog step uses
   `RankingsCapture::Navigation`, which requires path `/api/v1/tfRankings/GetNavInfo`, method GET, and a
   **body-less** captured request. The UI listing path
   `/TrackAndField/rankings/list/168416/m/100m/?page=1&grades=11` must serve HTML whose inline script
   issues `fetch('/api/v1/tfRankings/GetNavInfo')` (the interceptor captures page-initiated requests).
   Response must be `{divListId:168416, levelDivId:<nonzero>, seasons:{"2026":168416}, events:[NavEvent…]}`
   where each NavEvent has exactly `{id,name,t,m,r,h,short,w,fmt,so}`; shorts must match the canonical
   manifest (`100m`, `200m`, …) for `matches_requested`; shorts containing "walk" are excluded.
3. Fixture returns the **same 101-row payload for every page** (page 2, page 3, and unknown events all
   75571 bytes) → `next_page_after` always advances → pagination never terminates. Fix: page 1 rows,
   **page ≥ 2 empty**.
4. Payload carries top-level **`minCount: 1064`** but only 101 rows → terminal seal check
   `row_positions >= min_count` in `publication.rs:144-147` can never pass. Fix: `minCount: 100`
   (and keep it present on every response — parser errors `MissingMinCount` otherwise).
The terminating page must NOT be a minimal stub: `validate_scope` requires `division.ID`, `division.SeasonID`,
numeric `minCount`, and a `groupedRankings` array. Reuse the page-1 payload with only `groupedRankings`
emptied, `settings.page` set, `eventShort` echoed.

## Contracts you must not re-derive
- Canonical scope: `RankingsScope::requested(max_pages_per_event)`; validates list 168416, **season must be
  2026** (`EventCatalog::from_nav` hardcodes `season_id: 2026` and requires `seasons["2026"] == divListId`),
  gender `m`, grade 11, country USA, level 4, 46 families.
- Collection key = `collection_fingerprint(scope.revision, source_snapshot_digest)`.
- `RankingsCollectionState` is `ingress_private` — NOT reachable from ingress. Public surface is the
  **service `PipelineControl`** (`prepare`, `run_and_export`, `rankings_progress`, `rankings_pause`,
  `rankings_resume`), keyed by the **run** digest. CLI wraps all of it:
  `start`, `status`, `browser-start`, `browser-status`, `rankings-status`, `rankings-pause`,
  `rankings-resume`, `export`, `verify`.
- `start` = `prepare` (idempotent) then `RunCoordinator.run(...).send()`, returns
  `{run, invocation, state:"submitted"}`. **Gotcha:** harness `cpu_workers = 2` and `validate()` requires
  `concurrency ≤ row_concurrency` → you MUST pass `--concurrency 2` (default 64 gives
  HTTP 500 "requested concurrency exceeds worker capacity").
- The run starts the collection and polls `snapshot_ref()` every 1s until sealed → a paused collection
  blocks the run indefinitely.
- Gate starts CLOSED; after any worker restart the browser must re-bootstrap, and `rankings-resume`
  performs browser recovery.
- `next_page_after`: `Some(page+1)` iff rows, else `None`; `terminal = next_page.is_none()`;
  `page_count` increments on **every** published page including the terminal one.
- `prepare` WITHOUT `rankings_scope` already succeeded on the harness (manifest `74fb9e1a…`,
  snapshot `437ae4f8…`) — the import path works; the rankings-specific 500 message was still being
  captured when this handoff was written.

## Agents (collect their results — `hub jobs`, `agent://<id>`, `history://<id>`)
In flight at handoff: `FixtureNav` (the four fixture fixes), `SeederTool` (`tests/rankings_lane_seed.rs`
hand-seeded fallback), `ImportPath` (import recipe research), `IngressDriver2` (driver script over
`PipelineControl` + CLI), `PlanTest` (`.../rankings-lane-harness/test-plan.md`), `WriteTests` (durable
pause/resume + export-ownership tests), `ReviewAll` (adversarial review of `d9f817e..HEAD`, the new tests,
and the chain logic), `JournalProbe` (deepseek-flash; journal-level durable-replay evidence via the
`restate` CLI or admin API; specimen invocation `inv_1j0e8Brb1QDe6zJEByAgb76nkx0lpRVd9e`).
Completed: `BrowserWiring` (worker `[browser]` config + restart evidence), `CaseAdvisory2` (chain verdict).
Agent routing: `task`/`reviewer`/`scout` route to codex models that are **dead**
(`usage_limit_reached`). Use `gpu3090-coder`, `gpu5090-coder`, `gpu3090-reviewer`, `gpu5090-reviewer`,
and the newly added `deepseek-flash` agent (`.omp/agents/deepseek-flash.md`,
`model: deepseek/deepseek-v4-flash:max`).

## Remaining steps
1. Capture the real `start --rankings` 500 message (curl `PipelineControl/prepare` with a hand-built
   `RankingsScope` JSON, or read it from the admin invocation record).
2. Verify FixtureNav's four fixes by curl (`GET /` 200, nav JSON parses, page 1 rows / page 2 empty,
   `minCount` 100) and by `browser-status` reaching `ready`.
3. Rebuild the binary (it predates HEAD by ~10s and new tests exist), then restart
   `rankings-lane-worker` via hub with the exact args from its launch spec.
4. `start --all --rankings --max-pages-per-event 3 --concurrency 2` → capture the run key;
   `rankings-status --run <key>` → observe `CatalogStep` → `PageStep`s.
5. Crash window: (A) `rankings-pause` → kill worker → restart → `rankings-resume` → assert continuation
   (generation / `page_count` monotonic, no reset to page 1); (B) ideally kill mid-flight without pausing
   and assert the in-flight step resumes and that exactly one new physical POST occurs per page
   (assert on fixture `/state` counter **deltas**; restart the fixture first — counters are cumulative).
6. Export ownership: the export must bind the owning collection's sealed snapshot and **reject** a
   foreign or unsealed snapshot.
7. Final gates: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
   `cargo test --workspace --all-features`; the two ignored `lane_smoke` tests; then commit and push.
8. Update `CHROMIUM_DESIGN.md` and the todo ledger — mark
   "Verify durable replay pause resume and export ownership" done, or restate its blocker honestly.

## Hard rules
- **NO DOCKER.** Native `restate-server` only.
- **Never touch** `athletic-local` (holds a retained `SourceGateway/global.blocked` key that MUST stay),
  `cache-cutover-v1`, `evidence-repairs-v8`, `fixture-native-v2`.
- No live Athletic.net requests; no CAPTCHA automation; no UA/fingerprint spoofing; browser cookies only.
- Subagents must not commit; GPU agents must not run project-wide fmt/clippy/test sweeps (Main runs those once).
- Test code is exempt from no-unwrap; **nothing** is exempt from truthfulness — raw output only.
- `skill://restate` is NOT registered (only an SDK-0.8.0-era backup at
  `/home/lewis/.claude/skills.backup/claude-skills/restate/SKILL.md`; the repo uses SDK 0.12.0) —
  salvage only the still-valid parts (error taxonomy, `ctx.run` discipline, object exclusivity,
  introspection commands).
