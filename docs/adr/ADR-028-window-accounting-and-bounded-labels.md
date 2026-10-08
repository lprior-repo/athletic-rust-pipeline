# ADR-028 — Window completion is counted exactly and only a bounded label ring is retained

**Status:** Accepted (2026-10-07). Clause 5 (`seen_operations` retention) is superseded by
[ADR-029](ADR-029-receipt-keyed-observation-credit.md).

## Context

The `Ingest` object kept two unbounded `Vec<String>` fields in its durable state: every completed
window label and every applied operation id. Review bead `athletic-rust-pipeline-hfhk.40` (DUR-16)
recorded the consequences: state size, `ctx.set` payload and lookup cost grew with the source's
lifetime rather than with the current chunk, `complete_window` sorted the whole label set before
cloning the complete state, and a sequence of successful empty operations could grow the state
without bound because such operations bypass the observation-table row ceiling.

The two histories are not equally necessary:

- Completion evidence needs only *how many* windows completed. `census/state/open.rs` treats a
  source object as terminal on `windows > 0`, and `EndpointObservation.completed_windows` reports a
  count; nothing consumes the labels themselves.
- Label identity exists only so a repeated `complete_window` call (a retried or re-driven window)
  does not count the same window twice, and a repeated operation id must not credit its rows a
  second time (`credited_appended`, bead `athletic-rust-pipeline-hfhk.28`/DUR-03).

## Decision

1. `IngestState.windows_completed: u64` is the authoritative count of distinct completed windows.
   `windows: Vec<String>` becomes a bounded ring of the most recent labels kept for duplicate
   detection (`WINDOW_LABEL_RING = 64`).
2. `IngestState::completed_windows()` returns `max(windows_completed, windows.len())`. A state
   written before this ADR carries only its label list, so the list length seeds the count; a state
   written after it carries a count that already includes trimmed labels, so the maximum is exact in
   both directions and the ring never double counts.
3. Operation ids and window labels are length-bounded (`MAX_OPERATION_ID_BYTES` and
   `MAX_WINDOW_LABEL_BYTES`, 128 bytes each) and refused as terminal errors, so identifier bytes
   cannot grow the state either.
4. `record_window` is the single mutation of the ring: it refuses a duplicate label, increments the
   count through `completed_windows()`, sorts, and trims the smallest label once the ring is full.
   `complete_window` writes object state only when `record_window` reports a change.
5. `seen_operations` is removed by
   [ADR-029](ADR-029-receipt-keyed-observation-credit.md), which moves the credit guard to the store
   receipt. It was the exact credit guard for rows applied by an attempt whose object-state update
   never landed: the store reports such a repeat as `Repeated` with `appended() == 0`, so crediting
   per-call appends alone would undercount a source's observations, and the object-state list
   overcounted instead when the state was lost after a credit. The receipt marker and the
   per-endpoint total decide the credit in one atomic batch.

## Consequences

- Object-state bytes are bounded by the ring plus the identifier limits instead of by the source's
  lifetime, and a chunk's cost no longer grows with history: the count stays exact while the labels
  are trimmed.
- Duplicate detection is exact only for the most recent 64 labels. A repeated window whose label was
  already trimmed is counted again. The count is used as completion evidence (`> 0`) and as a
  reported number, so the effect is an overcount of an idempotency artifact, never a lost window.
- A pre-ADR state keeps its full label list until new windows arrive; each new window folds the list
  length into `windows_completed` and trims to the ring, so migration is incremental rather than a
  rewrite of every object.
- `--max-concurrent` and the `Semaphore` ceiling are validated at both boundaries
  (`ServeOptions::from_env_with_path` and `restate_services::validate_concurrency`), replacing the
  silent `max(1)` clamp that let an accepted CLI value panic endpoint construction
  (`athletic-rust-pipeline-hfhk.39`/DUR-15).

## Implementation status

Landed in `census-service`: `restate_services/wire/ingest.rs` (count, ring bound, identifier limits,
`completed_windows()`), `restate_services/ingest.rs` (`record_window`, `check_identifier`, handler
use), `restate_services/open_work.rs` and `restate_services/sweep/mod.rs` (count consumers),
`bootstrap.rs`/`bootstrap/error.rs`/`bootstrap/options.rs`/`bootstrap/serve.rs` and
`restate_services/mod.rs` (concurrency validation). Unit tests:
`window_labels_are_counted_exactly_and_the_ring_stays_bounded` and
`endpoint_concurrency_is_validated_before_the_semaphore` in `restate_services/tests.rs`, and
`options_reject_concurrency_above_the_operational_ceiling` in `bootstrap/tests.rs`.
