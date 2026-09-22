# `tools/` — the quality gate

`tools/gate.sh` is the repository's one quality gate: full-strength lints, no allow-lists, no
weakened checks. Existing debt is recorded in `tools/quality-baseline.json` and may only shrink.

```bash
tools/gate.sh                     # every lane, then compare the measurements with the baseline
tools/gate.sh --full              # ... plus the mutation-testing lane (pre-release only, it is slow)
tools/gate.sh --update-baseline   # rewrite the baseline from current measurements
tools/gate.sh --update-baseline --allow-increase   # ... accepting a number that grew
cargo xtask gate                  # the same script, through the developer command
```

The script exits `0` on `gate: PASS` and `1` on `gate: FAIL -> <lanes>`; the summary names every
lane that failed. Every lane's output is printed as it runs, so a failure is readable without a
re-run.

The check and strict-clippy lanes run through the pinned nightly with
`-Zallow-features=portable_simd,try_blocks`. That is the source-policy allowlist: a `#![feature(..)]`
outside it fails the gate, and on a stable toolchain the two lanes fail on the `-Z` flag rather than
silently dropping the check.

## Lanes

| Lane | Command | Fails when |
| --- | --- | --- |
| fmt | `cargo fmt --all -- --check` | any file is not rustfmt-clean |
| check | `cargo -Zallow-features=portable_simd,try_blocks check --workspace --all-targets --all-features` | anything does not compile, tests and examples included |
| doc | `cargo doc --workspace --all-features --no-deps` | a doc comment breaks `rustdoc` |
| tests | `cargo nextest run --workspace --all-features`, else `cargo test --workspace --all-features --quiet` | a test fails |
| strict clippy | `cargo -Zallow-features=portable_simd,try_blocks clippy --workspace --lib --bins --examples --all-features -- <LINT_SET>` | (measurement lane, not a pass/fail lane: its tallies feed the ratchet) |
| production scan | `cargo xtask scan` | the scan cannot run; the numbers themselves are ratcheted below |
| domain type integrity | `cargo xtask integrity` | (measurement lane: review candidates, ratcheted in the DDD phase) |
| domain purity | `cargo xtask domain-purity` | a banned async/I/O package is in the `census-domain` normal tree |
| baseline update | `cargo xtask quality-baseline` | only with `--update-baseline`; refuses to raise a number without `--allow-increase` |
| debt ratchet | `cargo xtask ratchet` | any metric grew against the baseline |
| deny | `cargo deny check` | a dependency policy violation (needs `cargo-deny`; a missing tool fails this lane) |
| audit | `cargo audit --quiet` | an advisory covers a locked crate (prints `SKIP` when `cargo-audit` is absent) |
| vet | `cargo vet --locked` | a locked crate is neither audited nor exempted in `supply-chain/` (prints `SKIP` when `cargo-vet` is absent) |
| machete | `cargo machete` | an unused dependency is declared (prints `SKIP` when `cargo-machete` is absent) |
| geiger | `cargo geiger --workspace --all-features --output-format Json` | unsafe code appears (prints `SKIP` when `cargo-geiger` is absent) |
| feature powerset | `cargo hack check --workspace --feature-powerset` | a feature combination does not compile (prints `SKIP` when `cargo-hack` is absent) |
| bench presence | `cargo bench --workspace --no-run` | a benchmark target exists and does not build; with no `benches/` yet it prints why and passes |
| mutants | `cargo mutants --workspace --in-place` | a mutant survives the test suite (only with `--full`; prints `SKIP` when `cargo-mutants` is absent) |

## Supply chain

`supply-chain/` is the `cargo-vet` ledger: `config.toml` holds the exemptions the tree was
initialized with, `audits.toml` the audits, `imports.lock` the peer imports. The vet lane runs
`--locked`, so it answers from that ledger and never fetches new imports mid-gate; a dependency
added without an exemption or audit fails the lane and names itself.

## The debt baseline

`tools/quality-baseline.json` has a fixed shape — `note`, `clippy` (keyed `crate<TAB>lint`), `scan`
(keyed by crate) and `structure` — and is read by `cargo xtask ratchet` on every gate run. It is a
ratchet: `--update-baseline` refuses to raise any number unless `--allow-increase` says the increase
is deliberate, and the ratchet fails the gate on any metric that grew, printing every change as
`[DOWN]` or `[UP]`.

The measurements it records:

* **clippy** — one entry per `crate<TAB>lint` on the source targets, so burndown is visible per
  lint code.
* **scan** — per crate: `expect`, `unwrap`, `unsafe`, `panic`, `assert_family`, `indexing`,
  `as_cast`, `todo`, `unreachable`, `dbg`, plus `production_lines` and `files`. Production-reachable
  means not after a `#[cfg(test)]` that opens a module and not inside test files.
* **structure** — `files_over_300_lines` (one `crate:path (lines)` entry per oversized file),
  `functions_over_60_lines` and `functions_over_25_logical_lines`.

## Where the analysis lives

None of the analysis is a script in this directory: it is `xtask` subcommands, so the numbers the
gate prints are the same numbers `cargo xtask <command>` prints by hand.

| Subcommand | Emits |
| --- | --- |
| `cargo xtask scan` | the forbidden-construct and size-budget report, as JSON on stdout |
| `cargo xtask integrity` | the domain type-integrity candidates, as JSON on stdout |
| `cargo xtask domain-purity` | the `census-domain` normal dependency tree; fails on a banned package |
| `cargo xtask quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]` | rewrites the baseline |
| `cargo xtask ratchet <baseline> <clippy.tsv> <scan.json>` | compares measurements with the baseline |

The scan covers `src/` and `crates/midwest-census/src` — the crates the baseline records. Adding a
crate to the scan is a baseline change (`tools/gate.sh --update-baseline`), not a scan change: a
crate with no baseline entry reads as new debt for every count it contributes.

## `tools/athletic-net-pr-population/`

Archived provenance for the 2026-09-20 Athletic.net corpus build, not a lane of the gate. See its
own README.
