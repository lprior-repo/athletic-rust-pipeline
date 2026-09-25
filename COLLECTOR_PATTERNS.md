# Collector patterns — what exists, what to steal, what is missing

**Superseded 2026-09-25:** `SOURCE_ADAPTER_GUIDE.md` and `crates/census-crawl/src/registry/` own this subject now. Kept as the dated record of 2026-09-20 reconnaissance.

**Date:** 2026-09-20. **Reading scope:** read-only inventory of this tree at `c5c4c48` plus the
**untracked** `crates/census-service/**` (in-flight work owned by another working session; its source
was observed to change mid-inventory, so all line numbers are a snapshot). Companion documents:
[`SOURCES_SURVEY.md`](SOURCES_SURVEY.md) (source inventory + the ≤2 RPS operating policy),
[`PROFILE_REPLICATION.md`](PROFILE_REPLICATION.md) (cohort definition + phased plan).

**Tree note (2026-09-21):** the in-flight work landed. `crates/census-service/**` and
`crates/census-domain/**` are tracked in git now, the census domain types moved out of
`crates/census-service/src/model.rs` into `crates/census-domain/src/{model,jurisdiction}.rs`, and
`crates/census-crawl/src/net/` was rewritten into a module directory with retries, conditional GET
and robots counting. Line numbers below are the 2026-09-20 snapshot; §2 carries a status column with
what was re-checked on 2026-09-21.

**Evidence labels.** `[READ]` = static source reading with file:line; `[RUN]` = a command executed in
this session; `[RECORDED]` = a value taken from files the collector itself wrote. No `cargo`
build/test/lint was executed, so no behavioural claim below is an execution claim. The three claims
that carry the most weight were re-verified directly (`[RUN]` grep over
`crates/census-service/src`: zero matches for `retry|backoff|429|Retry-After|FetchError::Http`;
`net/execute.rs` contains `expect("host registered")`; `FetchStats` is only ever cloned into in-memory
reports). **Both of the first two of those had already changed by 2026-09-21** — see §2.

---

## 0. Headline

Three bodies of work already exist, and none of them is the missing piece on its own:

| Body | What it is | State |
|---|---|---|
| `crates/census-service/**` (untracked) | a **synchronous, dependency-light collector**: per-host paced fetch, robots, disk cache, journal-resume, canonical entity graph, 12 state adapters, coach/GPA-adjacent passes | **P1 essentially done for 12 midwest states** — 141,441 athletes, **31,657 Class-of-2027** with 100% profile-URL + grad-year evidence, 2,224 coaches, 611 with email `[RECORDED: var/census-service/out/report.json + census-by-state.csv]`. Performances/PRs/verification: **absent** |
| ~~`src/**`~~ (committed) | ~~the domain, verification and durability machinery: exact mark parsers + event ontology, recompute-don't-trust verifiers, per-row evidence envelopes, durable local-model lane, Restate topology~~ — **historical**: root package deleted 2026-09-23 (`Cargo.toml` header + `ARCHITECTURE.md` §1)
| `SOURCES_SURVEY.md` §0/§14 + `PROFILE_REPLICATION.md` | the **recorded policy and plan**: 2 RPS/host ceiling, `HostPolicy` fields, backoff ladder, phased P0–P6 | written; P2–P5 unimplemented |

The consequence: the collector's *acquisition* half is already built and running; the
**performance lane (P2/P3), deterministic PR computation, and the verification lane (P5)** are what
remain, and the main pipeline already contains working implementations of the *patterns* those need.

---

## 1. Reuse map by plan phase

### P0 — scheduler, pacing, budget

| Asset | Verdict | Why |
|---|---|---|
| `net/client.rs::HostState` + `net/execute.rs::host_gate` + `wait_turn` | **ADAPT** | Already start-to-start spacing per host and takes `robots Crawl-delay` as a floor; reserves the next slot before sleeping, so queued tasks cannot bunch. Missing: explicit `rps_ceiling`/`min_spacing_ms`/`max_inflight` fields, burst capacity, and a realised-delay record |
| `net/robots.rs::RobotsRules::allows` | **REUSE AS-IS** | Longest-prefix match, allow wins ties — correct enough for every host observed in the survey |
| `net/client.rs::Fetcher::new` | **REUSE AS-IS** | 45 s total / 15 s connect timeouts, 5-redirect limit, honest UA, 32 MiB body cap (`MAX_BODY_BYTES`) checked against both `Content-Length` and the read body |
| `net/mod.rs::FetchStats` | **ADAPT** | The right counters (`requests`, `cache_hits`, `conditional_304`, `robots_blocked`, `bytes_downloaded`, `errors`, `per_host`) but they are never persisted — P0 requires the per-host budget in the run log |
| ~~`src/runtime/source/admission.rs`~~ | ~~**PATTERN ONLY**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`src/runtime/source/retry.rs::next_delay`~~ | ~~**PATTERN ONLY**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`src/cli/flow_control.rs::source_rules`~~ | ~~**PATTERN ONLY**~~ — **historical**: root crate deleted 2026-09-23.

### P1 — cohort seed (**already delivered, midwest evidence only**)

| Asset | Verdict | Why |
|---|---|---|
| `census-domain/src/model.rs::GradYear::of` / `ObservedGrade` | **REUSE AS-IS** | `grad_year = school_year.start + 13 - grade`; the cohort key is derived, never assumed, and the observation carries its source — this is the `SCOPE.md` evidence rule in code |
| `census-domain/src/model.rs::SchoolYear::containing` | **REUSE AS-IS** | Aug-1 boundary; grade evidence is season-bound by construction |
| ~~`sources/milesplit.rs::parse_roster` + `roster_entities`~~ (historical: was `src/sources/milesplit.rs`) | ~~**REUSE AS-IS**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`report/`~~ (`build_census` + `write_census`) (historical: was `src/report/`) | ~~**REUSE AS-IS**~~ — **historical**: root crate deleted 2026-09-23.
| MaxPreps sitemap path (`PROFILE_REPLICATION.md` §1) | **GAP** | Not implemented anywhere; the census is MileSplit/association-seeded (2026-09-21: the jurisdiction type covers all 51 US jurisdictions and the MileSplit host is derived per jurisdiction, but the *ingested* evidence is still Midwest state associations plus MileSplit rosters) |

### P2 — performances + grade evidence per row (the main gap)

| Asset | Verdict | Why |
|---|---|---|
| `census-domain/src/model.rs::CanonicalPerformance` | **REUSE AS-IS** | `{athlete, team, event, meet, date, mark, wind_mps, place, heat, round, timing, observed_grade, evidence, source_key}` — already the full field set the target requires |
| `census-domain/src/model.rs::EventKind` + `SourceEventLabel` | **REUSE AS-IS** | 35 canonical variants with `Unmapped{label}` preservation; `from_source_label` maps vendor labels without losing the raw string |
| `census-domain/src/model.rs::Mark` | **ADAPT** | `Raw(String)` keeps unparsed marks honest, but the numeric variants are `f64`; the main pipeline's integer-exact parsers are strictly better (below) |
| ~~`store/entities.rs::Entity for CanonicalPerformance` + `source_key`~~ (historical: was `src/store/entities.rs`) | ~~**ADAPT**~~ — **historical**: root crate deleted 2026-09-23.
| MileSplit `/raw` HY-TEK text + performance API (`SOURCES_SURVEY.md` §2) | **GAP** | Nothing in the crate fetches per-performance data; `out/events.jsonl` and `out/performances.jsonl` are 0 B `[RECORDED]` |

### P3 — history / cross-check

| Asset | Verdict | Why |
|---|---|---|
| `census-domain/src/model.rs::SourceNamespace` | **REUSE AS-IS** | Already declares `TfrrsAthlete`, `TfrrsTeam`, `DirectAthletics*`, `TimerMeet{provider}`, `LegacyAthleticNet{kind}` — the history sources are pre-modelled |
| ~~`report/`~~ (`ProviderCoverage`) (historical: was `src/report/`) | ~~**REUSE AS-IS**~~ — **historical**: root crate deleted 2026-09-23.
| TFRRS adapter | **GAP** | No module; TFRRS is static HTML with `ETag`s and one request per athlete career |

### P4 — enrichment (coach, GPA)

| Asset | Verdict | Why |
|---|---|---|
| ~~`sources/wiaa/`, `ks.rs`, `ihsa/`, `ohsaa/`, `mshsl/`, `plain_names/`, `coach_contacts.rs`~~ (historical: was `src/sources/...`) | ~~**REUSE AS-IS**~~ — **historical**: root crate deleted 2026-09-23.
| `census-domain/src/model.rs::CanonicalCoach` / `CoachRole` | **REUSE AS-IS** | Role + optional sport binding + published email (classified as professional/personal by domain) + no phone, all evidence-carrying |
| GPA as a nullable field with a source enum | **GAP** | Not modelled yet; plan §6 requires `recruiting_profile | academic_list | school_page` and never-inferred values |

### P5 — verification lane (deterministic gates first)

| Asset | Verdict | Why |
|---|---|---|
| ~~`src/result_verify/assessment.rs::verify`~~ | ~~**PATTERN ONLY → port**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`src/result_verify/checks/mod.rs::verify_embedded_artifacts`~~ | ~~**PATTERN ONLY → port**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`checks/mod.rs::VerifiedState::add_to`~~ (historical: was `src/result_verify/checks/mod.rs`) | ~~**PATTERN ONLY → port**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`src/result_verify.rs::verify_results` + `validate_line`~~ | ~~**PATTERN ONLY → port**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`coverage.rs::verify` / `verify_discovery`~~ (historical: was `src/result_verify/coverage.rs`) | ~~**PATTERN ONLY → port**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`assessment.rs::admit` + `ByteCount`~~ (historical: was `src/result_verify/assessment.rs`) | ~~**PATTERN ONLY → port**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`src/runtime/reviewer/mod.rs::LocalReviewer`~~ | ~~**PATTERN ONLY**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`reviewer/model.rs::ChatRequest` + `input.rs::prepare`~~ (historical: was `src/runtime/reviewer/model.rs` / `input.rs`) | ~~**REUSE AS-IS (shape)**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`src/runtime/review_case.rs`~~ / ~~`src/runtime/protocol.rs::Review*`~~ | ~~**PATTERN ONLY**~~ — **historical**: root crate deleted 2026-09-23.

### P6 — PR computation (deterministic only)

| Asset | Verdict | Why |
|---|---|---|
| ~~`src/domain/marks/event.rs::EventName`~~ | ~~**PORT/EXTRACT**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`event.rs::track_event/field_event/hurdles_event/relay_event/cross_country_name`~~ (historical: was `src/domain/marks/event.rs`) | ~~**PORT/EXTRACT**~~ — **historical**.
| ~~`src/domain/marks/parser.rs::parse_time/parse_distance`~~ | ~~**PORT/EXTRACT**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`event.rs::flat_name/hurdle_name/…`~~ (historical: was `src/domain/marks/event.rs`) | ~~**ADAPT on port**~~ — **historical**: root crate deleted 2026-09-23.
| ~~`src/domain/performance_evidence.rs`~~ | ~~**PORT/EXTRACT**~~ — **historical**: root crate deleted 2026-09-23.
| PR/best computation itself | **GAP (both codebases)** | No PR function exists anywhere: group by `(event identity, gender)`, take the valid minimum under `is_lower_better`, honour wind/timing flags; the source's own PR table is stored as a cross-check only |

---

## 2. Transport defects found in `crates/census-service` (each is small and blocking)

**Status re-checked 2026-09-21** against the current tree (`net/` is now a module directory:
`net/{mod,request,execute,execute/attempt,cache,robots,decode,client,tests}.rs`). The list below is
the 2026-09-20 snapshot; four of the nine are fixed and one is structurally different:

| # | 2026-09-20 finding | Status 2026-09-21 |
|---|---|---|
| 1 | 429/5xx returned as `Ok`, no backoff, `FetchError::Http` never constructed | **Fixed.** `FetchError::Http` carries the status; `retry_loop` retries transient statuses (`FetchError::retryable`: 429/5xx) up to `MAX_RETRIES = 3` with `wait_backoff`, and `status_verdict` fails 404 when `allow_not_found` is false (`net/execute/attempt.rs`, `net/mod.rs:57-58,80,147`) |
| 2 | Conditional GET unreachable; 304 replays a cached error body | **Fixed.** A 304 goes to `replay_cached`, which publishes the cached body with refreshed timestamps and counts `stats.conditional_304` (`net/execute/attempt.rs:70,136`) |
| 3 | Cache key is request-keyed, not content-addressed; no `body_sha256 → key` index | **Still true.** `Fetcher::key_for` still hashes `method ␟ url ␟ extra`, and `CacheMeta.sha256` is still the 16-byte body prefix (`net/cache.rs`) |
| 4 | robots.txt fetches bypass pacing and counters and fail open on 5xx | **Still true.** `robots_for` calls `fetch_text_uncached`, which uses the raw client with no host gate and no `FetchStats`, and any non-200 (including 5xx) leaves `fetched: false`, i.e. allow-all (`net/robots.rs:45-79`) |
| 5 | `hosts.get_mut(host).expect("host registered")` panic surface in the pacing path | **Fixed.** No `expect(`/`unwrap()` remains in `crates/census-service/src` outside `#[cfg(test)]` (the gate's scan records `expect = 0`, `unwrap = 0` for the crate) |
| 6 | `FetchStats` never persisted; no run log | **Partly true.** Per-adapter stats now flow into `AdapterReport` (`sources/mshsl/collect/run.rs:235`, `sources/wiaa/collect.rs:96`) and are printed, but no `out/run-*.json` writer exists in the crate |
| 7 | `let _ = store.journal_done(...)` discards the append result | **Fixed.** `record_roster` matches on the journal result and reports the failure (`census/sweep.rs:105-110`) |
| 8 | Tail-tolerance inverted: append-log reader `bail!`s on the first bad line | **Structurally changed, not re-verified.** The JSONL path is now the one-time legacy import (`store/legacy.rs`, line-oriented `BufReader::lines()`); whether a crash-truncated tail still wedges it was not exercised |
| 9 | No global concurrency bound and no signal drain for the batch CLI | **Still true for the batch CLI.** Fan-out is still `concurrency × state_concurrency` (`census/mod.rs`); the drain protocol belongs to the service (`bootstrap/stop.rs`, `drain.rs`), not to `census-service collect` |

1. **429/5xx are returned as `Ok`.** `FetchError::Http` (net/mod.rs::FetchError) is never constructed `[RUN]`; a
   non-2xx is counted in `stats.errors` and `warn!`ed (net/execute.rs::fetch) and then handed back as a
   successful `FetchOutcome{status}`. There is **no** `Retry-After` handling, no 429 branch and no
   backoff anywhere in the crate `[RUN: grep for retry|backoff|429|Retry-After → 0 matches]`. Fix:
   port `retry.rs`'s ladder (429 → 60/120/240 s; else 1/2/4 s; ≤4 attempts) plus host cooldown.
2. **Conditional GET is unreachable on the happy path.** A cached 200 returns before the validator
   headers are attached, and `--refresh` suppresses them; the only reachable 304 replays a cached
   *error* body, so `conditional_304` is effectively dead. Fix: revalidate the cached 200 instead of
   blind-returning it, and never send `If-None-Match` together with `--refresh`.
3. **The archive is request-keyed, not content-addressed.** `key_for` (226-236) hashes
   `method ␟ url ␟ body`; `meta.sha256` is a separate, also-truncated 16-byte body digest, so
   request identity and content identity are indistinguishable in a listing and there is no
   `body_sha256 → key` index. `read_cache` (572-583) never re-verifies the body against the meta.
   Fix: store the full 64-hex body digest in the meta, add an index, verify on read.
4. **Robots fetches bypass pacing and counters, and fail open on 5xx.** `fetch_text_uncached`
   (298-311) issues robots.txt outside the host gate and outside `FetchStats`, and any non-200 marks
   robots as “not fetched” → allow everything (275-296). The module doc (5-11) also claims a disk
   cache that does not exist. Fix: route robots through the gate, count it, treat 5xx as “retry
   later”, and correct the doc.
5. **A panic surface in the pacing path.** `hosts.get_mut(host).expect("host registered")`
   (net/execute.rs) `[RUN]` — an in-process invariant asserted with a panic, in the one function that
   every request passes through. Fix: return a typed error (the repo rule is no
   `expect`/`unwrap` in production paths).
6. **`FetchStats` is never persisted.** It is cloned into the in-memory `CollectReport`
   (census/aggregate.rs) and printed; nothing writes a run log, so the P0 requirement “projected
   per-host request budget written to the run log” is unmet `[RUN]`.
7. **Resume can silently repeat work.** `let _ = store.journal_done(...)` in the roster loop
   (census/sweep.rs::record_roster) discards the append result, and `source_key`'s documented upsert has no
   implementation. Fix: propagate the journal error; wire or delete `source_key`.
8. **Tail-tolerance is no longer inverted** (closed by the Fjall substrate). This item described
   `report::read_rows` tolerating one unparseable line in the *consolidated* snapshot while
   `store::consolidate` `bail!`ed on the first bad line of the *append logs* — the files a crash could
   truncate mid-line. Both halves are gone. The snapshot reader is strict (`store/read/snapshot.rs`,
   `StoreError::SnapshotRow`, bad middle and bad last row each covered by a test that says why), and
   consolidation merges rows out of Fjall (`store/read/mod.rs`) rather than reading append logs. The
   one remaining JSONL read is the one-time legacy import, which commits a chunk's rows together with
   the byte offset that follows them, so an interrupted import resumes at its last commit instead of
   restarting from the file's head (`store/legacy.rs`).
9. **No global concurrency bound and no signal drain.** Fan-out comes only from
   `concurrency × state_concurrency` (census/mod.rs); `main` runs to completion with no SIGINT
   drain, which matters for multi-hour walks. Fix: bound total in-flight requests and drain the
   journal on shutdown.

---

## 3. Where the work should live

- **Acquisition stays in `crates/census-service`.** It is synchronous, robots-aware, journal-resumable,
  and already 292 k athlete observations deep. Do **not** move it under Restate/Chromium: the
  recorded policy (`SOURCES_SURVEY.md` §0) is explicit that the 2 RPS collector is its own polite
  path, and the main pipeline's ingestion contract is workbook-shaped.
- **Share the domain rather than re-deriving it.** Two low-risk options, in order:
  1. `census-service` takes a **path dependency on the library crate** and uses
     `domain::marks::{event, parser, compare}` directly — one line in `Cargo.toml`, zero duplication.
  2. Extract `domain/marks` (plus `performance_evidence`) into a `crates/athletic-domain` shared
     crate, moving its tests with it. More work, smaller dependency closure.
  Never a third copy of the mark grammar: duplicate parsers are exactly how two conventions appear.
- **Verification is ported, not merged.** The `result_verify` *patterns* (recompute, three-way CAS
  identity, checked counters, streaming line validation, budget replay) belong in the collector as
  its own deterministic gate; the workbook verifier stays where it is.
~~- One prerequisite if the collector is ever wired into the pipeline: `runtime/config.rs::validate_source`~~ (historical: was `src/runtime/config.rs`, root crate deleted 2026-09-23)
- **Coordination note (updated 2026-09-21):** the earlier warning that "`crates/` and `var/` are
  untracked and were being written by another session" no longer applies to `crates/` — it is tracked
  in git (`git ls-files crates` lists it, including `crates/census-domain/**`). `var/` remains
  untracked runtime output. Nothing in this document should be applied to another session's files
  without checking who owns them (`AGENTS.md` §2).

---

## 4. Next actions, smallest-first

1. **Transport hardening (P0 remainder):** items 1–6 above — one file (`net/`) plus its first
   `#[tokio::test]`s (cache hit, 304 revalidation, 429 backoff, size cap, host gating).
2. **Persist the run log:** `out/run-<timestamp>.json` = `FetchStats` + effective per-host delay +
   projected budget, written before the first fetch (dry-run) and updated after.
3. **Performance lane (P2):** MileSplit `/raw` HY-TEK text first (exact columns, no JSON contract
   needed) → `CanonicalEvent` + `CanonicalPerformance` with `source_key` upserts; then the
   performance API for wind/round/place.
4. **PR computation:** port `domain/marks` per §3 and compute event-specific bests with the
   comparability/`is_lower_better` rules; keep the source's PR table as a cross-check that can
   disagree loudly.
5. **Verification lane (P5):** recompute-and-compare over the consolidated snapshots, three-way
   evidence binding, checked counters; then route genuine ambiguity only through the existing local
   Qwen lane using `reviewer/model.rs`'s request shape.
