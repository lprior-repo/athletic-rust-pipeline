# AGENTS.md — working in this repository

Primary instruction document for coding agents. Read this before touching anything; then
`ARCHITECTURE.md` (system shape), `DOMAIN.md` (types and evidence rules),
`SOURCE_ADAPTER_GUIDE.md` (adding a source adapter), `FJALL_SCHEMA.md` (storage),
`RESTATE_WORKFLOWS.md` (durable work), `TESTING.md` (how to run and extend the suite),
`PERFORMANCE.md` (measured baselines and the bench harness), and `docs/HARDENING-PROGRAM.md`
(current phase plan). Architecture decisions live in `docs/adr/`.

Statements about work that does not exist yet are marked **planned** and name the file that will
hold it. Anything not marked planned is in the tree today.

## 1. Repository map

One line per crate/directory: what it is, where it starts, where its tests and fixtures live.

| Area | Responsibility | Entry points | Tests | Fixtures |
|---|---|---|---|---|
| `crates/census-domain/` | Pure domain: canonical entities, deterministic `Id<T>`, `GradYear`/`ObservedGrade`, `Evidence`, `SourceNamespace`, `UsJurisdiction`; no async, no I/O | `src/lib.rs`, `src/model.rs`, `src/jurisdiction.rs`, `src/error.rs` | `src/model_tests.rs`, `src/jurisdiction_tests.rs` | none (pure; no fixture files) |
| `crates/census-domain/kani/` | Kani harnesses for the domain (grad-year law, id minting, publish redaction) | `kani/{gradyear,id_mint,publish,census_domain_wiring}.rs` | harnesses themselves | none |
| `crates/midwest-census/` | Census library + the `midwest-census` and `midwest-serve` binaries: fetcher, Fjall store, source adapters, census orchestration, report/bests/workbook, Restate services | `src/lib.rs`, `src/cli/mod.rs`, `src/bin/midwest-serve.rs` | `tests/{parity_national,parity_wisconsin,parity_north,parity_mideast,parity_pipeline,merge_properties,parser_roundtrip_properties,fjall_restate_e2e}.rs`, module `tests.rs` files | `tests/fixtures/<source>/` (11 source directories), `tests/golden/*.json` |
| `crates/midwest-census/kani/` | Kani harnesses for the census store: key encode/decode, merge, wiring | `kani/{keys,merge,store_wiring}.rs` | harnesses themselves | none |
| `crates/midwest-census/src/net/` | Polite fetcher: robots enforcement, per-host pacing, disk cache, conditional GET, bounded retries | `src/net/mod.rs`, `src/net/{execute,request,cache,robots,decode,client}.rs` | `src/net/tests.rs`, `src/net/execute/tests.rs` | HTTP bodies are cached under `--store`; none committed |
| `crates/midwest-census/src/store/` | Fjall observation store: append-only writes, read-time merge, journal, legacy import | `src/store/mod.rs`, `src/store/{read,write,keys,entities,sequences,legacy}.rs` | `src/store/tests.rs`, `src/store/loom_tests.rs`, `tests/fjall_restate_e2e.rs` | none |
| `crates/midwest-census/src/sources/` | One module per provider adapter; `result_file` dispatches vendor artifact layouts | `src/sources/mod.rs`, `src/sources/<name>.rs` + `src/sources/<name>/` | per-adapter `tests.rs` (mostly `src/sources/<name>/tests.rs`) | `crates/midwest-census/tests/fixtures/<name>/` |
| `crates/midwest-census/src/census/`, `report/`, `bests/`, `workbook/`, `school_index.rs` | Orchestration and reductions: team/roster walk, measured census, best marks, spreadsheet, school-label resolution | `src/census/mod.rs`, `src/report/mod.rs`, `src/bests/mod.rs`, `src/workbook/mod.rs`, `src/school_index.rs` | module `tests.rs` files | `tests/golden/` |
| `crates/midwest-census/src/restate_services/`, `src/spawn.rs`, `src/bootstrap/` | Durable services (`Census`, `Ingest`, `Sweep`, `JurisdictionCensus`, `NationalCensus`), region-owned task spawning, service supervisor with cancel/drain/finalize | `src/restate_services/mod.rs`, `src/restate_services/{jurisdiction,national}.rs`, `src/spawn.rs`, `src/bootstrap/mod.rs`, `src/bin/midwest-serve.rs` | `src/restate_services/tests.rs`, `src/spawn/tests.rs`, `src/bootstrap/tests.rs` | none |
| root package `athletic-rust-pipeline` | Acquisition pipeline and operator CLI: Restate worker, browser session, rankings collection, profile/row workers, export and verification | `src/main.rs`, `src/cli.rs`, `src/runtime/**` | `tests/*.rs` (15 targets) | `tests/fixtures/{cdp,rankings}/`, `fixtures/public/athletic-source-contract.json` |
| root `src/` (non-runtime) | Workbook ingest/verify/export, xlsx reader/writer, artifact store, HTML bounds, search, result verification | `src/lib.rs`, `src/workbook_export.rs`, `src/workbook_ingest.rs`, `src/store.rs`, `src/xlsx.rs` | `tests/{workbook_verify,workbook_zip_layout,bundle_verify,native_parser_properties,result_verify,search_html_bounds,profile_html_bounds,profile_merge_bounds}.rs` | `tests/fixtures/**`, `fuzz/fixtures/retained_results_jsonl/**`; `examples/native_fixture/{payloads,scenarios,server,workbook}.rs` is a synthetic fixture *origin*, not a fixture |
| `xtask/` | Repository developer commands; the gate's measurement layer | `xtask/src/main.rs`, `xtask/src/{scan,seams,integrity,purity,baseline,source_fixture,scaffold,templates}.rs` | inline `#[cfg(test)]` in `xtask/src/templates.rs` | none |
| `tools/` | `tools/gate.sh` (the one quality gate) and `tools/quality-baseline.json` (debt ratchet) | `tools/gate.sh`, `tools/README.md` | exercised by the gate itself | none |
| `benches/` (root) | Committed criterion measurements of the root crate: artifact store, blocking fan-out, workbook export | `benches/artifact_store.rs`, `benches/blocking_fanout.rs`, `benches/workbook_export.rs` | self-asserting datasets inside each target | synthetic; no fixture files |
| `crates/midwest-census/benches/` | Criterion measurement of census hot paths: result-file parsing, school-label resolution, store merge | `benches/core.rs`, `benches/core/{fixtures,labels,lcg,merge}.rs` | self-asserting counts inside the target | reads `tests/fixtures/**` |
| `crates/midwest-census/examples/` | Whole-pipeline and substrate throughput harnesses (`bench_census`, `bench_store`) | `examples/bench_census.rs`, `examples/bench_store.rs` | self-asserting phases | synthetic corpora |
| `fuzz/` | Standalone cargo-fuzz workspace (`[workspace]` of its own, not a member of the root workspace) | `fuzz/fuzz_targets/{hytek,compiled,xc,raceday}.rs`, `fuzz/Cargo.toml` | fuzz targets | `fuzz/corpus/<target>/`, `fuzz/fixtures/retained_results_jsonl/` |
| `research/` | Captured evidence for source decisions (read-only once written) | `research/sources/<topic>/samples/**` | not a test lane | the captures themselves |
| `docs/` | Program and operations documents, plus the ADR set and the storage companion | `docs/HARDENING-PROGRAM.md`, `docs/OPERATIONS.md`, `docs/VERIFICATION-EVIDENCE.md`, `docs/DECOMPOSITION.md`, `docs/FJALL_SCHEMA.md` (operational companion to root `FJALL_SCHEMA.md`), `docs/adr/ADR-00{1..6}-*.md` | not a test lane | none |
| root `*.md` | The instruction set: `AGENTS.md` (this file), `ARCHITECTURE.md`, `DOMAIN.md`, `SOURCE_ADAPTER_GUIDE.md`, `FJALL_SCHEMA.md`, `RESTATE_WORKFLOWS.md`, `TESTING.md`, `PERFORMANCE.md`, `COLLECTOR_PATTERNS.md`, `SCOPE.md`, `HANDOFF.md`, `README.md`, `CHROMIUM_DESIGN.md`, `WORKFLOW_REVIEW.md`, `PROFILE_REPLICATION.md` (cohort plan), `SOURCES_SURVEY.md` (source survey + 2 rps policy); `athletic-pipeline-project-pack.md` and `athletic-pipeline-handoff.md` are imported reference material, not current state (`HANDOFF.md` §Freeze and ownership) | — | not a test lane | none |
| `deploy/systemd/` | Unit/timer definitions for the census collect timer and the Restate endpoints | `deploy/systemd/{midwest-census-collect.service,midwest-census-collect.timer,midwest-serve.service,restate-server.service}` | not a test lane | none |

## 2. What you may edit, and what you may not

A ticket names the files one agent owns. Rules:

- Edit only your own files. Never reformat, re-sort, or "clean up" a file you do not own.
- Never edit, unless your ticket names them: the workspace manifests (`Cargo.toml`, `Cargo.lock`),
  `tools/gate.sh`, `tools/quality-baseline.json`, `xtask/**`, `.github/workflows/gate.yml`,
  `crates/census-domain/src/**` public types, `docs/adr/**`, `research/**` and `fixtures/**`
  captures, `var/**` and `reports/**` (run outputs).
- `research/**` and `tests/fixtures/**` are verbatim captures: add new files, never rewrite an
  existing capture. If a capture is wrong, add a corrected file and say why in the report.
- Adapters, fixtures and research notes for one source belong to one owner at a time.
- If your change needs a file you do not own, stop and report it under `NEEDS`.

### 2.1 Crate ownership map (shared contracts vs lanes)

| Owner | Paths | Notes |
|---|---|---|
| Main / integration | `Cargo.toml`, `Cargo.lock`, `crates/census-domain/src/{lib,model,error}.rs` public items, `crates/midwest-census/src/store/keys.rs` encodings, `crates/midwest-census/src/restate_services/wire.rs`, `tools/**`, `xtask/**`, `.github/**` | Shared contracts: domain types, public error enums, Fjall schema and key encodings, Restate workflow/service signatures, workspace manifests. Propose changes; do not apply them. |
| Census internals lane | `crates/midwest-census/src/{net,store,census,report,bests,workbook,spawn.rs,bootstrap,cli}/**` | One owner per module at a time; the module's `tests.rs` moves with it. |
| Adapter lane | `crates/midwest-census/src/sources/<name>.rs` + `sources/<name>/`, `crates/midwest-census/tests/fixtures/<name>/` | One source, one owner. |
| Root package lane | `src/**`, `tests/**`, `fixtures/**` | The acquisition pipeline and its operator CLI. |
| Benchmark lane | `benches/**`, `crates/midwest-census/benches/**`, `crates/midwest-census/examples/bench_*.rs` | Measured claims only; see §9. |
| Docs lane | root `*.md`, `docs/**` (not `docs/adr/**`), `crates/midwest-census/README.md` | Factual corrections; ADRs are separate. |
| Research lane | `research/**` | Append-only captures. |

## 3. Git discipline (hard rule)

Never run `git stash`, `git reset`, `git checkout`, `git restore`, `git clean`, `git switch`, or any
command that rewrites the working tree. Other agents work in the same tree at the same time; a
`reset`/`stash` silently destroys their in-flight edits. Read-only git (`status`, `diff`, `log`,
`show`) is fine. Re-read a file before editing it and verify your own hunks with `git diff -- <file>`
before reporting.

## 4. Commands that exist today

```bash
# The one gate: fmt, check --all-targets, doc, tests, strict clippy, scans, debt ratchet,
# optional tool lanes (deny/audit/machete/geiger), bench presence.
tools/gate.sh                                  # or: cargo xtask gate [-- <gate args>]
tools/gate.sh --update-baseline [--allow-increase]

# Fast local lanes (what the gate runs, one at a time)
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo nextest run --workspace --all-features   # the gate falls back to cargo test if nextest is absent
cargo clippy --workspace --lib --bins --examples --all-features -- <LINT_SET below>

# The gate's measurement layer
cargo xtask scan            # forbidden constructs + size budgets, JSON on stdout
cargo xtask integrity       # domain type-integrity review candidates, JSON
cargo xtask seams           # census top-level module edges vs the allowed table
cargo xtask domain-purity   # census-domain normal dependency tree
cargo xtask ratchet tools/quality-baseline.json <clippy.tsv> <scan.json>

# Per-source lanes
cargo xtask source-test <source>     # cargo nextest run -p midwest-census -E 'test(<source>)'
cargo xtask source-fixture <source>  # list crates/midwest-census/tests/fixtures/<source>/
cargo xtask new-source <name>        # scaffold the directory-layout adapter

# Census reports from a store
cargo xtask census-status --store var/midwest-census   # report --core
cargo xtask coverage      --store var/midwest-census   # report, every source

# Census pipeline (the shipped binary; `--store` defaults to var/midwest-census)
cargo run --release -p midwest-census --bin midwest-census -- teams    --states WI,MN --refresh
cargo run --release -p midwest-census --bin midwest-census -- collect  --states WI,MN
cargo run --release -p midwest-census --bin midwest-census -- provider wiaa
cargo run --release -p midwest-census --bin midwest-census -- consolidate
cargo run --release -p midwest-census --bin midwest-census -- report --print
cargo run --release -p midwest-census --bin midwest-census -- bests
cargo run --release -p midwest-census --bin midwest-census -- workbook
cargo run --release -p midwest-census --bin midwest-census -- run          # the whole cycle in one command
cargo run --release -p midwest-census --bin midwest-serve -- --listen 127.0.0.1:9080 --data-dir var/midwest-census

# Root acquisition pipeline / operator CLI
cargo run --release -p athletic-rust-pipeline -- worker --config config.native.toml --bind 127.0.0.1:19181
cargo run --release -p athletic-rust-pipeline -- <deploy|start|status|browser-start|browser-status|export|verify|rankings-status|rankings-pause|rankings-resume> …

# Benchmarks (§9)
cargo bench -p athletic-rust-pipeline --bench artifact_store
cargo bench -p athletic-rust-pipeline --bench blocking_fanout
cargo bench -p athletic-rust-pipeline --bench workbook_export
cargo bench -p midwest-census --bench core
cargo run --release -p midwest-census --example bench_store  -- --rows 200000 --batch 1000 --scan
cargo run --release -p midwest-census --example bench_census -- --schools 500

# Concurrency model lane (loom, opt-in feature)
cargo test -p midwest-census --features loom --lib
```

Notes that are load-bearing:

- `cargo xtask bench` is **not** implemented: it exits non-zero with the reason. Use the `cargo bench`
  lines above.
- **Planned** (§63): the short command set also names `cargo xtask source-check` and
  `cargo xtask export`. Neither exists yet — `source-test` is today's per-source test lane, and
  export is the root CLI's `export` subcommand plus the census `workbook`/`report` commands. They
  will be added as subcommands of the `Command` enum in `xtask/src/main.rs`.
- Cargo build-lock waits are normal when several agents run: cargo serializes on the build directory.

## 5. Coding standards

Doctrine is NASA/JPL Power of Ten adapted for Rust (see `skill://holzman-rust`), enforced by the
gate's clippy set plus the measurement scans (`cargo xtask scan`, `cargo xtask integrity`,
`cargo xtask seams`, `cargo xtask domain-purity`).

The lint set is exactly `LINT_SET` in `tools/gate.sh`, applied as
`cargo clippy --workspace --lib --bins --examples --all-features -- <LINT_SET>` (one `-D` each):
`warnings`, `unsafe_code`, `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`,
`clippy::panic_in_result_fn`, `clippy::todo`, `clippy::unimplemented`, `clippy::dbg_macro`,
`clippy::indexing_slicing`, `clippy::string_slice`, `clippy::get_unwrap`,
`clippy::arithmetic_side_effects`, `clippy::as_conversions`, `clippy::let_underscore_must_use`,
`clippy::await_holding_lock`.

The workspace manifests add `unsafe_code = "forbid"`, `unused_must_use = "deny"`, and
`clippy::{dbg_macro, todo, unimplemented, panic_in_result_fn} = "deny"` for every target, tests
included (`Cargo.toml`, `[workspace.lints.*]`).

Required shape:

- Errors are typed. Library crates use `thiserror`; the CLI/application boundary may use `anyhow`
  with `.context("…")`. Keep the existing message text when converting a panic into an error.
- Files stay under ~300 lines; functions under ~60 logical lines (≤25 for hot paths). Split by
  behaviour, not by line count. The scan reports `files_over_300_lines`,
  `functions_over_60_lines` and `functions_over_25_logical_lines`, and the ratchet fails when any
  of them grows.
- Prefer existing helpers, existing constants and the file's existing idiom over new abstractions.
  No compat wrappers or aliases: migrate the call sites.
- Values crossing a boundary are validated once, there, and carried in types (`Result`, newtypes,
  enums) rather than re-checked or stringly-typed. A state is `census_domain::UsJurisdiction`, not
  a `String`.
- Log with `tracing`, never `println!`/`eprintln!` in library code. Async entry points carry
  `#[tracing::instrument(skip_all, fields(…))]`.
- Debt is ratcheted, never blessed: `tools/quality-baseline.json` may only shrink, and a burndown
  is the only legitimate reason to move it (`tools/gate.sh --update-baseline`).

## 6. Source policy (non-negotiable)

- Respect `robots.txt` (enforced in `crates/midwest-census/src/net/mod.rs` → `net/robots.rs`): a
  disallowed path returns `FetchError::Robots` and counts in `FetchStats::robots_blocked`. An
  operator can name a host on `--authorized-host`; the rule is then counted as `robots_authorized`
  and the request proceeds under the 2 rps ceiling.
- Hard pacing: 2 requests/second per host, never closer than `MIN_AUTHORIZED_DELAY` (500 ms) for an
  authorized host, whatever `--delay-ms` says; default spacing 1 s; `gobound.com` 10 s
  (`sources::default_host_delays`). Bounded fan-out via `sources::CONCURRENCY_BOUND` (= 8).
- Cache before network: bodies are cached on disk by content hash; a re-run is free and an
  interrupted collection resumes without re-fetching.
- Never bypass a CAPTCHA, an authentication barrier or an access control. There is no cookie jar,
  no challenge evasion, and no browser-user-agent spoofing. A served challenge latches the browser
  into `HumanRequired`; that latch is a correct outcome, not a bug to route around.
- Retry ownership, precisely: **adapters and CLI shells perform one attempt** and never retry a
  `fetcher` call. Retries exist in exactly two places, both inside the transport: the retry loop
  retries the variants `FetchError::retryable` marks transient — `Transport`, `Timeout`,
  `RateLimited`, and `Http` with status ≥ 500 or 429 — at most `MAX_RETRIES = 3` attempts with
  ±25% jittered exponential backoff from `RETRY_BASE_DELAY_MS = 500`; and the conditional-GET path
  retries a `304` that arrived with no cached body (`net/execute/attempt.rs::replay_cached`, which
  retries in place rather than through `retryable`). Restate/workflows own everything else,
  including durable retry of a whole step (ADR-002).
- Collect only athletic evidence and publicly published professional school/sport contacts. Never
  athlete personal email, personal phone, home address, or unrelated personal data.
- A source failure is never equivalent to "no match": record the terminal state and continue
  elsewhere. There is no `OperationTerminal` type — use the layer's own enum: `FetchError`
  (`crates/midwest-census/src/net/mod.rs`: `Robots`, `Http`, `RateLimited { retry_after_secs }`,
  `TooLarge`, `Transport`, `Cache`, `Timeout`, …), `CrawlError`
  (`crates/midwest-census/src/sources/mod.rs`) in the census crate, or `FailureCode`
  (`src/runtime/protocol.rs`) / `BrowserError` (`src/runtime/browser.rs`) in the root crate.

## 7. Adding a provider adapter

Adapters are plain async functions taking `AdapterContext` and returning `AdapterReport`; there is no
trait indirection by design (`crates/midwest-census/src/sources/mod.rs`). `cargo xtask new-source
<name>` writes steps 1 and 5 for you; the worked example below is `wiaa`, the Wisconsin association
directory adapter.

1. **Module root** — `crates/midwest-census/src/sources/<name>.rs` (the facade that declares the
   child modules) and/or `crates/midwest-census/src/sources/<name>/`. `wiaa` has no flat file: it is
   `sources/wiaa/{mod.rs, parse.rs, map.rs, collect.rs, collect_schools.rs, primitives.rs, tests.rs}`.
   `mod.rs` (`wiaa/mod.rs:1-40`) documents the endpoints, the fields read, the fields deliberately
   ignored, and the email policy; `parse.rs` is pure; `map.rs` turns parsed rows into canonical
   entities; `collect.rs` drives the fetcher; `tests.rs` holds the fixture-driven tests.
2. **Registry** — append `pub mod <name>;` to the alphabetical module list in
   `crates/midwest-census/src/sources/mod.rs` (the list is at the top of the file).
3. **CLI dispatch** — add a `<name>` arm to the `match` in `run_provider`
   (`crates/midwest-census/src/cli/provider.rs`, the arms are `"ks" => …`, `"wiaa" => …`, …) with a
   thin `*_report` helper, and add the name to the `ProviderArgs::name` doc list in that file
   (`cli/provider.rs:15-16`), which is also the list the `unknown adapter` error prints. The
   `Command::Provider` variant itself lives in `crates/midwest-census/src/cli/mod.rs`.
4. **Namespace** — if the source is new, add a `SourceNamespace` variant in
   `crates/census-domain/src/model.rs`; never smuggle a source tag through a `String`. If the
   source cannot stand alone, add it to `report::NON_CORE_SOURCE_IDS`
   (`crates/midwest-census/src/report/core_scope.rs`) and cover `is_core_source("<namespace>") ==
   false` with a test.
5. **Observations** — append through `sources::append_all(&store, Table::X, &rows)`
   (`sources/mod.rs`); never write keys directly. Observations are append-only: a re-run appends
   new rows and the reader merges them (`crates/midwest-census/src/store/read.rs`), so a duplicate
   observation is merged, not overwritten. `refresh` only controls whether the network/parse step
   repeats; resume state is the journal.
6. **Fixtures** — captures go in `crates/midwest-census/tests/fixtures/<name>/`; see §8.
7. **Report** — return `AdapterReport` with real counters, and report source, states covered,
   identifiers, fields, join keys, cost, blocks, and the measured marginal coverage.

## 8. Adding fixtures

- Adapter captures: `crates/midwest-census/tests/fixtures/<name>/` (one directory per source; 11
  exist today). Root-crate captures: `tests/fixtures/{cdp,rankings}/`. Fuzz seed corpora:
  `fuzz/corpus/<target>/`.
- Fixtures are **verbatim captures**: keep the bytes, never paraphrase or hand-edit. When a
  fixture must be edited for a test, document the edit at the `include_str!`/loader site (that is
  how `plain_names` documents `ND_PAGE_NO_AD`).
- Tests must run offline from the fixtures: no test may touch the network. Captures are embedded at
  compile time (`include_str!`), so no fixture server and no flags are needed.
- `cargo xtask source-fixture <source>` lists what exists; `cargo xtask source-test <source>` runs
  the tests whose names carry that source. The fixture set is the coverage list — a green
  `source-test` does not prove the capture set is complete.
- Write down provenance with each capture: the real URL, the capture date and HTTP status, and what
  the file proves. Today that provenance lives in the doc comment above each `include_str!`
  constant; there are no README files under `crates/midwest-census/tests/fixtures/` yet, so follow
  the doc-comment pattern (`wiaa/mod.rs`) rather than inventing a new one.
- **Planned**: `cargo xtask new-source` already writes a `tests/fixtures/<name>/README.md`
  placeholder for new adapters, and `xtask/src/templates.rs` defines its text.

## 9. Benchmarking

No optimization without benchmark evidence: a performance claim may only cite a committed
benchmark or an example harness run on the machine in hand; nothing in the repo enforces a
regression threshold yet (the gate only compiles bench targets).

- Root crate, criterion, `harness = false`, registered in `Cargo.toml`:
  `benches/artifact_store.rs` (artifact-store publication and point reads),
  `benches/blocking_fanout.rs` (`Runtime::blocking` fan-out at 1/8/32 workers),
  `benches/workbook_export.rs` (`WorkbookExport::{new, write, finish}`).
  Run one: `cargo bench -p athletic-rust-pipeline --bench <name>`.
- Census crate, criterion: `crates/midwest-census/benches/core.rs` (groups `census/parse`,
  `census/school_index`, `census/merge`), helpers in `benches/core/`. Run:
  `cargo bench -p midwest-census --bench core`.
- Example harnesses (clap-based, print `metric=…` lines plus one `json={…}` summary):
  `crates/midwest-census/examples/bench_store.rs` and `…/bench_census.rs`; run them with
  `--release` (`cargo run --release -p midwest-census --example bench_store -- --rows 200000`).
- Rules for a new measurement: assert the dataset (row/item counts) before reporting a rate, so a
  rate can never describe a corpus that lost rows; report the corpus size with the number; keep the
  target deterministic; read `tests/fixtures/**` rather than the network.
- Coverage today versus the §57 lanes: source parsing (`census/parse`), canonical merge
  (`census/merge`), Fjall read/merge scans (`census/merge`, `bench_store --scan`), root artifact
  store and workbook export (root benches), and coarse identity resolution (`census/school_index`)
  are measured. **Planned** — meet-result reconciliation, athlete identity scoring, PR reduction,
  census report/XLSX generation and AI case preparation have no committed bench target yet; new
  census lanes belong in `crates/midwest-census/benches/core.rs` (new group) or a new
  `crates/midwest-census/benches/<lane>.rs` registered in `crates/midwest-census/Cargo.toml`.

## 10. Adding a durable workflow

`crates/midwest-census/src/restate_services/` shows the pattern:

- a **virtual object** keyed by endpoint for cursor/window bookkeeping, where state is written as
  one value so a partially updated endpoint cannot exist (`restate_services/ingest.rs`);
- a **service** for request/response jobs whose heavy work runs on the blocking pool inside
  `ctx.run` (`restate_services/census.rs`, `jobs.rs`);
- a **workflow** that observes endpoints in request order, sleeps durably between windows and exits
  early on its stop signal (`restate_services/sweep.rs`), with an explicit ceiling on fan-out
  (`MAX_SWEEP_ENDPOINTS`, `MAX_SWEEP_WINDOWS` in `restate_services/mod.rs`);
- a **root fan-out workflow** that calls one jurisdiction object per unit of work and folds the
  per-unit reports into one, turning a failed unit into a row instead of failing the run
  (`restate_services/national.rs` driving `restate_services/jurisdiction.rs`);
- wire types and handler names in `restate_services/wire.rs`; service names come from the struct
  names, so renaming one is a breaking API change — add `#[handler(name = "...")]` instead.
- Workflow identities are deterministic and stable across retries:
  `jurisdiction:{state}:{season}:{revision}` (with `national:{season}:{revision}` as its root run —
  both in `crates/midwest-census/src/census/identity.rs`), `meet:{source}:{id}:{revision}`,
  `athlete:{source}:{id}:{revision}`, `review:{evidence_digest}:{policy_revision}`.
- Every handler effect runs inside `ctx.run` so a retry replays the journaled result; the service
  does not fetch — a producer hands it a table and its observation rows.

## 11. Handing work back

Report in this shape and nothing else:

```
TARGET:    <files>
CHANGED:   <one line per change>
COMMANDS:  <command> => <observed tail, verbatim>
RESIDUAL:  <what was not converted and why>
NEEDS:     <other files that must change, else none>
```

Claimed edits are verified by the integrator; do not report success without the command tail that
shows it. If a lane is blocked, say exactly which command failed and where the failure lives.

## 12. Decisions already made

`docs/adr/`: ADR-001 (Fjall remains the primary store), ADR-002 (Restate owns retries), ADR-003
(`GraduationYear` is cohort identity), ADR-004 (meet-first result ingestion), ADR-005 (AI cannot
override a deterministic contradiction), ADR-006 (Excel is the recruiter query layer). Read the
ADR before proposing a change to any of these; amending an ADR is a separate ticket.

## 13. Developer commands (`cargo xtask`)

`xtask/` wraps the repository's real tools; it reimplements none of them and prints every child
command before running it. Full reference: `xtask/README.md`.

```bash
cargo xtask gate [-- --update-baseline]        # tools/gate.sh, arguments passed through
cargo xtask scan|seams|integrity|domain-purity # the gate's measurements; non-zero exit on failure
cargo xtask quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]
cargo xtask ratchet <baseline> <clippy.tsv> <scan.json>
cargo xtask source-test <source>               # nextest -E 'test(<source>)'
cargo xtask source-fixture <source>            # what is captured under tests/fixtures/<source>/
cargo xtask census-status --store <dir>        # midwest-census report --core
cargo xtask coverage      --store <dir>        # midwest-census report (every source)
cargo xtask new-source <name>                  # scaffold a directory-layout adapter
cargo xtask bench                              # not implemented; exits non-zero (see §9)
```

`new-source` writes the layout the decomposition moves the flat `sources/<name>.rs` adapters to:
`mod.rs`, `parse.rs` (pure), `map.rs`, a module README, the fixture README, and one appended
`pub mod <name>;` line. It refuses any name whose module directory, flat module or fixture
directory already exists, and never edits an existing adapter.

## 14. Planned state (do not treat as present)

- **Planned**: the target workspace split — `census-store`, `census-crawl`, `census-reconcile`,
  `census-review`, `census-report`, `census-service`, and the `acq-*` crates (source:
  `ARCHITECTURE.md` §6; tracked in `docs/HARDENING-PROGRAM.md`). Of that list only
  `crates/census-domain` exists today; everything else still lives in `crates/midwest-census` and
  the root package.
- **National fan-out — in flight, not qualified.** `JurisdictionCensus` (object keyed
  `jurisdiction:{state}:{season}:{revision}`) and `NationalCensus` (workflow keyed
  `national:{season}:{revision}`) exist in the tree and are bound in `build_endpoint`
  (`crates/midwest-census/src/restate_services/{jurisdiction,national}.rs`,
  `restate_services/mod.rs:235-252`), with identities in
  `crates/midwest-census/src/census/identity.rs`. What is **planned** around them: a census CLI
  command to drive a national run, and any live-server qualification — those two modules carry no
  inline tests and the e2e test asserts only that `Census`/`Ingest`/`Sweep` are advertised, as a
  subset check (`crates/midwest-census/tests/fjall_restate_e2e.rs:40`).
- **Planned**: `cargo xtask source-check` and `cargo xtask export` (§63) — see §4.
- **Planned**: bench lanes for meet-result reconciliation, identity scoring, PR reduction and report
  generation — see §9.
- **Planned**: the census is being widened from the Midwest to all 51 jurisdictions
  (`census_domain::UsJurisdiction`, `crates/census-domain/src/jurisdiction.rs`); documents that
  describe "Midwest-only" scope describe the census as it was built, not the scope of the type
  system.
