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

## Bounded source-capture follow-on

Base: PR #34, merge `6419181c8f5ca3cf77169464f8c2019115c16349`.
Fixtures: `e61605dc`; implementation: `bb5981f5`; flush batching: `d8eccee0`. Their commit diffs give exact file coverage.
The single SourceMeets capture now streams into owned Unix read-only temporary partitions;
selection semantics, historical record ordering, Store generation fencing and pending obligations
remain unchanged. The previous resident capture backend is removed. This is not whole-export
streaming: resident ExportDataset, population copies and verification indexes remain open work.

- PASS: final results_arms 107/107, 835 skipped; nextest
  `b325727c-1a82-4f61-92e6-11f73cbee3c7` (1m55.109s task).
  Flush batching retains byte/digest parity, admission-before-write and failed-write charges.
  First repeat was 106/107: a fixture cloned strings before measuring capacity; corrected to
  serialize and move the same rows, retaining exact byte assertions. No production limit weakened.
- PASS: final source lint (30.332s) and formatting (9.159s). Initial fixture format failed;
  Moon fmt-write corrected it before the passing checks. Initial shell lacked restored toolchain
  PATH and executed no checks; sourced the existing toolchain environment and retried.
- PASS: final all-target/all-feature serial check (1m36.301s task).
- PASS: offline CLI publication and captured-fixture pipeline/readback, 3/3, zero skipped;
  nextest `cf6f4d2a-ef51-4df2-949d-9ec25d3879c0` (1m11.583s task).
  These isolated synthetic/captured fixtures do not certify current live-source discovery.
- PASS: strengthened offline CLI cycle 1/1, zero skipped; nextest
  `4d67326a-5ce1-4e1f-b104-0917fa9a6a7a` (1m5.674s task). Independently reads
  exactly ten athlete/result identities, 12-second Track100m PRs, date and source URL in XLSX,
  and matching fixed CSV values (fixture commit `c5d49f50`); empty PR output now fails. Earlier fixture lacked eligible event
  ownership and could pass without any PR rows. Production selection rules were unchanged.
- PASS: address/provenance and PR-summary regressions 17/17, 668 skipped; nextest
  `6b818e42-ac3d-4a61-988b-34a456f9e1fa` (3m16.430s task).
- FAILED then repaired: module path/import errors and a redundant closure; first expanded suite
  97/98 passed. The SDK oversized-frontier fixture now expects the existing earlier allocation
  guard; a separate direct-wire 4096/4097 test preserves exact frontier-boundary coverage.
- Measured debug synthetic 100,000 rows × 1,024 payload bytes, two accepted trials each:
  old child peak RSS 348,307,456 / 348,483,584 bytes; candidate 198,156,288 / 199,294,976.
  Setup plus capture child wall: old 20.217 / 21.668s; candidate 45.969 / 44.823s.
  Capture/selection plus oracle: old 4.009 / 4.257s; candidate 28.505 / 27.692s.
  Both selected exactly 1,000 admitted and 1,000 pending across 49 jurisdictions with identical
  selected-record SHA256 `dc68427c7464cdb5c5e32d4057fead3b25c2cc64fc8fdfe2b6cc0215e969bfd9`.
  Final batched candidate peak RSS: 199,229,440 / 199,094,272 bytes; child wall
  32.591 / 40.193s; capture/selection plus oracle 20.459 / 21.928s, with the same exact parity.
  These are debug observations, not an isolated release speedup; setup timing also varied.
  RSS includes fixture/Fjall setup; conservative scratch counters are not RSS. This trades
  memory for additional serialization/I/O and is not a release-throughput or 64-GB fix claim.
- Excluded instrumentation trials: concurrent discovery lock refusal before tests; one shell
  collection failure after a passing test. Fresh replacements passed and verified receipts.
- Review: Holzman resource/fault boundaries, Scott DDD and functional-core separation, async
  cancellation/generation ownership, architectural drift and independent assertion review applied
  to the changed files. No national acceptance or complete proof campaign claimed.
- Acceptance: specification bounded incremental discovery and consistent projection are partially
  exercised here; resident export/derivation/readback memory and source-profile qualification remain
  open. Existing historical specification and artifacts retained; bd CLI unavailable.
- NOT RUN: representative national RSS, eight-year replay throughput, production execution,
  complete release/security/fuzz/durability gates. Reusable/profile acceptance remains pending.

Home focused commands (repository root):

```sh
env -u CI tools/moon-local run pipeline:fmt pipeline:lint-src
env -u CI tools/moon-local run pipeline:check -- --jobs 1
env -u CI tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --lib -E 'test(restate_services::results_arms)' --no-capture
env -u CI tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --test parity_pipeline --test offline_cycle --no-capture
env -u CI tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-report --lib -E 'test(recruiting::tests::postal::) | test(recruiting::tests::summaries::)' --no-tests fail --no-capture
env -u CI tools/moon-local run pipeline:gate -- --release
```


## Exact flat-event worksheet and historical replay follow-on

Base: PR #35 merge `27bb8edd`; user update `06dcf66c` retained by normal merge
`dcaad265`. Reference harness `c205059e`, semantic/worksheet change `ba97ece7`;
their diffs provide exact file coverage. Policy 19 admits exact 300m, 500m, 600m,
2-mile, 2000m and 10000m flat races without distance conversions. Policy 18 retains
its catalog bytes and comparison semantics for historical decoding and verification.
Historical input does not become eligible for a new current-policy publication.

- PASS before the user-update merge: 67 focused domain/report tests, 1127 skipped;
  1m8.585s task. Initial 45/46 run exposed an obsolete negative 2000m fixture after
  that event became supported; replaced with the already specified unsupported
  `10,000m` spelling and expanded publication coverage to all six additions.
- PASS before that merge: both offline CLI worksheet tests, zero skipped, nextest
  `60ce37f5-3197-4e2e-9b90-847fbcc037c1` (6.890s task). Independent XLSX/CSV checks
  require ten athletes, thirty exact PR winners and all forty observation IDs;
  faster cross-source 100m wins, indoor/outdoor 400m stay distinct, and capture
  timestamps differ from the asserted competition dates. Canonical identities are
  supplied by this fixture; discovery/reconciliation and postal coverage are separate.
- PASS: rebuilt policy-19 reference verifier reopened the genuine policy-18 generation,
  preserved all fourteen artifact hashes and the original catalog/audit identities.
  The original reference SHA256 stayed
  `518b75fcc311a44f7ba7f946c0aafcfc42165b9fab60c964c987fa94f3636f60`.
  New-policy capture correctly refused to regenerate that reference (exit 1).
- FAILED then repaired: independent review found facility-first race labels rejected.
  Three new regression tests first failed, nextest `baa08fef-d2a5-438a-bbe9-0fee1a0472b5`;
  the bounded non-dimension declaration repair passed all three, 501 skipped,
  `c0ea19db-5346-41bc-948a-5c85837ca049` (27.915s task). Competing distances and
  facility-only labels remain refused; policy-18 behavior remains explicit.
- FAILED then repaired: expanded merged-source suite ran 250 tests: 248 passed, two
  failed, 948 skipped (`460eb1b6-3a4a-4592-bfc9-e29077f8a924`, 3m54.989s task).
  Both failures were additional obsolete 2000m-negative fixtures in `bests/tests/catalog.rs`;
  the existing unsupported `10,000m` spelling preserves refusal assertions. Repeat PASS:
  250/250, 948 skipped, `53807dc5-9452-4103-a7b0-c3710b5ef054` (2m11.245s task).
- PASS on the final merged implementation: offline CLI 2/2, zero skipped,
  `ab6bd6c2-849c-465c-b2de-6fd225b21610` (2m21.770s task).
- PASS after the mechanical regression-module split: 7/7 flat-semantics tests,
  687 skipped, `212794ae-66f4-4ce5-8ba3-b8b50c2b5a33` (50.044s task).
  Independent reconstruction matched all original 503 lines and 24 assertions exactly;
  the new modules are 67/154/233/56 lines. Three unrelated user-update files received
  formatter-only changes; their semantics are retained.
- INTERRUPTED/UNKNOWN: first merged-main all-target check lost its executor before
  an exit receipt. It is not a pass. The replacement final merged-source all-target /
  all-feature check PASSED (4m21.251s). Source lint exited 0 (1m20.901s), retaining
  two forced `too_many_arguments` warnings from user-update acquisition helpers
  (`cache.rs::process_eligible_entry`, `qualification/inventory.rs::process_root`);
  a follow-on refactor is pending, and no lint allowance was added.
- PASS on the final merged tree: formatting (9.367s task), rebuilt historical
  reference verifier (30.432s task) and verification of the unchanged original
  policy-18 reference (exit 0).
- Review: changed domain, historical dispatch and worksheet assertions reviewed with
  Holzman Rust, Scott DDD, functional Rust and test-review guidance. No new async
  execution path. Remaining whole-export memory work and end-to-end pilot are separate.
- Acceptance mapping: spec comparable PRs, cross-source provenance and historical
  compatibility partially exercised; full source discovery, reusable production,
  national coverage/RSS, complete security/fuzz/durability/release gates NOT RUN.

Home verification for this follow-on, from the repository root:

```sh
env -u CI tools/moon-local run pipeline:fmt
env -u CI tools/moon-local run pipeline:check -- --jobs 1
env -u CI tools/moon-local run pipeline:lint-src
env -u CI tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-report --lib -E 'test(recruiting_catalog) | test(event_ontology) | test(bests::) | test(policy_cutover) | test(policy_merge) | test(catalog_tests) | test(historical_policy_tests) | test(semantic_policy_tests)' --no-tests fail
env -u CI tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --test offline_cycle --no-tests fail
env -u CI tools/moon-local run pipeline:gate -- --release
```

The release command is the full home gate, not a claim that this run executed it.

### Profile descriptor compatibility follow-up

The policy-19 review found that configuration-only event descriptors inherited the
new default catalog while retaining revision 1/publication policy 18. Explicit
Legacy18 binding restores the original bytes and digest; no historical fixture or
reference was regenerated. Changes: `profile_execution/components.rs`, integration
`descriptors.rs` and `preservation.rs`. New literal-reference regression checks
compilation without changing profile bytes or identity.

- FAILED: actual integration target 15/17 passed, two failed, zero skipped;
  nextest `d222fd4d-6d81-4d4d-a2be-b7fc3a76ee6d` (14.501s task).
- PASS after repair: 18/18, zero skipped;
  `98ef2eb4-b80e-4e18-a5ef-81b173862b7f` (15.317s task).
- NOT COVERAGE: first incorrect lib selector ran zero tests and failed explicitly.
- Current profile execution remains configuration/enrollment-only; policy-19 runtime
  profile support requires a separately versioned coherent contract.

Home regression: `env -u CI tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain --test profile_execution --no-tests fail`.

## Canonical Cargo job override

`tools/moon-cargo` accepts optional positive-decimal `MOON_RUST_BUILD_JOBS`, overriding Moon's
fixed task-level Cargo job count immediately before the existing Cargo exec. Unset defaults,
selected-Cargo guard, literal arguments and exit propagation are unchanged. No alternate serial
build engine/task was added. `xtask/README.md` documents task-hash identity and inherited children.

- PASS: patch applicability/whitespace checks and static wrapper/child-process review.
- NOT RUN: six actual-wrapper integration regressions in `xtask/tests/moon_cargo.rs`, formatting,
  compilation and gate execution. Their mock Cargo makes environment/argument/exit checks explicit.
- This limits Cargo compilation jobs only. Explicit CLI job flags retain precedence; Nextest test
  threads, rustc internal threads and other runtime/native pools remain separately controlled.
  It is not a whole-process memory cap or resource-admission waiver.

Focused home checks (not run for this slice):

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p xtask --test moon_cargo --test-threads 1 --no-tests fail
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:gate -- --release
```
