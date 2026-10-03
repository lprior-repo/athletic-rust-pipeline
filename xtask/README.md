# xtask — developer command reference

Run `cargo xtask <command> [args]`, or `cargo run -p xtask -- <command> [args]`.
Child commands execute at the repository root, print their exact argument vector and propagate
failure. Measurement commands emit their own reports. Run `cargo xtask --help` for the current CLI.

## Commands

| Command | Current behavior |
|---|---|
| `gate [-- <args>]` | Invokes `tools/gate.sh`; gate policy and lane details live in [tools/gate.sh](../tools/gate.sh) |
| `scan` | Production forbidden-construct and size measurements for workspace members, discovered with Cargo metadata; JSON |
| `comments` | Lexical no-comments check over project Rust, including tests/examples/benches/fuzz; strings are data |
| `panic-extraction [--root <dir>]` | Fatal owned-Rust lexical extraction and suppression check, including cfg-disabled proofs and three rendered source templates; captured literals are data |
| `contract` | Eight architecture checks, including scope, transports, retry ceilings, Python artifacts, admission, scan parity, front-door documents and adapter registration |
| `seams` | Checks both module and sibling-crate allowed-edge tables; JSON and failing status on violation |
| `integrity` | Domain type-integrity review candidates; JSON, not proof that types enforce their contracts |
| `domain-purity` | Checks `census-domain` normal dependency tree for banned runtime/I/O dependencies |
| `quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]` | Updates debt measurements; increases require the explicit flag and owner-authorized policy change |
| `ratchet <baseline> <clippy.tsv> <scan.json>` | Fails on growing measured debt; emits `[DOWN]`/`[UP]` changes |
| `source-test <source>` / `source-check <source>` | Runs Nextest **only in `census-service`**, selecting `test(<source>)`; not all extracted adapter tests |
| `source-tests` | Colocated workspace source tests (`--lib --bins --examples`); Nextest, or Cargo test fallback; excludes integration binaries |
| `source-fixture <source>` | Lists captured files beneath `crates/census-crawl/tests/fixtures/<source>/`; missing directory fails |
| `replay <name>` | Offline supported fixture replay through published parse paths; unsupported/empty inputs fail |
| `census-status` | Serving `Census/status`, or offline `report --core`; see routing below |
| `coverage` | Serving `Report/run`, or offline all-source `report` |
| `export` | Serving `Workbook/run`, or offline `workbook`; accepts `--out`, `--grad-year`, `--core`, `--limit` |
| `bench [-- <args>]` | Forwards to `cargo bench -p census-service`; does not select every workspace benchmark |
| `perf record`, `perf check`, `perf profile <group>` | Record/compare/profile configured benchmarks; each lane's own reports own interpretation and limits |
| `kani [-- <harnesses>]` | No selection means all eight mandatory contract kernels and fails if any is absent; an explicit selection is a focused proof run, not the release gate |
| `dump-sheet <workbook> <sheets>...` | Prints nonempty worksheet rows as `column=value` fields |
| `new-source <name>` | Writes a source scaffold and module declaration; not a qualified or fully registered adapter |

`tools/gate.sh --full` and `--release` invoke the unfiltered mandatory proof lane.
A passing explicitly selected harness is not coverage of missing kernels. Release also fails when
Cargo metadata contains no benchmark target; compiling a benchmark is not a measured performance
result. See the dated evidence for actually executed lanes and remaining blockers.

## Serving versus offline routing

`census-status`, `coverage` and `export` default to the serving census. `--ingress [<origin>]` uses
loopback HTTP, default `http://127.0.0.1:18095/`, without credentials or a path. `--store <dir>` selects
in-process offline execution and cannot be combined with `--ingress`.

```sh
cargo xtask census-status
cargo xtask coverage --ingress http://127.0.0.1:18095/
cargo xtask export --ingress --out out/publication --grad-year 2027
cargo xtask census-status --store var/census-service
```

The last form requires the serving store owner to be stopped. A second-process Fjall lock error is
correct; do not bypass it. Serving status is the handler's count view, whereas offline status builds
a Core report; do not claim identical semantic populations merely because labels match. Export
reads existing evidence and does not acquire missing history. `--out` names a publication directory;
the XLSX is `<publication>/current/workbook.xlsx`. See [operations](../docs/OPERATIONS.md) for
lifecycle, complete frozen-bundle verification and current certification limits.

## Measurement and fixture boundaries

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
bytes and associated format/year metadata. A successful replay establishes those captures only;
`source-test`'s current service-only routing does not establish the crawl crate's complete coverage.
Use the owning crate's focused tests where necessary and retain this routing gap as implementation
work, not a reason to claim the wrapper runs more than it does.

`replay coach_directories` qualifies exactly five required root response captures. Provenance,
historical golden outputs and the separately qualified survey/probe subtree remain inventoried
by `source-fixture` but are not response inputs for this lane. Directory/summary wire extents
retain their historical records; current Census coach contexts use the shared
`coach_directories__census-contexts.json` oracle (sixteen distinct NC contexts and zero IN contexts).
Missing response/oracle files fail rather than silently reducing qualification. This does not
certify the entire survey corpus, live accessibility, current tenure or a national census.

`source-test`/`source-check` select test **function names**, not modules or files: the slug is passed
to nextest's `test(<slug>)` predicate, which matches any test whose name contains it. Name the
functions after the source, hyphens as underscores — `nces_*`, `state_ed_*`, `tssaa_*`,
`private_assoc_*` — and call the lane with the underscore spelling (`cargo xtask source-test
state_ed`). A source whose tests are named otherwise reports an empty lane while the reader still has
tests; fix the names rather than widening the filter.

The service's `core` Criterion target measures captured parsers, archive classification, label
normalization/resolution and store scans. The `pipeline` target measures
`pipeline/snapshot/athlete_evidence`: 20,000 deterministic provider-owned subjects, each with a
supported Class-of-2027 observation and a second unsupported graduation inference. Setup and exact
evidence/confidence checks precede timing; every timed iteration reads the frozen Fjall snapshot.
This synthetic workload measures readback, not census discovery or native recovery.

Run `cargo bench -p census-service --bench core -- --noplot` and the equivalent `--bench pipeline`
command on a quiet machine; retain Criterion units, sample/warmup settings and profiler context.
The historical `tools/perf-baseline.json` now tags all fifteen throughput measurements `Elements`,
as evidenced by the benchmark sources at its recorded commit. Its metadata, workload names and
numbers are unchanged; the original untagged JSON is retained with the dated
[verification evidence](../docs/VERIFICATION-EVIDENCE.md). The four legacy pipeline workloads
do not match the new snapshot-readback workload. Unit-format migration does not approve a
replacement workload or a new acceptance baseline; do not reset the baseline to conceal that gap.

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
`cargo run -p xtask --bin g1-audit -- --help`.
It inventories retained search response bodies offline; it is not a production census stage.
The [2026-09-22 capture audit](../research/G1-LIVE-SEARCH-CONTRACT.md) owns its historical commands
and findings.
