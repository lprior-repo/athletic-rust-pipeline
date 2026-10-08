# ADR-029 — Credited observations are keyed by operation receipt and totalled per ingest endpoint

**Status:** Accepted (2026-10-08). Supersedes clause 5 of
[ADR-028](ADR-028-window-accounting-and-bounded-labels.md) and closes the `seen_operations` half of
`athletic-rust-pipeline-hfhk.40` (DUR-16, DUR-03).

## Context

The `Ingest` object credited an operation's rows by remembering every applied operation id in
`IngestState.seen_operations`. That list was unbounded, and it was also lossy in both directions at
the exact crash window it existed for — between the store write that appends rows and the object
state write that records the credit:

- State lost *after* the credit: the operation is absent from the list again, so a re-delivery
  credits `receipt.appended` a second time. The endpoint's observation total overcounts and can
  exceed the rows the store actually holds.
- State lost *before* the credit, with the append landed: the store reports `Repeated` with
  `Application::appended() == 0`, so crediting per-call appends undercounts the source.

Exactly-once credit therefore cannot be decided from object state plus `Application::appended()`.
It needs one durable record that carries both the operation's append and whether that append has
been counted, updated atomically with the count it feeds.

## Decision

1. `Receipt` gains a `credited: bool` marker (serde default `false`), written `false` by the same
   batch that writes the receipt itself.
2. The credited observation total per endpoint lives in the store's `meta` keyspace under
   `endpoint_credit/<endpoint>`. It is a counter key, not a `Table`: entity tables, generations,
   snapshots and derived publication are untouched by this decision.
3. `Store::credit_observations(endpoint, operation) -> Credit { credited, total }` decides the
   credit. Under the append lock, in one `PersistMode::SyncData` batch, it reads the receipt, and
   when the receipt is not yet marked it marks it and adds `receipt.appended` to the endpoint total.
   An already-marked receipt returns `credited: 0` and the standing total; an operation without a
   receipt, or an empty endpoint, is refused rather than counted.
4. The `Ingest` object credits inside a journaled `ctx.run` immediately after `apply`, and reports
   `IngestReply.appended = credit.credited` and `IngestReply.total_observations = credit.total`.
   `IngestState.total_observations` becomes a mirror of the store total, rewritten from the credit
   on every `record` so a lost state heals instead of resetting; `seen_operations` is removed from
   the state shape.
5. Journaling the credit keeps a replayed handler deterministic: the reply records the amount the
   first execution credited, and a re-delivery of the same operation — whether it is a retry, a
   replay, or a fresh invocation after state loss — returns `credited: 0` from the marker.

## Consequences

- Object state no longer grows with the number of applied operations; the credit decision is one
  receipt read and one batch write regardless of source lifetime.
- The two crash-window outcomes are now both exact: a lost acknowledgement credits the stable
  receipt once, and a repeat after a landed credit credits nothing.
- The store, not the object, is the single authority for how many observations an endpoint has
  contributed; `census/state` and the sweep continue to read the mirrored count for classification.
- Receipt pruning (365 days, `athletic-rust-pipeline-hfhk.34`) is unchanged: a receipt is credited
  within the same handler invocation that writes it, so a pruned receipt was already counted.
- Migration: receipts written before this ADR decode as uncredited, and the first credit call for
  such an operation counts its rows. A state that credited operations under `seen_operations` and is
  restored after this ADR would therefore be re-counted once. This is not reachable in the preserved
  corpus: no store under `var/*/fjall/keyspaces` contains a `receipts` keyspace (checked
  2026-10-08) and no census run has credited operations. Restoring such a state is forbidden; the
  receipt marker is the credit authority, and the `seen_operations` field is ignored on decode.

## Implementation status

Landed in `census-store`: `receipt.rs` (`Receipt.credited`, `Credit`,
`Store::credit_observations`, `Store::endpoint_credit`) with tests in `receipt_tests.rs`
(`a_credited_operation_counts_its_rows_once_however_often_it_is_offered`,
`an_endpoints_total_survives_reopening_and_accumulates_across_operations`,
`an_operation_without_a_receipt_is_never_credited`). Landed in `census-service`:
`restate_services/ingest.rs` (journaled credit, state mirror), `restate_services/wire/ingest.rs`
(`seen_operations` removed), with
`a_lost_commit_acknowledgement_still_counts_the_operations_rows_once` in `restate_services/tests.rs`
rewritten to the store-authoritative contract.
