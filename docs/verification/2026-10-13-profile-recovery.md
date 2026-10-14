# Profile capability and frozen job ownership evidence

Base: `aaa11920d1f8cc19f111bcd9f49e88baa16bdc25`.
Date: 2026-10-13. No production Store, census or service was used.

## Scope and review

The frozen-input repair checks the caller's logical export job before recovery lease/record
changes, cancellation, or replacement of a retained pointer. Frozen reuse now classifies
only its job mismatch as Preserve. Other corruption/resource/version classifications are
unchanged. No policy, schema, record or token shape changes.

Five synthetic A/B regressions cover Requested, Freezing, Frozen, Missing and Abandoned
roots for resume/cancel. They compare pointer bytes, all object bytes, records, leases and
Store sequence across repeated refusal and Store reopen. Fixtures validate B's artifact
and record inventory before placing that valid foreign root under A's directory.

The configuration compiler is documented in ../PROFILE-EXECUTION-V1.md. It is additive and
has no asynchronous/runtime/publication effects. Main reviewed the scoped detached proposals;
the Frozen-reuse pointer-retirement finding was added during integration review. Initial
field-retention-only policy descriptors were rejected before integration and replaced with
actual evidence semantics and explicit unfinished-runtime disclosures.

## Observed commands

All commands run at the repository root through `tools/moon-local` with the pinned toolchain.
Manual tasks unset CI. Test runs set CARGO_INCREMENTAL=0. No gate was weakened.

- Baseline `pipeline:fmt`, `pipeline:check`, `pipeline:lint-src`: passed.
- Baseline `pipeline:tests-scoped -- -p census-domain --lib -E 'test(profile)'`: 16 passed, 406 skipped.
- Regression-only `pipeline:tests-scoped -- -p census-report --lib -E 'test(job_ownership)'`: five failed as expected; run e93c777b-58c9-4cdd-9857-15342618d4d4. Foreign roots were resumed/cancelled/replaced, or the pointer disappeared after refusal.
- Guard-applied `pipeline:tests-scoped -- -p census-report --lib -E 'test(export::dataset::frozen)'`: 138 passed, 512 skipped; run ddbd0a6d-1b81-417d-8e0c-73c2830184b9. Includes historical policy/resource preservation and the existing durable-step crash matrix.
- `pipeline:tests-scoped -- -p census-domain --test profile_execution`: 15 passed; run b9366270-a27a-4288-b0b7-5f0531ccfb9f.
- `pipeline:fmt-write`: passed; intentional formatting of owned changes only.
- Post-format `pipeline:fmt`, `pipeline:check` (all targets/features), and `pipeline:lint-src`: passed.
- Final post-format `pipeline:tests-scoped -- -p census-domain -p census-report --lib --test profile_execution -E 'test(profile) + test(export::dataset::frozen)'`: 178 passed, 909 skipped; test execution 27.084 seconds. This filter also includes existing workbook profile/readback tests; it is not a complete workspace test run.

These focused tests are not the separate seventeen native Restate fault obligations.
Full release, parser-fuzz/performance campaigns, home-machine/native service checks,
selected-profile workbook readback, and complete specification acceptance are not run here.

## Specification acceptance mapping

- Section 4: configuration identity/refusal and raw-profile preservation covered; runtime selected eligibility/publication incomplete.
- Sections 9/12: foreign-job recovery/cancellation preservation covered by exact-state regressions; broader durability release obligations remain.
- Section 13: existing frozen format/history regression suite passes; full selected workbook plus eight normalized files incomplete.
- Section 14: multi-year profile configuration retained; annual production operation not certified.

The current Beads export is unavailable in this checkout. A deleted historical JSONL snapshot
is not treated as current backlog state. These are evidence records, not an alternative tracker.

## Full verification handoff

`tools/moon-local ci --force --summary detailed`

`env -u CI tools/moon-local run pipeline:gate -- --release`

The release gate does not itself execute all native fault or fuzz obligations. Their documented
Moon lanes in xtask/README.md and docs/NATIONAL-CENSUS-FAULTS.md remain separately required.
