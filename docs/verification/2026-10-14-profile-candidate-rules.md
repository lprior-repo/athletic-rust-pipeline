# Profile candidate rules — 2026-10-14

R1 maps to `census-domain/src/participation_candidates/` and its lib export.
R2 maps to `census-domain/src/affiliation_candidates/current{.rs,/}` and
`selection.rs`'s explicit historical/current temporal envelope.
These pure APIs return candidate assessments, never accepted profile authority.

R1 binds profile, policy, frozen view, athlete/result, dated school status and
cohort-specific eligibility windows. Competing boundaries stay unresolved rather
than admitting their union. R2 preserves retained affiliation evidence outside
an acquisition scheduling window, while enforcing capture visibility and knowledge
cutoffs separately from effective dates. The historical selector retains its old
acquisition/effective-date constraint through the same canonical reduction.

Cloud command, CI unset and MOON_RUST_BUILD_JOBS=1:
`tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain --lib -E 'test(participation_candidates) | test(affiliation_candidates)' --no-capture --no-tests fail`
- Initial: 105 PASS, 1 FAIL, 476 skipped; a stale exact test expectation omitted
  SummerBoundaryUnresolved after changing the cohort without its bound window.
  Corrected only the expectation; both refusal reasons and school-status evidence remain.
- Final: 106 PASS, 0 FAIL, 476 skipped; Nextest
  3bfc2fca-387f-4343-95f2-3395257e8f42; 1.094s tests, 14.032s task.
- PENDING: changed-tree format/all-target check. NOT RUN: strict lint/full release,
  real-source qualification, selected-profile E2E and representative peak RSS.

Independent changed-surface review caught conflicting-window union and premature
expiry of old affiliation facts; regressions cover both. Functional-core, typed
DDD boundaries and bounded-inventory review applied. No async effects were added.
Owner-authenticated semantic payloads, durable accepted projections, frozen profile
binding and actual geography/contact/mark consumers remain to be integrated.
R1 and R2 remain OPEN; syntactically consistent references alone are not evidence.
