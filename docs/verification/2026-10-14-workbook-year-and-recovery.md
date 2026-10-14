# Selected workbook year and frozen ownership evidence

Date: 2026-10-14. Code commit: `e22c5b346707edb7199f193546e3508ec7dfb4e4`.
Published baseline: PR #26 merge `5867b091acc7256bd821f43de843afd982453388`.
No live census, production Store or production service was used.

## Changed scope and review

Coverage, cohort-dependent Review/Conflicts, and their independent readback now use the
existing selected recruiting derivation. Unverified and contradictory candidates remain
visible. Run Metrics distinguishes selected workbook cohort from unchanged legacy 2027
census counters. The public legacy retained-records API keeps its explicit previous scope.

Semantic policy advances from 16 to 17 so corrected output cannot reuse a policy-16
Published receipt. Schema/codec/token protocol 3, manifest 2, physical Store schema 2,
15 streams and 16 fingerprints are unchanged. Historical policy-16 descriptor bytes,
archives, root markers, records, leases and receipts remain preserved and refused by
current semantic acceptance. The new historical Published fixture is explicitly synthetic
and integrity-only; it is not certification of an old workbook's contents.

Frozen resume/cancel now checks that the retained manifest is the first object in the
durable attempt inventory before dispatch or mutation. Mixed A-manifest/B-attempt roots
are refused without changing either job. The earlier manifest-job check remains intact.

Capture can consume one supplied StoreSnapshot internally; origin checks, run metadata,
streams and fingerprints use that view. The legacy entrypoint delegates to this seam.
This is a prerequisite for selected-profile publication, not completed profile execution.

Independent changed-surface review covered the scalar derivation, policy and descriptor
cutover, history matrices, receipt transition, and mixed-pointer ownership consequences.
Root reviewed and integrated the capture seam. The current token golden omitted from the
initial policy proposal was found by execution and corrected from 16 to 17; historical
fixtures were not retagged. All 40 changed code/ADR paths and exact Git blob IDs are listed
in the adjacent `2026-10-14-workbook-year-and-recovery-files.json`.

## Observed evidence

All developer commands used `tools/moon-local` and the pinned toolchain. Tests used
`env -u CI CARGO_INCREMENTAL=0`; no gate or test concurrency setting was weakened.

- Scalar regressions before repair: 11 failed, one legacy guard passed. After repair:
  12 passed; Nextest `219a4506-7e21-4ce0-b99e-b622ff1c0465`.
- Mixed-pointer regressions before repair: 2 failed, 5 existing ownership tests passed;
  run `9a7339ca-8e3b-4417-9806-e7775c80ff51`. After repair: all 7 passed;
  run `a13df737-1da5-46e1-9044-e26e9f11e015`.
- Same-snapshot capture focused run: 6 passed;
  run `32325947-70bd-4429-95c8-61530b7ad23a`.
- First integrated 178-test run: 175 passed, current token golden failed, two resource
  cancellation matrices stalled and were interrupted normally. An attempted extra
  `--test-threads 1` was rejected before testing because the task already supplies 16.
- Both stalled tests subsequently passed alone and in repeated combined execution without
  code or concurrency changes. The initial stall remains unexplained; no flake repair is claimed.
- After correcting the current token golden, the exact combined selector below passed
  **178/178**, 933 skipped, run `8ccdd093-b418-42eb-9f8e-5d460c1b8def`, 25.763 test seconds.
- On code commit `e22c5b34`, `pipeline:fmt`, `pipeline:check` (workspace/all-targets/all-features)
  and `pipeline:lint-src` all passed. A later documentation-only commit does not change Rust inputs.

Earlier broader workbook testing passed 290/294. Four publication kill/abort tests failed
with `Operation not permitted` while binding a local Unix socket. An escalated retry failed
before execution with a sandbox mount setup error. These four fault tests remain unverified
here; neither this focused pass nor the freeze crash matrix substitutes for them.

## Reproduction and full-verification handoff

From the repository root with the pinned toolchain:

```sh
env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- -p census-report -p census-domain -E 'test(profile_execution) | test(export::dataset::frozen) | test(scalar_year_) | test(policy_seventeen) | test(policy_sixteen)'
env -u CI tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- -p census-report --lib -E 'test(workbook::)'
tools/moon-local ci --force --summary detailed
env -u CI tools/moon-local run pipeline:gate -- --release
```

The first two commands passed as recorded above. The third needs a post-cutover complete
rerun in an environment supporting its fault tests. Full CI, release, separate 17 native
fault obligations, parser fuzzing and performance campaigns were not run for this delivery.
Their exact supported Moon lanes and prerequisites remain in `xtask/README.md` and
`docs/NATIONAL-CENSUS-FAULTS.md`; do not run them against production data.

## Specification acceptance mapping and remaining boundaries

- Sections 4/13/14: selected scalar cohort output and tamper refusal tested; complete
  configured five-state/UP profile execution remains unfinished.
- Sections 9/12: mixed-attempt recovery preservation tested across retained states/reopen;
  complete native fault and production lifecycle qualification remain unfinished.
- Sections 12/13: coherent supplied snapshot capture tested; full selected evidence-family
  closure, incremental publication and eight normalized files remain unfinished.
- Sections 8/13: no newly qualified dated affiliation or coach evidence is claimed.

The complete requested v2 document is preserved at
`docs/specifications/athletic-recruiting-end-vision-spec-v2.md`, source SHA-256
`a3b687cc06952dbceaeda2f21300e4769acb96130570fc1174ff70210a09f11e`.
Current Beads/Dolt export is unavailable; historical JSONL is not treated as current backlog.
Merging these repairs does not verify the full deliverable.
