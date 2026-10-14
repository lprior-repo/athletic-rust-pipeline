# Independent prerequisite: repair the existing Crawl lock graph

**Status: Root-adopted narrow prerequisite at 08:00 UTC, including the source-verified capacity-parse timing correction; no implementation by this review.** Root requested this scope after reviewing M1–M4. The full v2 contract remains unadopted and blocked. This slice changes no Store/domain/public wire, qualification version, acquisition transition, retry authority or Restate history layout.

## TASK / acceptance boundary

Repair the current Crawl archive/eligible publication and replay call graph so no path holding eligible acquires the archive OS lock, and no caller already holding archive reacquires it. Use one existing lock pathname and the same publication/capacity implementation. Make the fix useful for current v1/legacy callers immediately; do not add dormant v2 scaffolding.

Required nested order is **archive OS lock → eligible-cache lock → Store appends** where those resources are needed. Pure eligible readers/compare fences may acquire eligible alone and call Store under it; they never acquire archive. Network awaits remain outside the synchronous file-lock region. The existing service dedicated worker/WorkAdmission boundary remains unchanged.

The archive lock is blocking `File::lock`; eligible remains `try_lock` with its exact `WouldBlock` error. Do not replace either with an async mutex, change retryability, add a scheduler, widen admission or change data addressing. No Store-exported `ArchiveWriteFence` is necessary for this prerequisite.

## Actual call sites that must move together

All paths below are current production code, not proposed v2 callers.

| Entry | Current graph | Required graph |
|---|---|---|
| `cache/archive.rs::write_preserved_cache` | archive admission → eligible try-lock → old/new archive preservation → cache replacement | Same order and behavior; reuse the common guard-owned admission. On eligible busy, still preserve the incoming response and return exact WouldBlock without replacing eligible bytes. |
| `cache/archive.rs::write_preserved_capture` | archive admission → preserve new capture | Same archive-only behavior. |
| `cache/archive.rs::replay_preserved_cache` | capacity parse → eligible → read/compare → archive admission → preserve previous | Preserve capacity parsing first; then acquire only the archive lock, then eligible, then read/compare. Begin full initialization/publication admission only after a hit with equal expected metadata. Missing/changed cache still returns None without inventing an archive capture. |
| `cache/eligible.rs::publish_eligible` | eligible → `publish_eligible_held` → archive admission | archive lock → eligible → held publication using that same archive guard. |
| `net/acquisition/read.rs::recover_prepared` | eligible → Store finish/read → held publication → archive admission | archive lock → eligible → identical v1 Store finish/read → held publication using that same guard. Missing/corrupt Prepared still fails under its existing Store law and never restores/closes/reserves. |
| `cache/eligible.rs::PinnedEligibleCache::compare_body` | eligible → callback, possibly Store | Unchanged; callback must not gain an archive-acquiring edge. Current callers are cache verification/reuse and 304 finish. |

Current source pins: `archive.rs:41–120,238–247`; `eligible.rs:66–79,112–144`; `archive/admission.rs:14–45`; `archive_io.rs:54–73`; `acquisition/read.rs:58–97`; `acquisition/retention.rs:12–55,59–105`.

## Minimal helper and lifetime contract

These are proposed internal signatures; Root owns the implementation cut. They deliberately keep the current algorithms in Crawl and separate lock ownership from capacity initialization. All new types stay at the narrowest existing `crate::net`/cache visibility. None is serialized, cloneable, constructible from raw flags, exported by Store, or a recovery capability.

1. Extract the existing archive lock open/`lock()` into a small Crawl-local `ArchiveLock` owning the same `File` and validated HTTP-root path. Construction uses exactly `<http>/archive-capacity.lock`, `.create(true).truncate(false).read(true).write(true)`, with existing `FetchError::Cache { path, source }` mapping. Drop releases that descriptor. There is no lock-file removal or new pathname.

2. `ArchiveAdmission<'lock>` borrows `&'lock ArchiveLock` instead of owning a newly opened file. Its existing root/archive/capacity and reserve/finish methods retain their meanings. Change its private/internal `begin_with_capacity` seam to accept the already-held guard and a checked positive capacity. It derives or verifies the archive is `<guard.root>/archive`, then performs existing reservation-directory initialization, archive inventory and reservation recovery exactly once. It never opens/locks `archive-capacity.lock`. Invalid/zero capacity remains checked before any file creation or lock acquisition at the existing acquiring entry points. Update all internal/test callers; do not retain an alternative archive-lock implementation.

   Concrete constructor clarification, agreed with the implementation worker: a private `ArchiveCapacity(NonZeroU64)` may represent the already checked capacity. `ArchiveCapacity::new(raw, archive)` retains the existing zero-error path/message/kind; `archive_capacity()` returns that checked type; `ArchiveAdmission::begin_with_capacity(&ArchiveLock, ArchiveCapacity)` derives the archive path from the guard. Existing explicit-zero tests validate before acquiring the guard or creating files. This is an internal validation token, not a persisted capacity grant. Recovery defers capacity parsing and admission until successful200 publication. Legacy replay retains its existing capacity parse before any lock/cache read (`archive.rs:99`), while deferring only full initialization/admission until a matching cache hit (`archive.rs:114–115`).

3. Add a synchronous callback wrapper for the current recovery/eligible callers, conceptually:

   `with_archive_eligible_fence(body_path, meta_path, action: FnOnce(&ArchiveLock) -> Result<T, FetchError>) -> Result<T, FetchError>`.

   Validate the same cache-root pairing; acquire `ArchiveLock`; acquire the existing eligible try-lock; invoke the callback with a borrow. The owned archive guard and eligible file remain local to the wrapper until the callback returns. The borrow cannot escape and admission cannot outlive it. Drop/error releases both; no `.await` or callback network I/O. This replaces the old `with_eligible_fence` only at its current Prepared-recovery caller. Do not change `PinnedEligibleCache::compare_body` to pay for archive locks unnecessarily.

4. Change `publish_eligible_held` to require that borrowed archive guard in addition to the caller-held eligible fence. It performs existing envelope encoding/staging and obtains `ArchiveAdmission<'_>` from the held guard only when publication is actually reached. The current `preserve_eligible_previous` becomes a held-admission helper accepting `&mut ArchiveAdmission<'_>` and the existing stage/paths; it must not call `begin_with_capacity` or acquire a lock. Reuse the single `preserve_previous` and `preserve_capture` implementations. An acquiring `publish_eligible` wrapper delegates through the wrapper in step3.

5. The archive-only and legacy acquiring entries validate capacity, then acquire `ArchiveLock` before building their borrowed admission; any subsequent eligible lock is inner. Legacy replay preserves its configuration-first boundary, acquires the lock-only guard before eligible/read, and defers archive initialization/admission until an actual matching cache hit. The ordinary cache writer retains its current preservation-on-eligible-busy branch and exact return error. No new retry/fallback occurs.

The borrowed form is sufficient; do not add an Owned/Borrowed guard enum, a global registry or a generic lock manager just to keep the old private constructor signature. Existing public Crawl/Store method signatures and result types stay unchanged. One small lock helper file is acceptable if it makes ownership clear and respects the existing 300/60-line limits.

### Why lock-only must be distinct from admission

Do not move full `ArchiveAdmission::begin_with_capacity` before v1 Store finishing. Today recovery only reaches archive capacity configuration, initialization and reservation recovery when it has successfully finished/read a status200 capture (`acquisition/read.rs:70–92` → `eligible.rs:132–134`). Doing so earlier would make an unrelated malformed reservation/configuration fail instead of the existing `RequiredBackingMissing`/`BackingIntegrity` refusal, and could create missing archive directories before refusing Prepared. The lock-only extraction prevents that accidental semantic expansion. Non200 recovery must not begin unnecessary capacity/publication work.

Likewise legacy replay returns None for a miss/changed expected metadata before initializing archive admission (`archive.rs:100–117`); keep that boundary. Acquiring the actual lock file itself is the sole new mechanical filesystem effect needed before that check. No archive body/metadata, journal, receipt or cache pair is rewritten merely to obtain the guard.

## Error and persisted-byte compatibility

Preserve on each singly failing supported path:

- exact archive/capture/eligible metadata encoding, content addresses, file bytes, timestamps, historical unknown metadata treatment, hard-link behavior and immutable previous archives;
- capacity parsing/defaults and InvalidData/non-retryable classification, including invalid environment refusal before file creation for the existing archive entry points;
- capacity refusal payload (`used`, `requested`, `capacity`, archive path), incoming-response preservation on busy ordinary cache writer, eligible WouldBlock kind/path, missing/changed-cache None and existing I/O propagation;
- v1 Prepared exact-present success, absent/corrupt fail-closed result, same original terminal/receipt/provenance and zero recovery HTTP/new ordinal;
- existing stage/reservation cleanup and evidence-preservation policies. This slice is not authorization for archive corruption repair or broader cleanup.

**Combined-fault ordering is explicit.** Root accepted at 07:55 UTC that an archive-admission refusal takes precedence over eligible `WouldBlock` when both faults coexist, to permit consistent archive→eligible ordering. Preserve the existing error types and ordinary successful bytes/replay. This is the only accepted compatibility change here; it is not permission to revise v1 source acceptance, terminal laws or other persisted behavior.

Acquiring archive before eligible moves its lock open/lock refusal ahead of eligible busy. Full capacity initialization can remain deferred as above where current v1 finish/cache-miss behavior requires it. Table-test archive-lock failure + eligible busy, archive-admission/configuration refusal + eligible busy, cache miss + invalid archive contents, and Prepared missing + malformed reservation/configuration. At boundaries that do perform archive admission before eligible, assert the Root-approved archive refusal exactly. Preserve old non-busy refusal/None behavior by the deferred-admission arrangement. Do not describe all historical error ordering as unchanged, and do not add speculative try-lock/retry loops or probing locks to emulate racy timing.

## OWNERSHIP / exact file cut

Candidate production ownership is limited to:

- `crates/census-crawl/src/net/cache/archive/admission.rs` and at most one small subordinate lock helper;
- `crates/census-crawl/src/net/cache/archive.rs`;
- `crates/census-crawl/src/net/cache/eligible.rs` and its re-exports in `net/cache.rs`;
- `crates/census-crawl/src/net/acquisition/read.rs` only for the held-fence callback and publisher argument plumbing.

Existing `archive/publication.rs`/`archive_io.rs` need change only if the internal guard plumbing genuinely requires it; their serialization, freeze, sync, hard-link and replace algorithms are otherwise outside the cut. Tests live beside the affected existing paths. No domain, Store, service, schema marker, new recovery terminal, candidate importer, altered read budget or user command is in scope. Main applies any approved interface changes and owns integration.

## Deterministic executable tests

Proposed names start `archive_lock_order_`; none exists or ran as a result of this review. Use actual OS locks in owned temp roots and test-only barriers at lock acquisition/before callback, with bounded process/thread supervision. If a lock-order mutant can deadlock, use the existing child-process test pattern so the harness can terminate only its owned child and report the exact held/waited locks. Never detach a stuck test worker or treat timeout as success. Native process termination in a test lane requires its normal separately approved test scope.

1. `archive_lock_order_current_v1_recovery_publishes_under_one_archive_guard`: seed a genuine retained-ready v1 Prepared200 and a prior valid eligible pair. Call the real current recovery/parser path. Record one archive acquisition, then eligible, then Store lock, and no second acquisition inside held publication. Assert exact terminal/receipt, old/new archive bytes, eligible pointer and zero HTTP. Test again with no prior eligible pair; an empty preservation branch still must not open the lock a second time.
2. `archive_lock_order_legacy_replay_and_writer_do_not_reverse_resources`: deterministic actor A pauses after archive acquisition; actor B enters legacy replay. The before-archive test hook proves B does not yet own eligible. A can acquire eligible and finish; B then continues and returns exactly Some/None based on the retained expected metadata. Use channel/barrier acknowledgments rather than sleeps. Repeat with `publish_eligible` and legacy `write_preserved_cache` as the other actor. Assertions must verify exact outputs or exact WouldBlock, not merely both threads exiting.
3. `archive_lock_order_busy_cache_preserves_incoming_archive_and_exact_error`: retain the existing `an_in_progress_cache_writer_retains_the_response_without_retry_or_partial_replacement` oracle, strengthened to exact path/kind and metadata bytes. Existing held eligible lock causes WouldBlock, old eligible pair remains, both original and incoming responses are safely archived. No re-dispatch.
4. `archive_lock_order_deferred_admission_keeps_v1_refusals`: missing/corrupt Prepared plus malformed reservation/invalid capacity stays the selected old Store error with original facts/files unchanged, no restoration/closure; non200 recovery never runs capacity admission. Legacy cache miss/expected mismatch returns None and does not create archive data. Record the lock-file-only effect distinctly.
5. `archive_lock_order_configuration_and_double_fault_precedence_is_exact`: retain `archive_capacity_environment_is_checked_in_isolated_processes`, `nonunicode_capacity_environment_is_refused_in_an_isolated_process`, and both explicit-zero tests. Add the combined-fault cases above and assert the Root-approved exact winner/path/kind. Do not mutate process-global environment in parallel tests.
6. `archive_lock_order_guards_release_on_all_current_error_paths`: induce envelope encode refusal, capacity refusal, staged-file/sync failure, Store finish refusal and panic in an owned test callback. A subsequent supported acquisition obtains the same actual lock and sees correct preserved state; exact source outputs remain immutable. For the real service cancellation regression, rerun existing dedicated-worker/WorkAdmission tests, without changing that ownership machinery.
7. `archive_lock_order_existing_archive_semantics_are_byte_identical`: preserve existing refresh/replay/shared-inode/unknown-metadata/capacity/reservation/quarantine-crash tests. Pin before/after fixture manifests for each preserved archive and eligible output. No changes to historical journal/receipt bytes.

Mutation thought experiments that must be detected: make a held helper acquire the lock; move legacy replay eligible acquisition above archive; drop archive before callback/publication; run capacity admission before v1 finish; return success instead of busy; remove incoming preservation on busy; replace an immutable old archive file.

## Candidate verification and HANDOFF

After implementation of this adopted slice, exact targeted test lane:

```sh
tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & test(archive_lock_order_)'
tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & (test(net::cache::archive_tests::) | test(net::cache::archive::admission::tests::) | test(net::acquisition::tests::))'
tools/moon-local run pipeline:tests -- -E 'package(census-service) & (test(jurisdiction::results_work::replay_tests::) | test(spawn::dedicated::tests::) | test(work_admission::tests::))'
```

Check each named target and nonzero selected count. Run required Moon formatting/check/source-lint/aggregate gates on the final integrated tree. No test or build was run here. This prerequisite fixes only the current lock graph. It does not close M2–M4, adopt v2, expand archive-scale qualification or remove the v1 Prepared missing-backing exclusion.
