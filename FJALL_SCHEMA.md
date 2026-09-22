# Fjall store schema

Operational companion: [`docs/FJALL_SCHEMA.md`](docs/FJALL_SCHEMA.md) — durability knobs and the
sharp-edge list with current line citations. This file is the design-side schema.

What `midwest-census` actually writes to its embedded store: directories, keyspaces, key bytes,
values, the write and read paths, and the limits the code enforces. Derived from
`crates/midwest-census/src/store/` (primary), `crates/midwest-census/src/cli/` (the `midwest-census`
binary, a thin clap shell),
`crates/midwest-census/src/bin/midwest-serve.rs`, `crates/midwest-census/src/bootstrap/`,
`crates/midwest-census/src/restate_services/`, `crates/census-domain/src/model.rs`, and the pinned
dependency
`fjall = "=3.1.10"` (`crates/midwest-census/Cargo.toml`). Dependency behaviour is cited from the
registry source of that version.

Line numbers are from the tree at the time of writing and will drift while the crate is being
hardened; **symbol names are authoritative**. Nothing here is measured: no store command was run for
this document (the build directory and the live store are owned by other processes), so footprint
numbers below are file-system sizes of the live root, not store statistics.

## 1. Overview

One Fjall database per store root. Fjall is an embedded LSM-tree key-value store: writes land in a
write-ahead journal and a memtable, sealed memtables are flushed and compacted into immutable
sorted tables (module doc, `store/mod.rs` header). The store is the system of record
(`ARCHITECTURE.md`: "Fjall is the system of record").

| Property | Value | Source |
| --- | --- | --- |
| Crate / version | `fjall = "=3.1.10"` (exact pin) | `crates/midwest-census/Cargo.toml` |
| Store root | `--store` (CLI), `--data-dir` (serve), default `var/midwest-census` | `cli/`, `bootstrap/options.rs::ServeOptions` |
| Database directory | `<root>/fjall` (`const DB_DIR`) | `store/mod.rs` |
| Unified cache | 256 MiB, set explicitly (`const CACHE_BYTES`) | `store/mod.rs`, `Store::open` |
| Entities table cap | 20,000,000 observations per table (`MAX_ROWS_PER_TABLE`) | `store/mod.rs` |
| Entity id cap | 512 bytes (`MAX_ID_BYTES`) | `store/mod.rs` |
| Ingest request cap | 50,000 rows (`MAX_ROWS_PER_REQUEST`) | `restate_services/mod.rs` |
| Worker threads | not configured by the crate; fjall default `min(cores, 4)` | fjall 3.1.10 `src/db_config.rs` |

### Directory layout

Created by `Store::open` before the database is opened: `<root>/http` (fetcher cache) and
`<root>/out` (snapshots and reports). Fjall creates `<root>/fjall` itself.

```text
<root>/
  fjall/                      # DB_DIR: the Fjall database (exclusive lock lives here)
    lock                      # fjall LOCK_FILE, exclusive; see "single writer"
    version                   # fjall VERSION_MARKER (format version)
    <n>.jnl                   # active write-ahead journal, e.g. 3.jnl
    keyspaces/<id>/           # one directory per keyspace, id 0 = fjall's internal meta tree
  http/                       # HTTP cache: <hash>.body + <hash>.meta.json (not Fjall)
  out/                        # CLI consolidate/report snapshots: <table>.jsonl, report*.json,
                              # census-by-state*.csv, best-results-<cohort>.*, workbook
  entities/<table>.jsonl      # pre-Fjall append logs; also the Restate consolidate output
  journal/<phase>.jsonl       # pre-Fjall resume ledger (import source only)
```

The live root (`var/midwest-census`) matches this layout, including both `entities/` (e.g.
`athletes.jsonl` at 1.0 GB) and `out/` (`athletes.jsonl` at 779 MB). It also holds
`out.pre-rust/` and a `.merged-athleticlive` marker, which no store code creates; treat those as
operator state.

Fjall's own key for the file names: `LOCK_FILE = "lock"`, `VERSION_MARKER = "version"`,
`KEYSPACES_FOLDER = "keyspaces"`, active journal `"0.jnl"` on a fresh database
(fjall 3.1.10 `src/file.rs`, `src/db.rs`). The live store is on `3.jnl`, i.e. the writer has
rotated. Keyspace directories are numeric ids; the id→name mapping is stored inside fjall's
internal meta keyspace (id `0`), not in this repository, so the numbers must not be treated as
configuration.

### Lifecycle

| Step | CLI (`midwest-census`) | Service (`midwest-serve`) |
| --- | --- | --- |
| Open | `Store::open(&cli.store)` in `main` | `spawn_blocking(Store::open)` in `bootstrap::serve_until` |
| Work | one subcommand | Restate handlers share `Arc<Store>` |
| Flush | `consolidate` (via `Store::consolidate`), and `Store::open`'s legacy import | `store.flush()` after the region drains |
| Close | `Store` dropped at process exit | `flush()`, then `drop(store)` |

`Store::open` performs, in order: create `http`/`out`; open the database with
`cache_size(CACHE_BYTES)`; open the three keyspaces; seed the per-table sequence counters from the
highest key present; run the one-time legacy import. There is no explicit `close`; dropping the
`Store` drops the `Database`, and the only shutdown-time durability action is `flush()`.

### Single writer

Opening the database takes an exclusive lock (`LockedFileGuard::try_acquire` on `fjall/lock`,
fjall 3.1.10 `src/db.rs`), so a second **process** cannot open a store that a live writer holds —
this is the rule stated in `main.rs` ("One process owns the store at a time"), `README.md`
("Never open a live Fjall directory from a second process") and `HANDOFF.md`. Inside one process
the store is shared: `midwest-serve` hands `Arc<Store>` to every handler, and heavy jobs run on
`spawn_blocking` behind a semaphore (`--max-concurrent`, default 8). Concurrent writers are
therefore normal in-process; cross-thread coordination is fjall's journal serialisation plus the
per-table `AtomicU64` sequence counters.

## 2. Keyspaces and tables

Three Fjall keyspaces exist, named by constants in `store/mod.rs`:

| Keyspace | Constant | Contents | Key shape |
| --- | --- | --- | --- |
| `entities` | `ENTITIES` | every observation row of all fifteen tables | `<table>\0<id>\0<seq:u64 BE>` |
| `journal` | `JOURNAL` | completed-unit resume entries | `<phase>\0<key>` |
| `meta` | `META` | import markers only | `imported:<table>`, `imported:resume-journals` |

There is no fourth keyspace and no per-table keyspace. The **tables** are logical partitions
inside `entities`, selected by the table-name byte prefix. `Table` (`store/table.rs`, `enum Table`)
is `Schools, Teams, Coaches, Athletes, Meets, Events, Performances, SourceIdentities, Conflicts,
ReviewCases, Coverage, Snapshots, SourceAccess, IdentityVerdicts, SourceMeets` (fifteen as of
2026-09-22; `Table::ALL` is the authority), with `Table::file()` giving the wire/prefix name
(`schools`, `teams`, … `source_meets`), `Table::ALL` the ordered list, and `Table::from_wire` the
ingest-side parser (unknown names are rejected so a typo cannot create a table nobody scans).

Correction to `ARCHITECTURE.md`: that file lists `schools … performances` as "census keyspaces"
alongside `entities`, `journal`, `meta`. Those names are tables inside the `entities` keyspace; only
`entities`, `journal`, `meta` are Fjall keyspaces.

| Table | Entity type | Observation body |
| --- | --- | --- |
| `schools` | `CanonicalSchool` | school with aliases, association, classification, enrollment, sites |
| `teams` | `CanonicalTeam` | (school, sport, gender, school year) |
| `coaches` | `CanonicalCoach` | coach roles plus contact fields under the publication contract |
| `athletes` | `CanonicalAthlete` | athlete identity, cohort, grade observations, profile URLs |
| `meets` | `CanonicalMeet` | meet identity, date/end date, location, level, sports |
| `events` | `CanonicalEvent` | event ontology row per meet (kind, gender, division, round) |
| `performances` | `CanonicalPerformance` | athlete/team/event/meet mark with provider `source_key` |

The `journal` keyspace holds one row per completed unit of work; `phase` is an
adapter-supplied string, not an enum. The live root's `journal/*.jsonl` files show 38 phases in use:
`milesplit_teams_<state>` and `milesplit_rosters_<state>` for 12 states (ia, il, in, ks, mi, mn,
mo, nd, ne, oh, sd, wi), `wiaa_schools`, `wiaa_coaches`, `wiaa_results`, `ihsa_schools`,
`kshsaa_schools`, `mshsl_schools`, `mshsl_coaches`, `ndhsaa_schools`, `ndhsaa_coaches`,
`nsaa_schools`, `nsaa_coaches`, `athleticlive_meets`, `athleticlive_rosters`, `wayzata_schedule`.
Each name is the string that adapter passes to `Store::journal_done`; a phase that has never
completed a unit has no row and no file.

The `meta` keyspace holds one `imported:<table>` marker per table plus
`imported:resume-journals`; nothing else writes to it.

## 3. Key and value encoding

### Observation keys (keyspace `entities`)

```text
key   = <table bytes> 0x00 <id bytes> 0x00 <sequence: u64 big-endian>
value = serde_json::to_vec(entity)
```

`table_prefix(table)` emits `<table>\0`; `observation_key(table, id, sequence)` appends the id, a
separator `0x00`, then the eight big-endian sequence bytes (`store/keys.rs`).

Byte order is load-bearing:

* The table name leads, so each table occupies one contiguous prefix range and `Keyspace::prefix`
  scans exactly that table.
* The id follows, so all observations of one entity are adjacent.
* The sequence is the fixed-width tail in big-endian order, so byte order equals numeric order: a
  prefix scan over one entity returns its observations in append order, and `Store::scan` folds them
  through `Entity::merge` in that order.
* Parsing is positional: `split_observation_key` takes `len-8` as the sequence, checks the byte
  before it is `0x00`, and splits the remaining text on the first `0x00`. A reverse search for the
  separator would misparse every key whose low sequence byte is `0` (every 256th key); the regression
  test `keys_with_a_zero_low_sequence_byte_still_reopen` exists for exactly that bug, and the
  temporary conversion harness `split_observation_key_indexed` plus
  `tmp_converted_split_matches_the_indexed_original_on_a_corpus` cross-checks the parsed form on a
  corpus that includes empty ids, spaces, non-ASCII, an embedded `0x00`, and a 600-byte id.

Ids are the canonical `Id<T>` string (see below), validated before use: `observation_id` borrows the
`id` field out of the serialized buffer with a `#[serde(borrow)]` view (`ObservationId`) and rejects
an empty id or one longer than `MAX_ID_BYTES` (512). The stated rationale is fjall's assertion that
keys stay below 64 KiB (module comment, `store/keys.rs`).

### Journal keys (keyspace `journal`)

```text
key   = <phase bytes> 0x00 <key bytes>          # journal_key(phase, key)
value = {"key": <key>, "at": <iso-8601>, "payload": <caller JSON>}
```

`journal_keys(phase)` scans prefix `<phase>\0` and returns the suffixes as a `HashSet<String>` (the
resume set); `journal_payloads(phase)` scans the same prefix and extracts `payload`. Because the
phase separator is a literal `0x00`, a phase name containing NUL would overlap another phase's range;
no adapter uses one.

### Meta keys (keyspace `meta`)

`imported:schools` … `imported:performances` (from `imported_marker(table)`) and
`imported:resume-journals`; value is the byte string `b"1"`. Presence of the marker is the only
import guard.

### Deletion and tombstones

Fjall supports them (`WriteBatch::remove`, `ValueType::Tombstone`, `Database::delete_keyspace`
exist in 3.1.10), but **this store never issues a delete**: the crate contains no `remove` on a
keyspace, no `delete_keyspace`, and imports only `Database, Keyspace, KeyspaceCreateOptions,
PersistMode`. An observation cannot be retracted; a later observation can only be outvoted by the
merge rules in §4. The one overwrite in the design is the journal, whose keys are upserts
(§5), and the `meta` markers, which are re-written with the same value.

### Identifier type

`model::Id<T>` is `#[serde(transparent)]` over a `String` with a `PhantomData<T>` tag, so a
`SchoolId` and a `TeamId` serialise and key identically to their string, and JSON carries no tag.
`Id::mint(prefix, parts)` produces `<prefix>_<16 lowercase hex>`: the first 64 bits of
`SHA-256(prefix ‖ 0x1f ‖ part_1 ‖ 0x1f ‖ part_2 …)` — the prefix first, then each part preceded by
the byte `0x1f`. Aliases `SchoolId`, `TeamId`, `CoachId`, `AthleteId`,
`MeetId`, `EventId`, `PerformanceId` tag the seven entity kinds, and each entity mints from its own
natural key (school: state + compressed normalized name; team: school + sport + gender + school
year; athlete: school + normalized name + grad year + gender; meet: state + date + normalized name;
performance: athlete + meet + event kind + date + provider `source_key`).

There is **no sealed trait and no `Id`-typed key at the store boundary**: `entities` keys are built
from `&str` (`Entity::entity_id()`), `append`/`append_many` accept any `Serialize`, and the typed
identity lives only in `model`. Key-type mistakes are therefore caught by serde/parse errors at read
time, not by the compiler.

## 4. Entity model on disk

All seven canonical entities are persisted, one table each, each row a complete JSON document.
`Entity` (`store/entities.rs`) supplies `entity_id`, `merge`, and two read-time hooks (`publish`,
`withheld_mailboxes`) applied by every read.

| Entity | Evidence carried | External identities | Other provenance |
| --- | --- | --- | --- |
| `CanonicalSchool` | `evidence: Vec<Evidence>` | `source_identities` | `aliases`, websites |
| `CanonicalTeam` | `evidence` | `source_identities` | — |
| `CanonicalCoach` | `evidence` | `source_identities` | `professional_email`, `phone` |
| `CanonicalAthlete` | `evidence` | `source_identities` | `public_profile_urls`, `observed_grades` (each with `SourceRef`) |
| `CanonicalMeet` | `evidence` | `source_identities` | `source_urls` |
| `CanonicalEvent` | `evidence` | — | `source_labels: Vec<SourceEventLabel>` |
| `CanonicalPerformance` | `evidence` | — | `source_key` (provider result key) |

`Evidence` = `{ source: SourceRef{id, url?}, method, observed_on, note? }` with
`EvidenceMethod ∈ {fetched, parsed, derived, inherited}`; `SourceIdentity` =
`{ namespace: SourceNamespace, id, url? }` with namespaces for MileSplit, TFRRS, DirectAthletics,
state associations, timers, `legacy_athletic_net` and an open `Other(String)`. Evidence lives
inside the observation body — there is no separate evidence table — and `union_vec` during merge
accumulates it, so a row consolidated from N observations carries the union of all their evidence.

Merge rules (`impl Entity for …`), which is what makes append-only safe:

| Entity | Fill-if-absent fields | Unioned | Recomputed |
| --- | --- | --- | --- |
| School | city, association, classification, enrollment, websites | aliases, source_identities, evidence; `co_op` OR | longer `name` wins when it extends the shorter one |
| Team | level | source_identities, evidence | — |
| Coach | professional_email, phone | source_identities, evidence | `publish()` drops consumer mailboxes |
| Athlete | — | known_names, sports, public_profile_urls, source_identities, evidence, observed_grades | `identity_confidence` from grade observations |
| Meet | location, end_date; level if `Unknown` | sports, source_identities, source_urls, evidence | — |
| Event | — | source_labels, evidence | — |
| Performance | wind_mps, place, observed_grade, timing | evidence | — |

Two entity-specific rules are applied on **read**, inside `Store::scan`, so they hold for the
report, workbook, snapshot and Restate handlers alike:

* `CanonicalCoach::publish` clears `professional_email` when `model::professional_email` rejects it
  (consumer mailbox or malformed) and sets `email_withheld`; `withheld_mailboxes` reports the count,
  which `Store::consolidate` sums in the same pass that writes the snapshot.
* `email_withheld` is `#[serde(skip)]`: it is **not on disk** — the byte stream keeps whatever the
  adapter wrote, and the gate is re-derived on every read.

## 5. Write path

| Writer | Keyspace | Batching | Durability |
| --- | --- | --- | --- |
| `append_many` / `append` | `entities` | one `WriteBatch` per call | `durability(SyncData)` then `commit()` |
| `journal_done` | `journal` | one-row `WriteBatch` | `durability(SyncData)` then `commit()` |
| `import_observations` | `entities` | one batch per legacy file | `durability(SyncData)` then `commit()` |
| `import_legacy_resume_journals` | `journal` | one batch for every phase file | `durability(SyncData)` then `commit()` |
| `import_legacy` markers | `meta` | single `Keyspace::insert` | journal persisted `Buffer`, then `flush()` = `SyncAll` |

`append_many` order of operations: serialize every record; parse and validate its id; `reserve` the
sequence range (a single `fetch_add` on the table's `AtomicU64`); build the batch with
`base + offset` per row; commit. Every record is validated before a single sequence is reserved, so a
rejected batch leaves both the keyspace and the counters untouched (test
`oversized_and_empty_ids_are_rejected_before_any_write`). A commit that fails inside fjall does not
roll the counter back: the reserved sequence numbers stay spent, so that table's numbering gains a
gap. Gaps are harmless — nothing reuses them, and reopening reseeds from the highest stored key —
and no stored observation can be overwritten by a later append.

Atomicity: fjall documents `commit` as "Commits the batch to the Database atomically"
(fjall 3.1.10 `src/batch/mod.rs`), which is what makes one adapter page one unit. This store only
ever touches one keyspace per batch, so no cross-keyspace invariant depends on that guarantee.
Batching is per call, not per observation: `append_many` carries the whole slice, `journal_done`
carries exactly one row. The largest batches are the pre-Fjall imports (one file each, capped at
`MAX_ROWS_PER_TABLE`) and Restate ingest (≤ 50,000 rows per request).

Durability knobs actually in play:

| Mode | fjall semantics (3.1.10 `src/journal/writer.rs`) | Where used |
| --- | --- | --- |
| `Buffer` | flush to OS buffers; lost on power loss or OS crash | fjall's default for `db.batch()` and for plain `Keyspace::insert`; only reachable here via the `meta` markers |
| `SyncData` | `fdatasync`; survives an application crash | every `entities`/`journal` batch |
| `SyncAll` | `fsync` data + metadata | `Store::flush`, called by `consolidate` and at service shutdown |

The crate never sets `manual_journal_persist`, so fjall's implicit post-commit persist stays on; the
explicit `SyncData` upgrade is what the code adds. No CLI flag, env var or config file exposes a
durability choice.

Sequence counters are **not persisted**: `Store` holds `sequences: BTreeMap<&'static str,
AtomicU64>`, seeded at open from the highest key in each table (`last_sequence`) and bumped by
`reserve`. Reopening therefore resumes rather than restarts (test
`observations_survive_reopen_without_overwriting`), and no observation is ever overwritten
because the sequence component is always fresh.

Error taxonomy: the store returns a dedicated `StoreError` enum under `StoreResult<T>`
(`store/mod.rs`), not `anyhow`. Variants cover the failure homes directly: `Open`/`Flush`/
`Write`/`Read` wrap `fjall::Error`; `Decode`/`Json` wrap `serde_json::Error` with the offending
key or a `detail` naming the step; `Invariant { detail }` carries the writer's own broken
assumption; `TooManyRows { table, max }` is the `MAX_ROWS_PER_TABLE` rejection in `scan` and the
importer; `JournalTooLarge`, `CounterOverflow`, `Io { path }` and `Legacy { detail }` cover the
resume journal ceiling, sequence exhaustion, sidecar/artifact I/O and the one-time legacy import.
Every fallible fjall or filesystem call is either `?`-propagated or mapped into one of those
variants; no error is swallowed.

### Import path (pre-Fjall journals)

`Store::open` always calls `import_legacy` before a command sees the store. Per table: if the
`meta` marker is absent and `<root>/entities/<table>.jsonl` exists, every non-empty line is inserted
under a fresh sequence (id parsed from the line, cap enforced); then the marker is written. Phase
files under `<root>/journal` are imported into the `journal` keyspace under `imported:resume-
journals`. Each entity file is committed as a single batch and the marker is written afterwards, so a
crash before the commit imports nothing and the next open starts that file clean; a crash after the
commit but before the marker re-imports the whole file, which duplicates its observations (see §8,
item 12). The method ends with `flush()` (`SyncAll`). The legacy JSONL files are left in place as the
record of what the database was built from, which is why `entities/*.jsonl` still exists in the live
root.

## 6. Read path

`Store::scan::<T>(table)` is the only entity read. It iterates `entities.prefix(<table>\0)`,
deserializes each row, merges by `entity_id` into a `BTreeMap<String, T>` (so the result is sorted by
id), applies `publish`, and returns `Vec<T>`. Consequences the code makes explicit: it holds the
whole merged table in memory, it enforces `MAX_ROWS_PER_TABLE` (`bail!` — "holds more than 20000000
observations; consolidate with a smaller window"), and merging happens at read time so an
unconsolidated store still reports one row per entity (test
`census_reads_merged_observations_without_consolidating` in `report.rs`).

There are **no point reads and no page/window API**. The crate contains no `get` on the `entities`
or `journal` keyspaces: `meta.contains_key` (import markers) is the only key lookup, everything else
is a prefix scan. Downstream consumers:

| Consumer | Read |
| --- | --- |
| `report::build_census`, `bests::build`, `workbook::build` | `store.scan` for the tables they need |
| Adapters that need existing state (e.g. `wayzata`, `athleticlive_athletes`, `wiaa_results`) | `store.scan` of schools/meets |
| `athleticnet` school resolution | `out/schools.jsonl` snapshot, when present |
| `Store::consolidate` | `scan` + write `<out_path>.jsonl` |
| Restate `Census::status` | `Store::stats` |

`Store::stats` returns `StoreStats { tables: Vec<(String, u64)>, observations: u64, bytes_on_disk:
u64 }`. Per-table counts are read from the in-memory sequence counters (`counter.load(Relaxed)`),
i.e. **observations ever appended to that table**, exact and monotone — not the LSM tree's estimate.
`bytes_on_disk` is `self.entities.disk_space()`, the `entities` keyspace tree only.

`midwest-census fjall-stats` (`Command::FjallStats` → `print_store_stats`) prints `store\t<root>`,
then one `<table>\t<observations>` line per table in `Table::ALL` order (`schools`, `teams`,
`coaches`, `athletes`, `meets`, `events`, `performances`), then `observations\t<sum>` and
`bytes_on_disk\t<entities keyspace bytes>` — tab-separated, no header.

`midwest-census import-legacy` prints the same block after listing, per table, the legacy file size
or `absent`, plus the `journal` directory path. The Restate `Census::status` reply carries the same
numbers as `{table, rows}` pairs plus `observations`, `bytes_on_disk`, `today`.

## 7. Backup, restore, footprint

* **No backup API is used.** The crate imports only `Database`, `Keyspace`, `KeyspaceCreateOptions`
  and `PersistMode`; fjall 3.1.10 offers `Database::snapshot()` but nothing calls it. There is no
  in-process export, no incremental backup, no checksum manifest.
* **Documented procedure** (`docs/OPERATIONS.md`): cold copy only — stop the unit (drain certificate
  printed), copy the whole `--data-dir`, start again. The store and the HTTP cache are a matched
  pair; a cache copy without the store is worthless. `README.md` and `HANDOFF.md` add the rule that
  a live directory is never opened from a second process.
* **Restore** is the same copy in reverse: the store has no restore code path. Because sequences are
  reseeded from stored keys at open, a restored directory behaves exactly like the original.
* **Pre-Fjall migration** is the only schema migration: `midwest-census import-legacy` (and every
  other command, since the import runs in `Store::open`), guarded by the `meta` markers. Operators
  are told to keep the old journals until `report` matches the pre-migration numbers.
* **Footprint measurement** is `bytes_on_disk` from `fjall-stats` / status. It reports the `entities`
  keyspace's LSM-tree size (`Keyspace::disk_space` → `tree.disk_space`, fjall 3.1.10
  `src/keyspace/mod.rs`), which **excludes** the `journal` and `meta` keyspaces and the write-ahead
  journal. Fjall's `Database::disk_space()` (journal + all keyspaces, `src/db.rs`) is not called.
  For an end-to-end figure, measure the directory.
* On-disk reality worth planning around: the consolidated JSONL snapshots are not small. In the live
  root, `out/athletes.jsonl` is 779 MB, `out/performances.jsonl` 116 MB, `entities/athletes.jsonl`
  1.0 GB and `entities/teams.jsonl` 60 MB — i.e. the materialized read model dwarfs the store itself
  for a store of this size. These are file sizes, not database statistics.

Build-time knobs in play (repo choice vs fjall default):

| Knob | Value | Origin |
| --- | --- | --- |
| Block cache | 256 MiB | repo (`cache_size(CACHE_BYTES)`); fjall default 32 MiB |
| Write buffer / memtable | fjall default 64 MiB per keyspace | not configured by the repo |
| Max journal size | fjall default 512 MiB, then rotate | not configured by the repo |
| Journal compression | LZ4 above a 4096-byte threshold (`lz4` is a default feature) | fjall default; the repo does not change it |
| Data-block compression | disabled by fjall's own tree config (`CompressionPolicy::disabled()`) | fjall 3.1.10 `src/db.rs`; the repo does not change it |
| Bloom filters | fjall default `FalsePositiveRate(0.0001)` | fjall; not configured by the repo |
| Keyspace create options | `KeyspaceCreateOptions::default` for all three keyspaces | repo (`Store::open`); no compaction filter, default compaction strategy |

## 8. Operational limits and sharp edges

1. **Open is O(keys), on every command.** `Store::open` seeds sequences by walking each of the seven
   table prefixes and decoding every key (`last_sequence`). Cost grows with the store, and every CLI
   invocation pays it before doing any work.
2. **Open can write.** The legacy import runs inside `Store::open`, so the first open of a
   pre-Fjall store mutates it (`SyncData` batches plus a final `SyncAll`), takes the exclusive lock,
   and can fail the command for reasons unrelated to that command.
3. **The 20M row cap is read-only.** `MAX_ROWS_PER_TABLE` is enforced in `scan` and in
   `import_observations`, **not** in `append_many`. A table can be grown past the cap, after which
   every read of that table fails with a typed error — including `report`, `bests` and `workbook`.
   The bail message recommends "a smaller window", but `scan`/`consolidate` take no window, page or
   range parameter; the only recovery is a different process trimming the table, which the store
   offers no API for.
4. **Reads materialise the table.** `scan` builds a `BTreeMap<String, T>` of every merged entity, so
   peak memory scales with the entity count, not the batch size. Nothing streams.
5. **`bytes_on_disk` is not the store's footprint.** It covers the `entities` keyspace only
   (see §7); the write-ahead journal (active or sealed) and the `journal`/`meta` keyspaces are
   invisible to it. The `StoreStats` doc comment describing counts as the LSM tree's
   `approximate_len` estimates is stale: the implementation reads the exact sequence counters.
6. **Two snapshot locations.** CLI `consolidate` writes `out/<table>.jsonl`
   (`census::consolidate`), while the Restate `Census::consolidate` handler writes
   `entities/<table>.jsonl` via `store.table_path(table)`. Both files are consolidated read models
   with different paths, and both exist in the live root; tooling must know which one it is reading.
7. **The journal is a ledger, not a log of history.** `journal_done(phase, key, payload)` upserts: the
   same `(phase, key)` overwrites its payload, and re-completing a unit therefore rewrites rather
   than appends. Nothing prunes it — no delete, no TTL, no compaction filter — so the `journal`
   keyspace keeps one row per distinct unit ever completed.
8. **Observation ranges are prefix-delimited, not name-exact.** `table_prefix` terminates the table
   name with `0x00`, so no table's key range can overlap another's even if one name were a prefix of
   another. The same construction makes a phase name containing a NUL byte ambiguous in the
   `journal` keyspace (`<phase>\0<key>`); no adapter generates one, and nothing validates it.
9. **No compaction filter, no retention.** Observations are never compacted away by the store's own
   rules; only fjall's LSM compaction applies, and consolidated output is written by overwriting the
   JSONL file, not by deleting store rows.
10. **One process only.** In-process concurrency is supported (`Arc<Store>` in the service, with
    `--max-concurrent`, default 8, gating heavy jobs), but a second process cannot open the
    directory at all. Sequence safety depends on that: the counters are in memory, so two writers
    would hand out the same sequences.
11. **Durability ends at `SyncData` unless something calls `flush()`.** A CLI run that neither
    consolidates nor imports at open exits without `SyncAll`; the service flushes after the region
    drains. The tail is still `fdatasync`-durable, and the resume journal is durable per completed
    unit, so a killed run redoes work at most for the in-flight unit.
12. **Import is exactly-once only if the crash lands before the commit.** `import_observations`
    commits one batch per file and the marker is written afterwards, so a crash between the commit
    and the marker re-imports the whole file on the next open, inserting every row a second time
    under fresh sequences. Merging at read time hides the duplicate entity but not the duplicate
    evidence, which is what the module comment's promise ("finishes importing on the next open
    without duplicating observations") covers only for crashes that happen before the commit.
