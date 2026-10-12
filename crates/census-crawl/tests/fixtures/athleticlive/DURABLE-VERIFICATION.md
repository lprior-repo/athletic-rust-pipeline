# AthleticLIVE durable obligation verification — 2026-10-12

Bead: `athletic-rust-pipeline-1adb`. Executor: AthleticLive, Sol, `openai-codex/gpt-6.1-sol`, shared main checkout. All fixture execution uses owned temporary Fjall stores; no live national-run state, native Restate recovery, live discovery or statewide completeness is certified.

## Retained applicability decisions

The applicability registry supplies exactly AL, IA, IL, IN, KS, MI, MN, MO, ND, NE, OH and WI. The missing-manifest test persists and independently reopens a refusal for **each of all twelve** when no manifest has been registered. The exact refusal is `CODE: no qualified retained AthleticLIVE manifest is registered for this jurisdiction run; operator captures with physical producer metadata are required; no live discovery or invented objects`, replacing CODE with the corresponding state code.

| State | Decision in the retained fixture registration scenario | Retained evidence |
|---|---|---|
| AL | Explicit missing-manifest refusal | Applicability registry; no registered qualified AL manifest in the retained scenario |
| IA | Armed; executes 136 rows and remains partial with the exact missing-summary locator owed | `durable-ia-manifest.json`; retained event 2150205, meet 58504; physical producer ledger at `research/sources/timing-providers-national/samples/capture-log.jsonl` |
| IL | Explicit missing-manifest refusal | Applicability registry; no registered qualified IL manifest in the retained scenario |
| IN | Explicit missing-manifest refusal | Applicability registry; no registered qualified IN manifest in the retained scenario |
| KS | Explicit missing-manifest refusal | Applicability registry; no registered qualified KS manifest in the retained scenario |
| MI | Explicit missing-manifest refusal | Applicability registry; the retained result-body fixture alone is not a registered manifest with qualified physical producer metadata |
| MN | Explicit missing-manifest refusal | Applicability registry; Wisconsin/Minnesota planned captures in this directory remain absent, not invented |
| MO | Explicit missing-manifest refusal | Applicability registry; no registered qualified MO manifest in the retained scenario |
| ND | Explicit missing-manifest refusal | Applicability registry; no registered qualified ND manifest in the retained scenario |
| NE | Explicit missing-manifest refusal | Applicability registry; no registered qualified NE manifest in the retained scenario |
| OH | Explicit missing-manifest refusal | Applicability registry; no registered qualified OH manifest in the retained scenario |
| WI | Explicit missing-manifest refusal | Applicability registry; Wisconsin/Minnesota planned captures in this directory remain absent, not invented |

IA capture: GET HTTP 200, 159,531 bytes, fetched `2026-09-22T03:59:19Z`, SHA-256 `d9a828d6ca14a531e93044b6f49ebcf84476e544c5d046cfc78c4a6521eb31e7`. The missing summary remains owed at `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_58504/event_summary.json?ns=trackmeet-io`; no summary was fabricated to change Partial into Complete. Re-execution adds zero performances, retains the original 136-row finish receipt and records a distinct bounded attempt rather than duplicating the obligation or capture.

## Round 1 — reproduced original gap

[OPEN] P1 crates/census-service/src/restate_services/jurisdiction/state.rs — AthleticLIVE was absent from named owed obligations — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and test(athleticlive_missing_manifest_has_a_named_owed_obligation)'` — task `7d98285f`, exit 1; nextest exit 100; 0/1 passed; assertion printed `AthleticLIVE manifest obligation absent: ["teams", "rosters", "meets", "results", "coach_lane"]`.

[FIXED] P1 crates/census-service/src/restate_services/jurisdiction/state.rs — Dedicated durable obligation/refusal is visible in owed-work — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and test(athleticlive)'` — task `d7adcdb6`, exit 0; 9/9 passed, including all twelve reopened exact refusals, fixture execution/reopen, malformed-first capture preservation, serde compatibility and four open-work projections.

## Round 2 — adversarial boundary repairs

[OPEN] P1 crates/census-crawl/src/athleticlive/results/manifest/frozen.rs:44 — A manifest owner with no retained objects armed rather than refusing — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and (test(athleticlive_manifest_without) or test(athleticlive_rebind_cannot))'` — task `f0cc5c69`, exit 1; nextest exit 100; 0/2 passed; exact Refused assertion failed.

[OPEN] P1 crates/census-service/src/restate_services/jurisdiction/athleticlive/ledger.rs:74 — Same-digest registration accepted a changed immutable history window — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and (test(athleticlive_manifest_without) or test(athleticlive_rebind_cannot))'` — task `f0cc5c69`, exit 1; nextest exit 100; 0/2 passed; expected exact terminal history-scope error was absent.

[FIXED] P1 crates/census-crawl/src/athleticlive/results/manifest/frozen.rs:44 — Empty owner captures persist the exact qualification refusal; malformed earlier capture still does not hide a later valid object — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and test(athleticlive)'` — task `95579253`, exit 0; 11/11 passed.

[FIXED] P1 crates/census-service/src/restate_services/jurisdiction/athleticlive/ledger.rs:74 — Idempotent registration now checks the frozen history window before returning existing status — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and test(athleticlive)'` — task `95579253`, exit 0; 11/11 passed.

[FIXED] P2 crates/census-service/src/restate_services/jurisdiction/athleticlive/tests.rs — Replaced conditional fixture assertions with the observed exact Partial disposition, missing-summary locator, owed progress, one archive body, preserved first finish receipt and zero duplicate effects — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and test(athleticlive)'` — task `37b92fa3` first failed 10/11 because observed disposition was Partial rather than the newly asserted Complete; task `95579253` subsequently passed 11/11 with the real partial behavior asserted, not a fabricated summary or narrowed collector.

## Round 3 — required acceptance, pending unrelated repair

[BLOCKED] P1 crates/census-crawl/src/ohsaa/tests/collect/owner.rs — Required crawl package acceptance fails outside the assigned lane — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-crawl)'` — task `85f79455`, exit 1; 1214/1215 passed; sole failure `ohsaa::tests::collect::owner::absent_or_empty_ohsaa_owner_is_explicit_unfinished_work_without_guessed_requests`, `CheckFailure("left=Blocked right=Partial")`; raw output `artifact://4667`, lines 944–958; routed to Main, no out-of-scope repair applied by AthleticLive.

## Round 4 — final assigned-lane acceptance and owner-authorized closure

[FIXED] P1 crates/census-service/src/restate_services/jurisdiction/athleticlive/tests.rs — Final composed obligation and owed-work cases pass, including both demonstrated boundary regressions — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and test(athleticlive)'` — task `95579253`, nextest run `391fcfd5-285e-4a0a-82fd-10fbb6d79b3f`, exit 0; 11/11 passed, 917 skipped.

[FIXED] P1 crates/census-service/src/restate_services/jurisdiction/athleticlive.rs — Required workspace compilation completes after all own source repairs — `env -u CI tools/moon-local run pipeline:check` — task `7ce78930`, exit 0; `Finished dev profile`, no diagnostics.

[FIXED] P2 crates/census-service/src/restate_services/jurisdiction/athleticlive/tests.rs — Required formatting read is clean after authorized package formatting — `env -u CI tools/moon-local run pipeline:fmt-write -- -p census-crawl -p census-service && env -u CI tools/moon-local run pipeline:fmt` — write task `b2a835c7`, read task `6b304e1c`, exit 0; both tasks completed with no diffs. Earlier read `76f9bb9d` exited 1 solely for concurrent formatting in MeetStream's `jurisdiction/stage_runs.rs`, with no owned-file diffs.

[BLOCKED] P1 crates/census-crawl/src/ohsaa/tests/collect/owner.rs — Exact required crawl package lane remains externally blocked; no passing result is claimed — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-crawl)'` — task `85f79455`, exit 1; 1214/1215 passed, sole OHSAA `Blocked` versus `Partial` failure. Main explicitly authorized closing 1adb with this disclosure, assigned that defect to RestoreCounters and retained responsibility to rerun this exact package lane at landing.

Source budgets measured with `wc -l`: lane runtime 231, durable ledger 210, wire obligation 52, manifest collector 177, frozen manifest 83 lines. A scoped lexical search found no comments, `unsafe`, unwrap/expect/panic family, suppressions or `deny_unknown_fields` in the owned lane files. No formal proof, native Restate kill/recovery, live national invocation, release gate or statewide completeness is claimed. No historical evidence, `docs/**`, `var/**` or another run's durable state was modified. Package formatting was authorized by Main and may normalize peer files; Moon did not print a touched-file list.
