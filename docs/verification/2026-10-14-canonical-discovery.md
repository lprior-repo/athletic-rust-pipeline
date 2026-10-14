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
  the final repeat below verified their refactors.
- PASS: final all-target/all-feature check (3m20.257s); the two subsequently added public
  recovery test files compiled and ran in the 15-case integration target.
- PASS: frozen source-lint repeat (1m12.419s), formatting (7.899s), and authoritative
  size scan (8m7.455s): 19,341 callables, 1,676 files, zero functions over 60 lines.
  Three changed production files exceeded 300 physical lines; their follow-on split
  preserves complete function bodies; the follow-on all-target check below compiled them.
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

## Atomic source-access follow-on

Base: PR #33, merge `9120e53c83fa6961ed22c5b1b0e8db1dabf0586d`.
Changed surfaces: Store `write_batch/{source_access,append,commit,conditional}`;
Crawl `net/access_conditions` and fallible callers; Service team-source acquisition/ledger,
roster refusal and CLI callers. Store acquisition read/recovery/cache module splits retain bodies.
The commit diff is the exact file-coverage inventory.

Source-access rows merge under the existing store commit lock with stage/refusal evidence in
one durable batch. Existing same-ID evidence is merged rather than stale-overwritten; unrelated
IDs remain. Shared row/byte limits and conditional staged-byte limits reject the whole batch.
Historical schema, receipts and replay semantics are unchanged. Bounded fallible access snapshots
retain source fields and deterministic ordering. Persistence failure retains the original failure
class and both causes; it does not manufacture successful acquisition or renew retry allowance.

- PASS: Store acquisition/write-batch tests 125/125, 570 skipped;
  nextest `94de984d-96c1-4bf8-baf8-b61f96e2f8b1` (1m5.868s task).
- PASS: Service team-source/roster/access tests 81/81, 794 skipped;
  nextest `ee0339b0-352a-4d81-8127-6c0604bace28` (2m34.022s task).
- PASS: all-target/all-feature serial check (3m50.358s task).
- Earlier failures: missing test closing brace and extracted `classify` import were repaired;
  the initial non-serial all-target check ended with rustc SIGKILL. The serial repeat passed.
- PASS: Crawl access-snapshot tests 2/2, 1,445 skipped;
  nextest `547ddeaa-92ef-44fa-a368-7bb1f515d845` (2m33.471s task).
- PASS: after review required a pre-materialization bound on retained SourceAccess,
  Store repeat 126/126, 570 skipped; nextest `0d273cbd-7622-4236-b3fd-dd389ddba721`
  (52.625s task). Size and payload are read from one snapshot under the commit lock.
- PASS: source lint after retained-read repair (1m19.189s task).
- Implemented: early-index failure receipt plus access evidence use one batch; roster/index reuse
  one service-local causal-error type. Sweep finalization retains primary and secondary failures.
- FAILED then repaired: five new Service assertions used private Store::count; they now inspect
  physical rows through public Store::walk_table. No tests executed in that failed build.
- FAILED then repaired: strict lint found a 224-byte causal-error result. Only its secondary
  error was boxed; the executed size and exact-primary-source identity regressions passed.
  This fixed-size error allocation is not a recoverable-OOM guarantee.
- Service first expanded execution: 105/106 passed. The failed new test incorrectly expected
  an arbitrary old locator to survive the unchanged replacement policy; its fixture now uses
  the explicitly retained legacy-inventory obligation. No production merge rule was weakened.
- PASS: final expanded Service/CLI repeat 112/112, 919 skipped;
  nextest `bd96b27b-4b75-45c4-ba41-627c07581013` (2m40.168s task).
- PASS: final format (7.979s task) and serial all-target check (3m36.656s task).
- FAILED: final source lint still rejects the 128-byte causal error (31.866s task).
  Both retention causes are now boxed, with exact primary Error::source identity and a
  strictly-below-128-byte regression. Compact-layout source lint PASS (30.100s task).
- Review repair: failed-attempt persistence retains the primary class/cause even with zero access
  observations. Four injected pre-commit refusal tests cover transient, terminal, typed resource
  and successful-attempt handling; these do not establish physical disk-failure durability.
- PASS: compact-layout/empty-evidence Service/CLI repeat 116/116, 919 skipped;
  nextest `ed3268af-2303-4f1f-857f-868278d53992` (3m15.487s task). All four injected
  failure cases and both exact Error::source identity regressions passed.
- PASS: final integrated format (8.379s), serial all-target check (1m31.812s), and
  source lint (24.254s). These are the final Rust-tree checks for this slice.
- OPEN release blocker: the shared lifetime access-condition map can exceed the bounded snapshot
  batch and then refuse unrelated source completion. Per-operation evidence ownership/continuation
  is a separate repair; current overflow refusal is not evidence of sustained production liveness.
- NOT RUN: full release/native durability/fuzz campaign, representative peak-memory qualification.
  Broader publication/production requirements remain open. Early-index evidence and sweep
  dual-error retention are implemented, with latest verification pending as stated above.

Focused home commands (in addition to the full commands above):

```sh
env -u CI tools/moon-local run pipeline:check -- --jobs 1
env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-store --lib --build-jobs 1 -E 'test(acquisition::) | test(write_batch::)'
env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-service --lib --bin census-service --build-jobs 1 -E 'test(team_source::) | test(census::sweep::) | test(census::tests::collection_access::) | test(restate_services::jobs::source_access::) | test(cli::tests::teams::)'
env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-crawl --lib --build-jobs 1 -E 'test(net::access_conditions::tests)'
```
