# AGENTS.md — working in this repository

Primary instruction document for coding agents. Read it first, then the one document a task needs.
Every path and command below is one the tree actually has; §8 names the ones the objective asks for
that do **not** exist yet.

| Document | Read it for |
|---|---|
| `ARCHITECTURE.md` | system shape: the census crates, the root acquisition pipeline, module seams |
| `DOMAIN.md` | canonical types, identity, evidence and cohort rules |
| `SOURCE_ADAPTER_GUIDE.md` | the adapter contract in full |
| `FJALL_SCHEMA.md`, `docs/FJALL_SCHEMA.md` | store schema and keys; durability and sharp edges |
| `RESTATE_WORKFLOWS.md` | durable services, workflow identities, retry ownership |
| `TESTING.md` | test layout, fixtures, how to run one test |
| `PERFORMANCE.md` | which measurements may be quoted |
| `SCOPE.md`, `HANDOFF.md` | product scope; executed-evidence handoff (the history lives here, not in `README.md`) |
| `COLLECTOR_PATTERNS.md`, `SOURCES_SURVEY.md`, `PROFILE_REPLICATION.md` | dated source reconnaissance — research input, not current state |
| `docs/HARDENING-PROGRAM.md` | the hardening program (plan, with measured gap numbers) |
| `docs/DECOMPOSITION.md`, `docs/OPERATIONS.md`, `docs/VERIFICATION-EVIDENCE.md` | file-layout plan, runbook, executed evidence |
| `docs/adr/` | the six architecture decisions; amending one is a separate ticket |
| `research/README.md` | source-research lanes and their evidence contract |
| `xtask/README.md`, `tools/README.md`, `crates/midwest-census/README.md` | developer commands, the gate, the census crate |

## 1. Workspace map

Workspace root `Cargo.toml` (package `athletic-rust-pipeline`) with members `crates/census-domain`,
`crates/midwest-census`, `xtask`. `fuzz/` is a separate workspace of its own.

| Area | What it is | Entry points | Tests |
|---|---|---|---|
| `crates/census-domain/` | pure domain: canonical entities, `Id<T>`, `GradYear`/`ObservedGrade`, `Evidence`, `SourceNamespace`, `UsJurisdiction`; no async, no I/O | `src/lib.rs`, `src/model.rs`, `src/jurisdiction.rs`, `src/error.rs` | `src/model_tests.rs`, `src/jurisdiction_tests.rs` |
| `crates/census-domain/kani/` | Kani harnesses for the domain | `kani/{gradyear,id_mint,publish,census_domain_wiring}.rs` | the harnesses |
| `crates/midwest-census/` | the census: polite fetcher, Fjall store, source adapters, orchestration, report/bests/workbook, Restate services; bins `midwest-census` and `midwest-serve` | `src/lib.rs`, `src/cli/mod.rs`, `src/bin/midwest-serve.rs` | `tests/*.rs` (`parity_*`, `merge_properties`, `parser_roundtrip_properties`, `fjall_restate_e2e`, `backup_restore`, `recovery`), per-module `tests.rs` |
| `crates/midwest-census/src/net/` | fetcher: robots enforcement, per-host pacing, disk cache, conditional GET, bounded retries | `src/net/mod.rs`, `src/net/{execute,request,cache,robots,decode,client}.rs` | `src/net/tests.rs`, `src/net/execute/tests.rs` |
| `crates/midwest-census/src/store/` | Fjall observation store: append-only writes, read-time merge, journal, legacy import | `src/store/mod.rs`, `src/store/{read,write,keys,entities,sequences,legacy}.rs` | `src/store/tests.rs`, `src/store/loom_tests.rs` |
| `crates/midwest-census/src/sources/` | one module per provider adapter, plus `src/sources/registry/` (capability declarations) and `result_file` (vendor-artifact dispatch) | `src/sources/mod.rs`, `src/sources/<name>.rs`, `src/sources/<name>/` | per-adapter `tests.rs` |
| `crates/midwest-census/src/{census,report,bests,workbook,spawn.rs,bootstrap}/` | orchestration and reductions: roster walk, measured census, best marks, workbook, region-owned task spawning, service supervisor | `src/census/mod.rs`, `src/report/mod.rs`, `src/bests/mod.rs`, `src/workbook/mod.rs`, `src/bootstrap.rs` | module `tests.rs` files |
| `crates/midwest-census/src/restate_services/` | durable services (`Census`, `Ingest`, `Sweep`, `JurisdictionCensus`, `NationalCensus`) | `src/restate_services/mod.rs`, `src/restate_services/{jurisdiction,national,wire}.rs` | `src/restate_services/tests.rs` |
| root package `athletic-rust-pipeline` | the older Athletic.net-facing acquisition pipeline and operator CLI (Restate worker, browser session, rankings collection, workbook export/verify) | `src/main.rs`, `src/cli.rs`, `src/runtime/**` | `tests/*.rs` |
| `xtask/` | developer commands; the gate's measurement layer (`scan`, `seams`, `integrity`, `domain-purity`, `quality-baseline`, `ratchet`) | `xtask/src/main.rs` | 9 inline `#[cfg(test)]` tests in `xtask/src/{templates,seams,scan}.rs` |
| `tools/` | `tools/gate.sh` — the one quality gate — and `tools/quality-baseline.json`, the debt ratchet | `tools/README.md` | the gate itself |
| `benches/`, `crates/midwest-census/benches/`, `crates/midwest-census/examples/bench_*` | committed measurements; a performance claim may cite only these | `benches/*.rs`, `crates/midwest-census/benches/core.rs` | self-asserting datasets inside each target |
| `fuzz/` | standalone cargo-fuzz workspace | `fuzz/fuzz_targets/{hytek,compiled,xc,raceday}.rs` | the fuzz targets; seeds in `fuzz/corpus/` |
| `research/` | source-research lanes, append-only captures | `research/README.md`, `research/sources/<lane>/` | not a test lane |
| `deploy/systemd/` | unit/timer definitions for the collect timer and the Restate endpoints | `deploy/systemd/*.service`, `*.timer` | not a test lane |

## 2. Ownership and lanes

A ticket names the files one agent owns. Rules the workspace enforces:

- **One lane, one file set.** Edit only your files; never reformat, re-sort or "clean up" a file you
  do not own. An adapter lane owns `crates/midwest-census/src/sources/<name>.rs` and/or
  `src/sources/<name>/`, its fixtures `crates/midwest-census/tests/fixtures/<name>/`, and its
  research lane `research/sources/<name>/`.
- **Integration-owned files.** Do not edit unless your ticket says so: `Cargo.toml`, `Cargo.lock`,
  `crates/*/Cargo.toml`, the census crate's facades
  (`crates/midwest-census/src/{lib.rs,cli/**,bin/**}`, `crates/midwest-census/src/sources/mod.rs`,
  `crates/midwest-census/src/sources/registry/table.rs`), `tools/**`, `xtask/**`, `.github/**`,
  `docs/HARDENING-PROGRAM.md`, `HANDOFF.md`. When your change needs lines in one of them, post the
  exact lines to the file's owner (`hub send` to `Main` for the integration list) and keep working.
- **Registry declarations.** Never edit `src/sources/registry/table.rs` yourself: send one message
  with slug, capabilities, origin, admission and the symbol each capability rests on to the registry
  owner.
- **Scoped commands only while lanes are in flight.** A workspace-wide sweep mid-flight blocks on
  siblings' half-finished edits and reports phantom failures. Validate your change with
  `cargo check -p midwest-census --all-targets`, `cargo test -p midwest-census <path>`,
  `cargo xtask source-test <source>`, `rustfmt --edition 2021 <your files>`. The project-wide sweep —
  `cargo fmt --all`, `cargo clippy --workspace`, a workspace-wide `cargo test`, `tools/gate.sh`,
  `cargo xtask gate` — belongs to integration, once, after the lanes land.
- **No new dependencies.** Manifests are integration-owned; a new crate or dependency is a request.
- **No Python.** No `.py` file exists anywhere in this repository and none may be added; tooling is
  Rust (`xtask`) plus `tools/gate.sh`.
- **Git discipline.** Never run `git stash`, `reset`, `checkout`, `restore`, `clean` or `switch`:
  other agents work in the same tree, and those commands destroy in-flight edits silently. Read-only
  git (`status`, `diff`, `log`, `show`) is fine. Re-read a file before editing it.
- **Captures are verbatim.** `research/**`, `crates/midwest-census/tests/fixtures/**`, `tests/fixtures/**`
  and `fixtures/**` are raw captures: add new files, never rewrite an existing one. If a capture is
  wrong, add a corrected file and say why in the report.

## 3. Commands

`cargo xtask <command>` is the alias in `.cargo/config.toml` for `cargo run -p xtask -- <command>`.
Every wrapper prints the child command it runs (`+ …`) before running it.

### 3.1 The gate (integration runs it once, after the lanes land)

```bash
cargo xtask gate                          # == bash tools/gate.sh
cargo xtask gate -- --full                # arguments after `--` go to gate.sh
cargo xtask gate -- --update-baseline --allow-increase
```

| Lane | Command the gate runs (`tools/gate.sh`) |
|---|---|
| fmt | `cargo fmt --all -- --check` |
| check | `cargo -Zallow-features="$FEATURE_ALLOWLIST" check --workspace --all-targets --all-features` |
| doc | `cargo doc --workspace --all-features --no-deps` |
| tests | `cargo nextest run --workspace --all-features` (`cargo test --workspace --all-features --quiet` if nextest is absent) |
| strict clippy | `cargo -Zallow-features="$FEATURE_ALLOWLIST" clippy --workspace --lib --bins --examples --all-features --message-format=json -- <LINT_SET>` |
| production scan | `cargo run -q -p xtask -- scan`, then the size budgets |
| module seams | `cargo run -q -p xtask -- seams` |
| domain purity | `cargo run -q -p xtask -- domain-purity` |
| debt ratchet | `cargo run -q -p xtask -- ratchet tools/quality-baseline.json <clippy.tsv> <scan.json>` |
| deny / audit / vet | `cargo deny check` / `cargo audit --quiet` / `cargo vet --locked` |
| machete / geiger | `cargo machete` / `cargo geiger --workspace --all-features --output-format Json` |
| feature powerset | `cargo hack check --workspace --feature-powerset` |
| bench presence | `cargo bench --workspace --no-run` |
| mutants (`--full` only) | `cargo mutants --workspace --in-place` |

A cargo subcommand that is not installed prints `SKIP` for its lane; `jq` missing is a hard failure
(the clippy and scan tallies flow through it). `LINT_SET` is `-D warnings -D unsafe_code -D
clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D
clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D
clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions
-D clippy::let_underscore_must_use -D clippy::await_holding_lock`.

### 3.2 The gate's measurement layer, one at a time

```bash
cargo xtask scan           # forbidden constructs + size budgets, JSON on stdout
cargo xtask seams          # crate::… edges between the census crate's top-level modules
cargo xtask integrity      # type-integrity review candidates, JSON on stdout
cargo xtask domain-purity  # census-domain's normal dependency tree
cargo xtask quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]
cargo xtask ratchet <baseline> <clippy.tsv> <scan.json>
```

`quality-baseline` refuses to raise a number without `--allow-increase` (`refusing to raise the
baseline without --allow-increase`, exit 1); `ratchet` exits non-zero when any metric grew. The
`clippy.tsv` shape is `crate<TAB>lint<TAB>count`, which is what the gate's clippy lane writes.

### 3.3 Per-source lanes

```bash
cargo xtask source-fixture ks   # list crates/midwest-census/tests/fixtures/ks/, recursive, with sizes
cargo xtask source-test ks      # cargo nextest run -p midwest-census -E 'test(ks)'
cargo xtask source-check ks     # the same command: `source-check` is a visible alias of source-test
cargo xtask new-source <name>   # scaffold the directory-layout adapter
```

`source-test`'s filter is a substring match over test names, so `ks` also selects
`bests::tests::marks_print_in_the_notation_a_reader_expects` (`marks` contains `ks`); it runs
whatever matches and does not prove the fixture set is complete. A missing fixture directory is an
error naming the directories that do exist. See §5 and §6 for the layouts these commands serve.

### 3.4 Census reports from a store

```bash
cargo xtask census-status --store <dir>   # midwest-census report --core
cargo xtask coverage      --store <dir>   # midwest-census report (every source)
cargo xtask export        --store <dir> [--out FILE] [--grad-year YYYY] [--all-sources] [--limit N]
                                          # midwest-census workbook, from an existing store
```

All three run the shipped `midwest-census` binary and write into `<store>/out/`; the binary takes an
exclusive Fjall lock, so stop `midwest-serve` first or use a different store. The store default is
`var/midwest-census`. The census CLI itself:

```bash
cargo run --release -p midwest-census --bin midwest-census -- teams   --states WI,MN --refresh
cargo run --release -p midwest-census --bin midwest-census -- teams   --all-states --refresh
cargo run --release -p midwest-census --bin midwest-census -- collect --states WI,MN
cargo run --release -p midwest-census --bin midwest-census -- collect --all-states
cargo run --release -p midwest-census --bin midwest-census -- provider wiaa
cargo run --release -p midwest-census --bin midwest-census -- provider milesplit --states WI
cargo run --release -p midwest-census --bin midwest-census -- consolidate
cargo run --release -p midwest-census --bin midwest-census -- report --print
cargo run --release -p midwest-census --bin midwest-census -- bests
cargo run --release -p midwest-census --bin midwest-census -- workbook
cargo run --release -p midwest-census --bin midwest-census -- run
```

Global flags: `--store <dir>`, `--delay-ms <n>`, `--user-agent <ua>`, `--authorized-host <host>`
(repeatable). `cargo run --release -p midwest-census --bin midwest-census -- --help` prints the full
subcommand list; `crates/midwest-census/README.md` documents each one.

`--states <CODES>` and `--all-states` are the two jurisdiction selectors, and they resolve
differently for a roster walk than for a restriction-shaped adapter (`cli::resolve_states` /
`cli::resolve_restriction`). `collect`, `teams` and `provider milesplit` default to Wisconsin when
neither flag is given. Every other `provider` arm treats an empty `--states` as "no restriction" and
answers from its own coverage — `provider ohsaa` still covers Ohio — while a non-empty list that
omits that state fails (`states ["IA"] do not include OH; this adapter covers Ohio only`, from
`sources/ohsaa/collect.rs`). `--all-states` means 50 states + DC, and giving it together with
`--states` is refused (`--all-states cannot be combined with --states`, exit 1) rather than silently
preferring one.

### 3.5 The root acquisition pipeline

```bash
cargo run --release -p athletic-rust-pipeline -- --help   # the operator CLI's command list
cargo run --release -p athletic-rust-pipeline -- worker --config config.native.toml --bind 127.0.0.1:19181
cargo run --release -p athletic-rust-pipeline -- <deploy|start|status|browser-start|browser-status|export|verify|rankings-status|rankings-pause|rankings-resume> …
```

### 3.6 Benchmarks and the concurrency lane

```bash
cargo xtask bench -- parse         # == cargo bench -p midwest-census parse; the `--` is required, and
                                   # `parse` is a case in crates/midwest-census/benches/pipeline.rs
cargo bench -p athletic-rust-pipeline --bench artifact_store
cargo bench -p athletic-rust-pipeline --bench blocking_fanout
cargo bench -p athletic-rust-pipeline --bench workbook_export
cargo bench -p midwest-census --bench core
cargo run --release -p midwest-census --example bench_store  -- --rows 200000 --batch 1000 --scan
cargo run --release -p midwest-census --example bench_census -- --schools 500
cargo test -p midwest-census --features loom --lib     # loom models, opt-in feature
```

A performance claim may only cite a committed benchmark or a harness run on the machine in hand, with
the dataset assertion it printed (`PERFORMANCE.md`).

## 4. Source policy (non-negotiable)

- Every source request goes through `crate::net::Fetcher` (`crates/midwest-census/src/net/mod.rs`).
  No raw client, no parallel path around pacing, cache or robots.
- `robots.txt` is enforced: a disallowed path returns `FetchError::Robots` and counts in
  `FetchStats::robots_blocked`. An operator may name a host on `--authorized-host`; the rule is then
  counted as `robots_authorized` and the request proceeds under the 2 rps ceiling. Default: every
  host's robots rules are enforced.
- Hard pacing: 2 requests/second per host, never closer than `MIN_AUTHORIZED_DELAY` (500 ms) for an
  authorized host whatever `--delay-ms` says; default spacing 1 s; `gobound.com` 10 s
  (`sources::default_host_delays`). Bounded fan-out via `sources::CONCURRENCY_BOUND`.
- Cache before network: bodies are cached on disk by content hash, so a re-run is free and an
  interrupted collection resumes without re-fetching.
- Never bypass a CAPTCHA, an authentication barrier or an access control: no cookie jar, no challenge
  evasion, no user-agent spoofing. A served challenge is a terminal state, not a bug to route around.
- Retry ownership: **adapters and CLI shells perform one attempt** and never retry a fetcher call.
  Retries live in exactly two places inside the transport (the `FetchError::retryable` loop, at most
  `MAX_RETRIES = 3` attempts with jittered backoff, and the conditional-GET replay of a `304` with no
  cached body). Durable retry of a whole step belongs to Restate (ADR-002).
- Collect only athletic evidence and publicly published professional school/sport contacts. Never
  athlete personal email, personal phone, home address or unrelated personal data.
- A source failure is never "no match": record the terminal state and continue elsewhere. There is no
  `OperationTerminal` type — use `FetchError` (net), `CrawlError` (census sources), or `FailureCode` /
  `BrowserError` (root `src/runtime/`).

## 5. Adapter layout and adding an adapter

```text
crates/midwest-census/src/sources/
  <name>.rs            flat module: the whole adapter in one file (older adapters: ks, milesplit, …)
  <name>/mod.rs        module root: doc header naming the measured request unit, SOURCE_ID, Options, collect
  <name>/parse.rs      pure parsing: captured bytes in, parsed rows out, no I/O
  <name>/map.rs        parsed rows -> canonical entities
  <name>/tests.rs      fixture-driven tests (#[cfg(test)])
  <name>/<other>.rs    adapter-specific stages when one file would be too large (wiaa/collect.rs, …)
```

- `collect` is a plain async function, no trait indirection:
  `pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport>`.
  `AdapterReport` carries `adapter`, `rows`, `requests`, `from_cache`, `errors`, `with_email`, `unit`,
  `notes` (`src/sources/mod.rs`).
- `cargo xtask new-source <name>` writes `mod.rs`, `parse.rs`, `map.rs`, a module README, the fixture
  README, and appends `pub mod <name>;` to the module list. Hyphens become underscores. It refuses a
  name that is not a lowercase identifier, a Rust keyword, or one whose module directory, flat module
  or fixture directory already exists. The generated `collect` carries the canonical signature —
  `CrawlResult<AdapterReport>`, failing with `CrawlError::Invariant` until implemented — and
  `xtask/src/templates.rs` pins that shape with its own tests.
- Adapter steps: fixtures first (§6), then `parse.rs` (pure), then `map.rs`, then `collect` driving
  `ctx.fetcher` and appending through `sources::append_all` — never by writing store keys directly.
- Registration: the module list in `src/sources/mod.rs` (integration-owned), a registry declaration in
  `src/sources/registry/table.rs` (send it to the registry owner), a CLI arm in `src/cli/provider.rs`
  (integration-owned), and a `SourceNamespace` variant in `crates/census-domain/src/model.rs` if the
  source is new. A non-core source also goes into `report::NON_CORE_SOURCE_IDS`.
- Observations are append-only: a re-run appends rows and the reader merges them
  (`src/store/read.rs`). `refresh` only controls whether the network/parse step repeats; resume state is
  the journal.

## 6. Fixtures

```text
crates/midwest-census/tests/fixtures/<name>/   one directory per source (13 exist today), plus
                                               coach_contacts_sample.csv and tests/golden/*.json
tests/fixtures/{cdp,rankings}/                 root-crate captures
fuzz/corpus/<target>/                          fuzz seed corpora
```

- Verbatim captures: keep the bytes, never paraphrase or hand-edit. When a fixture must be edited for
  a test, document the edit at the `include_str!`/loader site.
- Tests run offline from the fixtures: no test may touch the network. Captures are embedded at compile
  time (`include_str!`), so no fixture server and no flags are needed.
- Provenance lives in the doc comment above each `include_str!` constant: the real URL, the capture
  date, the HTTP status, and what the file proves. There is no `index.json`/`raw`/`expected` scheme;
  follow the doc-comment pattern a neighbouring adapter uses.
- `cargo xtask source-fixture <name>` lists what exists; `cargo xtask source-test <name>` runs the
  tests whose names carry that source. A green `source-test` does not prove the capture set is
  complete — the fixture directory is the coverage list.

## 7. Coding standards

Adapted NASA/JPL Power of Ten, enforced by the gate's clippy set plus `scan`, `integrity`,
`seams` and `domain-purity`.

- Production code (every `src/**` tree — root `src/**` and `crates/*/src/**` — excluding files named
  `tests.rs` and code inside `#[cfg(test)]`) carries no
  `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unsafe`, `as` casts, slice/string
  indexing or unchecked arithmetic. Use `?`, `match`, `let … else`, `.get()`, `u32::try_from`,
  `checked_*`/`saturating_*`.
- Size budget: files stay under ~300 lines; functions under ~60 logical lines (≤25 for hot paths).
  `scan` reports `files_over_300_lines`, `functions_over_60_lines`,
  `functions_over_25_logical_lines`; the ratchet fails when the first two grow.
- Errors are typed: `thiserror` in libraries, `anyhow` with `.context(…)` at the CLI boundary.
- Values crossing a boundary are validated once, there, and carried in types rather than re-checked or
  stringly typed. A jurisdiction is `census_domain::UsJurisdiction`, never a `String`.
- Log with `tracing`, never `println!`/`eprintln!` in library code; async entry points carry
  `#[tracing::instrument(skip_all, fields(…))]`.
- Debt is ratcheted, never blessed: `tools/quality-baseline.json` may only shrink, and a burndown is
  the only legitimate reason to move it (`cargo xtask gate -- --update-baseline`).

## 8. What does not exist yet

- There is no `cargo xtask replay`: a truthful replay needs a cache-only read surface
  (`net::cache` is crate-private and the fetcher has no offline mode). Do not document one as if it
  existed.
- `cargo xtask baseline` does not exist either: the debt subcommands are `quality-baseline` and
  `ratchet`.
- The target workspace split (`census-store`, `census-crawl`, `census-reconcile`, `census-review`,
  `census-report`, `census-service`, the `acq-*` crates) is planned, not present: of that list only
  `crates/census-domain` exists.
- `JurisdictionCensus` and `NationalCensus` exist in `src/restate_services/` but have no CLI command
  and no live-server qualification yet.

## 9. Evidence discipline

- Every claim in a report is backed by a command you actually ran, with the raw tail pasted
  verbatim. If a thing is unverified, say which part and why; never invent a shape you could not
  verify from a capture — report the capture that would settle it.
- No stubs, no mock successes, no fabricated verification. A slice that cannot verify a behaviour says
  so.
- Hand work back in this shape and nothing else:

```text
TARGET:    <files>
CHANGED:   <one line per change>
COMMANDS:  <command> => <observed tail, verbatim>
RESIDUAL:  <what was not converted and why>
NEEDS:     <other files that must change, else none>
```

## 10. Decisions already made

`docs/adr/`: ADR-001 (Fjall remains the primary store), ADR-002 (Restate owns retries), ADR-003
(`GraduationYear` is cohort identity), ADR-004 (meet-first result ingestion), ADR-005 (AI cannot
override a deterministic contradiction), ADR-006 (Excel is the recruiter query layer). Read the ADR
before proposing a change to one of them; amending an ADR is a separate ticket.
