# Canonical discovery and operational diagnostics

Scope: one bounded MileSplit meet-page reader/parser across ordinary acquisition, provider CLI
and prepared J0 discovery. Superseded implementations are removed; historical input precedence,
URL ownership, duplicate/order behavior and schema quarantine semantics remain explicit.
OpenWork retains actual Teams stage and source refusal slug/kind/reason. Missing measurement
remains distinct from a measured empty list. Refusal counts do not establish rate blocking.

Recovery changes pre-admit boxed payloads, preserve committed terminal/receipt on consumption
refusal, and distinguish retained success from negative outcomes in the private Service type.
Function extractions retain validation, read/debit and archive-fence-before-append ordering.
National drive/resume behavior is preserved, with exact ordinal boundary fixtures.

Executed evidence:
- PASS: Store acquisition/recovery 91/91; 591 skipped; nextest
  `c69d91f6-b45d-4c94-bd15-7f7a2e10561f` (44.558s task).
- PASS: public parser/reader integration 12/12; zero skipped; nextest
  `670770a8-1912-4cfa-9b40-7f4461a88dc5` (2m26.405s task).
  Synthetic offline cache fixtures made zero physical requests.
- PASS: final public Crawl integration 15/15, zero skipped; nextest
  `a415e75b-3371-443d-b5a2-26234b0ef61f` (50.135s task). This repeats the 12 parser cases
  after tracing changes and adds three genuine Prepared recovery cases: one-shot body ownership,
  exact receipt/loss replay and publication refusal preserving committed truth. Counts overlap.
  No live source access or private SDK/native qualification is inferred.
- PASS: all-target/all-feature check at the preceding checkpoint (3m39.757s); source lint
  after the recovery/type repairs (24.236s); formatting after final fixture integration.
- Earlier failures: strict lint found four Crawl enum layouts, unnecessary borrows/returns,
  Service resolution layout and one Sized-reference error. Each was corrected. A task-name
  typo was rejected before running lint. The original strict size scan found 16 functions;
  refactors are included and its authoritative repeat is pending.
- PASS: final all-target/all-feature check (3m20.257s); the two subsequently added public
  recovery test files compiled and ran in the 15-case integration target.
- PENDING: final source-lint repeat and authoritative size scan; three changed production
  files remain over the 300-line architecture target, with a detached split prepared.
- NOT RUN: OpenWork/CLI/Service SDK runtime regressions, complete Crawl lib tests, native crash,
  cancellation, physical resource qualification and full release/security/fuzz gates.
  The prior monolithic Crawl test compilation SIGKILL remains unresolved, not a test pass.

Acceptance mapping: source discovery uses one implementation; operational coverage exposes
retained reasons; durable recovery retains owner/receipt/budget semantics. This does not certify
all-source acquisition, reusable production, selected-profile completion, national workbook
parity or the historical 64-GB memory target. SourceAccess persistence and streaming are separate.
The main specification and all historical artifacts remain unchanged. Current Beads export is
unavailable; historical snapshots are not treated as current.

Home verification, from repository root:

```sh
tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
tools/moon-local run pipeline:tests-scoped -- -p census-store --lib -E 'test(acquisition::)'
tools/moon-local run pipeline:tests-scoped -- -p census-crawl --test milesplit_meet_pages --test prepared_recovery_v2
tools/moon-local run pipeline:tests -- --build-jobs 1
env -u CI tools/moon-local run pipeline:xtask -- scan
env -u CI tools/moon-local run pipeline:xtask -- seams
env -u CI tools/moon-local run pipeline:gate -- --release
```
