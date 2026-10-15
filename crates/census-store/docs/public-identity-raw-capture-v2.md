# Pinned raw public identity capture V2

`PublicIdentityWriterV1::capture_raw_generation_v2` takes the originating current-V2
writer, its store, and a checked `RawGenerationOperationIdV2`. It returns
`RawGenerationCaptureOutcomeV2::Captured` after a fenced capture attempt, or
`HistoricalReplay` when reopening an already-complete graph. Each variant contains
a `RetainedRawGenerationV2` with a private capture binding and seal reference.
The result is retained raw evidence, never accepted identity authority or a
statement that the source is currently fresh. Existing identity consumers are
unchanged.

Input revisions and the shared manual-context budget require explicit enrollment.
Disabled revisions refuse; capture does not enroll, reset budgets, reclaim partial
objects, migrate state, or change epochs.

Capture holds one MVCC snapshot across two passes. Both passes use physical paged
reads for Athletes, SourceObservations, ReviewCases, IdentityVerdicts,
Performances, and RelayResults. Original key/value bytes are retained, including
observation variants and JSON spelling. The scan retains at most one raw page,
256 page references, and 256 block references per current table; graph admission
also bounds descriptors, physical rows, metadata, and lifetime bytes/roots.
Empty tables receive explicit exhausted manifests.

The first pass measures the complete graph under the existing mutation owner,
then reserves the shared lifetime charge before staging. The outer lock is
released before existing context staging acquires its own lock. The second pass
recreates the measured graph from the same held snapshot and stages children
before parents, with the seal last. Readback verifies every retained node,
reference, inventory, raw-row encoding, and greedy page boundary.

An incomplete retry must match original store, run, format, derived layout, and
six input counters, and reproduce the reserved content graph. A snapshot sequence
is an observation stamp, not a reopenable MVCC checkpoint. Equal counters alone
cannot authorize divergent or restored content.

Input movement is checked before seal staging and again at the final return
fence. The pre-seal check is deliberately not claimed to be atomic with staging.
Movement detected before the seal leaves charged partial data. Movement after
the pre-seal check may leave a fully retained historical graph, but the current
attempt refuses at the final fence. A later complete replay explicitly returns `HistoricalReplay` with that
verified historical graph, preserving its original binding, without claiming
current freshness. Complete replay does not require original source rows.

Scratch-store tests cover 8,193 physical variants, empty tables, pre-write budget
refusal, reopen/source absence, corrupt or absent children, incomplete recovery,
equal-counter divergence, and movement before and after the seal. These tests
were authored without running builds, tests, or formatting during this slice;
execution and integration qualification remain outstanding.

## Public workflow schedule and crash fixtures

A separate compile-time test-only rendezvous pauses the actual public capture
immediately after reservation releases the append lock, or after seal staging
returns and before the final fence. A thread-local single-use gate uses capacity-one
channels and 30-second rendezvous deadlines; non-test builds contain neither the
gate nor its call sites. No production environment switch, cap change, or alternate
capture/verifier path is introduced.

The public workflow fixtures append a real concurrent athlete observation at both
phases, verify before-seal refusal versus final-return refusal, and overlap two
calls for the same operation while checking identical retained graphs and unchanged
admission-header bytes, accepted registry head, and receipt rows. A Unix lib-test subprocess receives an abort directive only after its observer
verifies a complete retained graph at the post-seal rendezvous. The capture thread
executes the abort inside that checkpoint; an expired gate refuses the directive
and fails the test rather than claiming a crash before return. The parent reopens the scratch store
and checks explicit historical replay, original bytes, binding, seal, and no replay
writes. The fixture runner uses a 60-second bounded exit wait followed by a two-second
SIGTERM-only reap window, with one bounded cleanup retry during drop. If child death
cannot be verified, its owned fixture directory is retained. It never sends SIGKILL. The child-selection environment variable is confined to the
lib-test fixture and does not control production capture.

These additional fixtures are authored source, not executed concurrency or crash
evidence. Integrated compilation, targeted test execution, and the repository's
normal gates are still required before making those claims.
