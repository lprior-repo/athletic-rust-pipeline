# xtask — developer command reference

Run `cargo xtask <command> [args]`, or `cargo run -p xtask -- <command> [args]`.
Child commands execute at the repository root, print their exact argument vector and propagate
failure. Measurement commands emit their own reports. Run `cargo xtask --help` for the current CLI.

## Commands

| Command | Current behavior |
|---|---|
| `gate [-- <args>]` | Invokes `tools/gate.sh`; gate policy and lane details are in [TESTING.md](../TESTING.md) |
| `scan` | Production forbidden-construct and size measurements for workspace members, discovered with Cargo metadata; JSON |
| `comments` | Lexical no-comments check over project Rust, including tests/examples/benches/fuzz; strings are data |
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
| `perf record`, `perf check`, `perf profile <group>` | Record/compare/profile configured benchmarks; [PERFORMANCE.md](../PERFORMANCE.md) owns interpretation and limits |
| `kani [-- <harnesses>]` | Runs the tool's declared harness selection; [TESTING.md](../TESTING.md) distinguishes this inventory from all required proof kernels |
| `dump-sheet <workbook> <sheets>...` | Prints nonempty worksheet rows as `column=value` fields |
| `new-source <name>` | Writes a source scaffold and module declaration; not a qualified or fully registered adapter |

## Serving versus offline routing

`census-status`, `coverage` and `export` default to the serving census. `--ingress [<origin>]` uses
loopback HTTP, default `http://127.0.0.1:18095/`, without credentials or a path. `--store <dir>` selects
in-process offline execution and cannot be combined with `--ingress`.

```sh
cargo xtask census-status
cargo xtask coverage --ingress http://127.0.0.1:18095/
cargo xtask export --ingress --out out/census.xlsx --grad-year 2027
cargo xtask census-status --store var/census-service
```

The last form requires the serving store owner to be stopped. A second-process Fjall lock error is
correct; do not bypass it. Serving status is the handler's count view, whereas offline status builds
a Core report; do not claim identical semantic populations merely because labels match. Export
reads existing evidence and does not acquire missing history. See [operations](../docs/OPERATIONS.md)
for lifecycle and current publication limitations.

## Measurement and fixture boundaries

`scan` obtains members from Cargo metadata rather than a two-package list. Test-only regions and
recognized test files are excluded from production measurements. `seams` is a separate structural
check; a clean scan is not acceptance of identity, durability or coverage.

`comments` uses the compiler lexer, rejects prose `doc = ...` attributes, and permits non-prose
attributes such as `doc(hidden)`. Ordinary/raw/byte/C strings and captured comment-shaped bytes are
not code comments. Unreadable/invalid source, unterminated literals, excessive source counts and an
empty tree fail closed; the current per-file limit is 4 MiB.

`replay` does not fetch, open a store or consult the live source clock. It needs the committed fixture
bytes and associated format/year metadata. A successful replay establishes those captures only;
`source-test`'s current service-only routing does not establish the crawl crate's complete coverage.
Use the owning crate's focused tests where necessary and retain this routing gap as implementation
work, not a reason to claim the wrapper runs more than it does.

## Source scaffolding

Hyphens normalize to underscores; invalid identifiers, keywords and existing module/fixture paths
are refused. The scaffold creates `mod.rs`, `parse.rs`, `map.rs` and an adapter README under
`crates/census-crawl/src/<name>/`, plus a fixture README, then appends the module declaration to
`crates/census-crawl/src/lib.rs`. Creation/registration errors report partial files; they are not a
transactional rollback.

The generated collector/parser/map are incomplete authoring aids. Replace them with real behavior,
captures and assertions; complete registry/applicability/durable dispatch separately under
[SOURCE_ADAPTER_GUIDE.md](../SOURCE_ADAPTER_GUIDE.md). Scaffolding does not run formatting, gates,
source qualification or national coverage acceptance.

## Retained search audit

The auxiliary `g1-audit` binary lives at `xtask/src/g1/main.rs`:
`cargo run -p xtask --bin g1-audit -- --help`.
It inventories retained search response bodies offline; it is not a production census stage.
The [2026-09-22 capture audit](../research/G1-LIVE-SEARCH-CONTRACT.md) owns its historical commands
and findings.
