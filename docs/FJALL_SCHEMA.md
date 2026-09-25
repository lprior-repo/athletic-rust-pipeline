# FJALL_SCHEMA.md — Fjall store schema and operational behavior for the census crate

`crates/census-store` owns the census's embedded store (`fjall` 3.1.10). This document describes the
keyspaces, key format, durability, knobs, and sharp edges observed in `crates/census-store/src/`.

Design-side companion: [`FJALL_SCHEMA.md`](../FJALL_SCHEMA.md) — the full schema and lifecycle.

## 1. Keyspaces and tables

Three Fjall keyspaces are created on open (`crates/census-store/src/lib.rs:159-173`):

| Keyspace | Purpose | Tables (logical) |
|---|---|---|
| `entities` | All canonical entities | schools, teams, coaches, athletes, meets, events, performances, source_identities, conflicts, review_cases, coverage, snapshots, source_access, identity_verdicts, source_meets (15) |
| `journal` | Per-phase append logs | phases keyed by phase name |
| `meta` | Markers (import state, sequence metadata) | import markers, sequence tracking |

All three are created with `KeyspaceCreateOptions::default` — KV separation, custom compaction,
and bloom filters are **not configured**.

**2026-09-22 — the meet census table.** `source_meets` holds one row per meet a source enumerated
before its results were read (`census_domain::model::SourceMeetRef`, keyed `{source}:{meet_id}`),
written by the meet census stage through `append_many`: a row is an observation of a published
index, so a later crawl of the same meet merges into the row it wrote before — the earliest sighting
keeps the identity, and a field the later page publishes and the earlier one does not is filled in
rather than left blank. `milesplit_meet_index_<state>_<season>_<year>_v1` journals one entry per
index page the crawl read, so a re-run resumes at the first page this season has not recorded.

**2026-09-22 — the table set grew to fifteen.** The §29-§31 derivation layer (`index::derive`) keeps
four derived indexes and one snapshot row in the same `entities` keyspace as the evidence:
`source_identities` (provider id → canonical row), `conflicts` (merge findings retained),
`review_cases` (one row per review finding, keyed so a repeated finding reuses its case),
`coverage` (per-jurisdiction and per-source measurements), and `snapshots` (one row per finished
pass, keyed `<phase>:<date>`, overwritten so a re-run keeps the latest counts). Their ids are
functions of the finding they name, and the derivation writes them through `replace_many` rather
than `append_many`, so a repeated pass replaces the row it wrote before instead of adding another
observation: a store re-derived three times holds one row per key, not three.

## 2. Key format

### 2.1 Entity keys (`entities` keyspace)

```
<table>\0<id>\0<seq:u64 big-endian>
```

- `<table>`: one of the 15 table names (e.g. `schools`)
- `<id>`: the entity's stable identifier (≤ 512 bytes, non-empty)
- `<seq>`: monotonically increasing sequence number, big-endian encoded for prefix ordering

A prefix scan on `<table>\0` returns all observations for one entity in write order.

### 2.2 Journal keys (`journal` keyspace)

```
<phase>\0<entity_key>
```

- `<phase>`: the phase name (e.g. `athleticnet`)
- `<entity_key>`: the same `<table>\0<id>\0<seq>` format as entity keys

Journal rows carry `{ key, at, payload }` JSON — the key, an ISO timestamp, and the full
serialized entity. Used for replay after a crash.

## 3. Value format

All values are `serde_json::to_vec(entity)`. A `StoreError::Decode{key,source}` is raised
on any row that is not valid JSON for the expected type `T`.

## 4. Read path

`scan::<T>` (`read/mod.rs:134`, over the shared prefix walk):

1. Prefix iterate the `<table>\0` prefix in `entities`.
2. `serde_json::from_slice` each row into `T`.
3. Merge by id into a `BTreeMap<String, T>`.
4. Apply `Entity::publish` to each merged entity.
5. Tables above `MAX_ROWS_PER_TABLE` (20,000,000, `table.rs:19`) abort with a typed error
   (`read/mod.rs:97`).

`consolidate::<T>` (`read/mod.rs:152`), with the typed dispatcher `consolidate_table`
(`read/mod.rs:175`) that keeps callers free of a seven-arm match: scan + write_snapshot +
`flush()`.

`stats()` (`read/mod.rs:202`): exact `AtomicU64` sequence counters per table + `disk_space`
from the keyspace. **This reads exact counters, not LSM `approximate_len` estimates** — the
`StoreStats` doc comment claimed "approximate_len" when this was measured and states the
counters are exact as of 2026-09-22.

## 5. Write path

`append_many` (`write.rs:47`):

1. `Vec::with_capacity(records.len())` staging.
2. `serde_json::to_vec` per record.
3. `observation_id(&value)` validates non-empty, ≤ 512 bytes.
4. One `fetch_add(Relaxed)` per batch to reserve a sequence.
5. One `batch.insert` per row.
6. `.durability(Some(PersistMode::SyncData)).commit()` — one fdatasync per batch.

The reservation is what holds the row ceiling (`write.rs:26`, `batch.rs:26`): a batch that would
take a table past `MAX_ROWS_PER_TABLE` is refused before the counter moves, and the refusal leaves
the counter exactly where it found it.

`append` is `append_many` of a single record (`write.rs:110`). `journal_done` is one-row SyncData
batch (`write.rs:175`).

`replace_many` (`write.rs:137`) is the second write path, added 2026-09-22 for derived state:
one row per entity id, keyed with a fixed sequence of zero and no sequence reserved, in the same
one-`SyncData`-batch shape. A derivation is a function of the store rather than evidence about a
moment in it, so re-running it overwrites its rows in place — the table cannot grow with the pass
count, the 20M cap cannot be exhausted by re-deriving, and `stats()` keeps counting what was
*appended*: a table written only through `replace_many` reports zero there.

**No `try_reserve` calls exist in the census store.** The root crate had ~10 sites (historical: root package deleted 2026-09-23); census
has zero. This is a P7 gap named in PERFORMANCE.md §6.


## 6. Legacy import

The pre-Fjall resume ledger — `<root>/journal/<phase>.jsonl`, one entry per finished unit of work —
is imported by `Store::import_legacy_resume_journals` (`legacy/resume.rs:27`), under the entity
journals' policy because the stakes are the same: this ledger is the store's only record that a
unit of work finished, so an entry that vanishes becomes work a later run repeats.

- A complete line imports byte for byte; a line that is not a JSON object, or carries no `key`,
  fails the import naming the file and the line.
- One unterminated trailing line is the head of a line a writer died in the middle of: the import
  stops at the line boundary before it.
- Rows are buffered through `ImportChunk` (`legacy/chunk.rs:50`) and refused at the row ceiling
  (`legacy/chunk.rs:75`).
- The completion mark (`imported:resume-journals` in `meta`) rides in the same batch as the entries
  it describes, so the two land together or not at all: **the import is exactly-once** — a refusal
  leaves no mark and the import runs again, a finished import is never repeated.

The cost of that shape is memory: one ledger file is one WriteBatch before commit.

## 7. Tuning knobs

| Knob | Census | Root (acquisition) (historical: root package deleted 2026-09-23) | Fjall default |
|---|---|---|---|
| Cache size | 1 GiB (`CACHE_BYTES: u64 = 1024 * 1024 * 1024`, `crates/census-store/src/lib.rs:70`) | 32 MiB (`src/store.rs:25`) | 32 MiB (`db_config.rs:90`) |
| Filter hint | `expect_point_read_hits(true)` on `journal` and `meta` (`crates/census-store/src/lib.rs:162-173`); `entities` keeps default filters so a miss cannot fall through the last level | not configured (`src/store.rs:25`) | not configured (`db_config.rs:90`) |
| Journal persist | default (auto) | `manual_journal_persist(true)` (`backend.rs:33-35`) | auto |
| Worker threads | default | default | `min(cores, 4)` (`db_config.rs:70`) |
| Journal compression | LZ4 > 4096 bytes | same | LZ4 > 4096 (`db_config.rs:86-88`) |
| Data-block compression | disabled | disabled | disabled (`db.rs:602,604`) |
| Bloom filter FPR | default (1e-4) | same | 1e-4 |
| KV separation | **not configured** (`KeyspaceCreateOptions::default`) | same | not configured |
| Write buffer | default (64 MiB/keyspace) | same | 64 MiB/keyspace |
| Max journal size | 512 MiB (then rotate) | same | 512 MiB |

## 8. Sharp edges

1. **20M read-only cap** (`MAX_ROWS_PER_TABLE`): `scan` aborts the entire table scan if
   the prefix walk exceeds 20M rows. A single-table overflow kills `report`, `bests`,
   `workbook`, `consolidate`, and `fjall-stats`.
2. **Scan materializes in memory**: the entire merged `BTreeMap<String, T>` lives in the
   process heap. No streaming output.
3. **Single writer**: `append_many` acquires no lock but commits one WriteBatch at a time.
   Concurrent appends from different tasks serialize at commit.
4. **One ledger is one batch**: the resume-ledger import (`legacy/resume.rs:27`) accumulates a
   whole file into a single WriteBatch before commit, so a very large ledger's rows are resident
   until it lands (bounded by `MAX_ROWS_PER_TABLE`).
5. **Backup and restore are `backup/`, not `Database::snapshot()`**: `backup::backup`
   (`backup/copy.rs:36`) copies a closed store under a lock with a digest manifest, and
   `backup::restore` (`backup/restore.rs:29`) validates that manifest before writing.
   `backup/mod.rs:12-15` records why `Database::snapshot()` alone is not enough: it pins the LSM
   version for reads but stops neither compaction from rewriting SSTs nor the write-ahead journal
   from rotating mid-frame.
6. **No `try_reserve` in census**: all externally-sized growth points (legacy ledger ingest,
   parser buffers, journal payloads, observation batches) use `Vec::with_capacity` or
   `Vec::new()` without bounded pre-allocation.
7. **`println!`/`eprintln!` sit in the entry points, not the library**: the print sites are the
   `cli/` subtree and `bin/`; `bootstrap.rs`, `spawn.rs`, and `restate_services/` carry none, and the
   only print in `census-store` is in a test (`legacy_tests.rs`).
8. **StoreError is thiserror, not anyhow**: contrary to earlier claims that "store.rs uses
   anyhow throughout", census has a dedicated `StoreError` enum (`error.rs:9`)
   with `thiserror` derives. Zero `anyhow` matches under `src/` (historical: root package deleted 2026-09-23).
9. **StoreStats counts are exact, not approximate**: `stats()` reads `AtomicU64` sequence
   counters, not Fjall's `approximate_len` (corrected 2026-09-22, near `pub fn stats`).
10. **Journal payloads clone per row**: `journal_payloads` (`read/mod.rs:260`)
    allocates a fresh `Vec` and clones each payload — no `try_reserve`.
11. **Poison row aborts scan**: a single malformed JSON row in `scan` aborts the entire
    table scan (→ `StoreError::Decode`), and there is no tolerant
    reader anywhere in census to appeal to: the published-snapshot reader is strict as well
    (`read/snapshot.rs`, `StoreError::SnapshotRow`, tested for a bad middle row and
    a bad last row), so corruption fails loudly on every read path rather than being skipped.
12. **No `#[instrument]` on spawn**: zero `.instrument(…)` calls on any production spawn site,
    and the spawns are centralized in `spawn.rs` (one `Spawner`, one `JoinSet`, drain accounting)
    plus the memory-guard watcher.
