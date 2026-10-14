# Exact source and integration delta coverage

The [source inventory](SOURCE-PINS.json) has 38 unique paths: the 36 changed/new paths in the original `source-pins.json`, plus `net/acquisition/read.rs` and `net/acquisition/tests/lock_order.rs`. All are below `crates/census-crawl/src`. Each row contains the M1-base hash when present, original proposal hash when applicable, independent review-overlay hash, integrated hash and byte length.

The [formatted patch](formatted-source.patch) is constructed against the exact M1 source base, including the two new outcome files. It is not the original f5b3988 patch. The [review-to-integrated patch](review-to-integrated.patch) preserves every byte difference from the independent review's 38-path effective overlay. Twenty-nine paths are byte-identical to that overlay. Nine differ:

- `milesplit/fetch.rs`: formatting of the same interpreter call.
- `milesplit/owned/effect.rs`: formatting of the same interpreter/resource-retention match.
- `milesplit/owned/mod.rs`: formatter ordering of re-exports.
- `milesplit/owned/outcome/tests.rs`: formatting plus the separately preserved seven `Vec<String>` to `HashSet<String>` exact-empty expectation corrections.
- `milesplit/results/run/windows/completion.rs`: formatting of the same format arguments.
- `net/acquisition.rs`: formatting of the same retained-state destructure/construction.
- `net/acquisition/read.rs`: formatting of the held archive-lock call; its existing fence and held-lock argument remain present.
- `net/acquisition/tests.rs`: preservation of M1's `mod lock_order;` registration, which was intentionally excluded from the frozen envelope candidate.
- `net/acquisition/tests/lock_order.rs`: formatting of M1's code after its separate three-accessor envelope migration.

These classifications come from inspection of the preserved exact delta, not from a passing compiler or formatter-check. The two Root formatter invocations have their own terminal logs and exit attestations. No patch is silently overwritten to make the original proposal appear compile-clean.

## Shared integration

`proposal/read-integration.patch` and `proposal/lock-order-tests-accessor.patch` are kept separately with `proposal/shared-integration-pins.json`. The independent review preserves their combined detached source and hashes in `review/reviewed-candidate/` and `review/coverage.json`. Root's formatted current versions are independently pinned under `source/`. The shared read delta does not replace M1's recovery fence or archive-lock ownership, and the test delta changes read access only.

## Compile repair

The first integrated test source contained seven comparisons of journal-key `HashSet<String>` values against `Vec<String>::new()`. The failed raw log retains all seven original errors at formatted test lines 111, 131, 132, 150, 151, 199 and 200. The before/after snapshots and patch in `proposal/compile-repair-01/` show only the expected collection type changes, preserving exact-empty assertions. They make no production logic change and prove no passing test result by themselves.

The first formatted test source hash was `376eeee42a5406f1e0147f554a188cb37681c6599ff9964881bab56c836c3da4`; corrected source hash is `4ac406d1b4e3719b3805f222dd55c724cba828f505fdbe73abb306d95bf71a8c`. All 38 commit-to-snapshot byte comparisons now match source commit `771072a1013017a4f8b4993751f801df16f789a2`; see `COMMIT-RECEIPT.json`.
