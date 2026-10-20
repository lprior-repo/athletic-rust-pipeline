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
| `contract` | Ten architecture checks, including scope, transports, retry ceilings, Python artifacts, admission, scan parity, front-door documents, adapter registration, source reachability and crate boundaries |
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
| `review-matrix [--out <file>]` | Reads the applicability registry, the jurisdiction scope and the three arm tables out of the source tree and prints the family/jurisdiction matrix as JSON; deterministic, and it fails closed on an unresolved constant reference or a parsed entry count that disagrees with the declared array length |
| `review-readback --bundle <dir> [--out <file>]` | Re-reads a published bundle without the writer's code: recomputes every manifest artifact hash and length, reads the workbook with the independent XLSX reader for per-sheet used-range row and non-empty cell counts plus a content digest (`ref=value` records joined by `|`, trailing newline; artifact SHA-256 is the manifest equality check), and counts sidecar records; non-zero exit on any missing or mismatched artifact |
| `review-build --dir <dir> --run <dir> [--reviewed-at <rfc3339>]` | Assembles `census-completeness-review.json` and `census-completeness-review.md` from the review directory's curated inputs (`review_meta.json`, `gates.json`, `findings.json`, `prior_delta.json`, `limits.json`, the matrix at `matrix/static-matrix.rust.json` when present else `matrix/static-matrix.json` with the producing path recorded as `source_matrix.provenance`, optional `readback.json`, `open-work-summary.json`) and the run's retained evidence; with `--reviewed-at` omitted it records the run's own submit instant and reads no clock |
| `review-watch --run <dir> --dir <dir> [--once] [--interval-secs <n>]` | Samples `Census/open_work` from the running deployment into `open-work-latest.json` and `open-work-summary.json` and appends one compact block per sample to the run's `progress.log`; the invocation-status admin query is best-effort and needs the admin origin |

`env -u CI tools/moon-local run pipeline:gate -- --full` adds the heavy performance-threshold lane to
the default lane list; `--release` runs that same list with an absent tool and a missing performance
baseline failing instead of skipping. Neither invocation runs a proof lane: `xtask` and `tools/gate.sh`
carry no proof, Kani or Verus command and no kernel registry, and no `kani::proof` source exists in the
repository, so no release invocation certifies the twelve mandatory proof kernels of
[docs/NATIONAL-CENSUS-PLAN.md](../docs/NATIONAL-CENSUS-PLAN.md) §10 or fails for their absence. The
obligation and the eight currently missing kernels live in that plan and
[docs/VERIFICATION-EVIDENCE.md](../docs/VERIFICATION-EVIDENCE.md); the reconciliation is owned by beads
`7w5` (twelve-kernel requirement versus this wrapper) and `psx` (proof verdicts), and no explicitly
selected harness substitutes for a missing kernel. Release also fails when Cargo metadata contains no
benchmark target; compiling a benchmark is not a measured performance result. See the dated evidence
for actually executed lanes and remaining blockers.

## Fast Rust iteration and local Moon cache

Run all developer commands through `tools/moon-local` from the repository root. Cargo is an internal
implementation, not a second supported workflow. The opt-in iteration profile and Moon CI do not
replace the full gate: `env -u CI tools/moon-local run pipeline:gate -- --release` is the release
acceptance entrypoint for the policy, security, feature-powerset, benchmark-presence and measured
performance lanes; it carries no proof lane, so kernel coverage must be reported by name rather than
inferred from a passing release run. The whole-workspace mutation lane was retired by owner decision
(bead `5rtr`). Run one assigned release-debt component without the umbrella via `env -u CI tools/moon-local
run pipeline:gate -- --release --lane <contract|panic-extraction|scan|ratchet|durability|perf>`. Ratchet
still measures strict Clippy and production scan; selectors do not waive release prerequisites or permit
baseline updates, and a single lane pass is not a release pass. A scoped report
test, cached task result, successful build or benchmark scaffold is not national-census or
native-recovery acceptance. Manual tasks are excluded from automatic CI; `env -u CI` is required
when any `CI` value is present, including `CI=false`.

### Prerequisites and compiler iteration

Use the installed **Moon 2.2.4**, the pinned `nightly-2026-04-27` toolchain and its checked-in
components/target via `rustup`, Cargo Nextest, Bash, `jq`, and the system C/C++ compilers, archiver
and linker. The launcher uses `sha256sum` when available, otherwise `shasum -a 256`, and does not
require GNU `realpath`. Linux gate/fault tooling still requires GNU coreutils, and the Linux
launcher retains the pinned toolchain's bundled LLD fingerprint. It selects Cargo via
`rustup which cargo` at the repository root and supplies that executable to
`tools/moon-cargo`, keeping `rust-toolchain.toml` authoritative rather than duplicating its nightly
selection in task commands. The offline build/test tasks use `--locked --offline`; populate locked
dependencies with `env -u CI tools/moon-local run pipeline:fetch` when needed.
These tasks do not install tools, change global Cargo configuration or manage daemons.

Native macOS launcher support requires Xcode Command Line Tools and `xcrun`. It resolves system
compiler/archiver shims, selects the host Cargo linker driver, and fingerprints that driver's actual
linker. Apple `ar` has no standalone version option, so its executable SHA256 records its identity.
The wrappers support macOS system Bash 3.2. Native Mac scope is formatting, checking and scoped
tests; `build`, `build-portable`, `build-native`, `bench` and `fetch` retain their explicit
`x86_64-unknown-linux-gnu` target and are not native Mac lanes. Launcher support alone does not
certify Mac compilation, runtime behavior or the Linux release/fault gate.

For an intentional locked dependency change, use
`env -u CI tools/moon-local run pipeline:dependency-update -- -p <package> --precise <version>`,
then `pipeline:fetch` and the applicable compile/test/security lanes. This manual task changes
`Cargo.lock`; it is not an automatic upgrade or permission to ignore an advisory.

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

### Cargo build-job override

For memory-constrained compilation, set `MOON_RUST_BUILD_JOBS` on the canonical launcher:

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:check pipeline:lint-src
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:gate -- --release
```

The optional value must be a positive decimal integer without leading zeros; empty, zero, signed,
fractional and whitespace-containing values fail before Cargo starts. Unset leaves existing task
defaults unchanged. `tools/moon-cargo` exports the exact value as `CARGO_BUILD_JOBS` immediately before
executing the selected Cargo, overriding Moon's task-level `16`. Gate/xtask Cargo children inherit
that value. Gate's nested `tools/moon-local` calls retain `MOON_RUST_BUILD_JOBS`; build and durability
scenario calls through `tools/moon-cargo` reapply it after Moon restores task defaults. Explicit Cargo
`--jobs` or Nextest `--build-jobs` arguments still take precedence; the wrapper never rewrites arguments.

This controls Cargo compilation jobs, not whole-lane concurrency. Nextest tasks retain their explicit
16 test threads, the iteration profile retains rustc `-Z threads=4`, and compiler/linker, build-script
and runtime pools can still run concurrently. It does not certify a fully serial or memory-bounded lane.

The launcher preserves the override before Moon hashing. All Rust task inputs already include
`$MOON_RUST_*` and `tools/moon-*`, so the override and wrapper changes participate in task cache
identity. `tools/build-toolchain.json` records launcher-time Cargo configuration, before task-level
environment and this override; it is not the effective build-job record. Retain the invocation's
`MOON_RUST_BUILD_JOBS` value alongside verification evidence.

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
| `pipeline:tests-scoped` | Manual uncached Nextest, all features by default; pass package/target/filter arguments explicitly; uses `target/fast-iteration` |
| `pipeline:build-scoped` / `pipeline:run-scoped` | Manual uncached scoped build/run through the fast-iteration configuration; explicit package and binary/example selection |
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
| `pipeline:native-resources` | Manual uncached `tools/durability/native-resources.sh`: verifies the pinned Restate/QEMU/image/guest-capture resources, prepares owned scratch disks and probes the isolation mechanisms the native fault scenarios need; `--certificate <path>` records the observed identities |

Scoped tasks are manual: unset `CI`, including when its value is `false`.

```sh
env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-store --lib -E 'test(acquisition::tests::snapshot)'
env -u CI tools/moon-local run pipeline:build-scoped -- -p census-service --bin census-serve
env -u CI tools/moon-local run pipeline:run-scoped -- -p census-service --bin census-serve -- --help
```

These tasks do not replace workspace or release gates. Running a store-opening example against
a serving store still violates the single-owner boundary; use isolated scratch or documented ingress.

Source-policy gates exclude tool-owned `.muse` workspace storage, alongside the
existing cache/vendor exclusions. Owned source under crates, tests, examples and
tools remains checked; concurrent worker copies must not be deleted to pass a gate.

Historical-school proof readback opens no Store and performs no acquisition:

```sh
env -u CI tools/moon-local run pipeline:run-scoped -- -p census-crawl \
  --example smoke_result_school_proof -- <accepted.manifest.json> <revoked.manifest.json>
```

Use the original manifests and adjacent proof files; the consumer checks their pins,
exact result context/locator and expected school disposition. A configured
`CENSUS_RESULT_SCHOOL_RETENTION_DIR` retains these artifacts from the public school-review
qualification scenario. They prove school authority/revocation, not PR-window or national acceptance.

For the controlled eligible-identity split case, optional
`CENSUS_CONTROLLED_ELIGIBLE_IDENTITY_RETENTION_DIR` selects a fresh publication
root beneath the canonical task `TMPDIR`; existing destinations refuse.
The test prints pre/post-split workbook paths. Run
`pipeline:xtask -- review-readback --bundle <workbook-parent>` on each generation. This synthetic
Long Jump control is not the three-mark 50→51 or national acceptance artifact.

The full test task is deliberately uncached because native fault/recovery checks observe processes,
ports and durable state not represented by source hashes. Running it does not by itself certify all
separately required native fault scenarios. A report-library cache hit cannot stand in for this lane.

`pipeline:tests-scoped` keeps `--all-features` unless
`MOON_RUST_SCOPED_TEST_FEATURES=default` explicitly selects Cargo's default features.
Unset or `all` preserves the existing argument vector. This option is accepted only
for the canonical `pipeline:tests-scoped` command; other tasks and unknown modes fail
before Cargo starts. Default mode rejects additional feature-selection flags rather
than silently changing the requested feature set. Locked/offline execution, build-job
limits, filters, and empty-selection failure options remain unchanged. This does not
weaken the aggregate, source, or release gates, which still require all features.

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 MOON_RUST_SCOPED_TEST_FEATURES=default \
  tools/moon-local run pipeline:tests-scoped -- -p census-service --lib \
  -E 'test(restate_services::publish::runtime::publication_tests::)' --no-tests fail
```

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
recognized test files are excluded from production measurements. Callable sizes and arities use
the trusted Rust AST policy: 25 logical lines is advisory, 60 is the hard maximum for handwritten
production functions, methods, closures and project-macro callable templates. More than five
parameters is a cohesion-review warning. Statement packing does not reduce the logical count.
The workspace disables Clippy's separate hard arity threshold; the AST scan still reports every
handwritten callable above five parameters for cohesion review.
Compiler/vendor-generated code is excluded from style-size certification; its compilation, domain
contracts, safety and behavior obligations remain. Opaque handwritten callable syntax fails closed.
The gate rejects missing, malformed or stale size evidence rather than accepting a fabricated clean
summary. `seams` is a separate structural check; a clean scan is not acceptance of identity,
durability or coverage.

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

`perf check` compares each recorded metric against the baseline at the requested tolerance
(`--tolerance`, default 5%) with three exceptions. The sample-maximum tail is an extreme-order
statistic, and re-measuring unchanged release binaries on this machine moved it by 5.4% on an idle
machine and by 8.7% while the CPU was still cooling from the preceding gate lane, while the
criterion mean stayed inside the tolerance both times. The tail comparison therefore floors its
bound at 20% (`TAIL_JITTER_FLOOR` in `xtask/src/perf/compare.rs`), which still catches the
pathological worst-case regressions the metric exists for. Whole-process peak RSS from GNU time is
the other noisy metric: four recordings of the unchanged `census/school_index/normalize_label`
workload measured 12,796 KiB (sha `22de9cfb…`, 94,530-line corpus), 11,676 KiB (sha `6bd12620…`,
100,547 lines), 12,472 KiB and 12,856 KiB (the two release gates on this tree) while the Valgrind
allocation count moved by 0.06% (`112472` / `112401`), so a 5% bound on its smallest case sits
inside the measurement's own spread, and even the 1 MiB floor sat inside the 1,180 KiB the four
recordings span. Peak RSS therefore floors its bound at a seventh of the recorded baseline with a
1 MiB absolute minimum (`peak_rss_floor`), clamped so the floor never exceeds the recorded baseline
itself — 1,668 KiB at the current baseline, which covers the 10.1% the four recordings actually
spread while still failing the pathological regressions the metric exists for and halving the
masked window the earlier fifth allowed on the largest cases; the DHAT allocation counts and bytes
keep the exact requested tolerance because they repeated to 0.1%.
Wall time and throughput carry the third floor, from the `census/school_index/build_index` micro
case (16 schools per iteration). Its production path is unchanged since the baseline — the only
working-tree delta in `crates/census-domain/src/school_index.rs` is a test module relocated to
`school_index/tests.rs` — and three isolated criterion runs measured it at 1.7404 µs, 1.7357 µs and
1.7951 µs, faster than the recorded 2.068 µs, while whole-target `perf check` runs of the same tree
measured 2.068 µs (baseline record) and 2.206 µs. A case whose number moves with how the surrounding
target process has warmed, by far more than the requested 5%, cannot carry a threshold that tight,
so the wall-time and throughput bounds floor at the case's recorded tail-to-mean spread
(`case_timing_floor`, 8.7% here), capped at the same 20% jitter ceiling: a measurably noisy case
absorbs its own spread, while stable cases and pathological regressions keep the requested
tolerance. The recorded baseline is protocol-bound; replacing it needs the explicit approval
`tools/perf-baseline.json` already requires.

The historical `tools/perf-baseline.json` tagged its fifteen throughput measurements `Elements`, as
evidenced by the benchmark sources at its recorded commit (sha `0be3cfa…`, 2026-09-30, corpus
38,530 lines). Its metadata, workload names and numbers are unchanged; the original untagged JSON is
retained with the dated [verification evidence](../docs/VERIFICATION-EVIDENCE.md), and the
superseded tagged file is preserved byte-identical (md5 `f4181a9f0afb5abcf56937fbe71da2fc`) at
`var/audit-20261006/perf-baseline-before-revision-matched-record.json`. Its four legacy pipeline
entries do not describe the separate snapshot-readback workload. The current acceptance baseline was
re-recorded on explicit owner approval (`6yj.7`) and extended with peak RSS, DHAT allocation counts
and sample-maximum tail time at sha `22de9cfb…` with the same workload IDs and a 94,530-line corpus
(corpus sha256 `12c06847…`); a missing, malformed or regressed memory/tail measurement fails the
gate instead of passing silently, and a cold comparison without a recorded baseline fails closed.
That is not a reset of the historical numbers, which stay readable in the preserved file; the
pre-extension shape is retained at `var/gate-cc7c82f3/perf-baseline-before-20261009-record.json`.
Match target, workload, corpus and revision before comparing current results, and treat a new
acceptance baseline as requiring the same explicit approval.

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
The original 2026-09-22 capture-audit document is not retained in this tree.
Use current command help; historical audit notes do not qualify production search.
