# AGENTS.md — working in this repository

Primary instruction document for coding agents. Read this before touching anything; read
`ARCHITECTURE.md` for the system shape and `docs/HARDENING-PROGRAM.md` for the current phase plan.

## 1. Repository map

```
athletic-rust-pipeline/                  root crate `athletic-rust-pipeline`
  src/                                   acquisition pipeline: Restate services, browser session,
                                         ranking collection, source dispatch, row/profile workers,
                                         search, store, xlsx, workbook ingest/verify/export
  tests/                                 root integration tests (+ tests/fixtures/<source>/)
  tools/                                 gate.sh, ratchet.py, quality-baseline.json, scans
  deploy/systemd/                        unit + timer definitions
  docs/                                  HARDENING-PROGRAM.md, OPERATIONS.md
  crates/midwest-census/                 census library + two binaries
    src/model.rs                         canonical entities, ids, grad-year cohorts, evidence
    src/net.rs                           robots-enforcing, cache-first, per-host-paced fetcher
    src/store.rs                         Fjall observation store (append-only + read-time merge)
    src/sources/**                       one module per provider adapter
    src/census.rs                        resumable batch orchestration
    src/report.rs, bests.rs, workbook.rs measured census, PR reduction, spreadsheet export
    src/restate_services.rs              durable services over the same adapters
    src/bootstrap.rs                     service supervisor: task region, cancel/drain/finalize
    src/bin/midwest-serve.rs             Restate endpoint binary
    tests/                               end-to-end tests (+ tests/fixtures/<source>/)
```

## 2. Ownership

A ticket names the files one agent owns. Rules:

- Edit only your own files. Never reformat, re-sort, or "clean up" a file you do not own.
- Shared contracts are Main's: domain types, public error enums, Fjall schema and key encodings,
  Restate workflow/service signatures, workspace manifests. Propose changes; do not apply them.
- Adapters, fixtures and research notes for one source belong to one owner at a time.
- If your change needs a file you do not own, stop and report it under `NEEDS`.

## 3. Git discipline (hard rule)

Never run `git stash`, `git reset`, `git checkout`, `git restore`, `git clean`, `git switch`, or any
command that rewrites the working tree. Other agents work in the same tree at the same time; a
`reset`/`stash` silently destroys their in-flight edits. Read-only git (`status`, `diff`, `log`,
`show`) is fine. Re-read a file before editing it and verify your own hunks with `git diff -- <file>`
before reporting.

## 4. Commands

```bash
# Full gate (fmt, check --all-targets, strict clippy, tests, scans, ratchet, optional tool lanes)
tools/gate.sh

# Fast local lanes
cargo fmt --all
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --lib --bins --all-features -- -D warnings \
  -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn \
  -D clippy::indexing_slicing -D clippy::string_slice -D clippy::as_conversions \
  -D clippy::arithmetic_side_effects -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo nextest run --workspace            # or: cargo test --workspace

# Census pipeline
cargo run --release -p midwest-census --bin midwest-census -- teams  --states WI,MN --refresh
cargo run --release -p midwest-census --bin midwest-census -- collect --states WI,MN --grad-year 2027
cargo run --release -p midwest-census --bin midwest-census -- consolidate
cargo run --release -p midwest-census --bin midwest-census -- report --print
cargo run --release -p midwest-census --bin midwest-census -- bests
cargo run --release -p midwest-census --bin midwest-census -- workbook
cargo run --release -p midwest-census --bin midwest-serve -- --listen 127.0.0.1:9080 --data-dir var/midwest-census
```

Cargo build-lock waits are normal when several agents run: cargo serializes on the build directory.

## 5. Coding standards

Doctrine is NASA/JPL Power of Ten adapted for Rust (see `skill://holzman-rust`), enforced by the
gate's strict clippy set plus two scans (`tools/production_scan.py`, `tools/type_integrity_scan.py`).

Forbidden in production code: `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`,
`unreachable!`, production `assert!`-family, unchecked indexing and string slicing, `as` casts,
unchecked arithmetic on values that can overflow, ignored `Result`/must-use values, silent fallbacks
(`unwrap_or_default()`, ignored errors), and locks held across `await`.

Required shape:

- Errors are typed. Library crates use `thiserror`; the CLI/application boundary may use `anyhow`
  with `.context("…")`. Keep the existing message text when converting a panic into an error.
- Files stay under ~300 lines; functions under ~60 logical lines (≤25 for hot paths). Split by
  behaviour, not by line count.
- Prefer existing helpers, existing constants and the file's existing idiom over new abstractions.
  No compat wrappers or aliases: migrate the call sites.
- Values crossing a boundary are validated once, there, and carried in types (`Result`, newtypes,
  enums) rather than re-checked or stringly-typed.
- Log with `tracing`, never `println!`/`eprintln!` in library code. Async entry points carry
  `#[tracing::instrument(skip_all, fields(…))]`.

## 6. Source policy (non-negotiable)

- Respect `robots.txt` (enforced in `crates/midwest-census/src/net.rs`), per-host pacing (~2 rps,
  bounded concurrency via `sources::CONCURRENCY_BOUND`) and cache-before-network behaviour.
- Never bypass a CAPTCHA, an authentication barrier or an access control. A served challenge latches
  the browser into `HumanRequired`; that latch is a correct outcome, not a bug to route around.
- One retry owner: Restate owns retries; the transport performs one attempt. Do not stack retries.
- Collect only athletic evidence and publicly published professional school/sport contacts. Never
  athlete personal email, personal phone, home address, or unrelated personal data.
- A source failure is never equivalent to "no match": record the terminal state
  (`OperationTerminal`: `NotFound`, `RateLimited`, `HumanRequired`, `RetryExhausted`,
  `SourceUnavailable`, `PolicyBlocked`) and continue elsewhere.

## 7. Adding a provider adapter

Adapters are plain async functions taking `AdapterContext` and returning `AdapterReport`; there is no
trait indirection by design (`crates/midwest-census/src/sources/mod.rs`). To add one:

1. `crates/midwest-census/src/sources/<name>.rs` — parse at the boundary, emit canonical entities.
2. Register the module in `sources/mod.rs` and dispatch it from `census.rs` (or expose it as a
   `provider <name>` CLI subcommand if it is one-shot).
3. Append with `sources::append_all(&store, Table::X, &rows)`; never write keys directly.
4. Fixtures go in `crates/midwest-census/tests/fixtures/<name>/` with a short README explaining
   which real URL each file came from and what it proves. Tests must run offline from fixtures.
5. Report: source, states covered, identifiers, fields, join keys, cost, blocks, and the measured
   marginal coverage.

## 8. Adding a durable workflow

`crates/midwest-census/src/restate_services.rs` shows the pattern: a virtual object keyed by
endpoint for cursor/window bookkeeping (state written as one value so a partially updated endpoint
cannot exist), a sweep workflow that observes endpoints in request order, and an explicit ceiling
(`MAX_SWEEP_ENDPOINTS`) on fan-out. Workflow identities are deterministic and stable across retries:
`jurisdiction:{state}:{season}:{revision}`, `meet:{source}:{id}:{revision}`,
`athlete:{source}:{id}:{revision}`, `review:{evidence_digest}:{policy_revision}`.

## 9. Handing work back

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

## 10. Decisions already made

See `docs/adr/` for the short ADR set: Fjall remains the system of record; Restate owns retries;
`GraduationYear` (not grade) is cohort identity; meet-first acquisition beats per-athlete fetching;
AI cannot override a deterministic contradiction; Excel is the recruiter query layer.
