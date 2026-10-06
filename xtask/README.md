# xtask — developer command reference

Moon is the **only** repository developer entrypoint. From the repository root, run
`env -u CI tools/moon-local run pipeline:xtask -- <command> [args]`.
Child commands print their exact argument vector and propagate failure. Measurement commands emit
their own reports. Use `env -u CI tools/moon-local run pipeline:xtask -- --help` for the current CLI.
Do not invoke Cargo, bare Moon, `tools/gate.sh` or fault/backup wrappers directly.

## Commands

| Command | Current behavior |
|---|---|
| `gate [-- <args>]` | Invokes `tools/gate.sh`; gate policy and lane details live in [tools/gate.sh](../tools/gate.sh) |
| `scan` | Production forbidden-construct and size measurements for workspace members, discovered with Cargo metadata; JSON |
| `comments` | Lexical no-comments check over project Rust, including tests/examples/benches/fuzz; strings are data |
| `panic-extraction [--root <dir>]` | Fatal owned-Rust lexical extraction and suppression check, including cfg-disabled proofs and three rendered source templates; captured literals are data |
| `contract` | Nine architecture checks, including scope, transports, retry ceilings, Python artifacts, admission, scan parity, front-door documents, adapter registration and source reachability |
| `seams` | Checks both module and sibling-crate allowed-edge tables; JSON and failing status on violation |
| `integrity` | Domain type-integrity review candidates; JSON, not proof that types enforce their contracts |
| `domain-purity` | Checks `census-domain` normal dependency tree for banned runtime/I/O dependencies |
| `quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]` | Updates debt measurements; increases require the explicit flag and owner-authorized policy change |
| `ratchet <baseline> <clippy.tsv> <scan.json>` | Fails on growing measured debt; emits `[DOWN]`/`[UP]` changes |
| `source-test <source>` / `source-check <source>` | Runs the matching tests across the whole workspace with all features (`--workspace --all-features`), selecting `test(<source>)` in Nextest or the same substring in the Cargo fallback; an empty selection fails the lane |
| `source-tests` | Colocated workspace source tests (`--lib --bins --examples`); Nextest, or Cargo test fallback; excludes integration binaries |
| `source-fixture <source>` | Lists captured files beneath `crates/census-crawl/tests/fixtures/<source>/`; missing directory fails |
| `replay <name>` | Offline supported fixture replay through published parse paths; unsupported/empty inputs fail |
| `census-status` | Serving `Census/status`, or offline `report --core`; see routing below |
| `coverage` | Serving `Report/run`, or offline all-source `report` |
| `export` | Serving `Workbook/run`, or offline `workbook`; accepts `--out`, `--grad-year`, `--core`, `--limit` |
| `bench [-- <args>]` | Internal service-only benchmark forwarding; use `pipeline:bench` for the direct Moon benchmark lane |
| `perf record`, `perf check`, `perf profile <group>` | Record/compare/profile configured benchmarks; each lane's own reports own interpretation and limits |
| `dump-sheet <workbook> <sheets>...` | Prints nonempty worksheet rows as `column=value` fields |
| `new-source <name>` | Writes a source scaffold and module declaration; not a qualified or fully registered adapter |
| `storage-ab [-- <args>]` | Manual census-store keyspace A/B benchmark: single vs split evidence/derived keyspaces; run with small sizes first (e.g. `--evidence-rows 20000 --derived-rows 40000 --generations 2`) before production-scale runs |

`env -u CI tools/moon-local run pipeline:gate -- --full` and `--release` invoke the unfiltered mandatory proof lane.
A passing explicitly selected harness is not coverage of missing kernels. Release also fails when
Cargo metadata contains no benchmark target; compiling a benchmark is not a measured performance
result. See the dated evidence for actually executed lanes and remaining blockers.

## Fast Rust iteration and local Moon cache

Run all developer commands through `tools/moon-local` from the repository root. Cargo is an internal
implementation, not a second supported workflow. The opt-in iteration profile and Moon CI do not
replace the full gate: `env -u CI tools/moon-local run pipeline:gate -- --release` is the release
acceptance entrypoint, including proof, policy and performance obligations; the whole-workspace
mutation lane was retired by owner decision (bead `5rtr`). A scoped report
test, cached task result, successful build or benchmark scaffold is not national-census or
native-recovery acceptance. Manual tasks are excluded from automatic CI; `env -u CI` is required
when any `CI` value is present, including `CI=false`.

### Prerequisites and compiler iteration

Use the installed **Moon 2.2.4**, the pinned `nightly-2026-04-27` toolchain and its checked-in
components/target via `rustup`, Cargo Nextest, Bash, `jq`, GNU coreutils, and the system C/C++
compilers, archiver and linker. The parent launcher also fingerprints the pinned toolchain's bundled
LLD. It selects Cargo via `rustup which cargo` at the repository root and supplies that executable to
`tools/moon-cargo`, keeping `rust-toolchain.toml` authoritative rather than duplicating its nightly
selection in task commands. The offline build/test tasks use `--locked --offline`; populate locked
dependencies with `env -u CI tools/moon-local run pipeline:fetch` when needed.
These tasks do not install tools, change global Cargo configuration or manage daemons.

For a real scoped report-library test run without Moon task-result replay:

```sh
tools/moon-local run pipeline:report-test --force
```

Internally the Nextest task places `--config` after `nextest run`. The opt-in
`.cargo/fast-iteration.toml` uses test-profile line tables, incremental compilation, no third-party
debug info, and `-Z threads=4` for `x86_64-unknown-linux-gnu`. It does not weaken assertions or change
the repository's normal release profile. Keep `target/fast-iteration` state separate from release
artifacts. Tasks clear the compiler wrapper; sccache is not required for this lane.

Cargo can select `cargo-nextest` from `${CARGO_HOME:-$HOME/.cargo}/bin` before the executable visible
on `PATH`. In the measured snapshot Cargo selected Nextest 0.9.133 from `/cache/cargo-shared/bin`
even though a direct PATH invocation reported 0.9.137. Record the actual Cargo-selected tool, not
only `cargo-nextest --version`.

### Parent launcher and task boundaries

Always enter Moon through `tools/moon-local`, including CI. Before Moon can look up a cached task,
the parent captures actual tool executable identities, relevant environment/configuration and the
source root in `tools/build-toolchain.json`. A fingerprint task inside Moon would run too late to
invalidate an already-hit task. Bare `moon` bypasses this protection. The identity is source-root
bound; it is not a guarantee of relocatable cache reuse between worktrees.

The workspace is one Moon project, `pipeline`, with accuracy-mode glob hashing that includes
vendored Rust and captured fixtures. It uses the **existing** bazel-remote endpoint
`grpc://127.0.0.1:9092`, `zstd`, integrity verification, and instance `athletic-rust-pipeline`.
Do not start, reset or restart a cache daemon as part of this workflow.

Moon 2.2.4 requires `pipeline.syncWorkspace: true` for remote activation. The launcher sets
`MOON_SKIP_SYNC_WORKSPACE=true` to suppress workspace mutation while preserving that activation.
Keep dependency installation and project synchronization disabled. Do not use `--no-actions`:
removing workspace actions can also remove remote-cache activation.

```sh
tools/moon-local run pipeline:report-test
tools/moon-local run pipeline:tests
```

Tasks use direct process invocation, not shell command concatenation. Extra Nextest filter arguments
remain literal, including parentheses:

```sh
tools/moon-local run pipeline:tests -- -E 'binary(restate_kill_restart)'
```

| Task | Execution and cache boundary |
|---|---|
| `pipeline:report-test` | Cacheable scoped `census-report` library results, all features; a hit may replay prior logs instead of executing tests |
| `pipeline:tests` | Full workspace/all-features Nextest, 16 test threads, `cache: false`; executes tests rather than replaying a native-fault result |
| `pipeline:check` | Uncached workspace/all-targets/all-features compilation |
| `pipeline:lint-src` | Uncached workspace source Clippy (`--lib --bins --examples`, warnings denied); not test-style linting |
| `pipeline:fmt` | Uncached workspace formatting check |
| `pipeline:build-portable` / `pipeline:build-native` | Manual, cacheable fully optimized binaries; separate portable/native artifact roots |
| `pipeline:build` | Manual uncached general build, including explicit feature/example builds; `target/moon-build` |
| `pipeline:xtask` | Manual uncached developer verbs; arguments after Moon's `--` go to xtask |
| `pipeline:gate` | Manual uncached original quality gate; `-- --release` retains every release obligation |
| `pipeline:bench` | Manual uncached optimized service benchmarks; `-- --bench <target> -- <Criterion-args>` |
| `pipeline:fetch` | Manual uncached locked dependency fetch; network access unless `-- --offline` is supplied |
| `pipeline:fmt-write` | Manual uncached source formatting; run only when intentionally formatting owned changes |
| `pipeline:g1-audit` | Manual uncached retained search audit |
| `pipeline:durability` / `pipeline:backup-drill` | Manual uncached existing fault/backup wrappers, bound to portable Moon binaries; require owned scratch/stopped stores |

The full test task is deliberately uncached because native fault/recovery checks observe processes,
ports and durable state not represented by source hashes. Running it does not by itself certify all
separately required native fault scenarios. A report-library cache hit cannot stand in for this lane.

For fresh scoped execution rather than cached-log replay, use:

```sh
tools/moon-local run pipeline:report-test --force
```

`--force` forces task execution; Cargo may still reuse local compilation state. It is not a
cold-compiler benchmark. Inspect Moon's task status/report and cache-origin logs to distinguish
execution from local or remote hydration; replayed passing test output is not a newly executed test.
A compiler-cache miss and a Moon task hit are different events: Moon can hydrate a completed task
without invoking Cargo or rustc at all, while a task miss may compile using Cargo's existing state.

### Fully optimized portable binaries and CI

For a manual local portable build, explicitly remove `CI`:

```sh
env -u CI tools/moon-local run pipeline:build-portable
```

This task explicitly builds `census-service` and `census-serve` for
`x86_64-unknown-linux-gnu` in release mode with **opt-level 3, thin LTO, one codegen unit,
incremental disabled and generic CPU code generation**. The task sets `CARGO_ENCODED_RUSTFLAGS=''`,
which takes precedence over ambient `RUSTFLAGS` (including native CPU flags), and passes explicit
`--target x86_64-unknown-linux-gnu`. Normal release unwinding remains enabled; iteration debug
settings do not bleed into the build. `Cargo.toml`'s release defaults remain unchanged (opt-level 3
and unwind by Cargo default, thin LTO and one codegen unit), as does `.cargo/config.toml`.
Do not introduce global rustflags or a `target-cpu=native` default.

Only these two final binaries are declared cached outputs, not the entire Cargo target tree:

```text
target/moon-portable/x86_64-unknown-linux-gnu/release/census-service
target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve
```

`build-portable` has `runInCI: false`. In Moon 2.2.4, even `CI=false` still marks the environment as
CI and can skip that task; `env -u CI` is intentional. Add `--force` to the manual command when the
build must execute instead of hydrating an artifact. A build or cache hydration is not a runtime
performance measurement.

The canonical Moon CI entry is:

```sh
tools/moon-local ci --force --summary detailed
```

Use this forced form when affected-history comparison is unavailable or shallow; do not accept a
zero-affected-target run as verification. It runs the configured CI prerequisites and uncached full
tests, not the manual portable build. `moon run :ci` is not this entry point. The Moon iteration/CI
lanes remain narrower than `env -u CI tools/moon-local run pipeline:gate -- --release`.

### Measured scope, not a speed guarantee

The isolated `athletic-build-bench-20261004-083946/source` snapshot produced these medians:

| Snapshot workload | Original | Fast iteration | Ratio |
|---|---:|---:|---:|
| Cold Cargo state, dependency downloads warm | 74.510 s | 40.2155 s | 1.85× |
| Actual CSV predicate body edit; all workspace test binaries built, 198 report tests executed | 14.164 s | 7.386 s | 1.92× |
| Full workspace test suite | 130.158 s | 33.916 s | 3.84× |

These are snapshot measurements, not measurements of the current main tree or a promise of 2×
compile/edit speed. The snapshot full-suite comparison had the same 2,340 executed tests, 2,338
passing, two then-existing failures and three ignored; do not transfer those failures or counts to
main.

The runtime sweep evaluated five profiles across 16 workloads and three interleaved rounds
(240 measurements). Median-per-workload, equal-weight geometric-mean throughput ratios against
the portable release were: native/thin **0.976×**, native/fat **0.993×**, portable/16 codegen units
**0.952×**, and opt-level-2/no-LTO iteration release **0.778×**. The existing portable opt-level-3,
thin-LTO, one-codegen-unit profile remains the selected final build. The reduced-optimization
profile was not installed as a shipping profile. These are captured/synthetic workload results,
not national-census runtime certification or a claim that portable wins every individual case.

Native CPU builds remain explicitly opt-in and must not overwrite portable outputs:

```sh
env -u CI tools/moon-local run pipeline:build-native
```

The native task retains O3/thin-LTO/one-codegen-unit optimization, explicitly selects
`target-cpu=native`, and writes only under `target/moon-native/x86_64-unknown-linux-gnu/release/`.
It is not the default or a claim of better runtime throughput on every workload.

The dated [verification ledger](../docs/VERIFICATION-EVIDENCE.md) owns exact commands, raw evidence,
current main/cache/runtime results and their limits; benchmark targets alone establish none of them.


## Serving versus offline routing

`census-status`, `coverage` and `export` default to the serving census. `--ingress [<origin>]` uses
loopback HTTP, default `http://127.0.0.1:18095/`, without credentials or a path. `--store <dir>` selects
in-process offline execution and cannot be combined with `--ingress`.

```sh
env -u CI tools/moon-local run pipeline:xtask -- census-status
env -u CI tools/moon-local run pipeline:xtask -- coverage --ingress http://127.0.0.1:18095/
env -u CI tools/moon-local run pipeline:xtask -- export --ingress --out out/publication --grad-year 2027
env -u CI tools/moon-local run pipeline:xtask -- census-status --store var/census-service
```

The last form requires the serving store owner to be stopped. A second-process Fjall lock error is
correct; do not bypass it. Serving status is the handler's count view, whereas offline status builds
a Core report; do not claim identical semantic populations merely because labels match. Export
reads existing evidence and does not acquire missing history. `--out` names a publication directory;
the XLSX is `<publication>/current/workbook.xlsx`. See [operations](../docs/OPERATIONS.md) for
lifecycle, complete frozen-bundle verification and current certification limits.

## Measurement and fixture boundaries

`storage-ab` is a manual measurement task that compares two physical layouts: arm A stores all
evidence and derived data in a single `entities` keyspace, while arm B splits them into separate
`evidence` and `derived` keyspaces. The task generates deterministic synthetic workloads, measures
insert, rebuild, delete and point-read latencies plus disk footprint and peak RSS for both arms, and
prints tab-separated metrics for direct comparison. It does not modify production code paths or
declare a winner; the results gate a future architecture decision to split keyspaces only when
measured advantage justifies the change. Run it with small sizes first to verify the harness, then
scale up to production-relevant row counts before drawing conclusions.

`scan` obtains members from Cargo metadata rather than a two-package list. Test-only regions and
recognized test files are excluded from production measurements. `seams` is a separate structural
check; a clean scan is not acceptance of identity, durability or coverage.

`comments` and `panic-extraction` share the in-repository Rust lexer and bounded source walker.
`comments` rejects prose `doc = ...` attributes and permits non-prose attributes such as
`doc(hidden)`. Ordinary/raw/byte/C strings and captured comment-shaped bytes are data. Identifier
boundaries use Unicode XID rules. Captured corpus parity is finite evidence, not compiler-wide
lexical equivalence.

`panic-extraction` rejects calls and method/associated references to `unwrap`, `unwrap_err`,
`unwrap_unchecked`, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, `expect` and `expect_err`,
including raw identifiers, spacing, comments and cfg-disabled code. Non-panicking default helpers
are included in the source prohibition; use explicit matches or equivalent map combinators while
preserving the existing eager/lazy missing-value policy. It rejects owned lint suppressions for
the Clippy extraction checks and broad `warnings`, `clippy::all` or `clippy::restriction`
suppression. The generator check renders the adapter, parser and mapper
templates rather than treating captured string contents as source. The gate also runs
all-target/all-feature Clippy with extraction checks and warnings denied. Workspace lint levels
use `deny`, not `forbid`: external Clap derives emit their own blanket restriction attributes;
this does not permit project-owned suppression or exclude owned tests from the lexical gate.

Both commands exclude `.git`, `.jj`, `target`, `var`, `vendor` and `node_modules` below the chosen
root. Admission limits are 1,000,000 visited entries (including excluded and non-Rust entries),
100,000 selected Rust files, 128 simultaneous directory frames and 128 root path components.
Selected paths are admitted before collection growth and sorted only after bounded traversal.
Unreadable/invalid source, unterminated literals or block comments, exhausted budgets and an
empty source tree fail closed. Static root/ancestor/file/directory symlinks are refused outside
excluded subtrees; selected `.rs` files must be regular files. Reads recheck regular-file metadata,
use Linux no-follow/nonblocking open flags and a reused fallibly reserved 4-MiB-plus-one buffer.
The per-file source limit remains 4 MiB. Pathname-based filesystem operations do not provide a
snapshot against concurrent directory substitution, wall-clock filesystem bounds or a bound on
aggregate pathname bytes.

`replay` does not fetch, open a store or consult the live source clock. It needs the committed fixture
bytes and associated format/year metadata. A successful replay establishes those captures only.
`source-test` routes across the whole workspace with all features, so a source's crawl-crate adapter
tests run beside any service tests; the lane fails when the selection is empty, which is what keeps an
unmatched or misnamed source from reading as qualification.

`replay coach_directories` qualifies exactly five required root response captures. Provenance,
historical golden outputs and the separately qualified survey/probe subtree remain inventoried
by `source-fixture` but are not response inputs for this lane. Directory/summary wire extents
retain their historical records; current Census coach contexts use the shared
`coach_directories__census-contexts.json` oracle (sixteen distinct NC contexts and zero IN contexts).
Missing response/oracle files fail rather than silently reducing qualification. This does not
certify the entire survey corpus, live accessibility, current tenure or a national census.

`source-test`/`source-check` select test **function names**, not modules or files: the slug is passed
to nextest's `test(<slug>)` predicate, or to Cargo's substring filter when nextest is absent, and both
match any test whose full name contains it. Name the
functions after the source, hyphens as underscores — `nces_*`, `state_ed_*`, `tssaa_*`,
`private_assoc_*` — and call the lane with the underscore spelling
(`env -u CI tools/moon-local run pipeline:xtask -- source-test state_ed`).
A source whose tests are named otherwise reports an empty lane while the reader still has
tests; fix the names rather than widening the filter.

The service declares three independent Criterion targets:

| Target | Current timed workloads |
|---|---|
| `core` | 11 cases: five captured parsers, archive classification, label normalization/resolution, school-index construction, and school/coach store scans |
| `pipeline` | Four cases: captured WIAA result parsing, label resolution against a 4,900-school index, and scan/consolidation of 20,000 observations representing 5,000 performances |
| `snapshot` | One case, `pipeline/snapshot/athlete_evidence`: frozen Fjall readback of 20,000 deterministic provider-owned athletes |

The `snapshot` dataset retains a supported Class-of-2027 observation and a second unsupported
graduation inference per athlete. Setup and exact evidence/confidence checks precede timing; every
timed iteration reads the frozen snapshot. This synthetic workload measures readback, not census
discovery or native recovery, and belongs to `snapshot`, not `pipeline`.

Run all three targets on a quiet machine:

```sh
env -u CI tools/moon-local run pipeline:bench -- --bench core -- --noplot
env -u CI tools/moon-local run pipeline:bench -- --bench pipeline -- --noplot
env -u CI tools/moon-local run pipeline:bench -- --bench snapshot -- --noplot
```

Retain Criterion units, sample/warmup settings, corpus identity and profiler context. When invoking
a compiled Criterion binary manually, pass `--bench` and set `CRITERION_HOME` to the intended report
directory, as `xtask/src/perf/bench/runtime.rs` does; otherwise the invocation/output is not
equivalent to the measured lane. `pipeline:bench -- --no-run` only compiles; it is not a runtime measurement.

The historical `tools/perf-baseline.json` tags its fifteen throughput measurements `Elements`,
as evidenced by the benchmark sources at its recorded commit. Its metadata, workload names and
numbers are unchanged; the original untagged JSON is retained with the dated
[verification evidence](../docs/VERIFICATION-EVIDENCE.md). Its four legacy pipeline entries do not
describe the separate snapshot-readback workload. Match target, workload, corpus and revision before
comparing current results: unit-format migration does not approve a replacement workload or a new
acceptance baseline, and the preserved corpus/workload mismatch must not be hidden by resetting it.

## Source scaffolding

Hyphens normalize to underscores; invalid identifiers, keywords and existing module/fixture paths
are refused. The scaffold creates `mod.rs`, `parse.rs`, `map.rs` and an adapter README under
`crates/census-crawl/src/<name>/`, plus a fixture README, then appends the module declaration to
`crates/census-crawl/src/lib.rs`. Creation/registration errors report partial files; they are not a
transactional rollback.

The generated collector/parser/map are incomplete authoring aids. Replace them with real behavior,
captures and assertions; complete registry/applicability/durable dispatch separately under the
registry's own descriptor table (`crates/census-crawl/src/registry/table/`). Scaffolding does not run formatting, gates,
source qualification or national coverage acceptance.

## Retained search audit

The auxiliary `g1-audit` binary lives at `xtask/src/g1/main.rs`:
`env -u CI tools/moon-local run pipeline:g1-audit -- --help`.
It inventories retained search response bodies offline; it is not a production census stage.
The [2026-09-22 capture audit](../research/G1-LIVE-SEARCH-CONTRACT.md) owns its historical commands
and findings.
