# ADR-001: Fjall is the system of record; no PostgreSQL for this census

## Status

Accepted. Implemented: census store `crates/census-store/src/` (keyspaces `entities`,
`journal`, `meta`) and the root acquisition store (`src/store.rs`, `src/store/rankings/`)
(historical: root package deleted 2026-09-23). The two defects this ADR first named under
Consequences are recorded there as closed, each with the code that closed it.

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

* The store opens three keyspaces — `entities`, `journal`, `meta` — all with
  `KeyspaceCreateOptions::default` (`crates/census-store/src/lib.rs:153-161`) — with the tables as
  prefixes inside `entities` (`crates/census-store/src/table.rs:72-104` names them; sixteen today),
  keyed `<table>\0<id>\0<sequence:u64 big-endian>` so a prefix scan reads one entity's history in
  write order (`crates/census-store/src/keys.rs:40-48`; `docs/FJALL_SCHEMA.md:10-19,39-51`).
* PostgreSQL, and any second store, is **not** introduced. A query surface, if ever required, is a
  read-only projection over Fjall (`docs/adr/ADR-006-excel-recruiter-query-layer.md:33-34`).
* Batch writes commit with `PersistMode::SyncData`; `Store::flush` upgrades to `SyncAll` at
  consolidation and shutdown (`crates/census-store/src/lib.rs:40-42`; `docs/FJALL_SCHEMA.md:99`).
* Observations are append-only; merges are idempotent, commutative and associative, and nothing
  overwrites a prior observation — including school attribution after a transfer
  (`ARCHITECTURE.md:45-46`).
* The workbook is a projection that must reconcile against the store before a run is sealed
  (`crates/census-service/src/census/state.rs:13-16` refuses a seal over an unreconciled workbook;
  the sheets are written from the typed store in `crates/census-report/src/workbook/mod.rs:23-24`).

## Consequences

* No database server, pool, migration tool or network hop on the critical path: `Store::open` fails
  with the reason when another process holds the store (`crates/census-store/src/lib.rs:149-152`),
  and concurrent appends serialize at commit (`docs/FJALL_SCHEMA.md:159-160`).
* Schema evolution is explicit: new fields are additive in the stored JSON payloads, and a
  key-encoding change needs a revision marker in `meta`, because Fjall enforces no constraint for us.
* The store is bounded by construction: `MAX_ROWS_PER_TABLE = 20_000_000`
  (`crates/census-store/src/table.rs:19`) aborts a scan with a typed error
  (`docs/FJALL_SCHEMA.md:78-79`), and a scan materializes the merged map in the process heap
  (`docs/FJALL_SCHEMA.md:157-158`).
* Closed 2026-09-22 (`7790501`): the cold backup entry points exist — `Store::backup`
  (`crates/census-store/src/backup/copy.rs:36`), `Store::restore` (`backup/restore.rs:29`) and
  `Store::integrity` (`backup/integrity.rs:23`). The half that is still absent is the live one: no
  `fjall::Snapshot` is exposed, so a copy of an open store stays refused rather than published
  (`crates/census-store/src/backup/mod.rs:9-27`).
* The root crate used `Database::snapshot()` (historical: root package deleted 2026-09-23;
  census backup is in `docs/FJALL_SCHEMA.md` §8, sharp edge 5, and `docs/FJALL_BACKUP.md`).
* Closed 2026-09-22 (`7790501`): `StoreStats` is documented as exact counts, not LSM
  `approximate_len` estimates (`crates/census-store/src/lib.rs:103-105`), which is what `stats()`
  reads (`docs/FJALL_SCHEMA.md:85-88,179-180`).

## Alternatives considered

* **PostgreSQL or any relational warehouse.** Rejected: a second source of truth, a migration surface
  and an availability dependency for a query workload of a few exports per run (`ARCHITECTURE.md:36`).
* **An interactive query service now.** Rejected; admissible only later as a read-only projection
  (`docs/adr/ADR-006-excel-recruiter-query-layer.md:33-34`).
* **A second store beside Fjall.** Rejected by the standing rule against a second convention
  (`RESTATE_WORKFLOWS.md:371-373`).

## Evidence

Read directly: `ARCHITECTURE.md:36,38-41,45-46`;
`crates/census-store/src/lib.rs:40-42,103-105,113-114,149-161,178-189,213-217`;
`crates/census-store/src/keys.rs:40-48`; `crates/census-store/src/table.rs:19,72-128`;
`crates/census-store/src/read/mod.rs:134,152-175,202`;
`crates/census-store/src/backup/{copy.rs:36,restore.rs:29,integrity.rs:23}`;
`crates/census-service/src/census/state.rs:13-16`; `crates/census-report/src/workbook/mod.rs:23-24`;
`docs/adr/ADR-006-excel-recruiter-query-layer.md:27-34`; `RESTATE_WORKFLOWS.md:371-373`;
`src/store.rs:141-146` (historical: root package deleted 2026-09-23). This ADR's original citations
under `crates/census-service/src/store/**` never matched a committed path: that module was an
in-flight artifact, and the store is `crates/census-store/src/**`.

Cited through the doc, not re-derived: the scan/consolidate/stats lines
(`crates/census-store/src/read/mod.rs:134,152-175,202`), the 20M cap
(`crates/census-store/src/table.rs:19`), single-writer and legacy-import sharp edges, and the
root-crate `Database::snapshot()` sites (`docs/FJALL_SCHEMA.md:10-19,39-68,90-117,119-137`, historical:
root package deleted 2026-09-23). No heap measurement exists, and no restore timing is claimed.
