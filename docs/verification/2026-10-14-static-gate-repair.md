# Static gate repair, 2026-10-14

Scope: six bounded-function extractions, installation/bootstrap defaults sharing one existing
versioned domain value, and two stale future-format test fixtures. No gate thresholds or seam
allowances changed. The other eight acquisition size failures belong to the separate recovery slice.

Historical defaults stay exactly 8 concurrent, 48 GiB and 30 seconds. Literal bytes/digests,
metadata refusal precedence, co-op interval/audit order, endpoint ownership/drain and cached
response preservation have thirteen new regressions. Four boundary tests moved unchanged.

Executed before the two fixture corrections: Moon format and workspace all-target/all-feature
check passed. Focused Nextest run 34401d9f-6b5d-4928-b83a-75dc614a8e9c ran 416 tests:
415 passed, 1 failed, 3187 skipped; all thirteen new regressions passed. The existing failing
installation test treated lifecycle 4 as unsupported despite current support for 4. It and the
analogous migration fixture now use checked current+1 and still assert exact FormatNewer fields,
precedence and unchanged durable bytes. Corrected focused run
d0250724-87dc-4bc0-abda-25a002deb121 passed both tests (618 skipped).
Moon format-write, source lint, final all-target/all-feature check and format check all passed.

Final strict seams passed with no violations. Comments and panic-extraction passed over
2871 Rust files (plus three rendered templates for panic extraction). Strict scan failed on
the eight pre-existing acquisition functions listed below; all six functions changed here now
pass. No full 416-test rerun, release gate or full-spec verification was executed.

Remaining scan failures (logical lines, limit 60): transition/work::register_acquisition_work
66, terminal::reuse_acquisition_cache 66, read::read_work 77, read/closure::attempt 64,
qualification::declaration 62, qualification/inventory::inventory 94, cache::inventory 82,
backing::validate_metadata 63. These are owned by the next recovery slice.

Rerun from the repository root (commands below are not additional execution claims):

```sh
tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
tools/moon-local run pipeline:tests -- --build-jobs 1 -E 'test(affiliation_candidates) | test(model::installation) | test(installation::) | test(bootstrap::) | test(net::) | test(contract::boundaries) | test(format::migrate::exact_tests)'
env -u CI tools/moon-local run pipeline:xtask -- scan
env -u CI tools/moon-local run pipeline:xtask -- seams
```
