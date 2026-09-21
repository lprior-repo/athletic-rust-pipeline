# `xtask` — repository developer commands

One binary that runs the repository's real tools, and prints the exact child command before it runs
it, so a terminal session and this harness cannot drift apart. The measurement subcommands — `scan`,
`integrity`, `domain-purity`, `seams`, `quality-baseline`, `ratchet` — are the exception: they are
the gate's measurement layer, which used to be a set of Python scripts under `tools/`. Everything
else shells out or writes files.

## Running it

```bash
cargo xtask <command> [args]                  # via the alias in .cargo/config.toml
cargo run -p xtask -- <command> [args]        # the same thing, spelled out
```

`.cargo/config.toml` holds that one alias — `xtask = "run -p xtask --"` — and nothing else, so the
gate lanes keep running against the same toolchain and flags with or without it.

Children always execute with the repository root as their working directory, whatever directory
`xtask` was invoked from — so a relative `--store var/midwest-census` means here exactly what
`AGENTS.md` documents.

Errors go to stderr as `xtask: <message>`, the exit status is `1`, and a child's own exit code is
named in the message; there is no stack trace.

## Commands

| Command | Runs |
| --- | --- |
| `gate [-- <args>]` | `bash tools/gate.sh [<args>]` |
| `scan` | measures this repository's production code; JSON on stdout |
| `integrity` | lists the domain type-integrity candidates; JSON on stdout |
| `quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]` | rewrites the debt baseline |
| `ratchet <baseline> <clippy.tsv> <scan.json>` | compares measurements with the debt baseline |
| `domain-purity` | proves the `census-domain` tree carries no async/I/O package |
| `seams` | checks every `crate::…` edge between the census crate's top-level modules against the allowed table; JSON on stdout, non-zero exit on a violation |
| `source-test <source>` | `cargo nextest run -p midwest-census -E 'test(<source>)'` |
| `source-fixture <source>` | reads `crates/midwest-census/tests/fixtures/<source>/` and lists it |
| `census-status --store <dir>` | `cargo run -q -p midwest-census --bin midwest-census -- --store <dir> report --core` |
| `coverage --store <dir>` | same binary: `report` (no flag = every source) |
| `bench` | nothing yet: Phase 6 adds `benches/pipeline.rs` |
| `new-source <name>` | writes the adapter scaffold described below |

### `gate`

```bash
cargo xtask gate                       # every lane, like tools/gate.sh
cargo xtask gate -- --update-baseline  # arguments after `--` go to gate.sh
cargo xtask gate -- --allow-increase   # (only with --update-baseline)
```

The gate is the whole workspace: fmt, check `--all-targets`, doc, tests, strict clippy, the scans,
the debt ratchet, and every optional tool lane that is installed. This command does not weaken any
of it — it forwards arguments, prints the command, and reports the child's status.

### `scan`, `integrity`, `domain-purity`, `seams`

```bash
cargo xtask scan           # JSON: forbidden constructs + size budgets, per crate
cargo xtask integrity      # JSON: type-integrity candidates per domain root
cargo xtask domain-purity  # the census-domain normal tree; fails on a banned package
cargo xtask seams          # the census crate's top-level module edges; fails outside the table
```

`scan` and `integrity` write JSON to stdout and nothing else, which is how `tools/gate.sh` feeds them
to `jq`. `scan` covers `src/` and `crates/midwest-census/src` — exactly the crates the debt baseline
records — counts forbidden constructs in production-reachable code (a `#[cfg(test)]` that gates a
module ends the region; test files are skipped throughout), and reports the size budgets
(`files_over_300_lines`, `functions_over_60_lines`, `functions_over_25_logical_lines`).

`domain-purity` runs `cargo tree -p census-domain --edges normal --prefix none` and fails when the
tree carries an async runtime, store engine, HTTP client, service framework or browser engine: normal
edges only, so dev-dependencies and build scripts cannot taint the verdict either way.

`seams` walks every production `.rs` file under `crates/midwest-census/src`, resolves each `crate::…`
reference to its top-level module, and fails when the `(from, to)` pair is outside the allowed-edge
table in `xtask/src/seams.rs`. The table is the ratchet: adding an edge is a deliberate edit, and
deleting a row makes that edge a violation again, because the check fails closed. Comment lines and
test code are out of scope — `tests.rs`, files under a `tests/` directory, and the region after the
`#[cfg(test)]` that opens a module — because none of them can reach production callers.

### `quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]`

Rewrites the debt baseline from the gate's clippy tallies and a `scan` report. The shape is fixed —
`note`, `clippy` (keyed `crate<TAB>lint`), `scan` (keyed by crate), `structure` — and the update
refuses to raise any number without `--allow-increase`, because a burndown is the only legitimate
reason for the baseline to move down.

### `ratchet <baseline> <clippy.tsv> <scan.json>`

Compares those same two measurements with the baseline, exits non-zero when any metric grew, and
prints every change as `[DOWN]` or `[UP]`. A file over 300 lines is identified by its path: a new path
is debt, a known path that grew is debt, and a known path that shrank prints as burndown.

### `source-test <source>`

```bash
cargo xtask source-test wiaa
```

Nextest with a `test(<source>)` filter over the whole `midwest-census` package, which is how a
source's tests are selected by name. It runs whatever matches; it does not prove the fixture set is
complete — `source-fixture` shows what exists, and the fixture directories are the coverage list.

### `source-fixture <source>`

```bash
cargo xtask source-fixture wiaa
```

Lists every regular file under the source's fixture directory, recursively, sorted, with byte sizes.
The directory has to exist: a missing source is an error naming the fixture directories that do
exist, not an empty listing.

### `census-status --store <dir>` and `coverage --store <dir>`

```bash
cargo xtask census-status --store var/midwest-census   # report --core
cargo xtask coverage      --store var/midwest-census   # report, every source
```

Both run the shipped `midwest-census` binary, so both need the crate to build, and both write the
census JSON/CSV into `<store>/out/`. The CLI has no `--scope` and no `--out` flag: core scope is
`--core` and every source is the default, and this command does not pretend otherwise.

### `bench`

Exits non-zero with the reason: Phase 6 adds `benches/pipeline.rs` (and `tools/gate.sh` starts
running `cargo bench --workspace --no-run` with it). No benchmark target exists today, so any number
printed here would be invented.

### `new-source <name>`

```bash
cargo xtask new-source sondre-land   # names the module sondre_land
```

Writes the decomposition-target layout and registers the module:

```
crates/midwest-census/src/sources/<name>/mod.rs      module doc, SOURCE_ID, Options, collect (bails)
crates/midwest-census/src/sources/<name>/parse.rs    pure parsing placeholder + fixture-driven test
crates/midwest-census/src/sources/<name>/map.rs      canonical mapping placeholder
crates/midwest-census/src/sources/<name>/README.md   purpose, entry points, fixtures, commands
crates/midwest-census/tests/fixtures/<name>/README.md what to capture, form, naming
crates/midwest-census/src/sources/mod.rs             one appended `pub mod <name>;` line
```

Hyphens become underscores (a module name is a Rust identifier); everything else is refused: names
that are not lowercase identifiers, Rust keywords, or any name whose module directory, flat
`<name>.rs`, or fixture directory already exists. The generated code compiles and does nothing: the
test in `parse.rs` passes while the fixture directory holds only its README, and fails as soon as a
capture lands, until the real parser replaces the placeholder.

## What it does not do

- No reimplementation: `gate`, `source-test`, `census-status` and `coverage` are thin wrappers around
  the tools that own the behaviour. The measurement subcommands do implement the gate's measurements,
  because those measurements are this repository's own policy rather than another tool's job — they
  are the Rust replacements for the deleted `tools/*.py` scripts.
- No shell interpretation: arguments are passed as an argument vector, so an argument with spaces or
  quotes is never re-split. `--` separates this binary's flags from the child's.
- No project-wide validation by itself: `new-source` writes files and prints paths, it does not run
  `fmt`, clippy or the gate for you.
- No silent success: a missing fixture directory, an existing scaffold target, an unfinished
  benchmark and a non-zero child exit are all errors.
