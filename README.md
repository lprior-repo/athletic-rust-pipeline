# athletic-rust-pipeline

One Rust workspace holding a national Class-of-2027 high-school track & field / cross-country census
and the older Athletic.net-facing acquisition pipeline the census grew out of. This file is the front
door; it says only what is in the tree today and points at the document that owns each subject.

## Workspace

Workspace root `Cargo.toml` (package `athletic-rust-pipeline`) plus members `crates/census-domain`,
`crates/midwest-census`, `crates/g1-audit`, `xtask`. `fuzz/` is a cargo-fuzz workspace of its own.

| Path | What it is | Entry point |
|---|---|---|
| `crates/midwest-census/` | the census: polite fetcher (robots, per-host pacing, disk cache), Fjall observation store, one module per source adapter, orchestration, census report, best marks, workbook, Restate services | `crates/midwest-census/README.md`, `src/cli/mod.rs`; bins `midwest-census`, `midwest-serve` |
| `crates/census-domain/` | pure domain types: canonical entities, `Id<T>`, `GradYear`/`ObservedGrade`, `Evidence`, `SourceNamespace`, `UsJurisdiction`; no async, no I/O | `src/lib.rs` |
| `crates/g1-audit/` | read-only counted inventory of the retained athletic.net `/Search.aspx/runSearch` response bodies (G1 slice): digests, interstitial markers, id/href classes — the Rust port of the deleted `research/captures/g1/inventory-search-pages.py`, byte-identical on both retained corpora | `crates/g1-audit/src/main.rs`; bin `g1-audit`
| root package `athletic-rust-pipeline` | the Athletic.net-faced acquisition pipeline and operator CLI: Restate worker, persistent Chromium transport, rankings collection, workbook export and verification | `src/main.rs`, `src/cli.rs`, `src/runtime/**` |
| `xtask/` | developer commands and the gate's measurement layer (`scan`, `seams`, `integrity`, `domain-purity`, `quality-baseline`, `ratchet`) | `xtask/README.md`, `xtask/src/main.rs` |
| `tools/` | `tools/gate.sh` — the one quality gate — and `tools/quality-baseline.json`, the debt ratchet | `tools/README.md` |
| `benches/`, `crates/midwest-census/benches/`, `crates/midwest-census/examples/bench_*` | committed measurements; a performance claim may cite only these | `PERFORMANCE.md` |
| `fuzz/` | cargo-fuzz targets for the result-file parsers | `fuzz/fuzz_targets/`, seeds in `fuzz/corpus/` |
| `research/` | source-research lanes, append-only captures (`research/sources/<lane>/`) | `research/README.md` |
| `docs/` | hardening program, operations runbook, executed evidence, decomposition plan, ADRs | `docs/HARDENING-PROGRAM.md`, `docs/adr/` |
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
- `crates/midwest-census/README.md`, `xtask/README.md`, `tools/README.md` — the census crate, the
  developer commands and the gate.

## Run it

```bash
# Census pipeline (store defaults to var/midwest-census; the binary takes an exclusive Fjall lock)
cargo run --release -p midwest-census --bin midwest-census -- teams   --states WI,MN --refresh
cargo run --release -p midwest-census --bin midwest-census -- collect --states WI,MN
cargo run --release -p midwest-census --bin midwest-census -- provider wiaa
cargo run --release -p midwest-census --bin midwest-census -- consolidate
cargo run --release -p midwest-census --bin midwest-census -- report --print
cargo run --release -p midwest-census --bin midwest-census -- bests
cargo run --release -p midwest-census --bin midwest-census -- workbook
cargo run --release -p midwest-census --bin midwest-census -- run

# Census reports and the workbook through the developer command wrapper
cargo xtask census-status --store var/midwest-census   # report --core
cargo xtask coverage      --store var/midwest-census   # report, every source
cargo xtask export        --store var/midwest-census   # workbook into <store>/out/
cargo xtask source-test   ks                           # one source's tests (alias: source-check)
cargo xtask bench         -- parse                     # cargo bench -p midwest-census parse

# Root acquisition pipeline / operator CLI
cargo run --release -p athletic-rust-pipeline -- worker --config config.native.toml --bind 127.0.0.1:19181
cargo run --release -p athletic-rust-pipeline -- --help

# Quality gate (integration runs it once, after lanes land; see AGENTS.md §2)
cargo xtask gate
```

## Ground rules

- Every source request goes through `crate::net::Fetcher` in `crates/midwest-census/src/net/`:
  robots.txt enforced, 2 requests/second per host, disk cache before network, bounded retries inside
  the transport only. No raw client and no bypass — see `AGENTS.md` §4.
- Production code carries no `unwrap`/`expect`/`panic!`/`unsafe`/`as` casts/indexing; the gate's
  clippy set and `cargo xtask scan` fail the build on them.
- Fixtures under `crates/midwest-census/tests/fixtures/**`, `tests/fixtures/**` and `research/**` are
  verbatim captures; tests run offline against them.
- No Python anywhere in this repository; tooling is Rust (`xtask`) plus `tools/gate.sh`.
- No new dependencies without the integration owner: manifests are integration-owned.

## Status

`README.md` records no run results. End-to-end evidence, its scope and what remains unqualified live
in `HANDOFF.md`, `docs/VERIFICATION-EVIDENCE.md` and `crates/midwest-census/README.md`; the current
hardening program and its measured gaps live in `docs/HARDENING-PROGRAM.md`. The target workspace split
described in `ARCHITECTURE.md` is a plan: of the proposed crates only `crates/census-domain` exists.
