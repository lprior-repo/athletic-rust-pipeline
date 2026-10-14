# Independent archive lock-order review

## Verdict: REJECTED for the first frozen candidate

Mode: focused STATIC ANALYSIS. No Moon build, test, format, lint, benchmark, or release gate was executed by this reviewer. Root is the sole live integrator/buildrunner. This report is not runtime or release approval.

Reviewed patch: `archive-lock-order-proposal/context.patch`, SHA256 `b0b29fd4ad00324e0dce57e1ac9232ab7144843a26c8c92e6f9bb6ff3a10c7e4`. Adopted prerequisite SHA256 `fe9b1f711f4383cdc03456dca9b4be40547ee2d6c8fb594327d46cbd483eba67`. All 41 declared candidate/baseline/live-source hash comparisons matched at 08:10 UTC. The first candidate has 16 changed files, four new files, and 16 named lock-order test functions (one is an isolated child entry).

## Findings, ordered by consequence

### AL01: new symlink-root refusal exceeds the accepted compatibility change

Disposition: blocker. Candidate `cache/archive/lock.rs:15–18` adds `symlink_metadata(root)` and rejects a root that is not directly a directory. Baseline did not validate the root this way: archive directory creation validates archive children, while traversal through a symlink cache-root alias is accepted. The candidate now rejects that supported alias with a new InvalidData message and changes missing/non-directory-root errors. This is a new filesystem acceptance policy, distinct from Root's accepted archive-first combined-fault precedence. Remove the added validation and retain existing cache-root pair validation, or obtain an explicit revised Root contract. The new `archive_lock_order_invalid_roots_are_preserved_without_following_symlinks` test currently enshrines the changed policy and must follow the same disposition.

### AL02: accidental TempDir method insertion prevents tests from compiling

Disposition: blocker. Candidate `cache/archive/admission/tests.rs:56` changes `tempfile::tempdir()?` to `tempfile::tempdir()?.get()`. `get` is the new capacity-token accessor, not a TempDir method. Restore the original expression. This finding is static; no compiler was run.

### AL03: panic cleanup oracle is too weak

Disposition: blocker pending small test repair. Candidate `cache/archive_tests/lock_order.rs:189–200` accepts any panic using `result.is_err()` and only probes lock availability afterward. It does not establish that the intended callback panicked while both OS locks were held. Add both held probes inside that callback before `resume_unwind`, then check the exact owned panic payload. An unrelated panic or removal of acquisition from this specific path must not count as the intended unwind-cleanup observation.

### AL04: unnecessary Clone/Copy on checked capacity conflicts with literal prerequisite wording

Disposition: blocker pending removal or explicit Root interpretation. Candidate `cache/archive/admission.rs:13` derives Clone/Copy on `ArchiveCapacity`, while the adopted helper section says new helper types are not cloneable. This token is a pure checked scalar, so this is not duplication of an OS lock/capability; neither trait is required by any current caller. Removing them resolves the textual mismatch without behavior change. The actual ArchiveLock and ArchiveAdmission are not cloneable.

## Lock graph and lifetime closure otherwise verified statically

- The four acquiring production sites are legacy write, archive-only capture, legacy replay, and `with_archive_eligible_fence`.
- The single production archive open/lock implementation is `lock.rs`; it uses the original pathname, OpenOptions, real blocking `File::lock`, and Cache error mapper. No test conditional replaces the OS acquisition. Hooks occur before and after it and can fail a test before a recursive acquisition.
- `ArchiveAdmission<'lock>` has a private borrowed guard, derives the archive from that guard's root, performs reservation creation/inventory/recovery without opening the lock, and has no second lock path. Capacity still uses positive u64/default parsing and exact old error text.
- Every production `begin_with_capacity` call has archive initialization before it: legacy write/capture/replay and held eligible publication. Test helper callers also initialize their archive fixture first.
- Writer keeps parse/capture validation before archive acquisition, initializes admission before eligible try-lock, and preserves incoming capture on eligible error. Old/new publication, hard-linking, freezing, reservation accounting, and cache replacement are unchanged; publication.rs changes only its lifetime annotation.
- Replay keeps capacity parsing first, then archive, then eligible, then read/expected comparison. Missing/mismatched cache returns None before full archive initialization/admission.
- The wrapper validates the original cache-root pairing, owns archive and eligible locals until the synchronous callback returns, and passes only a borrow. `publish_eligible_held` checks that both paths belong to the same held root before staging. The return type cannot borrow this local guard. No production await or spawn exists in the changed fence/admission/recovery functions.
- Recovery only changes the fence and held-publisher argument. Store finish/read/outcome and status200 branch are identical. Capacity parsing/init remains inside successful200 publication; missing/corrupt Prepared and non200 do not enter it.
- `publish_eligible` and `recover_prepared` are the complete callers of held publication. Both invoke it inside the new wrapper. Retention calls acquiring publish only after `write_archive` has returned and dropped its own guard; no nested reacquire remains.
- The unchanged eligible-only compare callers are acquisition.rs cache verification/reuse and retention.rs 304 finish. Their callbacks invoke Store verify/reuse/read/finish only. The inspected Store methods acquire/release Store appends internally and never call back into Crawl/archive. No eligible-to-archive edge remains in that caller closure.

## Test and resource review

- All old capacity/reservation sequential reacquisitions explicitly drop the owning guard after dropping the borrowed admission. Loop/scoped guards release normally. No missed old constructor caller was found.
- The actor test owns its worker and channels. After spawn, the main actor records timeout/error results without early return, releases the eligible/archive resources and channel, then joins before reporting errors. The worker's hook uses bounded receive, checks eligible availability before archive, and rejects recursive archive acquisition before entering a blocking second lock. There are no sleeps or detached workers. The test exercises real lock ownership, though the hook deliberately coordinates before the actual contested acquire; it is not a stress/load or lock-wait latency measurement.
- Actual `try_lock` probes verify archive held after acquisition and both locks held before the recovery callback. The ordinary error callback additionally probes inside the callback. The panic callback needs AL03.
- Added exact state assertions cover old/new archive bytes, old eligible body+metadata preservation on busy, replay miss/mismatch, malformed reservation precedence, real parser recovery, unchanged replay journals/receipts, original occurrence/ordinal/time, zero new fixture requests, and missing/corrupt backing refusal before admission.
- The isolated invalid-capacity recovery child sets its environment only through Command and asserts a nonzero child test count. Its entry returns early only when not selected by the parent; do not count that standalone no-op as a substantive regression.
- No Store/domain/service/schema/serialized shape is changed. This review does not certify v2 recovery, broader archive scale, or missing-backing repair.

## Evidence and gate status

Commands executed were read-only `cat`, `sed`, `nl`, `rg`, `find`, `wc`, and SHA256 comparisons, plus writes confined to this review directory. The candidate source scan finds exactly one production `File::lock`, no old `with_eligible_fence`, no await/spawn in changed fence/admission/recovery functions, and all constructor call sites migrated. No implementation execution has occurred.

Root must rerun nonzero lock-order, archive/capacity/reservation/acquisition, and existing service dedicated-worker/WorkAdmission tests after integrating a corrected frozen patch, followed by the required Moon gates. Commands and detailed scenarios are in `TEST-PLAN.md`. Formatting is intentionally unverified because only Root may run it. No performance improvement or zero-cost claim is made; archive serialization remains blocking and borrowed guards change ownership, not concurrency budgets.

## References read

Repository AGENTS.md and ARCHITECTURE.md, ADR-002, ADR-012, ADR-017, ADR-028, ADR-037, ADR-038; recovered rust-contract, holzman-rust, scott-ddd-refactor, async-rust-reviewer, test-planner, and test-reviewer skills; `holzman-rust/references/nasa-jpl-standards.md`; `async-rust-reviewer/references/structural-correctness.md` and `cancellation-safety.md`; `test-planner/references/testing-philosophy.md`; `test-reviewer/references/determinism-and-mutation.md` and `behavior-test-rubric.md`.
