# `xtask` — repository developer commands

One binary that runs the repository's real tools, and prints the exact child command before it runs
it, so a terminal session and this harness cannot drift apart. Nothing here reimplements `gate.sh`,
`nextest`, the census CLI or the scaffold: every subcommand either shells out or writes files.

## Running it

```bash
cargo run -p xtask -- <command> [args]        # always works
```

`cargo xtask <command>` is the shorter form, and needs the alias in `.cargo/config.toml`:

```toml
[alias]
xtask = "run -p xtask --"
```

Children always execute with the repository root as their working directory, whatever directory
`xtask` was invoked from — so a relative `--store var/midwest-census` means here exactly what
`AGENTS.md` documents.

Errors go to stderr as `xtask: <message>`, the exit status is `1`, and a child's own exit code is
named in the message; there is no stack trace.

## Commands

| Command | Runs |
| --- | --- |
| `gate [-- <args>]` | `bash tools/gate.sh [<args>]` |
| `source-test <source>` | `cargo nextest run -p midwest-census -E 'test(<source>)'` |
| `source-fixture <source>` | reads `crates/midwest-census/tests/fixtures/<source>/` and lists it |
| `census-status --store <dir>` | `cargo run -q -p midwest-census --bin midwest-census -- --store <dir> report --core` |
| `coverage --store <dir>` | same binary: `report` (no flag = every source) |
| `bench` | nothing yet: Phase 6 adds `benches/pipeline.rs` |
| `new-source <name>` | writes the adapter scaffold described below |

### `gate`

```bash
cargo run -p xtask -- gate                       # every lane, like tools/gate.sh
cargo run -p xtask -- gate -- --update-baseline  # arguments after `--` go to gate.sh
cargo run -p xtask -- gate -- --allow-increase   # (only with --update-baseline)
```

The gate is the whole workspace: fmt, check `--all-targets`, doc, tests, strict clippy, the scans,
the debt ratchet, and every optional tool lane that is installed. This command does not weaken any
of it — it forwards arguments, prints the command, and reports the child's status.

### `source-test <source>`

```bash
cargo run -p xtask -- source-test wiaa
```

Nextest with a `test(<source>)` filter over the whole `midwest-census` package, which is how a
source's tests are selected by name. It runs whatever matches; it does not prove the fixture set is
complete — `source-fixture` shows what exists, and the fixture directories are the coverage list.

### `source-fixture <source>`

```bash
cargo run -p xtask -- source-fixture wiaa
```

Lists every regular file under the source's fixture directory, recursively, sorted, with byte sizes.
The directory has to exist: a missing source is an error naming the fixture directories that do
exist, not an empty listing.

### `census-status --store <dir>` and `coverage --store <dir>`

```bash
cargo run -p xtask -- census-status --store var/midwest-census   # report --core
cargo run -p xtask -- coverage      --store var/midwest-census   # report, every source
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
cargo run -p xtask -- new-source sondre-land   # names the module sondre_land
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
  the tools that own the behaviour.
- No shell interpretation: arguments are passed as an argument vector, so an argument with spaces or
  quotes is never re-split. `--` separates this binary's flags from the child's.
- No project-wide validation by itself: `new-source` writes files and prints paths, it does not run
  `fmt`, clippy or the gate for you.
- No silent success: a missing fixture directory, an existing scaffold target, an unfinished
  benchmark and a non-zero child exit are all errors.
