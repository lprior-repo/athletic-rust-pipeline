# Parser-envelope portable evidence

Status: **Scoped tests, API checks and basic gates PASS; strict-size scan and module-seam gates FAIL. Formal acceptance REJECTED.** All 107 selected tests passed, including the six new envelope regressions. Exact source commit binding is verified. No evidence in this bundle authorizes production publication or asserts full-specification verification.

## Identity and provenance

- Original proposal base: `49ad9922a9ba24148bd4e73dbb46993ee7f39fce`.
- Original frozen `proposal/context.patch`: SHA256 `f5b3988005370686f93c351949d804c0b8b72b3c6c6cd2f895cbd11c0d4b8aba`.
- M1 source integration base: `0cde4e29ffbb95baa4d427b460a31dc0925dfd71`.
- Observed evidence-only repository HEAD: `7996734d87d0305eb8bb2060d5e65199cee95a6e`. Its delta from M1 is confined to documentation; [SOURCE-PINS.json](SOURCE-PINS.json) preserves the exact path list.
- Parser-envelope source commit: `771072a1013017a4f8b4993751f801df16f789a2`, tree `1a811cfe9328a4e3eaf7b923f2c18671c540aaef`. All 38 snapshot files were compared against that commit and match exactly; [COMMIT-RECEIPT.json](COMMIT-RECEIPT.json) retains the result.
- Separate seven-expectation repair patch: SHA256 `6b81c436ef73da969cb7bab283b1926b5a471a83c8e2d2bb5cc747aa299ebfbb`.

## Read order

0. [Additive status and historical snapshot](ADDITIVE-STATUS.md).
1. [Acceptance mapping](ACCEPTANCE-MAPPING.md) and [assurance bundle](assurance-bundle.md).
2. [Gate status](GATE-STATUS.json), [command receipts](raw/root-command-receipts.json), and raw terminal logs under `raw/`.
3. [Source pins](SOURCE-PINS.json), [delta coverage](DELTA-COVERAGE.md), [formatted source patch](formatted-source.patch), and [review-to-integrated delta](review-to-integrated.patch).
4. [Independent review](review/REVIEW.md), [six executed new tests](TEST-COVERAGE.json), [all 107 executed tests](TEST-RESULTS.json), and [eight API probe states](API-PROBE-STATUS.json).
5. [Terminal static gate failures and passes](STATIC-GATE-RESULTS.md), [exact static findings](STATIC-GATE-RESULTS.json), and [executed API boundaries](API-RESULTS.md).
6. [Limits](LIMITS.md), [truth-serum status](truth-serum-report.md), and [final evidence decision](final-evidence-decision.md).

## Layout and portability

`proposal/` preserves the original before/candidate files, frozen patch and both separate shared-file patches. `proposal/compile-repair-01/` preserves the formatted first-run test source and exact corrected source independently of the frozen proposal. `review/` preserves the independent reviewer output, source overlays, context, static scans and complete compiler-probe snippets. `source/` contains the 38 root-integrated files; `integration-base/` contains their existing M1-base counterparts. Original repository-relative paths are preserved below these directories.

`configuration/` pins the Moon launcher, wrapper, workspace/test configuration, toolchain declaration, lockfile and manifests. `context/` retains repository instructions, architecture and task reference. These files support interpretation and reproduction against the named repository commits; this bundle is not a complete buildable checkout or dependency mirror.

Existing reports and raw logs retain their original absolute source paths as provenance. All material source, proposal, review, configuration and terminal logs cited by the bundle are copied locally. A running command is represented by its attested pending state and has no frozen final-log pointer.

Root command receipts explicitly identify themselves as executor attestations, not raw tool transcripts. Exact invocation and outer exit status come from those receipts. Raw logs independently retain compiler diagnostics and Moon outcomes. The assembler ran only reads, copying, hashing, patch comparison and artifact audits; it did not run builds, tests, formatters, source gates, live-source edits or publication.

The original independent review/proposal and archived provisional describe their earlier states accurately. Additive terminal evidence updates the current bundle without rewriting those historical reports. Counts derive from the actual Nextest PASS records and terminal summary. The former 09:18:46Z provisional is preserved completely in `history/provisional-20261014T091846Z.tar.gz`, with its original SHA256SUMS hash recorded in `history/PROVISIONAL-RECEIPT.json`.

The final command index contains 26 invocations/attempts with exact Root receipts. Expected negative API exits remain exit 1 and are labeled successful negative oracles separately. Strict-size and seam exit 1 remain genuine failing gates, irrespective of their serial wrapper exit 0. Artifact integrity PASS is not acceptance approval.
