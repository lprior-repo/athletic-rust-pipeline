# ADR-001: Fjall is the system of record; no PostgreSQL for this census

## Status

Accepted. Implemented: census store `crates/midwest-census/src/store/` (keyspaces `entities`,
`journal`, `meta`) and the root acquisition store (`src/store.rs`, `src/store/rankings/`). Two defects
present today are named under Consequences, not described as future work.

## Context

The census retains every observation, source identity, evidence item and reconciliation decision for
a national athlete population, across runs that last hours and are interrupted by politeness delays,
network faults and operators. Two consumers pull in different directions: the **pipeline** must never
overwrite a prior observation and must resume after a crash without babysitting a database server,
while the **recruiter** reads a workbook produced once per run, not an interactive SQL session.

`ARCHITECTURE.md:36` states the constraint directly: "Fjall is the system of record. No PostgreSQL:
the downstream interface is Excel, not interactive SQL." The resulting workload is append-heavy
observations, deterministic sortable keys, prefix scans over one entity's history, and a handful of
full exports per run.

## Decision

Fjall is the system of record for canonical entities (schools, teams, coaches, athletes, meets,
events, performances), source identities and raw observations, and evidence, coverage state and
review outcomes.

* The census crate opens three keyspaces — `entities`, `journal`, `meta`
  (`crates/midwest-census/src/store/mod.rs:135-138`, created at `:241-249` with
  `KeyspaceCreateOptions::default`) — with the seven tables as prefixes inside `entities`
  (`store/mod.rs:140-179`), keyed `<table>\0<id>\0<sequence:u64 big-endian>` so a prefix scan reads
  one entity's history in write order (`store/mod.rs:9-13`; `docs/FJALL_SCHEMA.md:22-30`).
* PostgreSQL, and any second store, is **not** introduced. A query surface, if ever required, is a
  read-only projection over Fjall (`docs/adr/ADR-006-excel-recruiter-query-layer.md:33-34`).
* Batch writes commit with `PersistMode::SyncData`; `Store::flush` upgrades to `SyncAll` at
  consolidation and shutdown (`store/mod.rs:24-27`; `docs/FJALL_SCHEMA.md:79`).
* Observations are append-only; merges are idempotent, commutative and associative, and nothing
  overwrites a prior observation — including school attribution after a transfer
  (`ARCHITECTURE.md:45-46`).
* The workbook is a projection that must reconcile against the store before a run is sealed
  (`crates/midwest-census/src/workbook/mod.rs:3-6`).

## Consequences

* No database server, pool, migration tool or network hop on the critical path: `Store::open` fails
  with the reason when another process holds the store, and concurrent appends serialize at commit
  (`docs/FJALL_SCHEMA.md:121-122`).
* Schema evolution is explicit: new fields are additive in the stored JSON payloads, and a
  key-encoding change needs a revision marker in `meta`, because Fjall enforces no constraint for us.
* The store is bounded by construction: `MAX_ROWS_PER_TABLE = 20_000_000` aborts a scan with a typed
  error (`store/mod.rs:125`; `docs/FJALL_SCHEMA.md:61,116-118`) and a scan materializes the merged map
  in the process heap (`docs/FJALL_SCHEMA.md:119-120`).
* Defect today: the census store has no backup/snapshot API (`store/mod.rs:38` imports none) while the
  root crate uses `Database::snapshot()` (`docs/FJALL_SCHEMA.md` §8, sharp edge 5).
* Defect today: `store/mod.rs:207-208` documents `StoreStats` as `approximate_len` estimates, but
  `stats()` reads exact `AtomicU64` counters (`docs/FJALL_SCHEMA.md:65-68,139-141`) — stale comment.

## Alternatives considered

* **PostgreSQL or any relational warehouse.** Rejected: a second source of truth, a migration surface
  and an availability dependency for a query workload of a few exports per run (`ARCHITECTURE.md:36`).
* **An interactive query service now.** Rejected; admissible only later as a read-only projection
  (`docs/adr/ADR-006-excel-recruiter-query-layer.md:33-34`).
* **A second store beside Fjall.** Rejected by the standing rule against a second convention
  (`RESTATE_WORKFLOWS.md:371-373`).

## Evidence

Read directly: `ARCHITECTURE.md:36,38-41,45-46`; `crates/midwest-census/src/store/mod.rs:9-13,38,125,
135-179,207-210,219-222,241-249`; `store/keys.rs`; `crates/midwest-census/src/workbook/mod.rs:3-6`;
`docs/adr/ADR-006-excel-recruiter-query-layer.md:27-34`; `RESTATE_WORKFLOWS.md:371-373`;
`src/store.rs:141-146`.

Cited through the doc, not re-derived: `store/read.rs` scan/consolidate/stats lines, the 20M cap,
single-writer and legacy-import sharp edges, and the root-crate `Database::snapshot()` sites
(`docs/FJALL_SCHEMA.md:11,55-68,79,116-124`). No heap or restore measurement exists; none is claimed.
