# Decomposition plan (Phase 2)

Phase 1 removed forbidden constructs; it did so by adding explicit error-handling lines, which raised
the line-count counters (root +121, census +641, `functions_over_25_logical_lines` 434 → 439). Phase 2
brings those numbers down by moving code to where it belongs — without changing a single published
behavior. The golden-corpus parity harness is the proof, and it must exist **before** any file in this
plan is touched (already the case for every file below on the day the harness landed).

## The guarantee

Every step below is a **move**, never a rewrite:

1. No public API changes. When a module becomes a directory, `mod.rs` re-exports what used to be
   public (`pub use parse::*;` / `pub use map::*;`), so `sources::<name>::parse_*` and the
   `sources/mod.rs` dispatch keep resolving unchanged.
2. No behavior changes. The parity tests (`crates/midwest-census/tests/parity_*.rs`) compare pretty
   JSON of parsed entities against checked-in goldens; a moved line that changes an output, an
   ordering, or a default fails them.
3. Tests move verbatim. A `#[cfg(test)] mod tests` block goes into `tests.rs` unchanged; if a test
   needs a private item, it stays reachable because `tests.rs` is a child module of the same module.

## Required proof per pull request

```bash
cargo nextest run -p midwest-census                      # full suite, must stay green
cargo nextest run -p midwest-census --test parity_<group> # golden parity for the touched sources
cargo clippy -p midwest-census --lib --bins --all-features -- -D warnings \
  -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn \
  -D clippy::indexing_slicing -D clippy::string_slice -D clippy::as_conversions \
  -D clippy::arithmetic_side_effects -D clippy::let_underscore_must_use   # no new diagnostics
python3 tools/production_scan.py | jq -c '.structure'                         # sizes: files >300 lines, functions >60 / >25 logical lines
bash tools/gate.sh                                                         # all lanes
```

The debt ratchet is only satisfied when `functions_over_60` / `functions_over_25_logical_lines` and
`files_over_300_lines` go **down** or stay flat for the touched crate. Do not refresh the baseline to
absorb a rise — that is the failure mode this phase exists to avoid.

## Layout rule

An adapter with more than one responsibility becomes a directory module:

```text
crates/midwest-census/src/sources/<name>/
    mod.rs      module doc, constants, Options, re-exports, collect orchestration (+ its private phases)
    parse.rs    pure parsing: string primitives, published row shapes, parse_* functions. No I/O.
    map.rs      parsed rows -> canonical entities (schools, coaches, athletes, meets, performances)
    tests.rs    the existing #[cfg(test)] module, moved verbatim
```

`cargo xtask new-source <name>` generates exactly this layout, so new adapters start where decomposed
ones end.

## Census targets (line counts at the start of Phase 2)

| File | Lines | Split |
| --- | --- | --- |
| `sources/plain_names.rs` | 2482 | `mod.rs` (collect + shared helpers), `nd.rs` (NDHSAA parse + map), `nsaa.rs` (NSAA parse + map), `parse.rs` (primitives), `tests.rs` |
| `sources/mshsl.rs` | 1870 | `mod.rs` (collect phases), `parse.rs` (school/team/coach records), `map.rs` (schools, coaches), `tests.rs` |
| `sources/wiaa.rs` | 1559 | `mod.rs` (directory walk), `parse.rs` (index + school page), `map.rs` (school, admin, coach rows), `tests.rs` |
| `sources/ohsaa.rs` | 1393 | `mod.rs` (collect), `parse.rs` (search, sports, AD pages), `map.rs` (`school_entities`), `tests.rs` |
| `sources/athleticnet.rs` | 1315 | `mod.rs` (collect + registry walk), `parse.rs` (bio payload), `map.rs` (canonical mapping of seasons/meets/results), `tests.rs` |
| `sources/hytek.rs` | 1163 | `mod.rs` (collect), `parse.rs` (line/field formats), `map.rs` (mark + meet mapping), `tests.rs` |
| `sources/ihsa.rs` | 1147 | `mod.rs` (collect), `parse.rs` (staff/coach/title parsing), `map.rs` (school + coach entities), `tests.rs` |
| `sources/wiaa_results.rs` | 1009 | `mod.rs` (archive walk + PDF pipe), `parse.rs` (artifact + finish-list parsing), `map.rs` (events/performances), `tests.rs` |
| `sources/wayzata.rs` | 1006 | `mod.rs` (collect + journal), `parse.rs` (schedule tables), `map.rs` (meet minting), `tests.rs` |
| `sources/athleticlive_athletes.rs` | 985 | `mod.rs` (collect + batching), `parse.rs` (search hits, grade tokens), `map.rs` (athlete entities + Athletic.net seeds), `tests.rs` |

Smaller files (`compiled.rs` 674, `xc.rs` 604, `milesplit.rs` 591, `coach_contacts.rs` 586, `ks.rs` 528,
`athleticlive.rs` 526, `raceday.rs` 320) are already under the 300-line production budget once tests are
accounted separately — check each with the audit script before deciding they need a split.

## Root-crate targets

The baseline's `files_over_300_lines` list is the authority for the root crate; the largest are
`src/profile/bio.rs` (1035), `src/domain/performance_evidence/context.rs` (483),
`src/result_verify/rankings.rs` (464), `src/bundle_verify.rs` (433), `src/profile/html/stream.rs` (387),
`src/result_verify/positive.rs` (388), `src/domain/decision.rs` (381), `src/domain/marks/event.rs` (351),
`src/result_verify/checks.rs` (346), `src/cli.rs` (329). Root tests (`cargo nextest run -p
athletic-rust-pipeline`, 241 tests) plus the workbook/XLSX byte-parity tests are the proof there.

## Order of work

1. Landing the parity harness (done first — it is what makes the rest safe).
2. Census adapters, largest first, one agent per file, in the order of the table (biggest first).
3. Root crate: `profile/bio.rs`, then the `result_verify` family, then `cli.rs` and the runtime workers.
4. Re-run the full gate; the ratchet must show every size counter flat or down.

## Non-goals

- No new abstractions, traits, or indirection "for uniformity" — the split is by existing
  responsibility, and a module that is already one responsibility stays a file.
- No renaming of public functions, no changed error messages, no reordered fields.
- No behavioral "improvements" while moving code. If a defect is found mid-move, stop, report it, and
  fix it as its own change with its own evidence.
