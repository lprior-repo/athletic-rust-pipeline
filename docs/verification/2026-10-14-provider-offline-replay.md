# Explicit offline provider replay

`cli/provider.rs` adds `--offline`, requiring explicit global `--store` and conflicting with
`--refresh` before store access. It configures the existing Fetcher and collector dispatch.
No new acquisition or parsing engine, persisted schema, retry reset or provider activation.
`docs/OPERATIONS.md` describes stopped ownership, current-cache selection and partial outcomes.

Independent static review confirms GET/JSON/form requests share the guard before admission and
HTTP/browser dispatch. Eligible cache metadata retains its actual acquisition time. Twelve tests
in `cli/provider/offline_tests.rs` cover CLI constraints, loopback HTTP/browser ingress accept
traps with non-offline positive controls, corrupt/missing cache, exact fixture results and reopen.
The collector replay uses rebuilt provider composition plus existing dispatch; `run_provider`
orchestration itself is source-reviewed, not executed. Source-shaped fixture URLs are fenced;
held locks are not used as the independent zero-dispatch oracle.

- PASSED: patch applicability and whitespace checks; independent changed-surface source/test review.
- NOT RUN: all twelve Rust tests, formatting, compilation, lint and release checks. Cloud memory
  admission prevents execution; no zero-network runtime certification is claimed.
- PENDING: execute focused and full home verification below. Static review fixed a test helper
  shadowing error and the local browser ingress response's required `x-restate-id` header.

```sh
env -u CI tools/moon-local run pipeline:fmt
env -u CI tools/moon-local run pipeline:check -- --jobs 1
env -u CI tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --bin census-service -E 'test(cli::provider::offline_tests::)' --test-threads 1 --no-tests fail
env -u CI tools/moon-local run pipeline:check pipeline:lint-src
env -u CI tools/moon-local run pipeline:gate -- --release
```

This does not select arbitrary archived occurrences, resume completed Restate interpretations,
certify freshness/coverage, or change the existing CLI school-year context. Store recovery and
append-only interpretation writes remain possible. No live provider command was run.
