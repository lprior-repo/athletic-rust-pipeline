# FJALL_SCHEMA.md — Fjall store schema and operational behavior for the census crate

The census crate (`crates/midwest-census`) uses Fjall 3.1.10 as its embedded store.
This document describes the keyspaces, key format, durability, knobs, and sharp edges
observed in `crates/midwest-census/src/store/`.

Design-side companion: [`FJALL_SCHEMA.md`](../FJALL_SCHEMA.md) — the full schema and lifecycle.

## 1. Keyspaces and tables

Three Fjall keyspaces are created on open (`store/mod.rs:136-139`):

| Keyspace | Purpose | Tables (logical) |
|---|---|---|
| `entities` | All canonical entities | schools, teams, coaches, athletes, meets, events, performances (7) |
| `journal` | Per-phase append logs | phases keyed by phase name |
| `meta` | Markers (import state, sequence metadata) | import markers, sequence tracking |

All three are created with `KeyspaceCreateOptions::default` — KV separation, custom compaction,
and bloom filters are **not configured**.

## 2. Key format

### 2.1 Entity keys (`entities` keyspace)

```
<table>\0<id>\0<seq:u64 big-endian>
```

- `<table>`: one of the 7 table names (e.g. `schools`)
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

`scan::<T>` (`store/read.rs:43-78`):

1. Prefix iterate the `<table>\0` prefix in `entities`.
2. `serde_json::from_slice` each row into `T`.
3. Merge by id into a `BTreeMap<String, T>`.
4. Apply `Entity::publish` to each merged entity.
5. Tables above `MAX_ROWS_PER_TABLE` (20,000,000) abort with a typed error.

`consolidate::<T>` (`store/read.rs:83-97`): scan + write_snapshot + `flush()`.

`stats()` (`store/read.rs:113-133`): exact `AtomicU64` sequence counters per table + `disk_space`
from the keyspace. **This reads exact counters, not LSM `approximate_len` estimates** — the
doc comment at `store/mod.rs:208` claiming "approximate_len" is stale (confirmed by reading
`read.rs:113-133`).

## 5. Write path

`append_many` (`store/write.rs:28-58`):

1. `Vec::with_capacity(records.len())` staging.
2. `serde_json::to_vec` per record.
3. `observation_id(&value)` validates non-empty, ≤ 512 bytes.
4. One `fetch_add(Relaxed)` per batch to reserve a sequence.
5. One `batch.insert` per row.
6. `.durability(Some(PersistMode::SyncData)).commit()` — one fdatasync per batch.

`append` is `append_many` of a single record. `journal_done` is one-row SyncData batch.

**No `try_reserve` calls exist in the census crate.** The root crate has ~10 sites; census
has zero. This is a P7 gap named in PERFORMANCE.md §6.

## 6. Legacy import

`import_observations` (`store/legacy.rs:48-90`):

- Reads a whole JSONL file into ONE WriteBatch.
- Capped by `MAX_ROWS_PER_TABLE` (20M).
- Malformed line aborts with `StoreError::Legacy{path, line}`.
- Writes a `meta` marker after the commit.

**Idempotence is not guaranteed.** If a crash occurs between the commit and the marker write,
the file is re-imported on the next `Store::open`, producing duplicate observations
(`store/legacy.rs:86-88`). FJALL_SCHEMA §8.12 in the HARDENING-PROGRAM.md correctly documents
this window; the module doc comment at `store/mod.rs:33-36` overstates idempotence.

## 7. Tuning knobs

| Knob | Census | Root (acquisition) | Fjall default |
|---|---|---|---|
| Cache size | 256 MiB (`CACHE_BYTES: u64 = 256 * 1024 * 1024`, store/mod.rs:134) | 32 MiB (`src/store.rs:25`) | 32 MiB (`db_config.rs:90`) |
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
4. **Import exactly-once window**: between the WriteBatch commit and the meta marker write,
   a crash causes re-import (`store/legacy.rs:86-88`). The module doc comment claims
   idempotence but the code does not guarantee it.
5. **No backup API in census**: `store/mod.rs:38` imports only `Database, Keyspace,
   KeyspaceCreateOptions, PersistMode`. No `.snapshot()` call. The root crate uses
   `Database::snapshot()` (`src/store/rankings/backend.rs:311,346`). Fjall 3.1.10
   offers `Database::snapshot()` (registry `db.rs:150; snapshot.rs:17-28`).
6. **No `try_reserve` in census**: all externally-sized growth points (JSONL ingest,
   parser buffers, journal payloads, observation batches) use `Vec::with_capacity` or
   `Vec::new()` without bounded pre-allocation.
7. **`println!`/`eprintln!` in production**: 38 occurrences across 7 files (all in
   `cli/` and `bin/`; 0 in library paths). The HARDENING-PROGRAM.md §2.3 claimed 29 —
   stale after burndown.
8. **StoreError is thiserror, not anyhow**: contrary to earlier claims that "store.rs uses
   anyhow throughout", census has a dedicated `StoreError` enum (`store/mod.rs:53-113`)
   with `thiserror` derives. Zero `anyhow` matches under `store/`.
9. **StoreStats counts are exact, not approximate**: `stats()` reads `AtomicU64` sequence
   counters, not Fjall's `approximate_len`. The doc comment at `store/mod.rs:208-210`
   claiming "approximate_len" is stale.
10. **Journal payloads clone per row**: `journal_payloads` (`store/read.rs:151-169`)
    allocates a fresh `Vec` and clones each payload — no `try_reserve`.
11. **Poison row aborts scan**: a single malformed JSON row in `scan` aborts the entire
    table scan (`store/read.rs:51-57` → `StoreError::Decode`). No tolerance precedent
    exists in census (unlike `report/mod.rs:read_rows` which tolerates one bad line).
12. **No `#[instrument]` on spawn**: zero `.instrument(…)` calls on any of the three
    production spawn sites (`bootstrap.rs:160,180`, `restate_services/mod.rs:137`).
