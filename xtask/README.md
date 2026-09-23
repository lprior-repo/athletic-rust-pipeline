# `xtask` — repository developer commands

One binary that runs the repository's real tools, and prints the exact child command before it runs
it, so a terminal session and this harness cannot drift apart. The measurement subcommands — `scan`,
`integrity`, `domain-purity`, `seams`, `quality-baseline`, `ratchet` — are the exception: they are
the gate's measurement layer, which used to be a set of Python scripts under `tools/`. Everything
else shells out, submits an invocation to the running Restate deployment, or writes files.

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
| `source-test <source>` (alias `source-check`) | `cargo nextest run -p midwest-census -E 'test(<source>)'` |
| `source-fixture <source>` | reads `crates/midwest-census/tests/fixtures/<source>/` and lists it |
| `replay <name>` | replays `crates/midwest-census/tests/fixtures/<name>/` offline: prints each capture's parse result, same bytes every run |
| `census-status <--store <dir>\|--ingress [<origin>]>` | `Census/status` on the running deployment, or the binary's `report --core` offline |
| `coverage <--store <dir>\|--ingress [<origin>]>` | `Report/run` on the running deployment, or the binary's `report` (no flag = every source) offline |
| `bench [-- <filter>]` | `cargo bench -p midwest-census [<filter>]` |
| `export <--store <dir>\|--ingress [<origin>]> [--out <file>] [--grad-year <year>] [--all-sources] [--limit <n>]` | `Workbook/run` on the running deployment, or the binary's `workbook` offline |
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

### `replay <name>`

```bash
cargo xtask replay wiaa_results
```

Reads every capture under `crates/midwest-census/tests/fixtures/<name>/` and runs each body through
the same parse entry point that source's fixture tests call, printing what the parser published -
counts, route names, the published school or meet names. Nothing is fetched, no clock is read, no
store is opened and no environment is consulted, so two runs over the same tree print the same bytes:
that is what makes it the verb to reach for while `midwest-serve` holds the store, on a machine with
no network, or when a capture has to be re-read without re-crawling its host.

Two kinds of input a capture cannot state about itself come from the same place the harnesses get
them: a result file's format is classified from its extension and body by the adapter's own function,
and its archive year is read from the fixture's record under `crates/midwest-census/tests/golden/`.

Every capture is read this way: the arms reach the crate's published parse surface, the same
functions the source's fixture tests call, and a capture or a source none of the arms claims is
refused by name rather than skipped. Otherwise it is an error: `xtask: <message>` on stderr, exit 1,
never a panic and never a missing capture reported as an empty parse. A missing fixture directory, an
empty one, a capture no arm claims, an unreadable fixture record and a body whose parser refuses it
each fail that way.

### `census-status`, `coverage` and `export`

All three default to the running deployment. The flag-free form and `--ingress [<origin>]` submit the
matching handler to the census node; `--store <dir>` selects the offline path instead. Giving both is
a usage error naming both. The origin defaults to the project node, so `--ingress` alone is the local
deployment.

```bash
cargo xtask census-status                                   # Census/status, project node (18095)
cargo xtask census-status --ingress http://127.0.0.1:18095  # the same invocation, spelled out
cargo xtask census-status --store var/midwest-census        # midwest-census report --core, worker stopped
cargo xtask coverage                                        # Report/run, every source
cargo xtask coverage      --store var/midwest-census        # midwest-census report, every source
```

`--ingress` submits the handler the running deployment already owns and never opens the store, which
is the mode that works *while* `midwest-serve` holds it. `--store` runs the shipped `midwest-census`
binary, which opens the store in process: that is the backup-drill and CI path — the only one that
works with no server running — and it needs the worker stopped, because the store is single-writer
and a second handle fails with `FjallError: Locked`.

Both modes report the same numbers under the same labels. `census-status` prints a `schools=` /
`athletes=` line either way: offline it is the core report's totals plus the JSON/CSV paths it wrote,
and through the ingress it is the store's own status — those two tables as `Census/status` counts
them, with the observation count, on-disk footprint and date. `coverage` prints the every-source
totals and the written paths in both modes.

The offline paths forward no scope flag beyond the one the subcommand means — `census-status` is
`--core` and `coverage` is every source — because the CLI has no `--scope` and no `--out` flag, and
this command does not pretend otherwise. The ingress paths send the same scope as the wire value.

### `bench [-- <filter>]`

```bash
cargo xtask bench                 # every criterion benchmark target in the crate
cargo xtask bench -- parser       # criterion's own name filter, forwarded after `--`
```

Forwards to `cargo bench -p midwest-census`, so it needs the crate to build and the filter is a
substring match over criterion benchmark ids, not a target name. The `--` separator is required:
a bare `cargo xtask bench parser` is a clap error, and the wrapper prints the exact command it runs
so a surprising filter is visible.

### `export [--out <file>] [--grad-year <year>] [--all-sources] [--limit <n>]`

```bash
cargo xtask export --ingress --out out/census.xlsx --limit 5000
cargo xtask export --store var/midwest-census --out out/census.xlsx --limit 5000
```

Builds the census workbook from evidence the store already holds — no gathering, no network.
`--grad-year` (default 2027), `--all-sources` and `--limit` are forwarded to `midwest-census workbook`
offline and carried in the `Workbook/run` request through the ingress, and both modes print the
path they wrote and the cohort's graduation year. It is the short stable name for the artifact the
recruiting projection consumes; offline the child's exit status is this command's exit status, and
through the ingress a refused or failed invocation is a non-zero exit with Restate's own message.

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
adapter presents the registry's call shape (`CrawlResult<AdapterReport>`, the crate's own error type
rather than `anyhow`), and the test in `parse.rs` passes while the fixture directory holds only its
README, then fails as soon as a capture lands, until the real parser replaces the placeholder.

## What it does not do

- No reimplementation: `gate`, `source-test`, `census-status`, `coverage`, `bench` and `export` are
  thin wrappers around the tools that own the behaviour — the census commands either run the
  `midwest-census` binary or submit that deployment's own handlers (`Census/status`,
  `Report/run`, `Workbook/run`) through Restate's ingress. The measurement subcommands do
  implement the gate's measurements, because those measurements are this repository's own policy
  rather than another tool's job — they are the Rust replacements for the deleted `tools/*.py`
  scripts.
- No shell interpretation: arguments are passed as an argument vector, so an argument with spaces or
  quotes is never re-split. `--` separates this binary's flags from the child's.
- No project-wide validation by itself: `new-source` writes files and prints paths, it does not run
  `fmt`, clippy or the gate for you.
- No silent success: a missing fixture directory, an existing scaffold target, and a non-zero child
  exit are all errors.
