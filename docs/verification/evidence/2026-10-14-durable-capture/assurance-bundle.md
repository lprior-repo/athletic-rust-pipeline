# Durable capture integration: detached verification evidence

STATUS: UNVERIFIED

Evidence checkpoint: 2026-10-14T07:07:43.794833+00:00. This package freezes an explicitly unverified publication checkpoint after both narrow boxing repairs; Root will append later execution results. Root alone owns live integration and all builds. The evidence worker only read source/logs and wrote this detached package. No production database, live source census, native Restate deployment, commit, push or external publication was performed by this worker.

## Exact subject and authority

Base commit: `5111a603cc1d945e2da3f85c00d149796552e43e`. Observed HEAD: `5111a603cc1d945e2da3f85c00d149796552e43e`. Working branch: `implementation/durable-capture-integration-20261014`. HEAD names the base, not the dirty integrated source. `source-files.json` and `source-pins.sha256` identify all 237 changed files (146 tracked plus 91 new) with exact lengths and SHA-256 hashes. Full source-byte copies and the tracked patch remain in the detached forensic bundle rather than duplicating Rust source under documentation. No file changed between the two snapshot reads. This read-time consistency check does not assert a historical runtime source freeze. Ignore build products when counting source; include newly added files.

The authority is the full October 11 spec v2, AGENTS.md, ARCHITECTURE.md, and ADR-038, pinned in `contract-pins.json`. `acceptance-map.jsonl` maps this bounded capture scope to §§6,9,10,12,14,16–18 and explicitly retains the wider §1–19 product gaps. A design adoption, passed fixture or source review does not equal exact-build implementation qualification, configured-run completion or publication approval.

## Lifecycle boundary, without reinterpretation

- Historical lifecycle 1 and 2 stores, original identities, manifests, captures, receipts, artifacts and seals retain their original meaning. Normal fresh legacy initialization stays lifecycle 2.
- Lifecycle 3 remains explicitly enrolled production metadata/control under its existing capability restrictions; profile selection does not activate capture or annual execution.
- Newly supported lifecycle 4 is opt-in fresh capture qualification only. Explicit init requires a nonexistent or genuinely empty root. Explicit serving opens that existing capture root before listener/browser initialization. Ordinary open/migration refuses capture execution before recovery mutations; legacy fixtures explicitly choose LegacyFresh.
- No historical store is silently adopted, migrated, re-enrolled, re-keyed or re-sealed. Mixed-case StoreIdentity is preserved.
- The separate proposed lifecycle-5 lost-backing protocol is not adopted or implemented here. It is excluded from every implementation and recovery claim in this bundle.

## Observed execution, including failures

See `verification-ledger.jsonl` and hash-pinned raw logs. `commands.md` reproduces full cwd, toolchain exports, unmodified inherited HOME disclosure, command strings and logging. Internal Cargo diagnostics do not authorize bypassing Moon.

| Lane | Observed result | Limit |
|---|---|---|
| core-first | Compile failure: 3 errors, 2 warnings | No tests |
| core-second | 30 passed, 1,060 skipped; nextest 5dbabbff-56b2-4249-9200-c038942b2612; 1.686 s test phase | Earlier source before later changes |
| transport-first | Compile failure: 5 errors | No tests |
| composed-first | Compile failure: 4 test imports, 1 warning | No tests |
| composed-second | 144 run: 143 passed, 1 failed, 3,116 skipped; nextest e91a3300-7512-40c5-98fd-37af6afe75a9; 4.001 s test phase | Whole lane failed |
| format-first / second / third / fourth | Each fmt-write task completed; fourth 7.442s | Format application alone, not final fmt-check/build/test acceptance |
| basic-first | 6 crawl lint errors; 2 forced arity warnings | No complete check/fmt pass |
| all-target-first | 3 missing ServeOptions.capture_protocol fields | Source repair is not a pass |
| basic-second | Interrupted at 600 s; Root reported timeout exit 124 | Compile inconclusive, no tests; lint announced without result; fmt not run |
| all-target-serial | passed_check; 3m54.065s, Root exit 0 when raw completion is present | Before subsequent boxing repairs; no tests |
| lint-format-final | fmt-check PASS 7.240s; lint FAIL 86 errors, 1m13.031s | 85 propagated error-size diagnostics plus one private-context enum-size diagnostic |
| composed-third | Running at this freeze; no test outcome | Post-boxing expanded four-package rerun; pending source-bound terminal evidence |

The stale service test treated an armed transport-unknown outcome as known retry exhaustion. Current source expects `OutcomeUnknown` with `Armed(hop 0)`/`TransportFailed`, `AcquireDecision::Interrupted`, and `RetainedOutcomeUnknown`. A separate new observed-500/BodyIncomplete test asserts actual known-response exhaustion. Neither this correction nor the added test inherits a green result from the old lane.

The composed log includes 33 core/bounds passes, 22 new transport passes and 2 coach classification passes. It also includes dedicated Spawner, split-capacity, cancellation and drain tests. `selected-test-results.json` records actual selected names. Important omissions: the actual Results selector/DNS one-blocking-thread test, the 3 new SDK replay tests, 2 CLI binary tests and 3 CLI integration tests were not run in that lane. `cli::capture_protocol::tests` belongs to the binary target; a --lib filter cannot select it. Source existence is not execution.

Post-run changes include coherent core/source splits, SelectedHistoryV1 membership grouping and its new test, ExistingKeyspaces ownership grouping, the transport box-size/lint repair and 2 new regressions, corrected service oracle plus known-response cap test, 3 explicit LegacyFresh native-fixture fields, SDK bridge tests, the deterministic envelope-size test correction, seven-file nested StoreError boxing repair with three authored regressions, and two-file private capture-context boxing repair with one authored regression. The latter nine current source paths await exact delta review at this snapshot. Run affected and aggregate lanes on final pinned bytes before acceptance.

## Review coverage versus actual changed files

`review-coverage.json` preserves exact current and historical reviewer SHA-256 values per changed path; no filename-only approval is substituted. At this checkpoint: 228 exact-byte matches, 8 reviewed paths changed after pins, 1 without a found pin. All review observations are static unless explicitly tied to raw executed logs. Changed-byte status means a delta review is needed, not that a new semantic defect was found.

The retained review chain includes frozen core and compile delta, formatted core split/SelectedHistory grouping, original service OPEN 2, first v2 findings, corrected v2, service integration repairs, frozen transport, postformat transport, post-lint transport, and SDK bridge. Original rejected/open reports are preserved with later resolution evidence, not overwritten. `findings.jsonl` gives every listed concern a canonical disposition with evidence scope. Frozen static fixes do not certify later bytes or runtime.

## What this slice establishes narrowly

Historical fixture execution reaches the real common HTTP path over loopback TLS, one-shot physical dispatch, target-bound consumed grants, immutable response checkpoints, actual 429/500/truncated responses, cache/304 comparison fences, original body/date preservation, Prepared recovery when backing exists, crawl Run projection, stopped Store reopen/backup/restore, same-slot lifetime caps and typed ambiguity. The Store/transport fixtures do not establish production-source coverage or native Restate recovery.

The new Results replay bridge uses real SDK endpoint framing and production `load_object → results_owed → results_stage → ctx.run` methods. Its actual executed action is offline refusal after cached discovery; it is currently only authored/reviewed evidence. It does not claim successful network body retention, native journal persistence/restart or mid-I/O cancellation. The dedicated selector/DNS fixture is a separate necessary boundary from generic supervisor tests.

## Unresolved acceptance limits

1. Prepared with missing or corrupt original backing refuses without allocating a successor attempt. IDs and facts remain intact, but the current lifecycle-4 protocol cannot honestly claim recoverable completion for that failure. The separate lifecycle-5 design supplies no implementation evidence.
2. The acquisition inventory is capped at 1,048,576 root debits, 512 MiB encoded reads, 2 GiB unique backing-address verification and 64 MiB memo accounting. Legal configured writer output can exceed the 2 GiB reopen/backup limit; no override or scale qualification is established. Per-operation deduplication does not eliminate every second live-path verification. Independent backup scans lie outside this inventory budget. The default HTTP archive capacity is 100 MiB; a 200 GiB publication artifact allowance is a different dimension.
3. One-work closure is 42 roots; the largest legal 304 join is 83 under the default 84 root allowance, not 84 fabricated legal roots and not a total-I/O cap. Receipt, lineage, size and absent-key probes have their own costs. Complete canonical envelopes admit 65,536 bytes and refuse 65,537 before mutation, with physical key bytes charged separately. Those historical assertions were later corrected for a real-clock second-boundary race; rerun is required.
4. Final current-byte review closure, complete execution-time source/binary/fixture provenance, all-target/lint/fmt/trusted AST/safety gates and active-context evidence audit remain open. No missing output, interrupted compile, stale reviewed source or skipped lane is a pass.
5. Native Restate endpoint/coordinator kill/restart, dispatch/archive/ack/frontier/publication interruption, ENOSPC/short-write/fsync faults, full 17-scenario fault catalogue, parser-fuzz, security/dependency lanes, retention/GC/restore horizon and measured resource/performance qualification are not discharged by these fixtures.
6. The complete reusable recruiting product remains separate: accepted-affiliation and interpretation dependencies, all-source refresh/negative evidence, annual rollover, regional/event publication filtering, independent all-family oracle, real five-state/UP source qualification, workbook/normalized bundle, and any selected query database. No live census, national seal, full-spec readiness, performance gain or production release is claimed.

## Evidence integrity and handoff

`integration-checkpoint.json` is preserved as historical metadata and is stale: it still calls transport running/service pending. Raw later logs and the lane ledger supersede it; it is not silently rewritten. Log wall-clock prefixes are retained as emitted. Parent notifications are context for explicitly attributed exit status only, not replacements for raw proof.

`truth-serum-report.md` discloses the absent active-context audit. `final-evidence-decision.md` remains UNVERIFIED. Root may publish this explicitly unverified checkpoint under the owner’s authorization for an in-progress non-draft PR update before verification completes. Freeze and recheck source/artifact pins first, preserve all historical failures, then append terminal results as they arrive; never retroactively assign them to earlier bytes. This bundle contains no live repository mutation or authorization to run production.
