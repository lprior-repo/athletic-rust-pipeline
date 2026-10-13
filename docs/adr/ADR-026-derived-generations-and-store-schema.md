# ADR-026 — Derived state is published by generation, and the store names its schema

**Status:** Accepted (2026-10-06).

## Context

[ADR-001](ADR-001-fjall-primary-store.md) keeps Fjall as the sole evidence store and the domain
model as the schema. By October 2026 the store held every logical table in one `entities` keyspace
under `<table> 0x00 <entity-id> 0x00 <sequence big-endian>`, which keeps an entity's observations
adjacent and lets a merge walk run in bounded memory. Immutable evidence, the reconciliation index
and the review maps all shared that keyspace.

Four gaps remained, all in the application's database design rather than in Fjall:

1. **No persisted store schema identity.** Architecture required explicit versioned migration for
   persisted schema changes, and ADR-001 said the same, but nothing in the store said which schema
   the bytes belonged to. `MANIFEST_VERSION` was the backup manifest's format, not the store's.
   Serde's tolerance makes that failure mode quiet: a newer build can open an older store and
   reinterpret it happily.
2. **A fence over the wrong quantity.** Review decisions fenced themselves with the global Fjall
   sequence (`snapshot().sequence()`), so a journal or meta write that touched no decision input
   could refuse a valid decision, and any writer that bypassed the fence would have gone unnoticed.
3. **Derived publication was replay-safe but not atomic.** `index::derive` wrote `SourceIdentities`,
   `Conflicts`, `ReviewCases`, `Coverage` and `Snapshots` as five independent durable transactions
   and then committed the receipt. A crash converged on retry, but a reader could observe one table
   from the new pass beside another from the old, and a changed derived set was expressed as
   delete/overwrite churn in the same table it read from.
4. **Row-bounded, not byte-bounded.** Architecture bounds work by bytes; `replace_many` serialized
   every record of a pass into one batch, and a national pass produces millions of
   `SourceObjectIdentity` rows.

Two adjacent defects surfaced while closing those gaps: a derived table could be *appended* to (a
row per pass, unbounded, with the pass's identity lost), and an *empty* replacement emptied a whole
table — which made "replace these rows" and "delete everything" the same call.

## Decision

1. **The store names its schema in `meta` and refuses what it cannot interpret.**
   `schema_version`, `key_format`, `created_by`, `created_at` and a `migrating_to` resume marker are
   written at initialisation (`format.rs`, `STORE_SCHEMA_VERSION`/`KEY_FORMAT_VERSION`). Opening
   classifies the persisted format and either opens it, refuses it as newer
   (`StoreError::FormatNewer`), refuses it as unknown (`StoreError::SchemaUnknown`), or refuses it
   with `StoreError::MigrationRequired` naming the root and the detail.
2. **Migration is one explicit, resumable operation.** `Store::migrate(root)` opens a stopped store,
   initialises an empty one, or rewrites a versioned-predating store: derived tables move under
   generation partitions, map tables collapse to one row per id, and observation-log rows are left
   byte-identical. It writes the resume marker before rewriting, reopens the migrated store and
   reports its integrity (`MigrationReport`). `census-service --store DIR store-migrate` is the
   native entry point; a store whose migration was interrupted resumes it, and an already-current
   store reports `already_current` without rewriting.
3. **Derived tables are partitioned by generation; publication is one atomic flip.**
   `Store::stage_derived()` reserves a generation under the store's staging lock, exposes a snapshot
   view, carries forward the current generation table by table, and replaces rows by id within the
   staged generation. `publish(operation, digest)` is the only step that writes durable state a
   reader can see: one `SyncData` batch flipping `derived_current`, bumping the evidence generation,
   writing row marks and inserting the operation receipt. A reader reads the current generation and
   nothing else; an unpublished or superseded generation is invisible.
4. **Materialization is bounded; publication is not.** The stage flushes in row *and* byte batches
   (25 000 rows or 64 MiB, whichever comes first), so a pass over a nationwide derived set never has
   to hold the table in memory, and the atomic part of the write stays small.
5. **Reclaim is separate, budgeted and resumable.** `reclaim_derived_generations(budget)` deletes
   non-current generations above the current pointer and generations below it past the
   `derived:reclaim_from` marker, resuming where the last call stopped. A pass that runs out of
   budget deletes exactly what it charged and reports the generation unfinished, so the marker never
   advances past rows that are still there and the next call resumes them. Every publication
   reclaims with its own budget, and a repeat publication reclaims the stage it just abandoned
   instead of leaving it for the next pass.
6. **Decisions fence on the evidence generation.** `commit_once_at_evidence_generation(operation,
   digest, generation)` accepts a checkpoint only when no write that counts as evidence has moved
   the generation since the snapshot it was derived from. Journal, receipt, meta and derived-pointer
   writes do not invalidate a review decision; a change to canonical observations, canonical
   entities or review inputs does. A staged generation refuses to publish when the evidence moved
   under it (`StoreError::PublicationRefused`), which is retriable: the caller re-derives from the
   new snapshot, producing a new input digest and therefore a new operation identity.
7. **Append, replace and map writes are disjoint by table kind.** Observation logs append
   content-addressed rows (replacement refused); derived generation tables replace rows by id
   (append refused); maps — `source_access`, `identity_verdicts` and `athlete_identity_decisions` —
   replace by id in place. An empty replacement names no row and writes nothing — never a table-wide
   delete — and a batch may no longer hold one table both appended and replaced, because that state
   is now unrepresentable rather than merely refused.
8. **`index::derive` is the one stage consumer.** A pass computes the next generation: it carries the
   current generation forward, closes review cases whose finding is gone as `Superseded`, re-mints or
   merges the cases this pass reaches, writes identities, conflicts, coverage and the pass snapshot,
   and publishes once under `index-stage:<input digest>`. A pass whose inputs are unchanged publishes
   nothing, records no second receipt and reclaims its stage; a pass whose inputs changed publishes a
   whole new generation.

## Consequences

- A reader can no longer mix two passes: it sees the previous generation completely or the new one
  completely, and a crash during materialization leaves the previous generation current and the
  incomplete one invisible.
- Rebuilds stopped being delete/overwrite churn in a table the rebuild also reads: a pass writes one
  generation and one pointer, and reclaim does the deleting afterwards with a budget.
- A decision checkpoint is no longer invalidated by unrelated journal, receipt or derived writes, and
  a writer that bypasses the fence cannot silently invalidate one — the generation it fences on is
  written only by evidence-bearing paths.
- The store now fails closed on bytes it does not recognise. That is a deliberate behaviour change:
  a store written before this ADR is versioned on first open by migration, and an unrecognised newer
  store is refused rather than reinterpreted.
- Costs and limits, stated plainly: a pass writes every derived row it keeps, including carried
  forward rows, so derivation writes more bytes than the in-place overwrite it replaced (reclaim and
  compaction recover them on the store's own schedule); maps (`IdentityVerdicts`,
  `AthleteIdentityDecisions`, `SourceAccess`) are not generation-partitioned, so a map write becomes
  visible immediately and is not rolled back with a generation; and a repeat publication does not
  repair rows lost from the current generation — migration and reclaim are the repair paths, not a
  second publication of the same digest.
- Splitting `entities` into further keyspaces remains open and unclaimed: this ADR partitions derived
  state logically, and a physical split still needs an A/B measurement on a real store before it is
  worth doing.

## Implementation status

Landed with this ADR in `census-store` (`format.rs`, `format/migrate.rs` with its `rewrite` and
`folding` submodules, `derived.rs`, `derived/stage.rs`, `derived/reclaim.rs`, `generation.rs`,
`write_batch/`, `table.rs`, `rows.rs`, `inspect.rs`), `census-reconcile` (`index.rs`),
`census-review` (`review_checkpoint.rs`, `revocation.rs`, `athlete_clusters.rs` — planning split from
its fenced commit — and `review_process.rs`) and `census-service` (`cli/store.rs` printing the format
and both generations, the `store-migrate` verb, and error classification treating a refused
publication or a moved-evidence decision as retriable). Dated command evidence is in
the verification ledger (removed 2026-10-09); the fault matrix remains owned by
[NATIONAL-CENSUS-FAULTS.md](../NATIONAL-CENSUS-FAULTS.md).

## 2026-10-13: snapshot-bound application continuation

School-address application reads school ownership and current review-case presence together. An
entity-only fence cannot express the complete dependency: a replacement must target the derived
partition that supplied the decision. `StoreSnapshot::revision()` therefore exposes an opaque,
nonserializable token binding the originating store instance, evidence generation and derived
partition. Store clones share the same instance identity; another open instance cannot supply a
token merely because its numeric coordinates match.

`StoreBatch::commit_once_at_revision` checks that origin and both coordinates under append
serialization before checking a historical receipt. Effects, a versioned application witness and
receipt commit together with `SyncData`. The returned `RevisionedApplication` contains the existing
`Application` outcome and the exact revision captured before releasing the lock. A successful
first application advances continuation only by its own write; a strict repeat returns the
unchanged accepted coordinates. It cannot refresh an already-stale decision across another writer.
The existing commit APIs and their repeat-before-evidence-check behavior remain unchanged.

The address join uses one initial snapshot for both review presence and school iteration, and
commits each complete school/co-op mutation group before continuing to an independent target.
A concurrent entity mutation or derived publication refuses the next stale group, preserving
prior committed targets. A later invocation takes a fresh view and recomputes the ordinary report;
it never skips a whole generation or replays an old report over newly discovered schools.
Review-containing application identities bind the physical derived partition. If a later partition
omits a previously applied Pending case, fresh recomputation can insert that genuinely missing
case without duplicating unchanged school observations. Existing decided cases remain present.

This adds ephemeral API types and a versioned journal payload, without changing persisted Receipt,
physical store schema, public workflow wires, source qualification or census acceptance. Legacy
facts are retained without fabricated application receipts. Resource admission is fail-closed for
a whole target; it does not split an inseparable co-op group to bypass byte ceilings. Diagnostic
report files remain recomputable outputs rather than the transaction's durable authority.

The school-address application v1 scope and witness records declare serialized fields in lexical
key order, including nested lane and effect bindings. Their streaming typed JSON digests therefore
match digests recomputed from the journal's retained JSON values. The effects array preserves
first-occurrence School append order; only the scope's lane-reference set is sorted. This is a
local v1 witness encoding contract, not a change to the shared `serialized_digest` helper or
existing payload encodings. Retained-witness regression checks bind these bytes to actual receipt
keys and fail if a field-order change silently changes that contract.

## 2026-10-13: store-owned stable lineage and pinned identity reads

`census-store` owns the existing `export-lineage-v1/store-identity` journal record. The domain
`StoreIdentity` validates exactly 64 ASCII hexadecimal bytes and preserves their spelling,
including previously accepted uppercase identities. It is not interchangeable with the
lowercase-only publication digest. Existing publication and frozen-v2 lowercase checks remain.

`Store::store_identity` serializes check/create with the owning store's append mutex and commits
a genuinely absent identity through the existing inline `SyncData` journal operation. It retains
the original creation digest inputs and journal envelope. Reading an existing canonical record
performs no write and does not refresh its timestamp. Moving or reopening a store adopts that
same identity rather than deriving a replacement from its new path. No physical schema, key
format, receipt, run manifest or publication-policy revision changes for this ownership move.
The old optional `.export-lineage.lock` file is left untouched and is no longer needed.

Identity inspection uses one pinned view, examines at most two physical records under the phase,
and checks the sole value against the existing journal byte ceiling before parsing. Multiple,
foreign-key, malformed or chunk/object/alias representations refuse without replacement. The
canonical inline interpretation intentionally closes an older reader's accidental acceptance of
a sole payload under an unrelated key. Supported canonical historical envelopes remain unchanged;
refusal is not permission to rewrite unsupported retained bytes.

`StoreSnapshot::store_identity` reads only that snapshot and never initializes or takes the
append mutex. Snapshot-based report derivation requires identity initialization before capturing
the view. A snapshot predating initialization remains missing even after a live writer creates
the record. `StoreSnapshot::ensure_origin` reuses the opaque instance capability from
`StoreRevision`, without imposing current-generation equality: a same-origin historical view
remains usable, while another store's view is refused before report row decoding. This also
permits report derivation from an initialized `FencedSnapshot` without reentering its mutex.

All report capture and reconciliation-stage entry points initialize outside the snapshot/fence;
report serialization adapters no longer own identity persistence. The generic journal API is not
a separate identity owner. Deliberate raw journal corruption remains an integrity refusal, not a
supported identity transition. Live run-manifest metadata is still read separately by existing
lineage/token code; this change does not claim all run metadata is snapshot-pinned.

This is a prerequisite for installation/profile contracts. It does not register those contracts,
enroll reusable production, migrate a historical corpus or relax fresh-qualification and release
requirements. Source evidence and derived generations do not move merely because identity is
initialized, although the global storage sequence can advance for that durable journal write.
