# athletic-rust-pipeline

One Rust workspace holding a national Class-of-2027 high-school track & field / cross-country census
and the older Athletic.net-facing acquisition pipeline the census grew out of. This file is the front
door; it says only what is in the tree today and points at the document that owns each subject.

## Workspace

Workspace root `Cargo.toml` is virtual: it declares the nine members under `crates/` and `xtask/` and
no root package — the acquisition root `athletic-rust-pipeline` that used to sit here was deleted once
the census crates owned its work (see `ARCHITECTURE.md` §1 and the manifest's own header). `fuzz/` is
a cargo-fuzz workspace of its own.

| Path | What it is | Entry point |
|---|---|---|
| `crates/census-service/` | the census: polite fetcher (robots, per-host pacing, disk cache), Fjall observation store, one module per source adapter, orchestration, census report, best marks, workbook, Restate services | `crates/census-service/README.md`, `src/cli/mod.rs`; bins `census-service`, `census-serve` |
| `crates/census-domain/` | pure domain types: canonical entities, `Id<T>`, `GradYear`/`ObservedGrade`, `Evidence`, `SourceNamespace`, `UsJurisdiction`; no async, no I/O | `src/lib.rs` |
| `crates/athleticnet-browser/` | Athletic.net transport: one persistent headed profile, tab pool, CDP capture, challenge detection, 429 cooldown, bounded single-attempt admission | `src/lib.rs` |
| `crates/census-crawl/` | Acquisition plane: robots-enforcing fetcher, per-provider adapters, provider registry | `src/lib.rs` |
| `crates/census-reconcile/` | Reconciliation lane (§17): deterministic workflow identity and row-level workbook verification against the store | `src/lib.rs` |
| `crates/census-report/` | Reporting plane (§17): coverage, bests, PR projection, workbook export | `src/lib.rs` |
| `crates/census-review/` | Local-model review lane: asks a small model about retained merge findings; keeps only answers the store's evidence can back | `src/lib.rs` |
| `crates/census-store/` | Fjall system of record: append-only observations, keyspaces, snapshots, backup/restore/integrity | `src/lib.rs` |
| `xtask/` (bin `g1-audit`) | (moved) read-only counted inventory of retained athletic.net `/Search.aspx/runSearch` response bodies — now a second binary at `xtask/src/g1/main.rs`, invoked as `cargo run -p xtask --bin g1-audit` | `xtask/src/g1/main.rs`; bin `g1-audit` |
| `xtask/` | developer commands and the gate's measurement layer (`scan`, `seams`, `integrity`, `domain-purity`, `quality-baseline`, `ratchet`) | `xtask/README.md`, `xtask/src/main.rs` |
| `tools/` | `tools/gate.sh` — the one quality gate — and `tools/quality-baseline.json`, the debt ratchet | `tools/README.md` |
| `crates/census-service/benches/{core,pipeline}`, `crates/census-service/examples/bench_*` | committed measurements; a performance claim may cite only these | `PERFORMANCE.md` |
| `fuzz/` | cargo-fuzz targets for the result-file parsers | `fuzz/fuzz_targets/`, seeds in `fuzz/corpus/` |
| `research/` | source-research lanes, append-only captures (`research/sources/<lane>/`) | `research/README.md` |
| `docs/` | hardening program, operations runbook, executed evidence, Fjall backup/restore, deployment lifecycle, current-state reference, ADRs, module migration map | `docs/HARDENING-PROGRAM.md`, `docs/OPERATIONS.md`, `docs/FJALL_BACKUP.md`, `docs/deployment-lifecycle.md`, `docs/architecture.md`, `docs/adr/`, `docs/migration/module-map.md` |
| `deploy/systemd/` | unit and timer definitions for the collect timer and the Restate endpoints | `deploy/systemd/` |

## Start here

- [`AGENTS.md`](AGENTS.md) — **primary instruction document**: ownership and lanes, the exact
  commands, the gate lanes, adapter and fixture layouts, source policy, evidence discipline.
- `ARCHITECTURE.md`, `DOMAIN.md`, `SOURCE_ADAPTER_GUIDE.md`, `FJALL_SCHEMA.md`,
  `RESTATE_WORKFLOWS.md`, `TESTING.md`, `PERFORMANCE.md` — the per-subject contracts.
- `SCOPE.md`, `HANDOFF.md`, `docs/VERIFICATION-EVIDENCE.md` — product scope and the recorded
  end-to-end evidence; history belongs there, not here.
- `SOURCES_SURVEY.md`, `COLLECTOR_PATTERNS.md`, `PROFILE_REPLICATION.md` — dated source
  reconnaissance (research input).
- `crates/census-service/README.md`, `xtask/README.md`, `tools/README.md` — the census crate, the
  developer commands and the gate.
- `CHROMIUM_DESIGN.md` — Chromium browser automation design for the Athletic.net transport lane.
- `docs/OPERATIONS.md` — operations runbook; successor to the superseded `HANDOFF.md`.
- `docs/FJALL_BACKUP.md` — Fjall backup and restore procedures.
- `docs/deployment-lifecycle.md` — deployment and lifecycle management.
- `docs/architecture.md` — current-state reference (the binding architecture and standards live in root `ARCHITECTURE.md`).
- `docs/adr/README.md` — the decision record index; `docs/adr/ADR-*.md` are the individual decisions.
- `docs/migration/module-map.md` — the module inventory and the crate cut.
- `docs/DECOMPOSITION.md` **(Superseded 2026-09-25):** the crate layout in `ARCHITECTURE.md` §1 owns this subject now. Kept as the dated record of the Phase 2 split plan.
- `WORKFLOW_REVIEW.md` **(Superseded 2026-09-25):** root `ARCHITECTURE.md` owns this subject now. Kept as the dated record of the deleted root package architecture.
- `HANDOFF.md` **(Superseded 2026-09-25):** `docs/OPERATIONS.md` and `docs/VERIFICATION-EVIDENCE.md` own this subject now. Kept as the dated record of the 2026-09-21 live-chain handoff.
- `SOURCES_SURVEY.md` **(Superseded 2026-09-25):** `crates/census-crawl/src/registry/` and `SOURCE_ADAPTER_GUIDE.md` own this subject now. Kept as the dated record of 2026-09-20 source survey.
- `COLLECTOR_PATTERNS.md` **(Superseded 2026-09-25):** `SOURCE_ADAPTER_GUIDE.md` and `crates/census-crawl/src/registry/` own this subject now. Kept as the dated record of 2026-09-20 reconnaissance.
- `PROFILE_REPLICATION.md` **(Superseded 2026-09-25):** `crates/census-crawl/src/milesplit/` and the profile collection lane own this subject now. Kept as the dated record of 2026-09-20 profile replication plan.

## Run it

```bash
# Census pipeline (store defaults to var/census-service; the binary takes an exclusive Fjall lock)
cargo run --release -p census-service --bin census-service -- teams   --states WI,MN --refresh
cargo run --release -p census-service --bin census-service -- collect --states WI,MN
cargo run --release -p census-service --bin census-service -- provider wiaa
cargo run --release -p census-service --bin census-service -- consolidate
cargo run --release -p census-service --bin census-service -- report --print
cargo run --release -p census-service --bin census-service -- bests
cargo run --release -p census-service --bin census-service -- workbook
cargo run --release -p census-service --bin census-service -- run

# Census reports and the workbook through the developer command wrapper.
# Default: ask the running census (127.0.0.1:18095/). --store opens the store in process
# instead, so it needs census-serve stopped.
cargo xtask census-status                              # Census/status, counted by the service
cargo xtask coverage                                   # Report/run, every source
cargo xtask export                                     # Workbook/run into <store>/out/
cargo xtask census-status --store var/census-service   # report --core, offline
cargo xtask source-test   ks                           # one source's tests (alias: source-check)
cargo xtask bench         -- parse                     # cargo bench -p census-service parse

# Quality gate (integration runs it once, after lanes land; see AGENTS.md §2)
cargo xtask gate
```

## Ground rules

Engineering standards, gate commands, ownership, source policy and evidence discipline are in
`ARCHITECTURE.md` (binding) and `AGENTS.md` (checklist).
- No Python anywhere in this repository; tooling is Rust (`xtask`) plus `tools/gate.sh`.


## Status

`README.md` records no run results. End-to-end evidence, its scope and what remains unqualified live
in `docs/VERIFICATION-EVIDENCE.md`, `docs/OPERATIONS.md` and `crates/census-service/README.md`; the current
hardening program and its measured gaps live in `docs/HARDENING-PROGRAM.md`. The superseded records — `HANDOFF.md`,
`docs/DECOMPOSITION.md`, `WORKFLOW_REVIEW.md`, `SOURCES_SURVEY.md`, `COLLECTOR_PATTERNS.md`, `PROFILE_REPLICATION.md` — are kept
as dated snapshots and must not be followed as current guidance. The target workspace split described in
`ARCHITECTURE.md` was executed: all eight workspace-member crates now exist under `crates/`, plus the `xtask` developer-harness package and the `fuzz/` cargo-fuzz workspace.
