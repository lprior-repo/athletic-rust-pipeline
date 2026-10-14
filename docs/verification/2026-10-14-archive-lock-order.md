# Archive lock-order repair: focused verification

Source commit: `0cde4e29ffbb95baa4d427b460a31dc0925dfd71`.
Source tree: `73bb89461cf332eca30849c351c982431d1bbdf4`.

All 16 changed paths are pinned in the [portable evidence](evidence/2026-10-14-archive-lock-order/README.md).
The repair uses the same actual archive lock before eligible-cache and Store append locks, with
borrowed admission avoiding recursive archive acquisition. It preserves current v1 encodings,
original occurrences and deferred admission behavior. It does not implement Prepared recovery v2.

## Executed results

- Moon focused regressions: 96 passed, 3200 skipped; nextest `783d417a-ef14-4fbf-9071-bb75e22ee330`.
- All 16 named lock-order test entries selected. The isolated parent establishes actual child
  execution; the child's no-environment entry is not counted as independent invalid-capacity proof.
- Moon source lint, serial all-target check and format: each exit 0.
- Root verified every portable artifact hash before integration. Formal specialized evidence
  review remains unavailable/unexecuted, as disclosed in the retained bundle.

[Exact commands and executor attestations](evidence/2026-10-14-archive-lock-order/raw/root-command-receipts.json),
[raw logs](evidence/2026-10-14-archive-lock-order/raw/tests-first.log),
[independent review](evidence/2026-10-14-archive-lock-order/review/REVIEW.md), and
[acceptance mapping](evidence/2026-10-14-archive-lock-order/ACCEPTANCE-MAPPING.md)
retain exact scope, corrected first-candidate findings and combined-fault precedence.

## Remaining limits

This is focused local verification. Full release, native restart/fault scenarios, representative
archive scale, production lifecycle and the complete October 11 specification remain unverified.
Missing/corrupt Prepared backing still fails closed without hidden refetch; this repair does not
close that separate recovery gap. No performance improvement is claimed.

Full home verification follows the unchanged [capture checkpoint commands](2026-10-14-durable-capture-green.md).
