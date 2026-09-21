# Collector patterns — what exists, what to steal, what is missing

**Date:** 2026-09-20. **Reading scope:** read-only inventory of this tree at `c5c4c48` plus the
**untracked** `crates/midwest-census/**` (in-flight work owned by another working session; its source
was observed to change mid-inventory, so all line numbers are a snapshot). Companion documents:
[`SOURCES_SURVEY.md`](SOURCES_SURVEY.md) (source inventory + the ≤2 RPS operating policy),
[`PROFILE_REPLICATION.md`](PROFILE_REPLICATION.md) (cohort definition + phased plan).

**Evidence labels.** `[READ]` = static source reading with file:line; `[RUN]` = a command executed in
this session; `[RECORDED]` = a value taken from files the collector itself wrote. No `cargo`
build/test/lint was executed, so no behavioural claim below is an execution claim. The three claims
that carry the most weight were re-verified directly (`[RUN]` grep over
`crates/midwest-census/src`: zero matches for `retry|backoff|429|Retry-After|FetchError::Http`;
`net.rs:261` contains `expect("host registered")`; `FetchStats` is only ever cloned into in-memory
reports).

---

## 0. Headline

Three bodies of work already exist, and none of them is the missing piece on its own:

| Body | What it is | State |
|---|---|---|
| `crates/midwest-census/**` (untracked) | a **synchronous, dependency-light collector**: per-host paced fetch, robots, disk cache, journal-resume, canonical entity graph, 12 state adapters, coach/GPA-adjacent passes | **P1 essentially done for 12 midwest states** — 141,441 athletes, **31,657 Class-of-2027** with 100% profile-URL + grad-year evidence, 2,224 coaches, 611 with email `[RECORDED: var/midwest-census/out/report.json + census-by-state.csv]`. Performances/PRs/verification: **absent** |
| `src/**` (committed) | the **domain, verification and durability** machinery: exact mark parsers + event ontology, recompute-don't-trust verifiers, per-row evidence envelopes, durable local-model lane, Restate topology | mature, fixture-qualified; bound to one source origin and one workbook pipeline |
| `SOURCES_SURVEY.md` §0/§14 + `PROFILE_REPLICATION.md` | the **recorded policy and plan**: 2 RPS/host ceiling, `HostPolicy` fields, backoff ladder, phased P0–P6 | written; P2–P5 unimplemented |

The consequence: the collector's *acquisition* half is already built and running; the
**performance lane (P2/P3), deterministic PR computation, and the verification lane (P5)** are what
remain, and the main pipeline already contains working implementations of the *patterns* those need.

---

## 1. Reuse map by plan phase

### P0 — scheduler, pacing, budget

| Asset | Verdict | Why |
|---|---|---|
| `net.rs::HostState` + `host_gate` + `wait_turn` (238-273) | **ADAPT** | Already start-to-start spacing per host and takes `robots Crawl-delay` as a floor; reserves the next slot before sleeping, so queued tasks cannot bunch. Missing: explicit `rps_ceiling`/`min_spacing_ms`/`max_inflight` fields, burst capacity, and a realised-delay record |
| `net.rs::RobotsRules::allows` (131-156) | **REUSE AS-IS** | Longest-prefix match, allow wins ties — correct enough for every host observed in the survey |
| `net.rs::Fetcher::new` (178-204) | **REUSE AS-IS** | 45 s total / 15 s connect timeouts, 5-redirect limit, honest UA, 32 MiB body cap (`MAX_BODY_BYTES`) checked against both `Content-Length` and the read body |
| `net.rs::FetchStats` (112-120) | **ADAPT** | The right counters (`requests`, `cache_hits`, `conditional_304`, `robots_blocked`, `bytes_downloaded`, `errors`, `per_host`) but they are never persisted — P0 requires the per-host budget in the run log |
| `src/runtime/source/admission.rs` (23-46, 95-130) | **PATTERN ONLY** | Durable `not-before-ms` deadline + bounded waiter queue is the right *idea*, but it is Restate-object state; the collector is deliberately synchronous — port the semantics, not the code |
| `src/runtime/source/retry.rs` (5-74) | **PATTERN ONLY** | `next_delay`: `Retry-After` wins, else `429 → [60,120,240] s`, other retryable → `[1,2,4] s`, clamp by attempt, floor at the pacing interval, `MAX_ATTEMPTS = 4`, `MAX_RETRY_DELAY = 86_400 s`, no jitter. This is the exact ladder the census transport lacks |
| `src/cli/flow_control.rs::source_rules` (99-126) | **PATTERN ONLY** | Server-side pattern+concurrency rules and refusal to shadow a stricter wildcard; the collector's equivalent is a per-host concurrency of 1 enforced in-process |

### P1 — cohort seed (**already delivered, midwest only**)

| Asset | Verdict | Why |
|---|---|---|
| `model.rs::GradYear::of` / `ObservedGrade` (163-186) | **REUSE AS-IS** | `grad_year = school_year.start + 13 - grade`; the cohort key is derived, never assumed, and the observation carries its source — this is the `SCOPE.md` evidence rule in code |
| `model.rs::SchoolYear::containing` (105-123) | **REUSE AS-IS** | Aug-1 boundary; grade evidence is season-bound by construction |
| `sources/milesplit.rs::parse_roster` (148-208) + `roster_entities` (288-380) | **REUSE AS-IS** | Roster rows are dropped when the grad cell or name is missing (fail-closed), `identity_confidence = HIGH` only for the 2027 cohort |
| `report.rs::build_census` + `write_census` (133-439) | **REUSE AS-IS** | Every cohort number is paired with a coverage counter; `co2027_multisource = 0` is reported honestly rather than papered over |
| MaxPreps sitemap path (`PROFILE_REPLICATION.md` §1) | **GAP** | Not implemented anywhere; the census is Midwest-only and MileSplit/association-seeded |

### P2 — performances + grade evidence per row (the main gap)

| Asset | Verdict | Why |
|---|---|---|
| `model.rs::CanonicalPerformance` (897-921) | **REUSE AS-IS** | `{athlete, team, event, meet, date, mark, wind_mps, place, heat, round, timing, observed_grade, evidence, source_key}` — already the full field set the target requires |
| `model.rs::EventKind` (431-521) + `SourceEventLabel` | **REUSE AS-IS** | 35 canonical variants with `Unmapped{label}` preservation; `from_source_label` maps vendor labels without losing the raw string |
| `model.rs::Mark` (870-882) | **ADAPT** | `Raw(String)` keeps unparsed marks honest, but the numeric variants are `f64`; the main pipeline's integer-exact parsers are strictly better (below) |
| `store.rs::Entity for CanonicalPerformance` (371-400) + `source_key` | **ADAPT** | `source_key` is documented as the idempotent-upsert key but **no consumer exists** — wire it into `merge` or delete it; a documented-but-unused idempotency key is exactly what the verification lane will flag |
| MileSplit `/raw` HY-TEK text + performance API (`SOURCES_SURVEY.md` §2) | **GAP** | Nothing in the crate fetches per-performance data; `out/events.jsonl` and `out/performances.jsonl` are 0 B `[RECORDED]` |

### P3 — history / cross-check

| Asset | Verdict | Why |
|---|---|---|
| `model.rs::SourceNamespace` (276-294) | **REUSE AS-IS** | Already declares `TfrrsAthlete`, `TfrrsTeam`, `DirectAthletics*`, `TimerMeet{provider}`, `LegacyAthleticNet{kind}` — the history sources are pre-modelled |
| `report.rs::ProviderCoverage` (77-87) | **REUSE AS-IS** | Already counts “Athletic.net URLs known without any Athletic.net request” — the no-broad-crawl thesis, measured |
| TFRRS adapter | **GAP** | No module; TFRRS is static HTML with `ETag`s and one request per athlete career |

### P4 — enrichment (coach, GPA)

| Asset | Verdict | Why |
|---|---|---|
| `sources/wiaa.rs`, `ks.rs`, `ihsa.rs`, `mhsaa.rs`, `ohsaa.rs`, `mshsl.rs`, `plain_names.rs`, `coach_contacts.rs` | **REUSE AS-IS** | Association APIs/directories → staff → `CanonicalCoach` with `CoachRole` (AD is school-wide, never sport-bound) and per-field evidence; 526 KS + 526 emails already imported `[RECORDED]` |
| `model.rs::CanonicalCoach (638-653)` / `CoachRole (691-696)` | **REUSE AS-IS** | Role + optional sport binding + professional email + phone, all evidence-carrying |
| GPA as a nullable field with a source enum | **GAP** | Not modelled yet; plan §6 requires `recruiting_profile | academic_list | school_page` and never-inferred values |

### P5 — verification lane (deterministic gates first)

| Asset | Verdict | Why |
|---|---|---|
| `src/result_verify/assessment.rs::verify` (~10) | **PATTERN ONLY → port** | **Recompute** `decision::assess(...)` from the evidence and demand exact equality with the exported assessment — the single most valuable idea in the repo for the collector |
| `src/result_verify/checks/artifacts.rs::verify_embedded_artifacts` (~60) | **PATTERN ONLY → port** | Three-way identity per artifact: embedded JSON == typed serde re-serialization == retained bytes at the referenced digest |
| `checks/mod.rs::VerifiedState::add_to` (~33) | **PATTERN ONLY → port** | `checked_add` with an explicit overflow error; cohort counters must not wrap |
| `src/result_verify.rs::verify_results` (~61) + `validate_line` (~113) | **PATTERN ONLY → port** | Stream a JSONL snapshot, cap each line, reject duplicate keys — the shape a census verifier needs |
| `coverage.rs::verify` (~20) / `verify_discovery` (~115) | **PATTERN ONLY → port** | “Every reference is consumed exactly once, no unbound extras”; candidate-ID set equality between stages |
| `assessment.rs::admit` (~145) + `ByteCount` | **PATTERN ONLY → port** | Replays the runtime's per-row byte budget instead of trusting it — directly reusable as the collector's per-athlete parse budget |
| `src/runtime/reviewer.rs::LocalReviewer` (20-340) | **PATTERN ONLY** | Durable object, one lane key, request content-addressed **before** the call, `blocked` latch on artifact failure, cooldown = max `Retry-After`, receipts retained through exhaustion. The collector's AI lane should copy this shape without Restate |
| `reviewer/model.rs::ChatRequest` (31-53) + `input.rs::prepare` (15-45) | **REUSE AS-IS (shape)** | OpenAI-compatible request typing and 3-line endpoint/model resolution; the prompt text itself is task-specific (`PATTERN ONLY`) |
| `review_case.rs` / `model.rs::Review*` protocol types | **PATTERN ONLY** | Verdict + evidence envelope for “AI proposes, deterministic gate decides” |

### P6 — PR computation (deterministic only)

| Asset | Verdict | Why |
|---|---|---|
| `src/domain/marks/event.rs::EventName` (5-130) | **PORT/EXTRACT** | 17-variant identity with a lossless `Unsupported(String)` tail; `event_key`, `is_lower_better`, `is_comparable_event`, `value_kind` — comparability is a property of identity, so no caller table is needed |
| `event.rs::track_event/field_event/hurdles_event/relay_event/cross_country_name` (154-330) | **PORT/EXTRACT** | Flat events incl. 500/600/2000, hurdles set, relays, and **XC as distances** (`xc2k…xc12k`, `xc2mile|3mile|5mile|6mile`) — the “a 5 k and a 3-mile time are not interchangeable” rule is already encoded |
| `src/domain/marks/parser.rs::parse_time/parse_distance` (4-100) | **PORT/EXTRACT** | Exact integer arithmetic (milliseconds, nanometres), no floats, no silent unit assumption; bare numbers are hard errors |
| `event.rs::flat_name/hurdle_name/…` (278-330) | **ADAPT on port** | Silent `_ => "unsupported"` / `_ => "cross_country"` fallbacks merge identities — replace with explicit arms when adding events |
| `src/domain/performance_evidence.rs` | **PORT/EXTRACT** | Carries the indoor/outdoor surface (`season`) that `EventKind`/`TrackEvent` deliberately lacks |
| PR/best computation itself | **GAP (both codebases)** | No PR function exists anywhere: group by `(event identity, gender)`, take the valid minimum under `is_lower_better`, honour wind/timing flags; the source's own PR table is stored as a cross-check only |

---

## 2. Transport defects found in `crates/midwest-census` (each is small and blocking)

1. **429/5xx are returned as `Ok`.** `FetchError::Http` (net.rs:35) is never constructed `[RUN]`; a
   non-2xx is counted in `stats.errors` and `warn!`ed (net.rs:551-557) and then handed back as a
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
   (net.rs:261) `[RUN]` — an in-process invariant asserted with a panic, in the one function that
   every request passes through. Fix: return a typed error (the repo rule is no
   `expect`/`unwrap` in production paths).
6. **`FetchStats` is never persisted.** It is cloned into the in-memory `CollectReport`
   (census.rs:332-334) and printed; nothing writes a run log, so the P0 requirement “projected
   per-host request budget written to the run log” is unmet `[RUN]`.
7. **Resume can silently repeat work.** `let _ = store.journal_done(...)` in the roster loop
   (census.rs ≈215) discards the append result, and `source_key`'s documented upsert has no
   implementation. Fix: propagate the journal error; wire or delete `source_key`.
8. **Tail-tolerance is inverted.** `report::read_rows` (20-45) tolerates one unparseable row in the
   *consolidated* snapshot, while `store::consolidate` (119-164) `bail!`s on the first bad line of the
   *append logs* — the only files a crash can truncate mid-line. A crash during collection can
   therefore wedge consolidation permanently. Fix: skip a single trailing partial line in the
   append-log reader and fail on interior corruption.
9. **No global concurrency bound and no signal drain.** Fan-out comes only from
   `concurrency × state_concurrency` (census.rs 35-36); `main` runs to completion with no SIGINT
   drain, which matters for multi-hour walks. Fix: bound total in-flight requests and drain the
   journal on shutdown.

---

## 3. Where the work should live

- **Acquisition stays in `crates/midwest-census`.** It is synchronous, robots-aware, journal-resumable,
  and already 292 k athlete observations deep. Do **not** move it under Restate/Chromium: the
  recorded policy (`SOURCES_SURVEY.md` §0) is explicit that the 2 RPS collector is its own polite
  path, and the main pipeline's ingestion contract is workbook-shaped.
- **Share the domain rather than re-deriving it.** Two low-risk options, in order:
  1. `midwest-census` takes a **path dependency on the library crate** and uses
     `domain::marks::{event, parser, compare}` directly — one line in `Cargo.toml`, zero duplication.
  2. Extract `domain/marks` (plus `performance_evidence`) into a `crates/athletic-domain` shared
     crate, moving its tests with it. More work, smaller dependency closure.
  Never a third copy of the mark grammar: duplicate parsers are exactly how two conventions appear.
- **Verification is ported, not merged.** The `result_verify` *patterns* (recompute, three-way CAS
  identity, checked counters, streaming line validation, budget replay) belong in the collector as
  its own deterministic gate; the workbook verifier stays where it is.
- **One prerequisite if the collector is ever wired into the pipeline:** `runtime/config.rs::validate_source`
  hard-pins `https://www.athletic.net/` and `RawConfig` carries a single `source_origin`/`source_interval_ms`.
  A second host needs a per-origin allowlist with per-origin intervals before anything else works.
- **Coordination note:** `crates/` and `var/` are untracked and were being written by another session
  during this inventory. Nothing in this document should be applied to those files before that
  session's changes land.

---

## 4. Next actions, smallest-first

1. **Transport hardening (P0 remainder):** items 1–6 above — one file (`net.rs`) plus its first
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
