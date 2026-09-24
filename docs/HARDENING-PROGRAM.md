# Hardening program — Athletic.net-resistant Midwest census workspace

Status: **plan, not yet executed**. Baseline commit `4e5b828` (origin/main). Measured 2026-09-21 on
this workstation with the pinned toolchain from `rust-toolchain.toml` (nightly-2026-04-27,
rustc 1.97.0-nightly).

Target: the whole workspace — a **fjall**-backed observation store, **Restate** durable services,
**chromiumoxide**-driven Athletic.net acquisition — at Power-of-Ten / async-structural /
Wlaschin-DDD quality with in-repo enforced gates, reproducible performance evidence, and
deterministic replay.

Skills in force for this program: `holzman-rust` (Power of Ten + PLUS performance),
`async-rust-reviewer` (structural async correctness, Asupersync-inspired), `scott-ddd-refactor`
(illegal states unrepresentable), `architectural-drift` (file/function size budget).

---

## 1. What the workspace is

Nine crates, one workspace:

| Crate | Role | Production lines | Files |
|---|---|---|---|
| `athleticnet-browser` | Chromium transport: CDP session management, bounded tab pool, challenge state, request/response identity | — | — |
| `census-domain` | Pure canonical model: newtypes, error taxonomy, identity policy, mark arithmetic | — | — |
| `census-store` | Fjall-backed observation store: twelve tables, merge, snapshot, index write paths | — | — |
| `census-crawl` | Source adapters: athleticnet, hytek, milesplit, ohsaa, mshsl, ihsa, tfrrs, athleticlive, plain_names, ks, wiaa, wiaa_results | — | — |
| `census-service` | Restate services, CLI, workbook/report/bests/projection, browser session transport | — | — |
| `census-report` | Report surfaces | — | — |
| `census-review` | Review surfaces | — | — |
| `census-reconcile` | Reconciliation surfaces | — | — |
| `xtask` | Developer tooling: gates, scan, contract, seams, integrity, census reporting, g1 audit | — | — |

(historical: the root `athletic-rust-pipeline` package that used to sit at the workspace root was deleted 2026-09-23 once the census path owned its work — it previously carried the Athletic.net acquisition engine: HTTP + chromiumoxide browser runtime, rankings/search workers, XLSX matching/verification, CLI workflows.)

Already true (measured, good):

- **Zero `unsafe` in workspace crates.** Each crate carries its own `#![forbid(unsafe_code)]` or `#![deny(unsafe_code)]`. `cargo geiger` clean is a *consequence*, not a waiver.
- **Zero recursion** (bare self-call scan across all workspace crates).
- **Zero `unwrap`/`panic!`/`todo!`/`unimplemented!`/`unreachable!`/`dbg!`** in production code as clippy sees them; census-service has 1 `unwrap`, 75 `expect`, 0 panic macros. Independently measured with test paths and inline `#[cfg(test)]` modules excluded, production code contains **0** `assert!`-family, `panic!`, `expect`, and `unwrap` sites — the 126 `expect` and 268 `assert` grep hits reported earlier are all test code. The panic-surface burndown is therefore a census-service job.
- **Zero production `assert!`-family macros in census** (1,139 live only in `#[cfg(test)]`).
- **Pinned toolchain** with rustfmt/clippy/rust-src/llvm-tools-preview; **release profile** already `lto = "thin"`, `codegen-units = 1`, `strip` (Holzmann build-profile policy satisfied).
- **Restate e2e is real**: `Ingest` object, `Sweep` workflow, heavy jobs behind a semaphore on `spawn_blocking`; browser lifecycle in `athleticnet-browser` owns `TaskTracker` + `CancellationToken` + `JoinSet`.
- **Determinism evidence for the census output path**: rebuild from the Rust store reproduced 6 of 7 JSONL snapshots byte-identically against the Python-era record (`coaches.jsonl` differs only in that the 917 withheld rows omit `professional_email` instead of writing `null`); the freshly published workbook carries 0 of 459 published numbers missing, both state sheets cell-identical.
- **Assurance tooling installed**: `cargo-nextest`, `-audit`, `-deny`, `-vet`, `-geiger`, `-machete`, `-hack`, `-mutants`, `-llvm-lines`, `-bloat`, `perf`, `rg`. Missing: `cargo-fuzz`, `cargo-semver-checks`, `hyperfine`.

## 2. Measured gaps (evidence, not impression)

### 2.1 Strict clippy, doctrine lint set, source targets only

Command (this is the exact gate; `--lib --bins --examples` deliberately excludes test targets):

```bash
cargo clippy --workspace --lib --bins --examples --all-features -- \
  -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic \
  -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro \
  -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap \
  -D clippy::arithmetic_side_effects -D clippy::as_conversions \
  -D clippy::let_underscore_must_use -D clippy::await_holding_lock
```

Result today: **exit 101 — ≈440 errors** (392 census lib, 51 root lib).

| Lint | Total | census | root |
|---|---:|---:|---:|
| `clippy::arithmetic_side_effects` | 244 | 208 | 36 |
| `clippy::expect_used` | 75 | 75 | 0 |
| `clippy::string_slice` | 49 | 47 | 2 |
| `clippy::as_conversions` | 46 | 36 | 10 |
| `clippy::indexing_slicing` | 25 | 23 | 2 |
| `clippy::let_underscore_must_use` | 3 | 3 | 0 |
| `clippy::unwrap_used` | 1 | 1 | 0 |

The `expect` population is homogeneous: 75 sites, all shaped like
`.expect("…")` on statics/parses (regex compilation, literal parses) — i.e. they are
*startup invariant* panics, which the doctrine forbids as panic paths rather than converting to
typed errors. `arithmetic_side_effects` is the long tail (counter increments, offset math in
parsers and report aggregation).

### 2.2 Structure (Rules 4 and 6, drift budget)

- 134 of 552 census production functions exceed 25 logical lines; 33 exceed 60.
- Worst: `main.rs::main` 277 logical lines; `sources/wiaa.rs::collect` 260;
  `sources/wiaa_results.rs::collect` 259; `wiaa_results.rs::absorb` 216;
  `sources/ohsaa.rs::collect` 200; `net.rs::fetch` 190; `sources/ihsa.rs::collect` 173.
- **21 census files exceed the 300-line budget**, worst `sources/plain_names.rs` 2,259,
  `mshsl.rs` 1,855, `wiaa.rs` 1,558, `ohsaa.rs` 1,420, `model.rs` 1,279.
  (Measured at the time of this program. The Phase 2 decomposition later replaced those flat adapters
  with directory modules — see `docs/DECOMPOSITION.md` for the current layout and counters.)

### 2.3 Async structure (async-rust-reviewer phases)

| Check | census (measured now) | root (historical: deleted 2026-09-23) |
|---|---|---|
| `tokio::spawn` | 0 (bare; `JoinSet::spawn` at `bootstrap.rs:160`) | 4 |
| `spawn_blocking` | 2 (production: `bootstrap.rs:180`, `restate_services/support.rs:84`) | 2 |
| `JoinSet` / `TaskTracker` / `CancellationToken` | 3 / 0 / 0 | 5 / 3 / 10 |
| `select!` | 3 | 7 |
| imperative `while let … .next().await` | 0 | 1 |
| `buffer_unordered` | 4 | 0 |
| `#[instrument]` | 6 (`census/sweep.rs:24,131,239`, `net/mod.rs:260`, `net/request.rs:17,28`) | 0 |
| `.instrument(…)` on spawn | 0 | 0 |
| `println!`/`eprintln!` in production | 38 (7 files, all `cli/`/`bin/`; 0 in library paths) | — |
| tokio-console / OTLP | absent | absent |
| `tokio::time::pause` / loom / shuttle / turmoil | absent | absent |
| async fn with >3 `.await` | several | many (`browser/transport.rs::fetch` 25, `cli.rs::run` 13, `browser/shutdown.rs::close_browser` 8, `browser_session.rs::observe_ready` 8, `export_worker.rs::publish` 8) |
| ambient clock (`SystemTime::now` / `Instant::now`) | 0 / 3 production (`tokio::Instant` at `bootstrap.rs:261`; `std::Instant` at `net/execute.rs:49`, `census/sweep.rs:245`) — also 3 `chrono::Utc::now()` sites reaching durable journal (`store/write.rs:73`) and published artifacts (`report/projection.rs:112`) | 9 / 14 |
| `Arc<Mutex<_>>` / `std::sync::Mutex` in async paths | 2 / 1 | 3 / 3 |

Missing structurally: drain-progress certificate, outcome lattice (Ok/Err/Cancelled/Timeout/
Panicked), capability seams for clock/spawn, fairness checkpoints in long async loops, admission
budgets audit for the browser session pool.

| Check | census (measured now) | root (historical: deleted 2026-09-23, measured at program start) |
|---|---|---|
| `tokio::spawn` | 0 (bare; `JoinSet::spawn` at `bootstrap.rs:160`) | 4 |
| `spawn_blocking` | 2 (production: `bootstrap.rs:180`, `restate_services/support.rs:84`) | 2 |
| `#[instrument]` | 6 (`census/sweep.rs:24,131,239`, `net/mod.rs:260`, `net/request.rs:17,28`) | 0 |
| `println!`/`eprintln!` in production | 38 (7 files, all `cli/`/`bin/`; 0 in library paths) | — |
| ambient clock (`SystemTime::now` / `Instant::now`) | 0 / 3 production (`tokio::Instant` at `bootstrap.rs:261`; `std::Instant` at `net/execute.rs:49`, `census/sweep.rs:245`) — also 3 `chrono::Utc::now()` sites reaching durable journal (`store/write.rs:73`) and published artifacts (`report/projection.rs:112`) | 9 / 14 |

### 2.4 DDD / Wlaschin

- `crates/census-domain/src/model.rs` is already newtype-first (`SchoolId`, `AthleteId`, `MeetId`,
  `GradYear::new -> Option`, `ObservedGrade` separate from `GradYear`, `SourceNamespace`,
  deterministic id minting) — the spine exists. (Measured at the time of this program; Phase 3
  moved it out of `crates/census-service` into the pure `census-domain` crate.)
- **Root already has an error taxonomy (`DomainError`, `StoreError`, `BrowserError`, `PageParseError`, `CatalogError`, `StepError`; 86 `thiserror` references) (historical: root package deleted 2026-09-23; `DomainError` now lives in `crates/census-domain/src/error.rs`, `BrowserError` in `crates/athleticnet-browser/src/outcome.rs`).**
- **Census has no error taxonomy**: 38 `anyhow::` references, `thiserror` used in one place
  (`FetchError`, `net/`), `Result<…>` on ~118 signatures. Domain failures are not enumerable.
- Hexagonal violation by construction: one census crate mixes domain, fjall store, crawl adapters,
  reporting, workbook, and Restate services. Nothing prevents `tokio`/`fjall`/`reqwest` reaching
  domain code, because there is no domain crate.
- Not yet measured to closure: `bool` control flags and `String` id parameters in domain
  signatures; `Option`-as-state structs. Those scans are Phase 3 entry work, not unknowns.

### 2.5 Enforcement and evidence gaps

- **No CI at all** (`.github/workflows` absent), no `[workspace.lints]`, no `cargo-deny.toml`.
  *(Closed during this program — `[workspace.lints]` landed in `Cargo.toml`, `deny.toml` exists,
  and `.github/workflows/gate.yml` (commit `02e4189`) runs `bash tools/gate.sh` on push/PR.)*
- **No benches (`benches/` absent, no criterion/divan) (historical: root package's `benches/` was deleted 2026-09-23; the census crate also has no benches directory) — the earlier 559–627k obs/s figures live outside the repo, therefore not reproducible and not a gate. Per doctrine: *no benchmark exists* is a blocker before any performance claim.**
- **No fuzz targets (root already carries a `fuzzing` feature flag but no harness) (historical: root package deleted 2026-09-23).**
- No mutation, Kani, Verus/Flux, loom, proptest, miri configuration anywhere.
- No backup/restore drill, no metrics export, no deployment artifacts for the Restate server.

---

| Check | census (measured now) | root (historical: deleted 2026-09-23, measured at program start) |
|---|---|---|
| `[workspace.lints]` | exists (`Cargo.toml:61-73`: `unsafe_code=forbid`, `unused_must_use=deny`, clippy deny set) | exists |
| `deny.toml` | exists (30 lines: advisories, licenses, sources) — **no `[bans]` section**; the "bans incl. async runtimes" in the plan does not exist. The tree scan replaced `wrappers` bans (no package-level ban exists today). | exists |
| `.github/` CI | **exists** — `.github/workflows/gate.yml` (commit `02e4189`, 2026-09-21 16:01): pinned toolchain from `rust-toolchain.toml`, best-effort installs of the optional gate tools, then `bash tools/gate.sh`; official `actions/checkout` only. | exists |

## 3. Program

Sizes are engineer-days for one competent engineer; ranges reflect discovery risk.
"Parallel" marks streams that can run concurrently without file conflicts.

### Phase 0 — Gates and ratchets (2 d)
- `[workspace.lints]` with the doctrine set; `#![forbid(unsafe_code)]` + `#![deny(unused_must_use)]`
  in census lib/bin.
- `tools/gate.sh`: fmt → check --all-targets → strict clippy (source targets) → nextest → doc →
  panic-macro scan → `cargo audit`/`deny`/`vet`/`geiger`/`machete` → bench-regression lane.
- **Ratchet:** `tools/quality-baseline.json` records the forbidden-construct and strict-lint counts
  per crate; the gate fails on any *increase* and prints the remaining debt. This is how debt is
  frozen without weakening the gate (no allow-lists, no `#[allow]`). The ratcheted set is the debt
  only: forbidden constructs, the 60-line function budget, and the oversized-file ledger. Sizes that
  a feature legitimately raises (`files`, `production_lines`, `functions_over_25_logical_lines`)
  print with a context marker and never fail, and a metric the scan starts reporting later fails
  until the baseline records what it is.
- `cargo-deny.toml` (advisories, licenses, bans incl. async runtimes), `.github/workflows/gate.yml`
  (or equivalent) running `tools/gate.sh`.
- Acceptance: `tools/gate.sh` green; ratchet recorded; CI runs it.

### Phase 1 — Forbidden-construct burndown (4–6 d), serial per crate
- 75 census `expect` → fallible init: compile regexes/statics once at construction into a
  `Regexes`/`Patterns` value threaded through adapters (typed error at startup instead of panic).
- 1 `unwrap` (census) and the 3 `let _ =` sites → typed handling.
- 25 `indexing_slicing` + 49 `string_slice` → `get`/`split_once`/`strip_prefix`/iterator pipelines.
- 46 `as_conversions` → `TryFrom`/`From` with typed errors.
- 244 `arithmetic_side_effects` → checked/saturating arithmetic only where the value is external or
  cross-entity; counters get explicit wrapping semantics documented in the type.
- Acceptance: strict clippy **0 errors**; ratchet at zero; tests unchanged and green.

### Phase 2 — Decomposition to the size budget (4–6 d)
- Split the 21 oversized census files along bounded-context seams; bring every production function
  under 60 lines (hot paths ≤25 logical).
- Same pass for root's `browser/transport.rs`, `cli.rs`, `export_worker/`, `import_worker.rs`.
- Acceptance: no file > 300 lines; no production fn > 60 lines; parity of workbook/snapshot outputs
  re-verified byte-for-byte after the move.

### Phase 3 — DDD type spine, error taxonomy, ports and adapters (5–8 d)
- Workspace split: `census-domain` (pure; **no** tokio/fjall/reqwest/serde-json in its dependency
  tree — enforced by `cargo-deny` bans), `census-store` (fjall), `census-crawl` (net + adapters),
  `census-report`, `census-service` (Restate).
- Census error taxonomy: `DomainError` (enumerable, `thiserror`), `StoreError`, `CrawlError`,
  `ReportError`, `ServiceError`; `anyhow` permitted only in `main`/CLI shells.
- Type-integrity gate: no primitive obsession in domain signatures (no `String` ids, no `bool`
  control flags, no `Option`-as-state), all raw input parsed at boundaries into domain types,
  exhaustive transition matches.
- Acceptance: `cargo-deny` ban proof that the domain crate's tree has zero async/I/O deps; the
  integrity scan added to `tools/gate.sh`; crate-level tests unchanged.

### Phase 4 — Async structural correctness (5–7 d)
- Drain-progress certificate (`DrainReport { accepted, completed, cancelled, timed_out, aborted,
  panicked, remaining }`) for census bootstrap shutdown and root browser lifecycle; shutdown paths
  assert it in tests.
- Outcome lattice replacing flattened `anyhow` at async boundaries: `Outcome<T, E> { Ok, Err,
  Cancelled, Timeout, Panicked }` with `JoinError::is_panic` classification and per-state supervisor
  policy.
- Capability seams: `Clock` (replacing `SystemTime::now`/`Instant::now` sites), `Spawner`, transport
  port — so retry/backoff/timeouts are virtual-time testable.
- Split every async fn above 3 `.await` (root `transport::fetch` first), convert the remaining
  imperative stream loop to combinators, add fairness checkpoints to long loops.
- Observability: `#[tracing::instrument]` on shell entry points, `.instrument(span)` on every
  spawn, span propagation into worker tasks, `console-subscriber` + OTLP wiring behind a feature.
- Acceptance: async clippy lint set (`unused_async`, `await_holding_lock`,
  `await_holding_refcell_ref`, `large_futures`) green; capability scan reports zero ambient
  clock/spawn in domain/application code; drain certificate test exists and asserts counts.

### Phase 5 — Determinism and replay evidence (3–4 d)
- `tokio::time::pause/advance` tests for every retry/backoff/timeout path in `net/` and the root
  reviewer transport.
- `loom` models for the census store lock + ingest semaphore ordering; `proptest` seed capture for
  merge algebra (idempotent, commutative, associative) and for every parser round-trip.
- Acceptance: named tests in CI for each lane; no sleep-based race tests remain.

### Phase 6 — Verification pack (2–4 weeks, scoped by owner decision)
- `kani` harnesses: store sequence monotonicity, publish/withhold invariant (no consumer mailbox can
  be published), merge idempotency, `GradYear`/`ObservedGrade` cohort derivation.
- `verus` (or `flux`) for the bests/share reduction and the census aggregation arithmetic.
- `cargo-mutants` with a threshold on domain crates; `miri` on pure modules; `cargo-fuzz` targets for
  the four file parsers (hytek, result-file, XC, WIAA results) and the workbook XML surface.
- Golden-corpus determinism: fixtures → rebuild → byte-parity test in CI (institutionalizes the
  parity evidence already obtained).
- Assurance artifacts: requirements→test→evidence traceability matrix; hazard log (wrong athlete
  identity, mis-merge across same-name athletes, withheld-contact leak, dropped observation, partial
  write, stale read-after-restore).
- Acceptance: each harness runs in CI with recorded evidence; traceability matrix covers every
  public contract in §1 of the census README; hazard log has a disposition per entry.

### Phase 7 — Performance program (3–5 d + ongoing)
- In-repo benches (`criterion`, `benches/`): store ingest obs/s, consolidate wall time, report scan,
  workbook build; fetch concurrency scaling at N=1/8/64; commit baselines; CI fails on >10%
  regression.
- Allocation evidence: counting allocator in benches (or heaptrack) → per-observation allocation
  budget; `try_reserve` at every capped growth point (JSONL ingest, parser buffers).
- fjall tuning experiment with numbers: partition count, compaction settings, block cache
  (`256 MiB` today), KV separation for large observation payloads.
- Only if measurement justifies: `postcard`-encoded observation payloads replacing `serde_json` in
  the store hot path (requires migration + dual-read path).
- Acceptance: `cargo bench` reproducible in-repo; regression lane wired into the gate; documented
  budget vs measured.

### Phase 8 — Operations (2–3 d)
- Backup/restore drill with a scripted test; corrupt/poison-row quarantine policy (one malformed
  observation must not abort a census run); store-integrity check command.
- Metrics export (counters/gauges: observations ingested, merge conflicts, withheld contacts, job
  durations, drain outcomes) and alert thresholds.
- Deployment artifacts: systemd unit + compose for the Restate server, session-pool sizing guidance
  for the chromiumoxide path, ingress posture documented (currently loopback-only).
- Acceptance: restore drill reproducible from the runbook; quarantine path has a test; deployment
  artifacts start a working service on this workstation.

---

## 4. Sequencing and parallel streams

```
P0 gates ──► P1 burndown ──► P2 decomposition ──┬─► P3 DDD split ──┐
                                                └─► P4 async  ─────┼─► P5 determinism ─► P6 verification ─► P7 perf ─► P8 ops
```

Rationale: gates first so nothing regresses while churning; burndown before decomposition (smaller
diffs to review); decomposition before the crate split (module seams become crate seams); async
hardening touches disjoint files (the root runtime's browser modules `src/runtime/browser_*.rs` vs
census store/net) so P3 and P4 run concurrently (historical: those modules were deleted with the root
package on 2026-09-23; the browser transport is `crates/census-crawl/src/net/execute/browser.rs`
today); verification last but anchored by the golden corpus that already exists.

Parallel ownership map (no two streams edit the same file):

| Stream | Owns |
|---|---|
| A. Gates/burndown | `Cargo.toml`, `tools/`, `.github/`, then census `sources/*`, `net/`, `store/` |
| B. Decomposition/DDD | `crates/census-domain/**`, census `report/`, `workbook/`, `restate_services/`, new crates |
| C. Async hardening | root `src/runtime/**`, `src/main.rs`, `src/cli*` (historical: root package deleted 2026-09-23) |
| D. Verification | `benches/`, `fuzz/`, `kani/`, `tests/`, `docs/` (historical: `benches/` deleted with root package 2026-09-23) |
| E. Operations | `deploy/`, `tools/ops-*.sh`, runbooks |

Serialization rule: A completes census burndown before B touches census sources; C is independent from day one (historical: root package deleted 2026-09-23).

Total: **≈7–11 engineer-weeks serial; ≈2.5–3.5 weeks wall clock** with streams A–E running as above
(P6 is the only phase whose depth is a variable — Kani + mutation + golden only ≈1 week; adding
Verus/Flux across aggregation ≈2–3 weeks).

## 5. Owner decisions required

1. **P6 depth**: Kani + mutation + golden corpus only, or also Verus/Flux proofs over the
   aggregation arithmetic. (Cost delta ≈2 weeks.)
2. **Observation payload codec**: keep `serde_json` (with measured budget) or migrate to `postcard`
   (faster, requires migration + dual-read).
3. **Root crate scope (historical: root package deleted 2026-09-23):** the Athletic.net acquisition engine was the biggest crate; the program above included it for gates/burndown/async/structure but not a full DDD split.
4. **Restate deployment posture**: stay loopback-only, or expose ingress with token/mTLS for
   remote workers (adds auth work in P8).

## 6. Non-goals

- No change to source policy: Athletic.net stays non-core, entered only via ids from cheaper sources;
  no auth/CAPTCHA bypass; no athlete personal-contact collection.
- No runtime replacement of Tokio (no Asupersync migration) — patterns only.
- No `panic = "abort"`, no `target-cpu = "native"` in default config, no unsafe waiver requested.
- No rewrite of the adapter set for uniformity's sake; refactors must preserve byte-parity of
  published artifacts.

## 7. Risks

- **Refactor drift while decomposing parsers**: mitigated by the golden-corpus parity test being
  written *before* Phase 2 touches those files (pull it forward from P6 for the four parsers).
- **Ratchet fatigue**: burndown is per-crate and per-lint, ordered by count, so every phase shrinks a
  number that CI prints.
- **Restate/browser sessions in CI**: network-dependent lanes (adapters, browser) must run in a
  record/replay mode with captured fixtures; otherwise they are excluded from the gate by policy,
  not by accident.

## 8. Wave 1 outcome (2026-09-21)

A parallel lint-burndown wave — one agent per tracked file, sharing the target directory with cargo
serializing the lock — executed the construct-removal half of Phase 1 against the frozen tree, and
was validated by the full gate on a cleaned target directory.

Measured movement (baseline → current; `tools/quality-baseline.json` refreshed with
`--update-baseline --allow-increase` for the two line-count counters):

| Lane | Before | After |
| --- | --- | --- |
| root strict clippy: arithmetic / as / indexing / string_slice / let_underscore | 36 / 10 / 2 / 2 / 1 | 0 / 0 / 0 / 0 / 0 |
| census strict clippy: arithmetic | 240 | 68 |
| census strict clippy: as / indexing / string_slice / expect / unwrap | 36 / 23 / 47 / 75 / 1 | 7 / 4 / 1 / 0 / 0 |
| scan census: indexing / expect / as_cast | 128 / 75 / 32 | 0 / 0 / 7 |
| scan root: indexing / as_cast | 225 / 11 | 0 / 0 |
| gate tests | — | 465 run, 465 passed, 2 skipped |

Gate result: **PASS on every lane** (fmt, check, doc, tests, strict clippy, production scan, domain
type integrity, debt ratchet, deny, audit, machete, geiger, bench presence).

Notes for the next phase:

- Production-line and function-length counters rose by the converted code's explicit error handling
  (root +121 lines, census +641 lines, functions >25 logical lines 434 → 439). The ratchet now
  holds those numbers; Phase 2 decomposition is where they come back down.
- The same window restored, from the pre-reset snapshot, the Athletic.net adapter and the `run`
  one-command cycle that the 11:45 worktree reset had destroyed (see `HANDOFF.md`). Those ~1.3k
  lines are the bulk of the census production-line increase; the recovery is proven by the
  scratch-store smoke run and by the census suite (224 tests).
- `cargo geiger` reads target-dir fingerprints, so a shared/polluted `target/` can fail that lane
  with `Io(NotFound)` for deleted bench or bin targets (`benches/pipeline.rs`,
  `src/bin/ownership-profile.rs`) instead of printing an unsafe verdict. `cargo clean` fixed it;
  clean before trusting that lane when it fails with an Io error.
- The census's remaining 68 arithmetic / 7 as-conversion / 4 indexing diagnostics are the tracked
  Phase 1 remainder (aggregation and adapter modules), not regressions: every one of them is
  `[DOWN]` or equal against the recorded baseline.

## 9. Phase 3 as executed (partial, 2026-09-21)

- **`crates/census-domain` extracted** (commit `beb6e01`): the canonical model moved out of
  `census-service` into a new workspace crate whose *normal* dependency tree is `serde` + `sha2`
  and nothing else. The proof is a new `domain purity` lane in `tools/gate.sh`
  (`cargo tree -p census-domain --edges normal` against a banned-package list; cargo-deny's
  `wrappers` bans express the inverse relation, so the tree scan is the enforceable form).
- **Owner decision — one cohesive scraper:** the workspace does NOT fan out into
  `census-store` / `census-crawl` / `census-report` / `census-service` crates. The census stays a
  single crate with module seams; only the pure domain crate is split, because the ban proof needs
  a boundary that `cargo tree` can see.
- **Owner decision — zero Python in the repo:** every `tools/*.py` analysis tool is ported to
  `xtask` subcommands and deleted; `tools/gate.sh` invokes the Rust tools. Bash and jq remain.
- **Housekeeping:** three merged `feature/*` worktrees (87 GB of stale `target/`) removed and
  their branches deleted; a verified-dead-code sweep (zero-reference `pub` items) is in flight.

## 10. Wave 2 outcome (2026-09-21)

The parallel wave that carried Phases 1–7 from "planned" to "measured": capability seams and
region-owned drain in both crates, the decomposition of the remaining oversize files, the parser
defects the property suites found, the verification pack, determinism models, benchmarks and the
operations pass. It ran as concurrent lanes over disjoint file sets and was validated by the full
gate on the frozen tree:

```
--- fmt: PASS   --- check: PASS   --- doc: PASS   --- tests: PASS
--- domain purity: PASS   --- module seams: PASS   --- ratchet: PASS
--- deny: PASS  --- audit: PASS  --- machete: PASS  --- geiger: PASS
--- bench presence: PASS        gate: PASS (debt ratchet holds; counts above)
```

Measured movement (wave 1 → wave 2; `tools/quality-baseline.json` refreshed with
`--update-baseline --allow-increase` for the line-count counters):

| Metric | Wave 1 end | Wave 2 end |
| --- | --- | --- |
| census strict clippy (arithmetic / as / indexing / string_slice) | 68 / 7 / 4 / 1 | **0 / 0 / 0 / 0** |
| root strict clippy (whole gate lint set) | 0 | **0** (baseline `clippy` map is empty) |
| scan as-casts: root / census | 0 / 7 | 0 / **0** |
| scan expect / unwrap / panic / indexing / unsafe / assert | 0 | 0 (both crates) |
| files over 300 lines | 23 | **0** |
| functions over 60 lines / over 25 logical lines | 0 / 551 | 0 / **549** |
| production lines: root / census | 34308 / 22912 | 36656 (+2348) / 24039 (+1127) |
| scanned files: root / census | 238 / 125 | 305 (+67) / 160 (+35) |
| tests (gate lane, `--all-features`) | 465 passed, 2 skipped | **583 passed**, 2 skipped |

What the lanes landed:

- **Capability seams (root and census).** `clock`, `outcome` (outcome lattice), region-owned
  `spawn`/`Spawner` with a drain certificate, drain counters (`completed`/`cancelled`/`panicked`/
  `remaining`), telemetry instruments, and the console/OTLP feature gates.
- **Decomposition to the size budget.** The root's RD1–RD7 splits plus the census bootstrap,
  adapter and `wiaa::collect` splits took `files_over_300_lines` from 23 entries to none, without
  moving a behavior: every split is covered by the golden corpus and the parity suites.
- **Parser defects the new suites found, fixed with regression tests.** A FOUL row under a relay
  header in the shared Hy-Tek reader (the sections capture drops the jump section's header), and
  RaceDay's `rows_parsed` counting 0 for a finish list whose events carried 82 rows.
- **Property suites and a parse seam.** `merge_properties` (writer laws) and
  `parser_roundtrip_properties` (prefix laws, format dispatch across every fixture, both Hy-Tek
  front ends agreeing) plus the module-seam walker, which is now a gate lane with an allow-list
  that fails closed.
- **Verification pack.** Kani harnesses wired through `#[cfg(kani)]` includes in both crates,
  `cargo-mutants` configured for `census-domain` (116 caught, 0 missed), four fuzz targets seeded
  from fixtures (1000 iterations each, zero crash artifacts), and
  `docs/VERIFICATION-EVIDENCE.md` recording claims, commands, raw tails and the explicit non-claims.
- **Determinism.** Pause-time tests that assert the pacing and backoff timers exactly, and two
  loom models (store sequence allocation; the region ledger) that compile only under
  `--features loom`, with mutation-checked teeth.
- **Performance.** A census criterion harness (`benches/core.rs`: six parse benches, three
  school-index benches, two store-scan benches) whose corpora are asserted against the goldens
  before any number is reported, and the gate's bench-presence lane now compiles it.

Attribution for the baseline refresh (the ratchet holds these numbers from here):

- The line and file counters rose because wave 2 *added capability and evidence* — seams, drain
  plumbing, splits, tests-in-module-dirs, benches — not because debt was tolerated. No counter of
  a forbidden construct moved in either direction except `as_cast` (census 7 → 0).
- The census clippy map is empty: the Phase 1 remainder recorded in §8 is closed, and the root's
  two new diagnostics (`too_many_arguments`, `question_mark`) were fixed rather than recorded
  (`Actor::new`'s four shared handles became one `ActorHandles`; `join_actor` uses `?`).
- The `--all-features` lanes are now known to be a real constraint on feature-gated code:
  `console_subscriber::spawn()` panics without `RUSTFLAGS="--cfg tokio_unstable"`, so the
  `tokio-console` layer is installed only under that cfg (declared in
  `[workspace.lints.rust]` check-cfg) and the feature is a documented no-op without it.

Open items, recorded honestly rather than rounded up:

- **Kani verdicts are partial, and the sweep that closed the window did not change that.** Of 27
  harnesses: **4 verified** (cohort derivation, saturation, and the grade/year agreement:
  122/128/59/327 checks, 0 failed), **7 env-blocked** (six CBMC out-of-memory, one solver-conversion
  crash; no `Failed Checks:` line anywhere), **0 counterexamples**, **16 with no verdict** (13 never
  started, 3 killed before a verdict). The OOM is not a concurrency artifact: the largest harness
  died on CBMC's own memory path after 710 s as the *only* CBMC on the box, with 76 GiB free at
  start and a peak 21 GiB `VmHWM`. Two environment notes are recorded in the evidence doc:
  `cargo kani` fails in 0.1 s with `failed to start cargo metadata` unless `CARGO_HOME` is exported
  (while `cargo kani --version` still succeeds, so the probe looks green), and the `normalize_name`
  harnesses cost CBMC hundreds of seconds inside `core::slice::memchr` whatever the property.
- **The pack produced one real defect, and it is fixed.** `normalize_name` stripped each school-type
  suffix at most once per pass, so `"X School School"` normalized to `"x school"` and normalized
  again to `"x"` — not idempotent, while `SchoolId::mint` keys school identity on that string, so two
  adapters could mint two IDs for one school depending on how often a name passed through
  normalization. It now strips to a fixpoint (`strip_type_suffix` loops until nothing is removed),
  which is the property the pack published and the repeated-suffix harness asserted; the regression
  is pinned by `model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes` and the
  harness's doc comment was updated from "expected to fail" to the fixpoint contract.
- **Fuzzing is a bounded smoke run** (1000 iterations per target), not a soak; the SHA-NI `sha2`
  backend is stubbed away by the `cpuid` stub and is therefore not verified.
- **Integrity review candidates** (7 `struct_with_many_options` sites across the two crates) are
  reported but not ratcheted yet — they are Phase 3/4 review material, not a gate failure.
- **Geiger's stale-target failure mode recurred** in a new shape: a deleted scratch test
  (`crates/census-service/tests/zz_scratch_repro.rs`) left a `target/debug` dep-info artifact that
  the lane could not match. Deleting the artifact fixed it; a `cargo clean` is not required, and
  the lane now passes on the tree as it stands.

## 11. Wave 3 outcome (2026-09-21)

One commit, five lanes: `3af713b` — "Type jurisdictions through the domain and record the national
research base", **286 files, +66,070 / −1,184**. The commit message is the lane-by-lane record; the
numbers below were re-measured on the *frozen* commit after the fact, in a separate git worktree, so
they describe that tree rather than whatever the next wave happened to leave in the working copy.

What landed:

- **Type spine.** `census-domain::jurisdiction` adds `UsJurisdiction` (50 states + DC, `ALL`,
  `code()`, `parse()`), the canonical model's `state` fields become `Option<UsJurisdiction>`, and
  every adapter, reader, report, workbook, best-mark and index site follows in the same change. An
  unparseable source string yields `None` plus a counted observation — never a coerced state, never
  a dropped row.
- **Workflow identity and registry.** `census/identity.rs` (bounded, digesting identity),
  `sources/registry.rs` + `registry/table.rs` (what each adapter can be asked for, and what a
  request costs its origin, with the capability comments naming the symbol each `true` rests on),
  and the durable `restate_services/national.rs` + `restate_services/jurisdiction/`.
- **Quality policy.** The gate's `-Zallow-features` allowlist carries the two dependency-probed
  nightly features beside our own two, and the scan gained an `unstable_features` metric that
  enforces the same allowlist per crate — so source is pinned where the compiler flag cannot reach.
  Benches are typed and abort through `or_fatal`.
- **Documentation.** AGENTS.md, ARCHITECTURE.md, DOMAIN.md, FJALL_SCHEMA.md, PERFORMANCE.md,
  RESTATE_WORKFLOWS.md, SOURCE_ADAPTER_GUIDE.md and TESTING.md rewritten against the tree they
  describe.
- **Research base.** `research/sources/` carries twelve national lane reports (`SOURCE_REPORT.md`,
  `coverage.json`, `CAPTURES.md` per lane) whose ranked build order drives wave 4; raw captured
  payloads stay on disk and gitignored, and the 26 Python probe scripts that produced them were
  moved out of the repository to satisfy the zero-Python rule (they live in the owner's Downloads
  research folder, alongside the 30-agent Midwest report set).

Frozen-tree verification (`git worktree` at `3af713b`, run 2026-09-21 while wave 4 was editing the
working copy):

| Lane | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo xtask scan` root | files 305, production lines 36,656, every forbidden counter 0 |
| `cargo xtask scan` census | files 167, production lines 25,582, every forbidden counter 0 |
| structure | 0 files > 300 lines, 0 functions > 60 logical lines, 557 functions > 25 logical lines |

Movement against §10 (wave 2): census scanned files 160 → 167 and production lines 24,039 →
25,582 (+1,543) — capability and evidence again, not tolerated debt. The root crate's production
line count is unchanged at 36,656 across the wave.

Honest note on the gate: the first `tools/gate.sh` pass over that tree failed its **fmt** lane.
The drift was fixed, and the commit — which is what the table above measures — is fmt-clean; the
commit message records the full pass (fmt, check, doc, tests **618 passed / 2 skipped**, strict
clippy, scan, domain purity, module seams, debt ratchet, deny, audit, vet, machete, geiger). The
gate's verdict in the message therefore describes the tree that landed, and this section's table is
the independent re-check of exactly that tree.

## 12. Wave 4/5 outcome (2026-09-22)

Wave 4 took the four surfaces wave 3's research ranked highest, and wave 5 paid for them with
adversarial verification rather than more code. Both landed against the same rule as §10 and §11:
every lane edits its own files, the integrator owns `mod.rs`/registry/CLI and runs the gate.

**Surfaces.**

| Lane | What landed | Acceptance evidence |
| --- | --- | --- |
| MileSplit Ohio roster walk | `sources/milesplit` Ohio team index + roster walk (`fixtures/milesplit/oh_*`) | 13 Milesplit tests, roster walk e2e against the captured index |
| IHSA tournament decode | `sources/ihsa/tournament/**` — state-final index, event documents, summaries, XC qualifier lists, grade parsing | 20 tests; 1,571 qualifier rows with 1,569 grades; 240 schools, 145 teams, 1,636 athlete rows, 20 performances in one walk; resume run spends 4 cached requests and re-mints nothing |
| TFRRS adapter | `sources/tfrrs/**` — per-state performance lists and team rosters, parse/`map` split | 9 tests over 3 captures; list page 1.88 MB read for 1 request |
| AthleticLIVE result plane | `sources/athleticlive/results.rs` + `map_rows/`, `absorb.rs` — the three wire documents folded into canonical rows | 22 tests; 136 performances and 41 teams from two captures, second run merges to the same 136 |
| Athletic.net whole-meet acquisition | `sources/athleticnet/meet.rs` — meet → results → optional event metadata, 2 requests per meet | 10 module tests + 3 offline end-to-end parity tests; registry row now `bulk_results: true` |

**Defects the wave found, and what fixed them.** These are the point of the wave; the surfaces are
what made them reachable.

| Id | Defect | Fix and evidence |
| --- | --- | --- |
| D1 | Five walk sites journaled a unit as done *before* its rows were in the store, so a kill between the two left the journal claiming rows that never landed (`ks`, `plain_names` ND/NSAA, IHSA top-level, Athletic.net; TFRRS had the same shape) | append-then-journal at each site; TFRRS defers every claim until `collect` has appended. `tests/recovery.rs` ladder: `claimed_without_rows_at_kill=0 missing_from_the_final_store=0` (was 4/4), restart equals the control store, and `cargo xtask source-check ks` — 31/31 — green |
| D2 | `bootstrap/serve.rs` armed the drain deadline before waiting for a stop request, so an endpoint that was never asked to stop was killed at the deadline and reported `ServerExit` | stop-watch → cancel → drain ordering. `recovery drain-deadline`: `still_answering=true`, drain report `accepted=2 completed=2 timed_out=0`, operator stop `stop_reason: Signal` |
| D3 | The Coverage sheet's `Note` row embedded the store root, and the workbook parity normaliser blanked the store path only for the `Store`/`Core note` labels, so `pipeline_publishes_the_same_bytes_from_a_rebuilt_store` failed on a digest that changed every run | `volatile_cell` normalises `Note` too; the Coverage sheet digest is now identical across three consecutive rebuilds |

Lane-local bugs the same tests caught: `athleticlive::results` counted refusals inside a memo
(`or_insert_with`), so 8 of 16 refused rows were uncounted and the row-count invariant silently
broke; `Run::new` minted a meet but never placed it in the accumulator, leaving 136 performances
pointing at an absent meet; IHSA's `limit` counted *walked* meets so a resumed run re-walked 97
live pages; `plain_names` passed a `Vec` to single-record `append`; TFRRS read every section's
gender as `None`.

**Verification on the frozen tree.**

| Check | Result |
| --- | --- |
| `tools/gate.sh` | PASS every lane: fmt, check, doc, tests (**726 passed, 2 skipped**), strict clippy on source targets (**0 diagnostics**), production scan, domain purity, module seams, debt ratchet, deny, audit, vet, machete, geiger, feature powerset |
| `cargo xtask scan` | root 305 files / 36,656 production lines, census 251 / 39,340, every forbidden counter 0; **0 files > 300 lines, 0 functions > 60 logical lines** |
| `cargo xtask source-check ks` | 31 passed, 0 failed — including `recovery ks_directory_walk_claims_units_the_kill_can_lose` |
| `tests/recovery.rs` | 8 scenarios green (kill ladder, worker/service SIGKILL, drain deadline, resume, flush) |
| Legacy parity, same store | `census-by-state-all-sources.csv` byte-identical to the pre-Rust (JS) output; `census-by-state{,-core}.csv` differ only in one label (`ALL` → `TOTAL`), every figure identical |
| Workbook | 20 sheets: the legacy vocabulary (`Athletes`, `PRs`, `Performances_001`, `Coaches`, `Schools`, `Meets`, `Sources`, `Coverage`, `Conflicts`, `Review`, `Run Metrics`) plus the nine summary pages; five summary sheets byte-identical to the pre-wave export, four grew with the new sources' states |

**Honest notes.**

* Goldens were reseeded for the pipeline corpus (`report-core`, `report-all_sources`,
  `census-by-state-*`, `workbook-shape`) and for the new Ohio Milesplit captures. The churn is
  additive, checked per sheet: `Goal & method`, `Summary`, `Best results`, `Evidence mix` and
  `Method notes` are byte-identical, and the report's growth is new states (AK…WY appear under the
  new sources), not changed rows.
* Fixtures were split per adapter — `tests/fixtures/ihsa_tournament/` and
  `tests/fixtures/athleticlive_results/` — because both parity corpora require every file in a
  source's directory to be a payload kind that corpus can parse; the new payload families now have
  their own directories, and the IHSA corpus test is green again on three files.
* The `ihsa_tournament` adapter gained a `provider` CLI arm. The AthleticLIVE result plane did
  **not**: its route needs operator-supplied captures *and* the meet they belong to, so its entry
  point stays `athleticlive::collect_results` (the registry row documents that), rather than a CLI
  arm that could only fail for want of a target.
* Two of the wave's lanes (athleticlive results, TFRRS) had to be given explicit exit criteria
  mid-flight: both were at risk of landing a compiling adapter with failing tests. One exit — fix
  or delete with a reason — is what kept the tree's red set empty at freeze.

## 13. Wave 6 outcome (2026-09-22)

The objective's §29-§31 requirement: the store must hold the *derived* state a reader needs — the
join from a provider's own ids to canonical rows, the conflict and review queues, coverage, and one
collection snapshot per pass — so that answering those questions is a scan of one table rather than
a re-derivation of the whole store. What landed, all inside the census crate:

- **`index::derive` (`crates/census-service/src/index.rs`).** One pass writes five things:
  `source_identities` (one row per provider identity per canonical table, carrying the provider id
  verbatim, the table it belongs to, the canonical id it joins to and the observed URL),
  `conflicts` and `review_cases` (the retained queues), `coverage` (one row per jurisdiction and one
  per source namespace), and `snapshots` (one row per pass). The queues and coverage are *composed*
  from the same code the workbook's sheets print — `workbook::retained_records` and
  `report::coverage_report` — so a store reader and a workbook reader cannot be shown different
  findings; they are the same computation written twice.
- **A derived-state write path (`store/write.rs::replace_many`).** Derived rows are a function of
  the store, not evidence about a moment in it, so they are keyed with a fixed sequence of zero and
  reserve no observation number: re-deriving replaces the row it wrote before. Appending instead
  would add a full copy of every identity per pass — on a twelve-million-row identity table that
  reaches `MAX_ROWS_PER_TABLE` in days and then aborts every scan of the table. Measured on a real
  store: three consecutive `index` passes leave `source_identities`, `coverage` and `snapshots` at
  one row per key.
- **CLI and cycle.** `census-service index` derives and prints the per-table counts;
  `census-service run` derives between `consolidate` and the published scopes, so one command keeps
  the indexes current.
- **`store/table.rs`.** The twelve-table vocabulary moved out of `store/mod.rs`, which had crossed
  the 300-line budget (319 → 225 lines); the module re-exports `Entity`, `Table` and both ceilings,
  so no caller changed.
- **Documentation corrections found while wiring it.** `StoreStats`' doc comment claimed the counts
  were LSM `approximate_len` estimates (they are the store's exact sequence counters — the claim was
  flagged in `FJALL_SCHEMA.md` §8.9 and is now fixed at the source); `docs/FJALL_SCHEMA.md` §1, §2.1
  and §5 now carry twelve tables and both write paths, and the root `FJALL_SCHEMA.md` no longer says
  "seven tables".

Verification:

| Check | Result |
| --- | --- |
| `cargo nextest run --workspace --all-features` | **745 tests run: 745 passed, 2 skipped** — `cargo test -p census-service --lib` alone is **356 passed** (21 `index::tests`, 2 new `store::tests` for the replace path) and `cargo test -p census-domain` is **39 passed** |
| Real-store smoke | `provider coach_contacts` → `index` → `fjall-stats`: 6 schools / 13 coaches imported; `source_identities=19 conflicts=0 reviews=0 coverage=58 snapshots=1`; three passes leave one row per key |
| `census-service run` (offline) | `gather … skipped (no --input)` → `consolidate` → `index` → both report scopes → `bests` → `recruiting` → `workbook`; 0.23 s over the six-school store |
| `cargo xtask seams` | 15 modules, `violations: []` — the three new `index` edges are declared with their reasons in the table |
| `cargo xtask scan` | census 253 files / 39,909 production lines, every forbidden counter 0, **0 files > 300 lines**, 0 functions > 60 logical lines; root unchanged at 305 / 36,656 |
| `tools/gate.sh` | **PASS** (debt ratchet holds) — after one **FAIL** that was worth having: `check`, `tests` and `bench presence` all broke on one stale call, `SourceObjectIdentity::row_id()` in `census-domain`'s test target, which `cargo test -p census-service --lib` never builds. Fixed, plus two test updates for the new table set, both inspected: the `Run Metrics` sheet golden (13872 → 13882 cells, all twelve tables, every other sheet byte-identical) and the backup drill's expected table vector (the five derived tables asserted empty there, because that chain never derives) |

### 13.1 Deferred items (numbered)

Each item is a measured gap, with the file or check that carries the evidence. None of them is
required for the objective's §29-§31 deliverable; all of them are real.

30. **`(sources, store)` is a direction violation.** `AdapterContext` carries `&Store`, so every
    adapter can read and write the store directly. `xtask/src/seams.rs` lists the edge with that
    comment; when adapters return entity batches instead, delete the row and the walker enforces the
    narrower graph.
31. **`scan` materializes the whole merged table.** `store/read.rs::scan` builds one
    `BTreeMap<String, T>` in the heap, so a performance table at the 20M ceiling is a memory
    incident waiting for its trigger. A streaming/merge iterator would remove both this and item 32.
32. **`MAX_ROWS_PER_TABLE` fails whole-table reads.** Crossing 20M aborts `report`, `bests`,
    `workbook`, `consolidate` and the index pass together (`docs/FJALL_SCHEMA.md` §8.1). The bound
    should be per-batch with a partial-progress error, not per-table.
33. **Legacy import has a commit/marker crash window** (`store/legacy.rs`): a kill between the
    WriteBatch commit and the marker write re-imports the file on the next open. The import should
    be digest-keyed so re-running it is a no-op.
34. **`index::derive` is O(store).** Every pass re-derives all identities from the merged entity
    tables; nobody consumes the change set the snapshot counters already make visible. Incremental
    derivation keyed off `snapshots` is the fix when the identity table's pass cost matters.
35. **Snapshots publish appended-observation counters, not merged row counts.** `stats()` gives
    exact appended counts per table (and row counts for derived tables), but "how many schools does
    the store hold" still requires `consolidate`. A second counter map, recorded by the pass that
    already scans the tables, would answer it directly.
36. **No `try_reserve` anywhere in census.** Every externally-sized growth point — JSONL ingest,
    parser buffers, journal payloads, observation batches — grows through `Vec::with_capacity` or
    `Vec::new()`.
37. **`journal_payloads` clones every payload** (`store/read.rs`) instead of borrowing the stored
    bytes.
38. **One malformed row aborts a whole table scan** (`StoreError::Decode`, `store/read.rs`): no
    quarantine path and no tolerance precedent anywhere — the tolerant `report::read_rows` this
    item used to cite was removed, and the published-snapshot reader is strict
    (`store/read/snapshot.rs:179`, `StoreError::SnapshotRow`, covered for bad-middle and bad-last
    rows).
39. **No `#[instrument]` on the three production spawn sites** (`bootstrap.rs` ×2,
    `restate_services/mod.rs`), so a stuck task in a live run has no span to point at.
40. **Neither the pipeline golden nor the backup drill exercises the derived index tables.**
    `tests/parity_pipeline.rs` pins every published file byte-for-byte and rebuilds the store, but it
    drives `consolidate → report → bests → workbook` and never derives, so the five tables are only
    visible through the `Run Metrics` sheet's per-table counters (the wave 6 golden grew by exactly
    those rows: 13872 → 13882 cells, all twelve in `Run Metrics`, every other sheet untouched).
    `tests/backup_restore.rs` likewise asserts the index tables are *empty* in its corpus, because
    its chain never derives — so "a cold copy carries the derived rows" is unproven. Adding the
    derivation to both chains closes it.
41. **Coverage is derived for one cohort.** `index::coverage_rows` asks `coverage_report` for
    CO2027 because that is the sheet's cohort; the other grad years have no coverage row in the
    store.
