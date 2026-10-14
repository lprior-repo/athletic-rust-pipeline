# Results replay bridge: focused static review

Date: 2026-10-14 UTC. Reviewer: review_capture_service_ownership.

## Disposition

No confirmed source defect in the four-file bridge delta. The tests exercise the production Results workflow methods through the actual Restate SDK endpoint rather than a copied workflow. Static review accepts the bridge for Root-owned compilation and execution.

**VERDICT: BLOCKED 1 — bridge execution and integration checks remain unverified.** This is an evidence blocker, not an additional confirmed implementation defect. No build, test, lint, formatter, live service, or network command was run by this reviewer. Root reported the prior composed lane's dedicated-worker, split-capacity and cancellation tests passed; that report is not execution evidence for these new bridge tests.

The original v1 OPEN 2 report and both v2 review rounds remain unchanged.

## Exact input and ownership

- Proposal: `capture-replay-bridge-proposal`.
- Patch SHA256: `f50a2f8c83ad0dd638c46823d26925a580708a1fd1b689a39fb1b455bf5dbd9b`.
- All four `OUTPUTS.sha256` entries independently verified. Exact paths are retained in `changed-files.txt`, exact base/output bytes in `base/` and `staged/`, and per-file hashes/lengths in `pins.json`.
- The three new files contain 171, 141 and 138 lines. The only production-module change is the test-gated `mod replay_tests` registration; no production contract, workflow, serialized type or dependency is changed.
- The proposal's `results_work.rs` base is the earlier unformatted service source (`3ba5f813...`). Root's formatted live source at initial inspection was `c26108ca...`; Root then integrated only the test registration, yielding `7709fe39...`. The preserved `module-format-and-registration.diff` demonstrates that integration boundary. Do not replace the live production module with the older staged formatting.
- 25 of 26 dependency read pins matched at the final read. The sole expected mismatch is that newly registered `results_work.rs`; `integrated-module-pin.sha256` and `integrated-results-work.rs` retain its observed bytes. No other dependency mismatch was observed.
- Static `git diff --no-index --check base staged` emitted no whitespace diagnostic (exit 1 for differing trees). This is not a compiler/style-gate pass.

## Production reachability

`fixture.rs:30–44` defines only a test endpoint adapter: it delegates to `JurisdictionCensus::load_object`, then `results_owed`, and returns the resulting year outcome. It does not reimplement selection, capture admission, retries, result collection, or durable transitions.

The inspected production path is:

1. `jurisdiction/stages.rs:91–109`: actual SDK `ctx.get` loads retained `JurisdictionState`, and `guard_identity` binds it to the actual object key.
2. `jurisdiction/pipeline/history.rs:44–68,98–135`: real source-plan selection and requested history-year traversal invoke `collect_result_year`; that method creates the capture owner, calls the real Results stage, inserts the outcome and calls the real state saver.
3. `jurisdiction/capture_admission.rs:77–126`: the actual key, request identity, bound history, retained run manifest, selected MileSplit role and writer ownership are checked. The fixture creates a lifecycle-4 qualification Store and retained manifest using the real APIs, not a fake owner.
4. `jurisdiction/results_work.rs:27–66`: real `Jobs::admit_work(ctx.key())` creates the owner guard before `ctx.run`; the real closure transfers it to `Spawner::dedicated`, invokes `jobs::results_stage` on the existing runtime Handle, and preserves the actual outcome taxonomy and single inner Run attempt.
5. `results_arms.rs:104–145,166–197`: the non-replay test reaches actual selection, the cached meet page, discovered result-set options, capture-scope grant and MileSplit result collection. `capture_scope.rs` performs the actual checked target and single-continuation grant; the test does not mint that stage's grant itself.
6. `net/acquisition.rs:135–156` and `session.rs:161–195`: the uncached owned request in offline mode reserves the actual acquisition attempt and finalizes `NotDispatched(AdmissionRefused)` without dispatch. This is distinct from an ambiguous transport failure after dispatch.

The fixture's manually seeded failed attempt is restricted to the two replay-only tests. The non-replay test starts with no acquisition attempt, so its terminal-row assertion cannot be satisfied by that seeding helper.

## What the three tests establish if they pass

- `replayed_capture_results_run_releases_unused_original_owner_admission`: supplies a completed SDK Run journal, runs it twice through the actual endpoint, requires no new Run command/proposal/state read, checks the returned and saved outcome, and compares acquisition keys/payloads, source meets, store sequence and Fetcher statistics. Same-key readmission and full Coordinator resource recovery check that the unused closure's owner admission was released. Final drain requires accepted/completed zero and every other counter zero.
- `capture_results_stage_finalizes_offline_attempt_with_only_stage_permit_then_replays`: executes the real stage with Spawner capacity 1, then builds the replay completion from the SDK's actual Run proposal. Before replay it independently requires one selected meet, an explicit nonterminal failed source, its precise offline diagnostic, one metadata cache hit, zero physical requests, one reserved attempt, one immutable `NotDispatched(AdmissionRefused)` terminal and no dispatch or second attempt. Replay must leave the complete snapshot unchanged and restore same-key admission/resources. Final drain requires exactly one accepted/completed worker and no failure/cancellation/panic/remaining counts.
- `replayed_capture_results_failure_preserves_error_and_releases_original_owner_admission`: injects a recorded terminal 409 and checks its code and message through the SDK output. It requires no state save, no worker acceptance, unchanged durable/fetch state and released same-key admission/resources.

This avoids a tautological success oracle: the real first execution's result is independently checked before being reused as the replay input, while synthetic replay values deliberately test the SDK's skip/drop/error behavior rather than claiming those values were computed by source collection. State persistence is checked as emitted SDK `SetState` content derived from the unchanged retained state plus the expected year outcome and date, not by mutating fixture state to match the result.

## Explicit limits

- This is an SDK invocation-protocol bridge over an isolated loopback endpoint. It does not launch a native Restate server, persist a native journal, crash/restart a service, test deployment replacement, or prove native durable journal acknowledgement.
- The real physical-action branch is an offline refusal after a cached discovery page. It does not establish successful owned-body retention, Prepared recovery, physical request counts across native restart, unknown-outcome retry refusal, or known-retryable lifetime exhaustion. Those require the separate transport/core and native integration lanes.
- The fixture uses one year, one meet and one selected source. Broader history/source authorization and multi-target grant uniqueness retain their separate tests and reviews.
- Same-key readmission verifies absence of a leaked guard after replay/error; by itself it would not detect removing the admission entirely. The production call path and existing WorkAdmission ownership/contention tests provide the complementary evidence. This bridge adds no claim of standalone exhaustive lock-schedule proof.
- Capacity 1 here covers the Spawner worker lane and existing stage reservation path. The current-thread runtime does not explicitly set Tokio's blocking-pool capacity to 1; the separate dedicated-worker fixture provides that specific nested-selector/DNS starvation regression.
- The bridge never replaces the source's existing cancellation/drain tests. Its final drain reconciles all worker counters but is not an adversarial mid-I/O cancellation schedule.

## Required Root follow-through

Compile and execute the three exact new test names through the repository's Moon entrypoint on the final composed source, then retain command/status/output and final source pins. Resolve integration-wide check/lint failures without weakening these assertions. Root reported bridge execution still unrun while fixing three legacy native-fixture `ServeOptions` initializers and separate crawl lint findings; neither is represented here as a bridge pass.

## Review references

Repository `AGENTS.md`, `ARCHITECTURE.md` sections 10–12, ADR-002 and ADR-037. Skills used: `rust-contract`, `holzman-rust`, `async-rust-reviewer`, `scott-ddd-refactor`, `architectural-drift`; specific references: `holzman-rust/references/nasa-jpl-standards.md`, `async-rust-reviewer/references/structural-correctness.md`, `async-rust-reviewer/references/cancellation-safety.md`, and `scott-ddd-refactor/checklist.md`. Repository instructions take precedence over generic skill example commands and fixture-style rules.
