# Portable M1 lock-order evidence

Narrow M1 execution status: passed. Full release and formal bundle approval: unverified.

The first actual Root-run lane passed 96/96 tests, including all 16 named lock-order rows, with 3200 skipped. Corrected independent static review approved the frozen implementation for integration/execution. Source lint, all-target check and format also passed with exit 0. All 16 source pins match committed M1 source `0cde4e29ffbb95baa4d427b460a31dc0925dfd71`, tree `73bb89461cf332eca30849c351c982431d1bbdf4`. This is not a full-release claim.

This directory is self-contained. Paths below are relative to it:

- `ADOPTION.md` is the brief adoption/status summary for docs integration.
- `proposal/ADOPTED-PREREQUISITE.md` is the immutable exact adopted contract.
- `SOURCE-PINS.json` and `source/` retain the exact 16 Root-formatted source files used by the first test run.
- `formatted-source.patch` records reviewed-candidate to Root-formatted source changes.
- `proposal/` retains the original rejected and corrected patches, exact correction, pins, names and worker test plan.
- `review/` retains first-candidate findings, their exact corrected review/dispositions, pin checks and independent acceptance plan.
- `raw/` contains immutable copies of terminal command stdout/stderr logs and the Root command attestation when supplied.
- `TEST-COVERAGE.json` maps all 96 PASS rows and all 16 named cases to raw log lines and exact source hashes. Read its isolated-child qualification.
- `GATE-STATUS.json` records all terminal basic-gate results; `COMMIT-RECEIPT.json` binds the exact source commit/tree and all 16 file bytes.
- `ACCEPTANCE-MAPPING.md` maps preservation, precedence, lifetime and scope acceptance to executed tests and static review.
- `LIMITS.md` is the complete not-run/claim boundary.
- `configuration/` pins the checked-in toolchain/build inputs and Root's generated tool fingerprint.
- `ARTIFACT-AUDIT.json` and `SHA256SUMS` provide local integrity checks; neither is a release certificate.

The historical proposal's NOT RUN statements describe worker execution provenance. They do not override later, separately identified Root-run logs. Current status is determined by terminal gate receipts and source pins, not summaries.
