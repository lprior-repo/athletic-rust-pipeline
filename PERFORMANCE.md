# PERFORMANCE.md — measured baselines, harnesses, and the debt ratchet

What is measured today, where every number comes from, what the hot paths actually do, and what is
still unmeasured. A performance claim may only cite the sources in §1; anything else is a proposal.

There is **no throughput baseline in this repository yet**. `tools/quality-baseline.json` records
debt counts, not timings, and `docs/HARDENING-PROGRAM.md` §2.5 states that the earlier 559–627k obs/s
figures live outside the repo, so they are not reproducible and not a gate. The two bench harnesses
in §2 exist and are runnable, but nothing enforces a threshold on them.

## 1. Measured baselines

Two sources of record:

| Source | Produced by | Contains |
|---|---|---|
| `tools/quality-baseline.json` | `tools/gate.sh` (`measure_clippy`) + `cargo xtask scan` | per-crate strict-lint counts, forbidden-construct scan, size budgets |
| `docs/HARDENING-PROGRAM.md` | program measurement at baseline commit `4e5b828`, 2026-09-21, pinned toolchain (nightly-2026-04-27, rustc 1.97.0-nightly) | same class of counts, plus acquisition/structure inventory and determinism evidence |

The two were captured at different times and are not directly comparable: the program describes its
census figure as lib errors (`392 census lib`), while the gate measures `--lib --bins --examples`.
The drift is visible — census `arithmetic_side_effects` is 208 in the program's table and 240 in the
current baseline, while the root count is 36 in both. Cite one source per statement; do not mix rows
across them.

### 1.1 Debt baseline — `tools/quality-baseline.json`

Strict clippy, measured on source targets (`--lib --bins --examples`), counted per crate and lint
(a lint absent from the map is zero, which is how the ratchet compares it — see §5):

| Lint | `athletic_rust_pipeline` (root) | `midwest_census` |
|---|---:|---:|
| `clippy::arithmetic_side_effects` | 36 | 64 |
| `clippy::as_conversions` | 10 | 7 |
| `clippy::expect_used` | 0 | 0 |
| `clippy::indexing_slicing` | 2 | 4 |
| `clippy::let_underscore_must_use` | 1 | 0 |
| `clippy::string_slice` | 2 | 1 |
| `clippy::unwrap_used` | 0 | 0 |
| `clippy::explicit_counter_loop` | 0 | 0 |

Production scan (`cargo xtask scan`; "production-reachable" = before the first `#[cfg(test)]` in a
file, test files excluded):

| Metric | root (`athletic-rust-pipeline`) | census (`midwest-census`) |
|---|---:|---:|
| `production_lines` | 34,308 | 22,912 |
| `files` | 238 | 125 |
| `unsafe` | 0 | 0 |
| `unwrap` | 0 | 0 |
| `expect` | 0 | 0 |
| `indexing` | 0 | 0 |
| `as_cast` | 0 | 7 |
| `assert_family`, `panic`, `todo`, `dbg`, `unreachable` | 0 | 0 |

Structure, workspace-wide: **22** files over 300 lines, **0** functions over 60 logical lines,
**551** functions over 25 logical lines. Hot-path files currently on the oversize list:
`src/xlsx/parser/rows.rs` (331), `src/workbook_ingest/guards.rs` (322),
`src/workbook_ingest/stream.rs` (310), `crates/midwest-census/src/bootstrap.rs` (384),
`crates/midwest-census/src/sources/coach_contacts.rs` (553),
`crates/midwest-census/src/sources/compiled.rs` (579),
`crates/midwest-census/src/sources/ks.rs` (578),
`crates/midwest-census/src/sources/milesplit.rs` (610),
`crates/midwest-census/src/sources/raceday.rs` (327),
`crates/midwest-census/src/sources/xc.rs` (551). Phase 2 of the hardening program is scheduled to split
them; until then any hot-path edit has to fit inside the size budget or shrink it.

### 1.2 Program baselines — `docs/HARDENING-PROGRAM.md`

| Measurement | Value |
|---|---|
| Code size (program §1) | root 33,608 production lines / 182 files; census 17,854 / 31 |
| Strict clippy result (§2.1) | exit 101, ≈440 errors (392 census lib, 51 root) |
| Program per-lint totals (§2.1) | `arithmetic_side_effects` 244 (208 census / 36 root); `expect_used` 75; `string_slice` 49 (47/2); `as_conversions` 46 (36/10); `indexing_slicing` 25 (23/2); `let_underscore_must_use` 3 (3/0); `unwrap_used` 1 |
| Census structure (§2.2) | 134 of 552 production functions over 25 logical lines; 33 over 60; 21 files over the 300-line budget |
| Async inventory (§2.3) | `tokio::spawn` 0 census / 4 root; `spawn_blocking` 2/2; `buffer_unordered` 4/0; async fns with >3 `.await` "several"/"many"; tokio-console and OTLP absent; `tokio::time::pause`, loom, shuttle, turmoil absent |
| Determinism evidence (§1) | rebuild reproduced 6 of 7 JSONL snapshots byte-identically; `coaches.jsonl` differs only in 917 withheld rows; the published workbook had 0 of 459 published numbers missing |
| Build profile (§1, verified in `Cargo.toml`) | release: `lto = "thin"`, `codegen-units = 1`, `strip` |
| Absent (§2.5) | no CI, no `benches/`, no criterion/divan, no fuzz targets |
| Missing workstation tools (§1) | `cargo-fuzz`, `cargo-semver-checks`, `hyperfine` |

The program's Phase 7 target — criterion benches under `benches/`, committed baselines, CI failing on
a >10% regression, allocation budgets, and an fjall tuning experiment with numbers — is still open.
The harnesses in §2 are the first half of that work: measurement exists, gating does not.

## 2. Bench harness

Two clap-based example binaries in `crates/midwest-census/examples/`. They are examples, not
`benches/` targets: `cargo bench` has no target, and `tools/gate.sh`'s bench-presence lane looks for
a `benches/` directory and currently prints `no benchmark target exists yet`. Nothing enforces a
regression threshold on either harness.

Run from the repository root (examples are auto-discovered; the census crate's manifest declares no
`[[example]]` sections):

```bash
cargo run --release -p midwest-census --example bench_census -- --schools 500
cargo run --release -p midwest-census --example bench_store -- --rows 200000 --batch 1000 --scan
```

`--release` is required for a meaningful number; the workspace's release profile is the one under
test (`lto = "thin"`, `codegen-units = 1`, `strip = true`). Both harnesses open a `tempfile::TempDir`,
print `note=…` saying the store root is removed at process exit, and leave nothing behind.

### 2.1 `bench_census` — whole-pipeline throughput

Flags: `--schools` (default 500, minimum 1). The corpus is deterministic: a seeded 32-bit LCG
(Numerical Recipes constants, seed `0x5eed_2027`, no `rand` dependency), so two runs with the same
`--schools` build byte-identical corpora. Bound: exactly `--schools` schools, one team and one meet
per school, 8 athletes per school, one event per athlete, two performances per athlete — 35 × schools
appended rows in total. After the merge the distinct count is 27 per school (school, team, meet, 8
athletes, 16 performances) plus its deduplicated events; the run reports and asserts the exact
number. Rows are appended with one `append_many` per 1,000 rows (`APPEND_BATCH`). Every phase asserts
its counts before reporting a rate, so a rate can never describe data the store lost.

| Phase | Items | Unit | What it exercises |
|---|---|---|---|
| `append` | 35 × schools | rows | `append_many` over six tables, batched at 1,000 rows |
| `consolidate` | merged rows | entities | `census::consolidate` merges the append logs into `out/` |
| `census_core` | athletes | athletes | `report::build_census` with `Scope::Core` |
| `census_all_sources` | athletes | athletes | the same snapshot with `Scope::AllSources` |
| `bests` | performances | performances | `bests::build` reduces best marks per (athlete, event) |
| `workbook` | merged rows | entities | `workbook::build` writes a real `.xlsx` into the temp dir |

### 2.2 `bench_store` — Fjall substrate throughput

Flags: `--rows` (default 200,000, minimum 1), `--batch` (default 1,000, minimum 1), `--scan` (off by
default). The dataset is exactly `2 × rows` observations of the `schools` table and `rows` distinct
schools: the same names are appended twice with different `observed_on` values, so the second pass
merges onto the first and a merged row must carry two evidence entries. `--rows` is a deliberate
operator choice — a single append pays one `fdatasync` (`PersistMode::SyncData`) per call, which is
the honest cost of the single-append phase.

| Phase | Items | Unit | What it exercises |
|---|---|---|---|
| `single_append` | `rows` | observations | one `Store::append` per row: the durability floor |
| `batched_append` | `rows` | observations | one `append_many` per `--batch` rows |
| `scan` (with `--scan`) | `2 × rows` | observations | `Store::scan` merges every observation, asserts evidence survived |
| `consolidate` (with `--scan`) | distinct schools | entities | `Store::consolidate` writes `out/schools.jsonl` |

Extra counters: `store_observations` and `store_bytes_on_disk` from `Store::stats`, and
`consolidate_bytes` from the written snapshot (`--scan` only).

### 2.3 Output grammar

Each phase prints three lines, then the summary line:

```text
metric=<phase>_items value=<n> unit=<unit>
metric=<phase>_seconds value=<s> unit=s
metric=<phase>_rate value=<n> unit=<unit>/s
metric=wall_seconds value=<s> unit=s
json={ … }
```

The `json=` line is the machine-readable record: harness name, flags, corpus counts, every phase
(`items`, `seconds`, `rate_per_second`), store counters, and wall time. Read it as follows:

- Compare runs with identical flags on the same machine; the corpus is deterministic, so timing
  differences are real.
- The `single_append` / `batched_append` pair isolates the durability cost: the same rows, 1
  `fdatasync` per row versus 1 per batch. The `--batch` value is the knob.
- `census_core` versus `census_all_sources` isolates the second full scan of the same snapshot;
  `workbook` includes both census scopes plus the bests reduction plus the XLSX write.
- Item counts are converted with `u32::try_from`; a corpus above `u32::MAX` items fails the harness
  instead of reporting a truncated rate.

Not verified in this revision: the harnesses were not executed while this document was written (the
build tree was mid-repair and the workstream is read-only). The phases, bounds and output above are
read from the harness source.

## 3. Hot paths and their current shape

### 3.1 XLSX parse — visitor path (`src/xlsx/parser.rs`, `src/xlsx/parser/rows.rs`)

`visit_records(path, visitor: impl FnMut(SourceRecord) -> Result<()>)` opens the container once to
load shared strings, once for sheet metadata, then once per selected worksheet inside the sheet fold
(`ZipArchive::new(File::open(path))` per sheet). Shared strings are collected once into a `Vec<String>`
with hard ceilings: 2,000,000 entries, 4 MiB per string, 256 MiB total. Each worksheet streams
`quick_xml` events through a reusable `Vec<u8>` buffer (`buffer.clear()` per event) into a row state
machine: a per-row `BTreeMap<usize, String>` keyed by column, the first non-empty row becomes the
header, and each later row is emitted as `SourceRecord { source_key, sheet, excel_row, fields }`.
Every malformed input — duplicate cell position, out-of-order or out-of-range row, bad shared-string
index, text outside a cell — is a typed `bail!` error, never a panic. Cold-start cost therefore
scales with the number of sheets (container reopens), while per-row cost is one map insert and a
clone per retained field; values are materialized as `String` at parse time.

### 3.2 ZIP/deflate handling

The root crate depends on `zip = "=8.6.0"` with `default-features = false, features = ["deflate"]`:
deflate only. Reads go through `bounded_entry`, which rejects a declared uncompressed size over
512 MiB and then enforces the same limit on *actual* decompressed bytes via `BoundedReader` — a lying
central directory cannot get past it. The workbook preflight adds container-wide budgets:
`MAX_ZIP_ENTRIES` 4096 and `MAX_TOTAL_DECOMPRESSED_BYTES` 1 GiB across entries, 512 MiB per entry,
256 MiB of shared-string payload (`src/workbook_ingest/preflight.rs`). The preflight also validates
the zip record structure itself (local/central signatures, EOCD, zip64 extras, data descriptors) in
`src/workbook_ingest/preflight/zip.rs`. Writing copies untouched entries with
`std::io::copy(bounded_entry(file), &mut destination)` without materializing them, preserving each
entry's compression method; generated sheets are `CompressionMethod::Deflated`.

### 3.3 Workbook ingest — calamine path (`src/workbook_ingest.rs`, `stream.rs`, `guards.rs`)

`visit_records` for the root crate is stateful and single-pass per row, but it touches the container
at least three times before the first data row reaches the callback: preflight (decompress-and-scan
every entry with nesting/order guard state machines), `load_sheet_metadata` (OOXML path mapping),
then `calamine::open_workbook` with `Xlsx<BufReader<File>>`. Rows are then streamed incrementally via
`worksheet_cells_reader`'s `next_cell()` inside an `std::iter::from_fn` adapter — no sheet is
materialized. The `SheetState` reuses one `BTreeMap<u32, String>` per row and moves it out with
`std::mem::take` at row end; the byte budgets are 8 MiB per materialized row (values plus retained
headers) and 1 MiB of retained headers across the workbook. Header validation is duplicate-checked
through a `BTreeSet`, and cells in unnamed columns are rejected. Numeric cells are converted through
`finite_number`, which rejects non-finite doubles rather than writing a bogus cell text.

### 3.4 Workbook write (`crates/midwest-census/src/workbook/`, `src/xlsx/writer.rs`)

Census: `workbook::build` calls `rust_xlsxwriter::Workbook::new()` and saves cell-by-cell; both
manifests pin `rust_xlsxwriter = "=0.99.1"` with `features = ["constant_memory"]`, so the writer
streams rows instead of holding the sheet in memory. The builder recomputes nothing: it copies the
typed census from `report` and `bests`.

Root: `append_matches_sheet` streams the input archive entry-by-entry into a `ZipWriter` over a
`BufWriter<File>`, replacing `xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, `[Content_Types].xml`
and the matches sheet; every other entry is copied with bounded reads. Output goes to a temporary
path and is promoted with `fs::rename`; existing outputs are never overwritten.

### 3.5 Census store writes (`crates/midwest-census/src/store/`)

Append path: `append` is `append_many` of a single record. `append_many` serializes each record with
`serde_json::to_vec`, borrows the id back out of the serialized bytes (a structural `Deserialize`
of the `id` field, not a second full parse), and validates every id (non-empty, ≤ `MAX_ID_BYTES` 512)
**before** reserving a single sequence — a rejected batch leaves the keyspace and the counters
untouched. Sequences come from one `fetch_add` per batch; the rows go into one Fjall `WriteBatch` with
one insert each and a single `commit().durability(Some(PersistMode::SyncData))`: one `fdatasync` per
batch, not per row. Keys are `table\0id\0sequence:u64 big-endian`, so a prefix scan reads one
entity's observation history in write order.

Read path: `scan::<T>` iterates the table prefix, deserializes every observation, merges by id into a
`BTreeMap<String, T>`, and applies `Entity::publish`; tables above `MAX_ROWS_PER_TABLE` 20,000,000
abort with a typed error instead of exhausting memory. `consolidate::<T>` is scan plus a JSONL write
(`BufWriter`, `serde_json::to_writer` per row, flush) followed by an explicit `Store::flush`
(`PersistMode::SyncAll`). `stats()` is O(1) per table: it reads the in-memory sequence counters and
the keyspace's `disk_space`, so status commands never scan.

Durability and placement: the database is opened with
`Database::builder(root.join("fjall")).cache_size(CACHE_BYTES)` where `CACHE_BYTES` is 256 MiB
(deliberate: the process shares the machine with a browser); keyspaces `entities`, `journal`, `meta`
are created with `KeyspaceCreateOptions::default`, i.e. **KV separation is not configured**. The
resume journal commits one `SyncData` batch per completed unit of work. Legacy `entities/*.jsonl` and
`journal/*.jsonl` are imported once, in batched commits, behind a `meta` marker.

### 3.6 Root artifact store — contrast (`src/store.rs`, `src/store/backend.rs`)

The acquisition-side store makes different placements: a 32 MiB block cache and
`manual_journal_persist(true)`, keyspaces `documents`, `sources`, `attempts`, `rankings`, a `Mutex`
serialising writers, batch publication capped at `MAX_BATCH_RECORDS` 4,096 and `MAX_BATCH_BYTES`
32 MiB with a typed `StoreError::BatchTooLarge`, single-document cap `MAX_DOCUMENT_BYTES` 32 MiB, and
commits at `PersistMode::SyncAll`. Same substrate, different durability/latency trade-off stated in
code — worth citing whenever the two stores are compared.

### 3.7 Browser pool concurrency (`src/runtime/browser/`)

`pool.rs` is small by design: `PageSlot { page, busy }`, a `VecDeque<Pending>` of
request + `oneshot::Sender` pairs, profile-directory preparation, and `reject_pending`, which drains
the queue with an explicit error when the browser aborts. The scheduler lives in the actor:
`accept_fetch` rejects when the gate is not `Ready` (`BrowserError::HumanRequired`) or the queue is
full (`BrowserError::Unavailable`); `queue_capacity` is `tabs × QUEUE_MULTIPLIER (4)`, matching the
`mpsc` command channel sizing. `schedule_pending` repeatedly takes the first free slot and pops one
pending request, spawning one task per (slot, request) into a `JoinSet` inside a `select!` against
shutdown, wrapped in a `browser.job` span carrying the slot and nonce. `complete_job` frees the slot,
latches challenges, applies a cooldown on HTTP 429, and on panic/cancellation/abort tears down:
abort all jobs, release every slot, revoke the gate, reject pending. Concurrency is bounded by the
validated `tabs` count (1..=8 in `BrowserSettings::validate`), not by origin; per-origin admission
lives in the fetcher.

### 3.8 Adapter fan-out and HTTP fetch (`crates/midwest-census/src/sources/mod.rs`, `net/`)

`sources::CONCURRENCY_BOUND = 8` is the default ceiling on concurrent async operations per adapter
(overridable locally); adapters are plain async functions over `AdapterContext`, called directly
(no trait objects). `net/` makes politeness the first-order constraint: at most one in-flight
request per host, 1 s default spacing (10 s floor for Bound), a hard 2 requests/second ceiling even
for operator-authorized hosts, a 45 s request timeout, three retry attempts with jittered backoff,
32 MiB response cap, and a disk cache keyed by content hash with conditional GETs. Wall-clock
throughput here is set by policy and per-host latency, not by the Rust side — measure before tuning.

### 3.9 Streaming HTML extraction (`src/html_bounds.rs`)

`rewrite_bounded` caps input at 32 MiB, sets lol_html's `MemorySettings` to a 4 KiB preallocated
buffer with an 8 MiB parser memory ceiling, enables strict parsing, and discards rewritten output
(`|_: &[u8]| {}`): only extraction callbacks retain evidence. Input is fed in 4 KiB chunks, so parser
memory is bounded independently of document size.

## 4. Allocation and safety policy

**Unsafe.** `#![forbid(unsafe_code)]` is on `src/lib.rs:1`, `src/main.rs:1`, `src/store.rs:1`, and in
the census crate on `src/lib.rs:25`, `src/main.rs:9`, `src/bin/midwest-serve.rs:10`. The scan counts
0 `unsafe` in both crates, and the gate runs `cargo geiger`. There is no unsafe waiver, and none is
requested (program §6).

**Lints.** Workspace lints forbid `unsafe_code` and deny `unused_must_use`, `dbg_macro`, `todo`,
`unimplemented`, `panic_in_result_fn` for every target. The gate's strict lane adds, for source
targets only, `-D warnings` plus `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`,
`string_slice`, `get_unwrap`, `arithmetic_side_effects`, `as_conversions`,
`let_underscore_must_use`, `await_holding_lock`. Existing violations are frozen in the ratchet (§5),
not waived per site.

**Panic surface, as measured.** Root: 0 `unwrap`, 0 `expect`, 0 assert-family, 0 panic macros in
production code. Census: 1 `unwrap`, 75 `expect` (per the program, uniformly startup-invariant
shapes — regex/static parses), 0 panic macros, 0 production `assert!`-family. The gap is census-only
and is Phase 1 work.

**Errors.** The root store returns typed errors (`StoreError::BatchTooLarge`, …). The census store,
adapters and report path return `anyhow::Result` with `.context(...)` on every fallible step;
`thiserror` appears only in `net/` (`FetchError`). The program records the missing census error
taxonomy as a Phase 3 gap. Typed failure at every bound is the pattern in both crates — a limit is
never enforced with a panic.

**Static dispatch.** The hot APIs are generic, not object-safe: `append_many<T: Serialize>`,
`scan<T: Entity>`, `consolidate<T: Entity>`, `consolidate_table` dispatching through a seven-arm
match; the XLSX visitor is `impl FnMut(SourceRecord) -> Result<()>` threaded through
`visit_records`/`walk_records`, with no `Box<dyn FnMut>`; adapters are free functions called directly
("no trait indirection", `sources/mod.rs`); calamine hands out `DataRef<'_>` borrows that are
materialized into `String` only when a value is actually retained.

**Allocation hygiene visible in the code.** One reusable event buffer per XML reader
(`buffer.clear()`); per-row maps reused and moved out with `std::mem::take`; `Vec::with_capacity`
where the size is known; `BufReader`/`BufWriter` at file and zip boundaries; pass-through zip entries
copied without materializing; strings cloned only at the point a record is built; counters converted
with `try_from` rather than `as`. There is no counting allocator and no per-observation allocation
budget yet — that is Phase 7 work, and until it lands, allocation claims are structural, not
measured.

**Bounds as placement.** Every limit in §3 is a typed error and shows up in the same places:

| Bound | Value | Enforced in |
|---|---:|---|
| `MAX_ZIP_ENTRY_BYTES` | 512 MiB declared and actual, per entry | `src/xlsx.rs`, `metadata.rs::bounded_entry` |
| `MAX_TOTAL_DECOMPRESSED_BYTES` | 1 GiB per workbook | `workbook_ingest/preflight.rs` |
| `MAX_ZIP_ENTRIES` | 4096 | `workbook_ingest/preflight.rs` |
| `MAX_SHARED_STRINGS` / per-string / total | 2,000,000 / 4 MiB / 256 MiB | `src/xlsx.rs`, `parser.rs::load_shared_strings` |
| `MAX_MATERIALIZED_ROW_BYTES` / `MAX_RETAINED_HEADER_BYTES` | 8 MiB per row / 1 MiB per workbook | `workbook_ingest/stream.rs` |
| `MAX_ROWS_PER_TABLE` | 20,000,000 observations | `crates/midwest-census/src/store/` |
| `MAX_ID_BYTES` | 512 bytes | `crates/midwest-census/src/store/` |
| Census cache / root cache | 256 MiB / 32 MiB | `Database::builder(..).cache_size(..)` |
| `MAX_BATCH_RECORDS` / `MAX_BATCH_BYTES` (root) | 4,096 / 32 MiB | `src/store.rs`, `backend.rs` |
| `MAX_ROWS_PER_REQUEST` (Restate ingest) | 50,000 rows per invocation | `crates/midwest-census/src/restate_services/` |
| `MAX_HTML_BYTES` / parser memory | 32 MiB / 8 MiB | `src/html_bounds.rs` |
| HTTP body cap | 32 MiB | `crates/midwest-census/src/net/` |

Storage placement still open: Fjall KV separation is **not** configured (both stores use
`KeyspaceCreateOptions::default`), partition/compaction settings are untouched, and the observation
payload codec is still `serde_json` (program §5, owner decision 2).

## 5. Ratchet workflow

The ratchet freezes debt so hardening cannot silently undo itself. It is a debt gate, not a
performance gate.

**Inputs.** `tools/gate.sh` produces both measurement files in a temp dir:

- `clippy.tsv` — `measure_clippy` runs the strict lane with `--message-format=json`, keeps
  `level == "error"` diagnostics, and emits `crate\tlint\tcount` (one line per crate/lint pair).
- `scan.json` — `cargo xtask scan` emits `{ "crates": { <crate>: { …counts… } },
  "structure": { … } }`; `cargo xtask integrity` prints review candidates but is **not**
  ratcheted yet.

**`cargo xtask ratchet`**

```text
cargo xtask ratchet <baseline> <clippy.tsv> <scan.json>
```

Compares current measurements against the baseline and exits non-zero if any metric grew. It prints
one line per changed metric with `[DOWN]`/`[UP]`, then either `ratchet: no metric grew` or a
`ratchet failures (debt grew)` list. Oversized files are compared as a **set**, not a count: a new
file over 300 lines is a failure even when another file shrank. Absent keys count as zero, so a new
lint or metric at zero is fine and any nonzero value fails.

**`cargo xtask quality-baseline`**

```text
cargo xtask quality-baseline <baseline> <clippy.tsv> <scan.json> [--allow-increase]
```

Rewrites the baseline from current measurements after a burndown. Without `--allow-increase` it
refuses to raise any clippy count, any scan count, or a structure count (`files_over_300_lines` as a
length, `functions_over_60_lines`, `functions_over_25_logical_lines`), printing the proposed increases
and exiting 1. The file is rewritten with the fixed `note` field, `scan` flattened to
`scan["crates"]`, and keys sorted — keep the diff reviewable and commit it with the code that shrank
the numbers.

**`tools/gate.sh`**

```text
tools/gate.sh                                # every lane, ratchet compared against the baseline
tools/gate.sh --update-baseline [--allow-increase]
```

Lanes: `fmt`, `check --all-targets`, `doc`, tests (nextest when installed), strict clippy, production
scan, domain purity, type-integrity report, debt ratchet, deny, audit/machete/geiger (skipped when the tool is
absent), and bench presence. Exact invocations live in `tools/gate.sh`; it is the only supported way
to refresh `tools/quality-baseline.json`, because `cargo xtask ratchet` and `cargo xtask quality-baseline` only compare —
they never measure.

One naming gotcha when reading either file: the clippy TSV keys crates by cargo target name
(`athletic_rust_pipeline`, underscores) while `cargo xtask scan` keys them by directory
(`athletic-rust-pipeline`, hyphens). Both appear in `tools/quality-baseline.json`.

## 6. Known unknowns and next measurement

**Not wired.**

- No criterion/divan: there is no `benches/` directory in either crate, and no criterion, flamegraph,
  pprof or hyperfine reference exists in `src/`, `tools/`, or either `Cargo.toml`. (Program §1 notes
  `perf` is installed on the workstation; nothing in the repo calls it.)
- No regression gate: `tools/gate.sh`'s bench-presence lane checks for a `benches/` directory and
  otherwise prints `no benchmark target exists yet`. The two harnesses in `examples/` are not seen by
  it, so their numbers are not compared to anything.
- No CI (program §2.5), so even a wired threshold would not run on its own.
- No throughput number is recorded in the repo; the historical 559–627k obs/s figures are explicitly
  out-of-repo in program §2.5.
- No allocation accounting: no counting allocator, no `try_reserve` policy applied, no peak-RSS
  capture in the harnesses.
- Both harnesses measure synthetic corpora in a temp dir; they do not model production data
  distribution or contention with a live browser.

**Top measurement candidates**, in descending order of expected information per unit of work:

1. **Append batching and durability mode** (`crates/midwest-census/src/store/`): sweep `--batch`
   and compare `single_append` versus `batched_append`; the production knob equivalent is the batch
   size adapters pass to `append_many` and the per-batch `SyncData` commit. A group-commit policy is
   a design change, so measure first.
2. **Fjall tuning** (program Phase 7): cache size (256 MiB census / 32 MiB root), compaction and
   partition settings, and KV separation for large observation payloads — currently all defaults.
3. **Observation codec** (program §5, owner decision 2): every observation is `serde_json` on write
   and on every scan; `postcard` is the listed alternative. Requires migration plus a dual-read path,
   so it needs a measured budget before it is worth starting.
4. **Repeated full scans**: `report::build_census` scans schools/athletes/coaches/meets,
   `bests::build` scans athletes/meets/events/performances, and `workbook::build` does both plus the
   XLSX write. Quantify scan time and peak memory at scale before considering a shared read model.
5. **Workbook ingest passes** (`src/workbook_ingest.rs`): preflight decompresses and scans the
   container, metadata reads it again, calamine streams it a third time. Measure cost per megabyte
   and whether the preflight pass can validate during the parse instead of before it.
6. **XLSX visitor container opens** (`src/xlsx/parser.rs`): shared strings plus one archive open per
   sheet; measure whether a single open across sheets is worth the API change on wide workbooks.
7. **Browser/HTTP scaling** (program Phase 7 lists N=1/8/64): tabs are validated 1..=8 with a
   `tabs × 4` queue; the fetcher's per-host pacing and one-in-flight-per-host rule will dominate
   most curves, so measure latency and error rate as well as throughput.
8. **Allocation budget per observation**: attach a counting allocator to the store harness and derive
   a per-observation allocation target, then add `try_reserve` at the capped growth points the program
   names (JSONL ingest, parser buffers).

A throughput claim becomes citable only when a `benches/` target exists, the harness numbers move
into it, and `tools/gate.sh` fails on a regression threshold — that is the acceptance target the
program already sets for Phase 7.
