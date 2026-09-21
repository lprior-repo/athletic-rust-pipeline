# Decomposition plan (Phase 2)

Phase 1 removed forbidden constructs; it did so by adding explicit error-handling lines, which raised
the line-count counters (root +121, census +641, `functions_over_25_logical_lines` 434 → 439). Phase 2
brings those numbers down by moving code to where it belongs — without changing a single published
behavior. The golden-corpus parity harness is the proof, and it must exist **before** any file in this
plan is touched (already the case for every file below on the day the harness landed).

## Status

Completed (each proven by its parity target, `tests/golden/` unchanged, and flat-or-falling counters):

| Unit | Result |
| --- | --- |
| `sources/plain_names.rs` 2482 | `plain_names/{mod,parse,nd,nd_coaches,nsaa,nsaa_coaches,tests}.rs` — 163–279 lines each |
| `sources/mshsl.rs` 1870 | `mshsl/{mod,parse,text,teams,map,collect,tests}.rs` — 93–261 |
| `sources/wiaa.rs` 1559 | `wiaa/{mod,collect,primitives,parse,map,tests}.rs` — 125–470 (`collect.rs` 314 holds the unchanged 292-line `collect`, the Phase 2b target) |
| `sources/ohsaa.rs` 1393 | `ohsaa/{mod,collect,parse,pages,map,tests}.rs` — 69–273 |
| `sources/athleticnet.rs` 1315 | `athleticnet/{mod,parse,map,absorb,collect,tests}.rs` — 161–298 |
| `sources/hytek.rs` 1163 | `hytek/{mod,columns,parse,map,tests}.rs` — 171–287 |
| root `profile/bio.rs` 1035 | `profile/bio/{mod,identity,teams,results,distances,record,relays,fields}.rs` — 73–204 |
| root `domain/performance_evidence/context.rs` 483 | `context/{mod,classify,mark,assemble,best,text}.rs` — 10–172 |
| root `result_verify/rankings.rs` 464 | `rankings/{mod,records,verification,tests}.rs` |
| root `bundle_verify.rs` 433 | `bundle_verify/{mod,digest,binding,pr_summary}.rs` — 22–237 |
| root `profile/html/stream.rs` 387 | `stream/{mod,buffer,bounds,structure}.rs` — 43–254 |
| root `result_verify/checks.rs` + `positive.rs` (346 + 388) | `checks/{mod,row,artifacts}` + `checks/positive/{mod,identity,acceptance}` — 98–178 |
| `sources/ihsa.rs` 1147 | `ihsa/{mod,parse,staff,map,collect,tests}.rs` — 59–498 |
| `sources/wiaa_results.rs` 1009 | `wiaa_results/{mod,archive,classify,map,parse,run,tests}.rs` — 61–300 |
| `sources/wayzata.rs` 1006 | `wayzata/{mod,parse,map,tests}.rs` — 177–342 |
| `sources/athleticlive_athletes.rs` 985 | `athleticlive_athletes/{mod,parse,tokens,map,tests}.rs` — 67–299 |
| `net.rs` 1120 | `net/{mod,client,request,execute,cache,decode,robots,tests}.rs` — 55–293 |
| `store.rs` 1061 | `store/{mod,keys,write,read,legacy,entities,tests}.rs` — 81–241 |
| root `runtime/browser/transport/rankings.rs` 734 | `rankings/{mod,results,session,capture,tests}.rs` — 11–276 |
| root `runtime/browser/transport/rankings_helper.rs` 592 | `rankings_helper/{mod,interceptor,capture,request,pagination,tests}.rs` — 12–159 |
| root `runtime/profile_worker.rs` 616 | `profile_worker/{mod,intake,parsing,state,folding,reporting}.rs` — 81–213 |
| root `runtime/source/request.rs` 467 | `request/{mod,action,body,build,tests}.rs` — 25–208 |
| root `search/parser/stream.rs` 412 | `stream/{mod,capture,row,link,bounds}.rs` — 49–124 |

Remaining targets are the 39 entries in `files_over_300_lines` from `cargo xtask scan` (down
from 62): the census core modules (`model.rs` 1327 — since moved to `crates/census-domain/src/`,
`report.rs` 894, `restate_services.rs` 862,
`workbook.rs` 844, `main.rs` 740, `census.rs` 467, `bests.rs` 414, `bootstrap.rs` 382), the remaining
source adapters (`compiled.rs` 674 / `xc.rs` 604 / `milesplit.rs` 591 / `coach_contacts.rs` 586 /
`ks.rs` 528 / `athleticlive.rs` 526 / `raceday.rs` 320), and the root `runtime/`, `store/`, `domain/`
and `workbook_*` modules.

Three rules learned during this phase, all now enforced:

* A directory split adds lines (per-file `use` blocks, `mod` declarations, re-export lists). Crate-level
  `production_lines` may therefore rise while oversized files fall. That rise is split overhead, not
  drift: it is checked against a written list before the baseline is refreshed once, at the end of a
  wave — never per pull request.
* A file may exceed 300 lines only when the single unchanged function it holds is itself longer than
  300 lines (`sources/wiaa/collect.rs` 314 holds that 292-line `collect`; `sources/athleticnet/absorb.rs`
  298 sits just above it), and those functions are the targets of the next phase, which may not be
  reached by a move.
* The two function counters measure *visible* functions: a function is invisible while a predecessor's
  brace span overshoots (a `{` or `}` inside a string or comment), and invisible while a signature
  wraps under rustfmt's 100-column rule is a different kind of visible. Splitting a file can therefore
  *raise* `functions_over_60_lines` / `functions_over_25_logical_lines` without adding or rewriting a
  single function. Measured by name against the baseline commit, that accounted for all seven crossings
  in the 62 → 39 wave (+5 logical: four functions in `runtime/browser/transport/rankings.rs` that the
  overshooting span had hidden, plus `write_cache` 23 → 28 from its wrapped `pub(super)` signature;
  +2 of those also crossed 60). Attribute by name, then refresh the baseline once.

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
cargo xtask scan | jq -c '.structure'                         # sizes: files >300 lines, functions >60 / >25 logical lines
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

`cargo xtask new-source <name>` generates this layout with the fixture-driven `#[cfg(test)]` module
inside `parse.rs`; when an adapter's tests outgrow that file, the module moves to the sibling
`tests.rs` shown above (that is the step every decomposed adapter below performs).

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
