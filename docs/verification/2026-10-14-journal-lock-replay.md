# Journal lock and repeat admission

Base: `d0e12a76` (qualified raw-reader repair). Public `Store::journal_done` now owns
append serialization; already-locked lineage creation uses the internal helper.
Encoding and historical journal bytes are unchanged. Conditional applications validate
store/operation/count structure before replay; only first-write aggregate byte admission
is deferred until the locked outer receipt decision. Equal replay retains the original
receipt; malformed structure and conflicting identity still refuse.

Mapping: `write.rs`, `lineage.rs`, `lineage/{tests,journal_lock}.rs` cover serialization,
lineage recursion, pinned raw reads and error-path release. `write_batch/{commit,conditional}.rs`
and `write_batch/commit/tests.rs` cover exact 32-MiB boundary replay and structural refusals.

- PASSED: focused Moon tests, 36 passed, 0 failed, 688 skipped, including 12 new regressions.
- FAILED then fixed: initial test compilation used `scan<Value>`, which lacks Entity.
  The corrected test compares exact before/after physical-table digests instead.
- PASSED: Moon format check; independent static lock/admission review and patch applicability.
- NOT RUN for this delta: workspace all-target check, source Clippy, full release/fault/fuzz
  and representative resource qualification. No live store, census or service operation.
- PENDING: atomic projection witnesses and versioned raw recording consumers; these
  prerequisites alone do not fix concurrent projection duplicate writes.

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:fmt
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:check -- --jobs 1
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-store --lib -E 'test(write_batch::commit::tests::) | test(lineage::)' --no-capture --no-tests fail
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:lint-src
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:gate -- --release
```
