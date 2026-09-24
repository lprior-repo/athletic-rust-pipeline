# `tools/` — the quality gate

`tools/gate.sh` is the repository's one quality gate: full-strength lints, no allow-lists, no
weakened checks. Existing debt is recorded in `tools/quality-baseline.json` and may only shrink.

```bash
tools/gate.sh                     # every lane, then compare the measurements with the baseline
tools/gate.sh --full              # ... plus the mutation-testing lane (pre-release only, it is slow)
tools/gate.sh --release           # the pre-release pass: every lane, every tool required, heavy lanes on
tools/gate.sh --update-baseline   # rewrite the baseline from current measurements
tools/gate.sh --update-baseline --allow-increase   # ... accepting a number that grew
cargo xtask gate                  # the same script, through the developer command
```

The script exits `0` on `gate: PASS` and `1` on `gate: FAIL -> <lanes>`; the summary names every
lane that failed. Every lane's output is printed as it runs, so a failure is readable without a
re-run.

Two modes, and the difference is what a missing tool means:

* **dev** (default) — the pass for an edit. A tool that is not installed makes its lane print `SKIP`
  and return success, so a laptop without `cargo-audit` still runs everything else. `--full` adds the
  `mutants` lane.
* **`--release`** — the pass a release is signed off on. It implies `--full`, and a missing tool is a
  **failure**: a `SKIP` there would be a claim that a lane's coverage was not needed, and a release
  Install the whole set (`cargo-audit`, `cargo-vet`, `cargo-machete`,
  `cargo-geiger`, `cargo-hack`, `cargo-mutants`, `cargo-deny`) before invoking it. `cargo-deny` is never skipped in
  either mode: its scorecard is a pass/fail lane, not an advisory one.

The check and strict-clippy lanes run through the pinned nightly with
`-Zallow-features=portable_simd,try_blocks,proc_macro_span,error_generic_member_access`. That is the source-policy allowlist: a `#![feature(..)]`
outside it fails the gate, and on a stable toolchain the two lanes fail on the `-Z` flag rather than
silently dropping the check.

## Lanes

| Lane | Command | Fails when |
| --- | --- | --- |
| fmt | `cargo fmt --all -- --check` | any file is not rustfmt-clean |
| check | `cargo -Zallow-features=portable_simd,try_blocks,proc_macro_span,error_generic_member_access check --workspace --all-targets --all-features` | anything does not compile, tests and examples included |
| doc | `cargo doc --workspace --all-features --no-deps` | a doc comment breaks `rustdoc` |
| tests | `cargo nextest run --workspace --all-features`, else `cargo test --workspace --all-features --quiet` | a test fails |
| strict clippy | `cargo -Zallow-features=portable_simd,try_blocks,proc_macro_span,error_generic_member_access clippy --workspace --lib --bins --examples --all-features -- <LINT_SET>` | (measurement lane, not a pass/fail lane: its tallies feed the ratchet) |
| production scan | `cargo xtask scan` | the scan cannot run; the numbers themselves are ratcheted below |
| domain type integrity | `cargo xtask integrity` | (measurement lane: review candidates, ratcheted in the DDD phase) |
| domain purity | `cargo xtask domain-purity` | a banned async/I/O package is in the `census-domain` normal tree |
| module seams | `cargo xtask seams` | a top-level module re-exports a symbol that should be private (the gate prints the violation set on FAIL) |
| baseline update | `cargo xtask quality-baseline` | only with `--update-baseline`; refuses to raise a number without `--allow-increase` |
| debt ratchet | `cargo xtask ratchet` | any metric grew against the baseline |
| deny | `cargo deny check` | a dependency policy violation (needs `cargo-deny`; a missing tool fails this lane) |
| audit | `cargo audit --quiet` | an advisory covers a locked crate (a missing `cargo-audit` SKIPs in dev and fails `--release`) |
| vet | `cargo vet --locked` | a locked crate is neither audited nor exempted in `supply-chain/` (a missing `cargo-vet` SKIPs in dev and fails `--release`) |
| machete | `$(command -v cargo-machete) .` | an unused dependency is declared (a missing `cargo-machete` SKIPs in dev and fails `--release`; plain `cargo machete` fails through a mise shim) |
| geiger | `(cd crates/census-service && cargo geiger --all-features --output-format Json > /dev/null)` | unsafe code appears (a missing `cargo-geiger` SKIPs in dev and fails `--release`; the workspace form fails because geiger rejects a virtual manifest) |
| feature powerset | `cargo hack check --workspace --feature-powerset` | a feature combination does not compile (a missing `cargo-hack` SKIPs in dev and fails `--release`) |
| bench presence | `cargo bench --workspace --no-run` | a benchmark target exists and does not build (the `if [ -d benches ] || [ -d crates/census-service/benches ]` branch always runs; the else-branch was removed when root `benches/` was deleted) |
| mutants | `cargo mutants --workspace --in-place` | a mutant survives the test suite (only with `--full`, which `--release` implies; a missing `cargo-mutants` SKIPs in dev and fails `--release`) |

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
| `cargo xtask contract` | the eight architectural constants agents rely on (census scope, the source transports, handler ceilings, Python artifacts, descriptor admission, scanned-package parity, the front-door documents, every sources/ module is a registered source or listed reader and vice versa); one line per violated check |
| `cargo xtask quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]` | rewrites the baseline |
| `cargo xtask ratchet <baseline> <clippy.tsv> <scan.json>` | compares measurements with the baseline |

The scan covers **every workspace member**, not a hand-kept list: it asks `cargo metadata --no-deps`
for the packages, prints the list it covered, and reads each member's production roots (`src`, and
`benches`, `kani`, `fuzz/fuzz_targets` where they exist). Adding a member cannot escape it —
`cargo xtask contract` check 6 fails when the scanned set and the workspace's own set differ — and a
new member is therefore a baseline change (`tools/gate.sh --update-baseline`), because a crate with no
baseline entry reads as new debt for every count it contributes.

The size budgets apply to every scanned member too: a crate that enters the scan with files over 300
lines reports them as `files_over_300_lines` entries, and the ratchet holds them. That is debt, not a
number to record — `--update-baseline` is not the way to make a red structure lane green.

## `tools/athletic-net-pr-population/`

Archived provenance for the 2026-09-20 Athletic.net corpus build, not a lane of the gate. See its
own README.
