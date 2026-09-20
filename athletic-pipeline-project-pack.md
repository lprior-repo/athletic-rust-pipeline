# Athletic Rust Pipeline — standards, architecture, plans, code

**Audience:** an implementing agent or engineer joining mid-flight.
**Status of this pack:** synthesis written at tree `33e2adb` (+ uncommitted `src/result_verify/rankings.rs`, `tests/fixtures/`).
It **complements** the five in-repo documents, which are frozen to that set and remain authoritative:
`SCOPE.md`, `HANDOFF.md`, `CHROMIUM_DESIGN.md`, `WORKFLOW_REVIEW.md`, `README.md`.
Where this pack and the repo disagree, **the repo wins** — the repo is edited continuously by another working session.
Evidence labels used below: **[MEASURED]** = observed by this writer with raw output, **[RECORDED]** = stated in the repo's own
documents/ledger, **[DERIVED]** = read from source code in this tree.

---

## 1. Coding standards

### 1.1 The two-tier split is the primary standard
Every module belongs to exactly one tier, and the split is enforced by review, not by tooling:

- **Deterministic core** — anything recomputable from explicit inputs: workbook decoding, row identity, name/identity/mark
  validation, ranking parsing, catalog construction, page observation, export projection, arithmetic. Pure, testable, no I/O.
- **Effect shell** — anything that touches the world: filesystem/Fjall writes, Restate calls/timers/admission, Chromium/CDP
  capture, model transport, cancellation/drain, retry ownership.

Consequences that are non-negotiable in review:
- Effects never leak into the core; the core never learns about Retry-After, gates, sockets, or time.
- Effects are *bounded*: every effect has an explicit timeout/budget and a cancellation path.
- A model answer is an input to review, never source truth; a successful compilation is never end-to-end proof.

### 1.2 Holzmann / Power-of-10 discipline [RECORDED: SCOPE.md "Implementation retains the agreed functional Rust/Holzman discipline"]
- Validated domain types over primitive obsession; parse-don't-validate at the boundary.
- Pure calculations separated from effects; explicit errors, never silent coercion.
- Bounded resources (see §1.4); no unbounded growth of buffers, queues, tabs, or captured state.
- Owned cancellation and drain: work this process issued is drained or cancelled on every exit path, including error paths.
- **No avoidable hot-path cloning or allocation.** The rankings lane cutover was explicitly reviewed for this and found free of
  payload clones; the only unavoidable copies are Fjall byte→`String` conversions and small scalars/digests.
- No `unwrap`/`expect`/`panic`/`todo`/`unimplemented` in production paths. Test code is exempt; nothing is exempt from truthfulness.

### 1.3 Error taxonomy (Restate-specific, load-bearing)
- `TerminalError::new(msg).into()` — business-logic failures, validation, "not found". Never retried; surfaced to the caller.
- `HandlerError::from(msg)` — transient failures. Restate auto-retries per the handler's retry policy.
- Domain errors are typed enums (`CatalogError`, `PageParseError`, `StoreError`, `BrowserError`), not strings; strings only at
  the boundary where a durable handler must produce a message (e.g. `publication::catalog` returns
  `Json(result.map_err(|e| e.to_string()))` so the failure is journaled, not swallowed).
- A retry that would repeat a physical source request must be modeled as an *uncertain effect*, never as an exactly-once guarantee.

### 1.4 Explicit, validated bounds (all enforced in code, not prose)
| Bound | Value | Where |
|---|---|---|
| Source decoded body | 32 MiB | transport |
| Ranking parse accounting | 8 MiB | rankings parser |
| Browser tabs | 1..=8 (fixture default 1) | `RawBrowserConfig::validate` |
| Challenge wait | 15..=30 s | `RawBrowserConfig::validate` |
| `headed=false` | only when mode != Live | `RawBrowserConfig::validate` |
| `max_pages_per_event` | 1..=10000 | `RankingsScope::for_division` |
| Requested concurrency | ≤ worker `row_concurrency` and ≤ 256 | `PipelineControl::validate` |
| Labels (`snapshot`, `execution`) | 1..=128 bytes, no control chars | `PipelineControl::validate` |
| Page rows / candidates | ≤ 1024 each | `rankings/page/parse` |

### 1.5 Naming and test conventions (match these exactly)
- Tests live in inline `#[cfg(test)] mod tests` next to the code, or in focused `tests/` targets. **No project-wide suites are
  added for their own sake** — the repo's stance is "no broad unit suite or fuzz campaign is required".
- Test names are full sentences describing the defended invariant, e.g.
  `an_empty_page_terminates_the_chain`, `an_unparsable_page_terminates_rather_than_advancing`,
  `accepted_counts_that_overflow_are_rejected_rather_than_wrapped`, `rejects_indistinguishable_local_selection_with_rehashed_evidence`.
- Tests are grouped by behaviour into named modules (`mod pagination`, `mod lane_smoke`).
- Tests that need real infrastructure are `#[ignore]`d and gated on an env-provided fixture (defaults: `ADLAW_LANE_CDP`,
  `ADLAW_LANE_FIXTURE`), and are run explicitly.
- Module placement: logical children live in feature directories via `#[path]` (e.g. `#[path = "rankings_helper.rs"] mod rankings_helper;`
  inside `transport/rankings.rs`; `rankings/page/parse.rs` declaring `main`/`relay`) so the graph stays one compile-time unit.
  `pub(crate)` for internal helpers; public only for serialized ids and provenance. Never infer the graph from filesystem reachability.

### 1.6 Evidence discipline (truth-serum, applied to every claim)
- Every claim is labelled by what was actually executed: **executed** vs **reported** vs **pending**. "Model drafted", "compiles",
  "tests pass" are not end-to-end proof.
- Never re-interpret retained evidence under new semantics: a **frozen binary** verifies/export its own era. The pre-split rankings
  revision `2026-usa-boys-grade11-v1` is **retired**; a scope carrying it still deserializes (absent `season_kind` defaults to outdoor)
  but `validate()` rejects it by name — those stores are served by their retained pre-change binary, not by this tree.
- Never replace a changed binary under its old journals; never open a live Fjall store from a second process.
- Removed operational instructions must not reappear (alpha/exhaustive/run-restate commands, `tools/restate-native.sh`, campaign docs).

### 1.7 Privacy and source rules (hard, no exceptions)
- Chromium owns cookies/storage; the profile is private and persistent.
- **Manual human handling only** for real Cloudflare challenges, in the same headed profile. No evasion, no UA/fingerprint spoofing,
  no webdriver patching, no proxy rotation, no CAPTCHA service, no cookie extraction/replay, **no direct source-HTTP fallback**
  (`reqwest` is for local model/control traffic only).
- A challenge closes new admission profile-wide while issued work drains; resume requires successful non-challenged document evidence.
- Honor `Retry-After`; never promise zero upstream 429s; never treat evasion as optimization.
- Local model endpoints stay local; no PII to hosted models.

---

## 2. Architecture

### 2.1 End-to-end shape [RECORDED: SCOPE.md]
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
Workbook membership is eligibility: **no cohort/grade/graduation/junior/sport filter ever removes a real row.** Generated sheets
(`Athletic Matches`, `Corrections`, `Summary`) are outputs, not population.

### 2.2 Ownership boundaries [RECORDED: SCOPE.md]
| Area | Owns | Does not own |
|---|---|---|
| Workbook/domain/parser core | Streaming records, stable IDs, deterministic validation, ranking parsing, identity policy, mark arithmetic | Browser lifecycle, timers, model transport, eligibility shortcuts |
| Restate runtime | Durable calls, scheduling, retries, admission, cooldown, pause/resume, checkpoints, cancellation/drain | Exactly-once external HTTP, truth of upstream source |
| Chromium transport | Normal headed profile, tabs, CDP request/response identity, decoded bounded bytes, challenge gate | Cookie extraction/replay, evasion, source-rate authorization |
| SourceCache/Gateway | Snapshot-bound source outcomes, fresh ranking failures, bounded attempt evidence | Candidate decisions, stale failure success |
| Ranking collection | 46-family manifest, observed variant catalog, walk exclusion, page/roster joins, sealed indexes | Workbook eligibility or cohort selection |
| ArtifactStore | Immutable documents, row/source indexes, attempts, ranking indexes and seals | Live queue/lease, second writer, external truth |
| Export/verifier | Additive projections, JSONL overflow, receipts, original-field/evidence consistency | Mutating original input or certifying upstream authenticity |

### 2.3 Restate topology (13 services on one worker endpoint) [MEASURED: `GET /deployments` on the harness showed 13]
`PipelineControl` (service) · `RunCoordinator` (object, key `global`) · `WorkbookImport` · `RowWorker` · `QueryWorker` ·
`ProfileWorker` · `SourceGateway` · `SourceCache` · `BrowserSession` · `RankingsCollectionState` (VirtualObject) ·
`ExportWorker` · `ReviewCase` · `LocalReviewer`.

Key boundaries:
- `RankingsCollectionState` is **`ingress_private`** (object attrs: `ingress_private = true`, `inactivity_timeout = "2h"`).
  It is **not** callable from the ingress — the public surface is `PipelineControl`, keyed by the **run** digest.
- `PipelineControl`: `inactivity_timeout = "2h"`, `journal_retention = "30 days"`, `idempotency_retention = "30 days"`,
  `invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")`. [DERIVED: control.rs:45-51]
- Objects are exclusive per key: never call A→B→A on the same key (deadlock), and never hold a long sleep in an object handler.

### 2.4 Main call chain [RECORDED: WORKFLOW_REVIEW.md]
```text
main -> cli::run
  worker    -> runtime::worker::serve -> Endpoint services
  deploy    -> local admin capability/rule checks -> registration
  start     -> PipelineControl.prepare -> RunCoordinator.run
  start --output -> PipelineControl.run_and_export -> RunCoordinator + ExportWorker
  status    -> RunCoordinator.status
  browser-start/status -> BrowserSession await_ready/status
  rankings-status/pause/resume -> PipelineControl ranking controls
  export    -> ExportWorker.publish -> durable output observation
  verify    -> existing_stopped_store -> bundle_verify
```

### 2.5 Ranking collection chain [RECORDED: WORKFLOW_REVIEW.md, verified against source]
```text
RankingsScope::for_division(kind, gender, max_pages)
  -> PipelineControl / RankingsCollectionState
  -> SourceResource::Rankings(Navigation)      # catalog: GET /api/v1/tfRankings/GetNavInfo
  -> SourceCache -> SourceGateway -> browser/source receipt
  -> EventCatalog::from_nav
  -> RankingsPlan (46 requested families, observed variants [target 95], absent families)
  -> SourceResource::Rankings(Results) per event/page   # POST /api/v1/tfRankings/GetRankings
  -> parse_page_response
  -> RankingsPageCheckpoint + RankingPageIndex
  -> event terminality and lower-bound reconciliation
  -> immutable CollectionFinalSnapshot / sealed collection reference
  -> export coverage
```

### 2.6 Browser transport layering and the gate lifecycle [DERIVED]
```text
SourceResource::Rankings(Results)  -> transport::fetch  (persistent in-page lane)
SourceResource::Rankings(Navigation) -> fetch_rankings  (interceptor + navigation path)
```
- `ProfileGate` **starts CLOSED** (`ready=false, generation=0`); only `try_open(observed_generation)` opens it, and a concurrent
  `revoke()` invalidates a pending open. `revoke()` also fires on challenge, 403/429, browser loss, and shutdown.
- `transport::fetch` refuses with `BrowserError::HumanRequired` whenever the gate is closed — *both* lanes are gated.
- The actor bootstraps a page against `source_origin`; a failed/challenged bootstrap leaves the gate closed, so **a source origin
  that does not serve a clean 200 root page can never be fetched from at all** (this is the single most common false blocker).
- `BrowserState::Ready if !gate.is_ready()` is reported as `Challenged` — i.e. a closed gate can present as "challenged" even when
  no challenge header was seen.
- Browser process/page handles are effect-local and never serialized into durable state; only readiness/challenge/cooldown/
  human-required/recovery/stopped phases are durable.

### 2.7 The persistent rankings lane (the recent cutover) [DERIVED + RECORDED]
`RankingsCapture::Results` no longer navigates. It composes the site's own request and issues it **from the already-bootstrapped
source page**, so cookies/TLS/fingerprint stay in Chromium and no direct HTTP client touches the origin. `Navigation` keeps the
interceptor/navigation path unchanged. `RequestSpec` carries the physical `url` and the `semantic_url` separately, so cache keys,
checkpoints, receipts and provenance keep their established identity; `result_verify` requires both URLs on the frozen origin and
permits a distinct physical endpoint **only** for rankings actions.

### 2.8 Storage, ownership, publication
- `ArtifactStore` is the worker-owned **Fjall** boundary: content-addressed documents, source indexes, attempts, ranking page
  indexes, event statistics, final seals — immutable or conflict-checked. Fjall is **not** a queue or lease service.
- **One writer owns a live store.** Export is owner-online; then the writer is stopped before independent verification
  (`verify --store <dir>` reads a stopped store and must never open a live writer).
- Publication is non-clobbering and multi-artifact: XLSX + JSONL sidecar + receipt. Missing receipt or pending rows ⇒ not complete.
- Import identity is `(original path, workbook digest)`: a **fresh store pointed at the same path fails closed** at
  `PipelineControl/prepare` with `requested artifact is absent`. Give each store its own byte-identical copy.

---

## 3. Key contracts and invariants (the ones that bite)

### 3.1 Division binding — one scope = one seasonal division
[DERIVED: `src/runtime/rankings/division.rs`]
- Evidence-backed list ids: `seasons["2026"] -> 168416` (outdoor) and `seasons["12026"] -> 173005` (indoor) for level div `168416`.
  One division list serves both genders behind the request `gender` parameter (`m`/`f`).
- Revisions are per division: `2026-usa-hs-grade11-outdoor-boys-v2`, `2026-usa-hs-grade11-indoor-girls-v2`, etc.
  `SeasonKind` defaults to outdoor for pre-split frozen scopes; `validate()` rejects the retired v1 revision by name.
- `EventCatalog::from_nav` selects the list through the **kind-specific** season entry and requires the nav's own division to be
  either the selected list or its level — so a level-scoped navigation cannot be misread as another division.

### 3.2 Collection identity
`collection_key = collection_fingerprint(scope.revision, source_snapshot_digest)`; `start_or_resume` rejects a key that does not
match. A collection seals only when **every planned event has a terminal page** and the final snapshot binds the same immutable
scope/source snapshot.

### 3.3 Pagination and the sealed chain
- `next_page_after(body, page)`: `Some(page + 1)` iff the body carries ranked rows; `None` for empty **or unparsable** bodies.
- `publication::page`: `terminal = capture.next_page.is_none()`; a non-terminal page must advance to exactly `page + 1`.
- `run_page_step` increments `page_count` **unconditionally for every published page, including the terminal one**.
- `records.rs::chain()` requires `next_page == None` **exactly** at `page == page_count`, and `Some(page+1)` on every page before it.
  **A proposal to also accept `Some(page_count + 1)` on the head page was reviewed and rejected as unsound** — it would let a sealed
  event claim a successor page it never fetched. Independent adversarial re-review confirmed the rejection with this trace:
  page 1 → `Some(2)`, count 1; page 2 → `Some(3)`, count 2; page 3 (empty) → `None`, count 3 ⇒ chain accepts.
- Terminal pages must satisfy a **lower-bound coverage check**: `row_positions >= min_count` **and** `max_row_position >= min_count`,
  where `row_positions` is the count of distinct stored row positions and `max_row_position` the largest.
- **Pagination never derives from `minCount`.** Live measurement showed `minCount` is a *per-page* figure (701 on page 1, 1523 on
  page 2, 0 for a multi-event query), so it is not a total. Rows decide advancement; `minCount` only bounds terminal coverage.

### 3.4 Cache semantics
`SourceCache` caches successful outcomes and non-ranking failures. **Ranking failures are deliberately never cached**, so a resumed
source attempt gets a fresh audit identity and can make a genuinely new request.

### 3.5 Discovery ≠ eligibility
Rankings are optional discovery. The Grade 11 projection never filters workbook membership and never turns a missing ranking into
an ineligible row. Individual rows are accepted only in the expected Grade 11 context; relay rows require the roster join
(`RelayTeamID == row AthleteID`) and a Grade 11 member — never the relay row's placeholder grade.

---

## 4. Code (verbatim from this tree)

### 4.1 Canonical scope construction
```rust
// src/runtime/rankings/types.rs
pub fn requested(max_pages_per_event: u32) -> anyhow::Result<Self> {
    Self::for_division(SeasonKind::Outdoor, "m", max_pages_per_event)
}

pub fn for_division(
    season_kind: SeasonKind,
    gender: &str,
    max_pages_per_event: u32,
) -> anyhow::Result<Self> {
    if max_pages_per_event == 0 || max_pages_per_event > 10_000 {
        anyhow::bail!("max_pages_per_event must be 1..=10000");
    }
    let gender = normalize_gender(gender)
        .ok_or_else(|| anyhow::anyhow!("unsupported gender: {gender}"))?;
    let list_id = season_list_id(season_kind, division::SEASON_YEAR)
        .ok_or_else(|| anyhow::anyhow!("unsupported season: {season_kind:?}"))?;
    let revision = expected_revision(season_kind, gender)
        .ok_or_else(|| anyhow::anyhow!("unsupported gender: {gender}"))?;
    let scope = Self { revision, season_kind, list_id, season: division::SEASON_YEAR,
                       gender: gender.to_owned(), projection_grade: 11,
                       country: "USA".to_owned(), level: 4,
                       max_pages_per_event, requested_families: Self::default_families() };
    /* … validate() … */
}
```

### 4.2 Catalog validation from the captured navigation document
```rust
// src/runtime/rankings/catalog.rs
pub fn from_nav(
    nav: &serde_json::Value,
    requested_families: &[RequestedFamily],
    season_kind: SeasonKind,
    expected_list_id: u64,
) -> Result<Self, CatalogError> {
    let nav_div_list_id = nav.get("divListId").and_then(|v| v.as_u64())
        .ok_or(CatalogError::MissingDivListId)?;
    let level_div_id = nav.get("levelDivId").and_then(|v| v.as_u64())
        .ok_or(CatalogError::MissingLevelDivId)?;
    let season_key = season_kind.seasons_key(SEASON_YEAR);
    let list_id = nav.get("seasons").and_then(|v| v.as_object())
        .and_then(|seasons| seasons.get(&season_key)).and_then(|v| v.as_u64())
        .ok_or_else(|| CatalogError::MissingSeason { key: season_key.clone() })?;
    if list_id != expected_list_id {
        return Err(CatalogError::SeasonListIdMismatch { key: season_key, expected: expected_list_id, actual: list_id });
    }
    if nav_div_list_id != expected_list_id && nav_div_list_id != level_div_id {
        return Err(CatalogError::NavDivisionMismatch { nav_div_list_id, level_div_id, expected: expected_list_id });
    }
    // … events[] → NavEvent::from_value, reject id == 0 / empty / >64-byte short,
    //    drop walk events (is_excluded), reject conflicting duplicate ids …
}
```
`NavEvent` requires exactly `{id, name, t, m, r, h, short, w, fmt, so}` (serde: missing field ⇒ the whole payload is malformed).
Family matching is `event.short == requested.short`, with special cases for medley/relay/pentathlon families.

### 4.3 Pagination policy and the lane
```rust
// src/runtime/browser/transport/rankings.rs
fn next_page_after(body: &[u8], page: u32) -> Option<u32> {
    match response_has_rows(body) {
        Ok(true) => page.checked_add(1),
        Ok(false) | Err(_) => None,
    }
}

async fn fetch_results(page: &Page, action: &RankingsAction, request_timeout: Duration,
                       gate: Arc<ProfileGate>, source_origin: &url::Url)
    -> Result<BrowserResponse, BrowserError> {
    let request = request::rankings_spec(source_origin, action.clone())
        .map_err(|_| BrowserError::Protocol)?;
    let body = match request.body() { Some(b) => serde_json::to_string(&b)…, None => return Err(BrowserError::Protocol) };
    let mut response = super::fetch(page, &request, request_timeout, gate).await?;
    let next_page = next_page_after(&response.body, action.page);
    response.rankings = Some(RankingPageObservation {
        capture: RankingsCapture::Results, request_method: "POST".to_owned(),
        request_url: request.url.as_str().to_owned(), request_body: Some(body), next_page,
    });
    Ok(response)
}
```

### 4.4 Page publication, terminality, and the lower bound
```rust
// src/runtime/rankings_collection/publication.rs
if capture.next_page.is_some_and(|next| event.next_page.checked_add(1) != Some(next)) {
    bail!("pagination does not advance to the next consecutive page");
}
let terminal = capture.next_page.is_none();
let observation = parse_page_response(&raw(store, receipt)?, expected)?;
let min_count = observation.min_count;
/* … checkpoint + index persisted … */
if terminal {
    let stats = store.ranking_event_stats(&index.collection, &event.event_short)?;
    if stats.row_positions < min_count || stats.max_row_position < min_count {
        bail!("terminal ranking page does not cover the source minCount lower bound");
    }
}
```
```rust
// src/runtime/rankings/page/parse/main.rs
pub fn parse_page_response(raw: &serde_json::Value, expected: &ExpectedPageContext<'_>)
    -> Result<PageObservation, PageParseError> {
    let mut observation = PageObservation::default();
    validate_scope(raw, expected, &mut observation)?;          // division.ID, division.SeasonID, …
    let groups = raw.get("groupedRankings").and_then(|v| v.as_array())
        .ok_or(PageParseError::MissingGroupedRankings)?;
    let raw_min_count = raw.get("minCount").and_then(|v| v.as_u64())
        .ok_or(PageParseError::MissingMinCount)?;
    /* … parse_groups … */
    observation.min_count = raw_min_count;
    Ok(observation)
}
```

### 4.5 Prepare validation and the control surface
```rust
// src/runtime/control.rs
fn validate(request: &PrepareRequest, runtime: &Runtime) -> anyhow::Result<()> {
    if usize::from(request.concurrency.get()) > runtime.config.row_concurrency()
        || request.concurrency.get() > 256 {
        anyhow::bail!("requested concurrency exceeds worker capacity");
    }
    for label in [&request.snapshot_label, &request.execution] {
        if label.is_empty() || label.len() > 128 || label.chars().any(char::is_control) {
            anyhow::bail!("snapshot and execution labels must be 1..=128 bytes without controls");
        }
    }
    if let Some(scope) = &request.rankings_scope {
        scope.validate().map_err(|e| anyhow::anyhow!("rankings scope validation failed: {e}"))?;
    }
    Ok(())
}
```

### 4.6 How a run drives the collection (and why a paused collection blocks it)
```rust
// src/runtime/run.rs
if let Some(scope) = &snapshot.rankings {
    let collection = super::rankings_collection::collection_fingerprint(&scope.revision, &request.snapshot)?;
    let client = ctx.object_client::<RankingsCollectionStateClient>(collection.as_str());
    client.start_or_resume(Json(CollectionRequest { source_snapshot: request.snapshot.clone() })).call().await?;
    let sealed = wait_for_collection_sealed(&ctx, &collection).await?;
    results.update_collection_ref(sealed.clone())?;
    rows.update_rankings(sealed);
}

async fn wait_for_collection_sealed(ctx: &ObjectContext<'_>, collection: &EvidenceDigest)
    -> Result<RankingCollectionRef, HandlerError> {
    loop {
        let snapshot = ctx.object_client::<RankingsCollectionStateClient>(collection.as_str())
            .snapshot_ref().call().await?.0;
        if let Some(snapshot) = snapshot { return Ok(RankingCollectionRef { collection: collection.clone(), snapshot }); }
        ctx.sleep(std::time::Duration::from_secs(1)).await?;
    }
}
```

### 4.7 CLI: start and the division flags
```rust
// src/cli/args.rs (Start)
#[arg(long)] pub rankings: bool,
#[arg(long, default_value = "m", value_parser = ["m", "f"], requires = "rankings")]
pub rankings_gender: String,
#[arg(long, default_value = "outdoor", value_parser = ["outdoor", "indoor"], requires = "rankings")]
pub rankings_season: String,
#[arg(long, default_value = "10000")] pub max_pages_per_event: u32,
#[arg(long, default_value = "64")] pub concurrency: NonZeroU16,
```
```rust
// src/cli.rs
let rankings_scope = if args.rankings {
    Some(RankingsScope::for_division(…).context("building rankings scope")?)
} else { None };
let prepared = control.prepare(Json(request)).idempotency_key(preparation_key.as_str()).call().await?.into_body()?.0;
let key = prepared.key()?;
// without --output: RunCoordinator.run(Json(prepared)).idempotency_key(&key).send()  → {run, invocation, state:"submitted"}
// with --output:    PipelineControl.run_and_export(...).send()                      → {run, invocation, state:"submitted", automatic_export:true}
```

---

## 5. Verification and the quality ledger

**Current-tree gates** [RECORDED: HANDOFF.md ledger]:
```text
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --lib --bins \
  --test rankings_parser --test rankings_catalog --test rankings_scope --test rankings_indoor \
  --test rankings_storage --test profile_html_bounds --test profile_merge_bounds \
  --test search_html_bounds --test workbook_verify --test workbook_zip_layout \
  --test native_parser_properties --test result_verify --test bundle_verify
```
15/15 targets green (217 tests), library 144 passed / 2 ignored, zero failures.

**Lane-specific, ignored by default** (need a fixture origin + CDP browser):
```text
ADLAW_LANE_CDP=http://127.0.0.1:9223 ADLAW_LANE_FIXTURE=http://127.0.0.1:21045/ \
  cargo test --lib -- --ignored lane_smoke --test-threads=1 --nocapture
# results_capture_costs_one_physical_post
# challenge_response_revokes_the_gate_and_ends_pagination
```

**Counter-evidence discipline:** fixture counters are process-cumulative and sticky — restart the fixture before asserting deltas,
or the assertion is meaningless.

**Allocation measurement** [RECORDED]: 344,131 vs 13,131 allocations over 1,000 synthetic iterations (331 removed per iteration).
This is an allocation count, **not** throughput evidence.

**Retained corpus** [RECORDED]: 95 queries, 4,256 receipt digest checks, 340,238 individual results, 57,629 relay-member results,
142,705 unique athletes, 15,724 unresolved roster results retained explicitly.

**Executed end-to-end (fixture transport)** [RECORDED]:
- Indoor girls run `6f3c0c59…`, revision `2026-usa-hs-grade11-indoor-girls-v2`, list `173005`: 95 scheduled events, all terminal,
  phase `Complete`; every run request used division `173005` (99 requests); export → `result.xlsx` / `result.jsonl` / `result.commit.json`;
  stopped-writer verification passed; re-submitting the identical `start` reproduced byte-identical artifacts with **zero** new
  source requests (cached replay).
- Outdoor boys run `3c726c6d…`, revision `2026-usa-hs-grade11-outdoor-boys-v2`, list `168416`: `rankings-pause` latched
  `{"Paused":"Manual"}` and froze source traffic for a 25 s window (0 new requests); `rankings-resume` continued to 95/95 and published.

---

## 6. Long-term plan

### 6.1 The expanded roster target (requested, not implemented)
Source-verified **Grade 11/junior boys *and* girls**, within USA/2026, covering **indoor track, outdoor track, cross-country**.
Not an all-grade or alumni census. Junior status requires season-bound source evidence — never inferred from name, age, or assumed
graduation year. Required records:
- **Athletes** — stable source athlete id, name, athlete/profile URLs, evidenced junior status, source season, competing school;
  conflicting/unknown observations preserved explicitly.
- **Performances** — *every available high-school result* for selected athletes (not just ranking entries or junior-year bests):
  raw mark, normalized value where supported, sport, event, indoor/outdoor context, timing, wind, implement/hurdle spec, date, meet,
  result URL, source result id, competing school at that performance.
- **Personal records** — event-specific comparable bests linked to supporting results; source-reported PR claims kept distinct from
  calculated bests; a 5 km time and a three-mile time are not interchangeable; missing history cannot establish a career PR.
- **School history** — school identity plus the season/result supporting it; a transfer must not rewrite earlier performances.
- **Coverage accounting** — discovered athletes, verified juniors, completed source requests, available histories, and
  unavailable/blocked evidence reported separately. A completed rankings plan is **not** proof of a complete roster or career history.

### 6.2 Division reachability
Four divisions (outdoor boys/girls, indoor boys/girls) are *reachable*; **two are executed** (outdoor boys, indoor girls). The other
two need the same fixture qualification before any claim.

### 6.3 Remaining executable gates (in order)
1. **Live-source qualification** against real endpoints with manual challenge handling. Explicitly still only derivations: the indoor
   page season identity (`SeasonID = 12026`) and the indoor navigation's own division. The code fails closed on mismatch rather than
   accepting foreign evidence, and no real indoor navigation/page has been retained. **No live-readiness claim is permitted** until this exists.
2. **Workbook-scale identity-matched delivery** — the 142,705-athlete consolidated delivery is a *separate* rankings delivery, not an
   identity-matched original workbook; matching it to original source rows needs its own native run and verifier evidence.
3. **Expanded roster implementation** per §6.1.
4. **Verifier rankings fixture** — the verifier's rankings path is exercised end-to-end by the executed indoor stopped-writer
   verification, but it has no synthetic fixture of its own; a fixture-driven verifier case for a frozen rankings collection is a
   worthwhile addition.
5. **Failure-path proof** — `SourceCache` deliberately does not cache ranking failures; an execution proof of that failure path is
   still open.
6. **Throughput/latency measurement** before raising admission: requests per athlete, cache reuse, throttle/challenge counts.
   Collection throughput is bounded by the browser lane's page workers, so raising requested concurrency does not by itself raise parallelism.

### 6.4 Scaling rules for the expansion
- All scopes share profile admission/cooldown; never multiply independent request budgets per division.
- Bound network, CPU, storage, and local-model concurrency **independently**.
- Prefer available authorized bulk responses; deduplicate athlete/profile work across events; reuse immutable evidence.
- Pause on challenge; stop new profile admission; never promise zero upstream 429s.

---

## 7. Operations quick reference

### 7.1 Fixture deployment (the one used for executed qualification) [RECORDED]
Restate admin `21041`, ingress `21042`, fixture `21046`, worker `21140`, fresh stores such as `indoor-girls-store`,
source copy per store (`indoor-input/original-synthetic.xlsx`). (CHROMIUM_DESIGN.md still lists fixture `21043` — a doc drift worth fixing.)

### 7.2 Drive recipe
```bash
# 1. browser readiness (durable; a closed gate makes every fetch fail HumanRequired)
athletic-rust-pipeline browser-start  --ingress http://127.0.0.1:21042
athletic-rust-pipeline browser-status --ingress http://127.0.0.1:21042     # want: ready

# 2. submit the run (concurrency MUST be ≤ worker row_concurrency)
athletic-rust-pipeline start --ingress http://127.0.0.1:21042 \
  --input <xlsx copy owned by this store> --sha256 <hex> --all \
  --rankings --rankings-gender m --rankings-season outdoor \
  --max-pages-per-event 10000 --concurrency 2 --snapshot <label>          # → {run, invocation, state:"submitted"}

# 3. observe / control
athletic-rust-pipeline status          --ingress … --run <RUN>
athletic-rust-pipeline rankings-status --ingress … --run <RUN>
athletic-rust-pipeline rankings-pause  --ingress … --run <RUN>            # durable, genuine
athletic-rust-pipeline rankings-resume --ingress … --run <RUN>            # explicit; performs browser recovery

# 4. publish, then verify against the STOPPED store (owner-online export first, then stop the sole writer)
athletic-rust-pipeline export --ingress … --run <RUN> --output /path/result.xlsx
athletic-rust-pipeline verify --input /path/result.xlsx --output /path/result.jsonl --sha256 <hex> --store <store-dir>
```

### 7.3 Failure modes and their real causes
| Symptom | Real cause |
|---|---|
| `HTTP 500 requested concurrency exceeds worker capacity` | `--concurrency` > worker `row_concurrency` |
| `requested artifact is absent` at `prepare` | fresh store + same original path; give the store its own copy |
| Every fetch fails `HumanRequired`; state `challenged`/`restarting` | gate never opened — bootstrap page not a clean 200 (or a real challenge) |
| Collection runs to a `PageLimit` pause instead of sealing | source keeps returning rows for every page (non-terminating pagination) |
| `terminal ranking page does not cover the source minCount lower bound` | terminal page's `minCount` exceeds distinct/`max` stored row positions |
| `pagination does not advance to the next consecutive page` | capture's `next_page` != `page + 1` |
| Second process cannot open the store | Fjall is single-writer; stop the owner first |

---

## 8. Working rules for agents on this repo
- **No Docker, ever.** Restate is the native binary (`restate-server`); the `restate` CLI is *not* installed here.
- Never touch the private profiles/stores (`athletic-local`, `cache-cutover-v1`, `evidence-repairs-v8`, `fixture-native-v2`); the
  `athletic-local` instance holds a retained `SourceGateway/global.blocked` key that must stay, and `deploy` correctly refuses while it exists.
- No live Athletic.net requests while qualification is pending; manual challenge handling only.
- Model routing: codex-backed agent roles (`task`, `reviewer`, `scout`, `smol`, `slow`) currently fail with
  `usage_limit_reached`. Use the local GPU agents (`gpu5090-coder`, `gpu3090-coder`, `gpu5090-reviewer`, `gpu3090-reviewer`) or the
  DeepSeek-Flash agent (`.omp/agents/deepseek-flash.md`, `model: deepseek/deepseek-v4-flash:max`).
- A `restate` skill is **not** registered in this harness. Only a stale SDK-0.8.0-era copy exists at
  `/home/lewis/.claude/skills.backup/claude-skills/restate/SKILL.md` while the repo uses SDK 0.12.0 — salvage only its
  still-valid parts (error taxonomy, `ctx.run` discipline, object exclusivity, introspection commands).
- Main owns project-wide `fmt`/`clippy`/`test` sweeps and commits; subagents are confined to explicitly assigned files and never commit.
