# Bounded raw journal payload reads

Store and StoreSnapshot expose `journal_payload_raw_bounded`, sharing the existing legacy
root/chunk reader. The raw API retains exact payload tokens and duplicate members for typed
callers, rejecting duplicate or unknown outer fields. Existing Value-reader interpretation
remains permissive. No historical bytes, identities or schema are rewritten.

Files: `census-store/Cargo.toml`, `read/{mod.rs,journal_raw_tests.rs}` and
`read/view/{mod.rs,legacy.rs,legacy_wire.rs,legacy_raw.rs}`. The explicit serde_json raw_value
feature does not add a dependency version. Sixteen authored tests cover exact bytes, missing
versus null, duplicates, limits, chunk ownership/corruption, snapshots and reopen.

The physical root/chunk size is checked before fetch against the existing 1-MiB ceiling.
Borrowed fixed envelope slots permit logical grant checks before owned payload allocation;
chunk reconstruction and payload ownership use fallible reservation. Physical frames and
logical payload buffers can coexist: neither the grant nor this change is an RSS guarantee.

Independent changed-surface review corrected premature payload allocation, metadata tree
inflation and a borrowed-deserialization lifetime mismatch. Final static review found no
remaining identified blockers. The shared reader remains the single current implementation.

- PASSED: exact patch applicability on main, root whitespace check and independent static review.
- PASSED on source commit `551a94c3`: Moon format check; workspace all-target/all-feature
  check (3m 30s); `census-store` read regressions, 155 passed, 0 failed, 557 skipped.
- FAILED then fixed: initial format check; initial all-target check found the merged
  offline-provider test calling `TableWalk.len()`. Commit `8c72591a` uses its physical
  `rows` field and formats PR37–39 surfaces. Commit `551a94c3` adds bounded iterator
  envelope processing and the sixteenth raw-reader boundary regression.
- NOT RUN: source Clippy, full fault/release, fuzz campaigns and representative resource tests.
- PENDING: conditional projection transactions and versioned recording consumers, separately.
  This prerequisite alone does not fix concurrent duplicate writes.

Exact verification commands (first three passed above; remaining commands pending):

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:fmt
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:check -- --jobs 1
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-store --lib -E 'test(read::)' --no-capture --no-tests fail
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:lint-src
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:gate -- --release
```
