# Source acquisition policy holds

Base: `011518bdd3a4adae0e43a4e574575bc9a45d24f3` (PR43); focused tests below ran before the tooling-only fast-forward.

One typed registry policy preserves GoBound AccessBlocked and TFRRS/DirectAthletics
Terms classifications. GoBound and TFRRS online collectors refuse before empty-success,
cache interpretation or source work. Initial and redirect dispatch refuse held host
subtrees before DNS, origin charging or HTTP. The generic fetcher still permits eligible
retained cache reuse; explicit offline adapters preserve original captures and receipts.
No provider policy is relaxed and no live census was run.

Mapping: `census-crawl/src/registry/acquisition_policy*` owns the policy;
`bound/collect.rs`, `tfrrs/mod.rs` and `net/execute/{fetch,attempt}.rs` enforce it.
`net/acquisition/tests/gobound_policy.rs` verifies V1/V2 adopted redirect checkpoints,
settled replay and unrelated cache/archive retention. Service plan and SD coach-lane
changes retain exact refusal reasons, owed work and historical fingerprints/replay.
Static refusals do not manufacture HTTP evidence.

- PASSED: format write; 39 focused crawl tests, 0 failed, 1448 skipped.
  Nextest `8b766a78-78d6-4aa1-bd31-a930de09329e`, 6.672 seconds test execution.
- PASSED: 38 focused service tests, 0 failed, 911 skipped; Nextest
  `f4408399-9aee-47aa-bdf6-3ee45feaea74`, 0.720 seconds test execution.
- FAILED: expanded service/CLI selection, 201 passed, 6 failed, 891 skipped;
  Nextest `6f306b0b-13a7-4988-8302-eb56843fe8e0`, 5.258 seconds. All six
  failures are existing provider offline CLI cases: Clap rejects explicit global
  `--store` placed before the provider subcommand. One isolated rerun confirmed
  MissingRequiredArgument before collector dispatch; CLI repair is separate pending work.
- FAILED command invocation: combining fmt/check with `--jobs 1` forwards that
  option to fmt, which rejects it. Rerun the distinct commands listed below.
- PASSED: final format (7.290 seconds) and workspace all-target/all-feature check
  (3 minutes 10.286 seconds) after fast-forward to PR43.
- NOT RUN: source Clippy, full release/fault/fuzz and representative peak-memory lanes.
- Review: changed-surface static checks covered typed refusals, host label boundaries,
  pre-dispatch ordering, offline/cache distinction and historical identity compatibility.
  This is not full source coverage or full skill/release-gate approval.


## Registered-source acceptance map

Static audit covers all 29 registered families; none is promoted to verified active
runtime acquisition by registry/dispatch presence. No deployment or live source run
was inspected. Registry arrays are `registry/table/{through_milesplit,from_mshsl,
directories}.rs`; jurisdiction dispatch is in service `teams_arms`, `meets_arms`
and `results_arms`.

- Code-wired acquisition paths, current coverage unverified (21): aia, arbiter_orgs,
  chsaa, ciac, coach_directories, home_campus, ihsa, ks, milesplit, mpa, mshsl, ohsaa,
  pa_piaa, plain_names, riil, sidearm_staff, tssaa, uhsaa, wayzata, wiaa, wiaa_results.
- Inactive automatic discovery (2): athleticnet, athleticlive_athletes. Supplied
  athlete/meet collectors exist; qualified jurisdiction frontiers remain prerequisites.
- Artifact/run-binding gaps (4): athleticlive, coach_contacts, nces, state_ed.
  LIVE already has supplied-manifest registration/execution with producer metadata;
  automatic qualified discovery and an actual suitable run-bound manifest are unverified.
  Contact imports and school collectors exist; durable artifact/frontier selection gaps remain.
- Policy-held (2): bound, tfrrs. The changes above enforce their existing holds;
  offline replay does not establish permission, new captures or live coverage.

The user dossier at `76927400111b81568b08f652b5ac9e3deb21a25a` remains unchanged.
Actual team stages/full refusal reasons already have diagnostic presentation. The
archive-capacity, rate-blocking and state-transition explanations remain hypotheses;
this audit does not establish their causality. Meet access retention is a separate
identified completion gap, not a proven cause of that historical observation.

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:fmt
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:check -- --jobs 1
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-crawl --lib -E 'test(policy) | test(gobound)' --no-capture --no-tests fail
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --lib --bin census-service -E 'test(restate_services::plan::) | test(restate_services::wire::coach_lanes::) | test(restate_services::jurisdiction::team_source::tests::) | test(restate_services::open_work::tests::diagnostics::) | test(cli::provider::offline_tests::)' --no-capture --no-tests fail
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:lint-src
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:gate -- --release
```
