# ARCHITECTURE.md — what exists today, and where it is heading

Two layers, one evidence store, one durable execution model.

```
   ROOT CRATE: athletic-rust-pipeline           CENSUS: crates/midwest-census
   ─────────────────────────────────────        ─────────────────────────────────
   Restate services (runtime/*.rs)              net/       polite fetcher (robots, cache, pacing)
   browser session supervisor  ──┐              store/     Fjall observation store
   ranking collection            │              sources/*  one adapter per provider
   source dispatch / row workers │              census/    resumable orchestration
   search, xlsx, workbook tools  │              report/ / bests/ / workbook/
   Fjall-backed store            │              restate_services/  durable services
                                 │              bootstrap/  supervisor: drain + certificate
                                 └─────────────┐
                                               │
   DOMAIN: crates/census-domain                 │
   ─────────────────────────────────────        │
   model.rs     canonical entities + evidence   │
   jurisdiction.rs  UsJurisdiction (50 states + DC)
   error.rs     DomainError (no async, no I/O)
```

## 1. Execution model

Batch path (`midwest-census collect …`) and durable path (`midwest-serve`) share adapters, store and
reports. The difference is who owns the journal: in `midwest-serve`, Restate owns per-step progress,
so a crash resumes at the last recorded step and every write is idempotent.

`bootstrap/` (module root `crates/midwest-census/src/bootstrap/mod.rs`) owns exactly one task region
for the HTTP endpoint and implements the shutdown
protocol in five stages: stop intake (`serve_with_cancel` + hyper graceful shutdown) → drain inside
`--drain-timeout` → abort the remainder → finalize the store → print the `DrainReport`:

```
drained: accepted=<n> completed=<n> cancelled=<n> timed_out=<n> aborted=<n> panicked=<n>
```

`remaining = 0`, or an explicitly persisted unfinished workflow, is the only clean completion.

## 2. Store

Fjall is the system of record. No PostgreSQL: the downstream interface is Excel, not interactive SQL.

Three Fjall keyspaces exist — `entities`, `journal` and `meta` — all under the `fjall/` directory
inside `--data-dir`. The census **tables** (`Table` in `crates/midwest-census/src/store/mod.rs`:
`schools`, `teams`, `coaches`, `athletes`, `meets`, `events`, `performances`) are logical partitions
*inside* the `entities` keyspace, selected by the table-name byte prefix; `journal` holds the
write-journal/resume entries and `meta` the import markers. Keys are deterministic and sortable:
`table-file prefix | entity id | big-endian
sequence`, so a prefix scan reads one entity's observation history in write order. Readers merge
observations at read time; materialized JSONL snapshots (`out/*.jsonl`) are the read model.

Invariants: observations are append-only; merges are idempotent, commutative and associative;
nothing overwrites a prior observation, including school attribution after a transfer.

## 3. Source layer

Adapters are async functions over `AdapterContext` returning `AdapterReport` (no trait indirection).
Shared bounds: `sources::CONCURRENCY_BOUND` for fan-out, per-host delays from `default_host_delays()`
with a 10 s floor for Bound, and a cache-first fetcher that records request evidence (status, bytes,
sha256, fetched-at, cache hit) for every fetch.

Admission is per remote origin, not per Rust task: Athletic.net in particular draws from one shared
budget across rankings, profiles, bio/history and meet results. Tabs are execution lanes, never a
rate-limit budget.

Failure is recorded as evidence, never as absence. The root crate classifies source failure with
`FailureCode` (`src/runtime/protocol.rs`: `AccessDenied`, `BrowserChallenge`, `RateLimited`,
`RetryExhausted`, `MalformedResponse`, `UncertainEffect`, …) and browser failure with `BrowserError`
(`src/runtime/browser.rs`: `HumanRequired`, `Unavailable`, `TaskPanicked`, …); the census crate has
`FetchError` (`crates/midwest-census/src/net/mod.rs`) and `CrawlError`
(`crates/midwest-census/src/sources/mod.rs`). No `OperationTerminal<T>` type exists — earlier
revisions of this file named one.

## 4. Browser session

`src/runtime/browser/` is a supervisor around one persistent Chromium profile:

- `lifecycle.rs` launches (or attaches to) the browser and owns the actor task; both spawns are
  instrumented (`browser.handler`, `browser.actor`).
- `actor.rs` runs the supervisor loop over a `JoinSet` of jobs and observers.
- `request.rs` schedules work across page slots, wrapping each job in a `browser.job` span (slot,
  nonce), and classifies join outcomes: panic → `BrowserError::TaskPanicked`, cancellation →
  debug-level release, abort → warning; pending callers are rejected with the true cause.
- `management.rs` classifies observer exits the same way and restarts the browser through one path.
- `gate.rs`/`state.rs` implement the state machine: `Ready`, `Challenged`, `CoolingDown`,
  `HumanRequired`, `Restarting`, `Stopped`. A 429 applies a cooldown; a challenge closes admission
  and latches `HumanRequired` until a human resolves it. Nothing automates CAPTCHA solving.

## 5. Census data model

`crates/census-domain/src/model.rs` (pure; `cargo xtask domain-purity` proves its normal dependency
tree) carries weighted types instead of strings: `Id<T>`, `SchoolYear`, `Grade`, `GradYear`,
`ObservedGrade`, `Evidence { method, source }`, `SourceNamespace`, `SourceIdentity`, `Confidence`,
`Gender`, `Sport`, `CompetitionLevel`, `EventKind`, `Mark`, `TimingMethod`, `CoachRole`. Grade is
time-scoped evidence (`ObservedGrade` + season); the cohort is `GradYear` (Class of 2027), so
`grade 11 in 2025-2026` and `grade 12 in 2026-2027` are the same athlete by construction.

`crates/census-domain/src/jurisdiction.rs` holds `UsJurisdiction` — 50 states plus the District of
Columbia — and every adapter, workflow identity and Fjall key that needs a state carries that value
instead of a free-form string: `UsJurisdiction::ALL` is the census denominator, territories are
deliberately absent (parsing `"PR"` fails rather than widening coverage), and the MileSplit host is
derived from the code (`sources::milesplit::Site::host`) instead of a site table.

**Mid-refactor (do not read the sentence above as complete):** the cutover has landed in
`crates/midwest-census/src/cli/{mod,gather}.rs` (`teams`/`collect` take `Vec<UsJurisdiction>`) and
in `crates/midwest-census/src/sources/milesplit/**` plus `census/**`. The remaining association
adapters still declare `states: Vec<String>` in their `Options` — find them with
`grep -rn 'states: Vec<String>' crates/midwest-census/src/sources` (`athleticlive`,
`athleticlive_athletes`, `ihsa`, `ks`, `mshsl`, `ohsaa`, `plain_names`, `wiaa`, `wiaa_results` at the
time of writing; the set shrinks as the cutover lands). Those sites, and any counter or report field
that prints a state as a bare string, are the cutover remainder.

PRs are computed in Rust from comparable performances; source-reported PRs are stored separately and
disagreements are surfaced rather than resolved by a model. AI review is a scarce adjudication
resource for identity conflicts only, and cannot override a deterministic contradiction.

## 6. Where this is heading

Target workspace split: `crates/census-domain` **exists today** (pure: no tokio/fjall/reqwest/
chromiumoxide/restate/xlsx/llama), and the remaining crates — `census-store`, `census-crawl`,
`census-reconcile`, `census-review`, `census-report`, `census-service`, plus the `acq-*` crates for
the acquisition pipeline — are **planned**: their code still lives in `crates/midwest-census` and the
root package. Domain crates cannot depend on infrastructure; adapters and workflows depend on domain
types, never the reverse. The migration is tracked in `docs/HARDENING-PROGRAM.md` and the ADRs under
`docs/adr/`.

Scaling direction for the national census: jurisdiction workflows fan out per state, each owning
source sweeps, school/meet/athlete discovery and coach discovery; meet-first bulk result acquisition
dominates per-athlete profile fetches; gap analysis produces targeted second passes instead of a
blind full re-crawl.
