# Fjall schema and read/write behavior

Canonical storage reference for `census-store`, pinned to Fjall 3.1.10. Decision rationale is
[ADR-001](docs/adr/ADR-001-fjall-primary-store.md); operating procedures are in
[backup/restore](docs/FJALL_BACKUP.md). This describes current APIs, not proof of every delivery invariant.

## Physical layout and ownership

One database under `<store>/fjall`; `Store::open` also creates `http/` and `out/`. Existing
`entities/` and `journal/` directories may contain historical JSONL/materialized artifacts, not a
second authoritative database. Current startup does not run a legacy importer.

Fjall permits one owning process. The service shares `Arc<Store>` in-process, and store mutation
uses its append mutex. A second CLI opener must fail while the service holds the lock. The configured
cache is 1 GiB; `journal`, `meta` and `receipts` request point-read-hit hints, while `entities` uses
default keyspace options. Other engine tuning remains at the pinned dependency defaults.

| Keyspace | Keys and values |
|---|---|
| `entities` | `<table>\0<entity-id>\0<sequence:u64 big-endian>` → serialized JSON observation or derived entity |
| `journal` | `<phase>\0<logical-key>` → `{key, at, payload}` JSON completion/progress record |
| `meta` | `sequence:<table>` and `rows:<table>` → textual unsigned high-water/count values; historical stores may also retain old markers |
| `receipts` | Exact caller operation ID → `{operation, digest, at, appended}` JSON |

Prefix order groups all observations of one entity together in write order. `<table>\0` alone is a
table scan, not one-entity lookup. Journal logical keys are caller-defined, not entity-key encodings.
Malformed persisted keys/counts/JSON return store errors; a permissive reader must not hide corruption.

## Logical tables and replacement semantics

`Table::ALL` currently has **17** entries. They share `entities`; they are not 17 Fjall keyspaces.

| Storage mode | Tables | Meaning |
|---|---|---|
| `ObservationLog` | `schools`, `teams`, `coaches`, `athletes`, `meets`, `events`, `performances`, `source_meets`, `source_observations` | Append retained evidence under increasing sequence keys |
| `DerivedSnapshot` | `source_identities`, `conflicts`, `coverage` | Rebuild replaces the declared set and removes unnamed stale rows, including on an empty rebuild |
| `DerivedMap` | `review_cases`, `snapshots`, `source_access`, `identity_verdicts`, `athlete_identity_decisions` | Replace named derived entries without treating each derivation as another observation |

Derived rows use the store's reserved derived sequence. Do not call append APIs on a derived table
or apply snapshot-clearing semantics to a map. `source_meets` records enumerated meets independently
of acquired result rows. Domain `CollectionSnapshot` records are pass metadata, not Fjall MVCC
handles or durable copies of the underlying input generation.

`Entity::merge` combines observations and `publish` applies the entity's read-time projection.
Unioned evidence/aliases and fill-if-absent scalars have different laws: do not claim every merge is
commutative or that a later observation always wins. Domain/source-owner meaning lives in
[DOMAIN.md](DOMAIN.md), including [ADR-014](docs/adr/ADR-014-athlete-owner-identity-optional.md)'s
explicit historical athlete/performance decode shapes. Current writers emit the current shape only.

## Writes and accounting

`append`/`append_many`, journal writes, derived replacement and `StoreBatch` persist through Fjall
write batches with `SyncData`. `Store::flush` uses `SyncAll`. Planned sequence and row metadata are
staged with the rows; in-memory sequence publication follows the successful durable commit.
Missing metadata is reconstructed from table keys on open and persisted. Opening a store can
therefore write metadata and sweep stale snapshot temporaries; it is not a read-only inspection API.

The table ceiling is 20,000,000 observations/rows as enforced by the relevant paths, not a heap-byte
budget. Current named limits are 512 bytes for entity IDs and operation IDs, 256 bytes for receipt
digests, 4 KiB for journal keys and 1 MiB for journal values. These limits do not establish a total
serialized batch, export or process-memory bound.

`StoreBatch` can stage observations, journal records and derived replacements in one database batch.
Its `commit_once(operation, digest)` returns `Application::Written(receipt)` or
`Application::Repeated(original_receipt)`. A repeated application reports zero newly appended rows
while retaining the original receipt. A different digest under the same operation is
`StoreError::Invariant`, not an upsert. An empty receipted effect can still commit a receipt.

The caller computes and binds operation identity and digest; this API alone cannot guarantee that
source/run/capture/parser/schema semantics were included. Nor does its presence prove every adapter
uses it or that a native commit/lost-ack fault was exercised. See the active plan's
[capture/effect contract](docs/NATIONAL-CENSUS-PLAN.md#3-capture-and-external-effect-durability).

`prune_receipts` removes receipts older than an explicit date and counts undated entries. `Sweep`
uses a 90-day policy. That age threshold is implementation behavior, **not proof** that no retained
invocation can replay an old operation. Receipt retention must be reconciled with actual replay/run
retention before relying on collection as safe.

## Snapshots, streaming and materialization

`Store::snapshot()` exposes `StoreSnapshot`, backed by Fjall MVCC. On that one view:

| API | Behavior |
|---|---|
| `sequence()` | Live snapshot sequence; no API to reopen the same view after process death from this number alone |
| `for_each_observation` | Bounded-count table prefix traversal with strict decode |
| `for_each_merged` | Adjacent-entity streaming merge and `publish`; does not require a whole-table map |
| `for_each_merged_selected` | Traverses the whole table's keys/values, skips decode for IDs outside the supplied set; not a point index |
| `scan` | Collects the merged stream into a `Vec`; memory still scales with returned entities |
| `consolidate` / `consolidate_table` | Streams a table into an individually atomically published JSONL snapshot |

Convenience methods on `Store` acquire a new snapshot per call. A multi-table report must explicitly
share a view; several `Store::scan` calls do not establish a common generation. A live MVCC handle is
neither restartable export input nor permission to copy a changing database directory.

Snapshot file readers reject invalid rows rather than skipping them. Per-file atomic publication
and stale-temporary cleanup do not provide a fenced atomic workbook/sidecar/manifest bundle.
`journal_keys` and `journal_payloads` still materialize their results; the latter clones payloads.
Use bounded consumers where required rather than labeling every read streaming.

## Counts and footprint

`StoreStats.tables` reports physical current table rows using persisted row marks, with a key-walk
fallback when needed. `appended` and `observations` count observation-log rows, excluding derived
rows. These are not merged entities, accepted identities or cohort populations. Sequence high-water
marks are another quantity and must not be presented as current row counts.

`bytes_on_disk` comes from the `entities` keyspace's LSM disk-space metric; it is not the database
root's total footprint. `store_bytes` recursively measures the root, including journals, caches and
outputs. Journal preallocation/recovery can change bytes without changing accepted evidence.
Neither size equality nor equal total rows proves semantic restore integrity.

## Recovery and schema changes

Use [the cold backup procedure](docs/FJALL_BACKUP.md). Backup/restore cover the configured store
subtrees and a file/count manifest, not native Restate's separate durable directory or arbitrary
external evidence paths. Preserve all referenced capture bytes required for replay.

Fresh runs follow [ADR-013](docs/adr/ADR-013-fresh-national-source-census.md); historical schema work
follows [ADR-010](docs/adr/ADR-010-preserve-existing-corpus.md). For any future mark-unit migration:
quiesce and back up the old store, identify the actual writer/schema version, convert into an isolated
new generation, quarantine `AmbiguousLegacyUnit` rather than guessing from magnitude, compare exact
records/PR ownership, verify restored evidence, then explicitly promote with rollback retained.
No current legacy-import command is implied by this requirement.

Remaining durability/resource/retention and shared-export obligations live in
[the active delivery plan](docs/NATIONAL-CENSUS-PLAN.md); dated predecessor audits and measurements
live in [verification evidence](docs/VERIFICATION-EVIDENCE.md), not a competing storage plan.
