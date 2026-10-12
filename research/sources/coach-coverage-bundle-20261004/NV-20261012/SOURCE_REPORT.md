# Nevada coach-route qualification — 2026-10-12

Executor: CoachLaneB / Sol / openai-codex/gpt-6.1-sol.
Bead: `athletic-rust-pipeline-0p76`. Scope: the Nevada obligation module and this report; no live acquisition, production-store access, service restart or owner acceptance.

## Source decision

Record `nv_missing_coach_sources` as `EngineeringGap`, under candidate arm `niaa`. This is an explicit refusal, not an armed route, successful-empty coverage or an accepted refusal. The obligation remains `Blocked` and owed under jpx4.1 until a real arm or an actual owner acceptance exists.

Read the state matrix, `state-work-orders/group-c.md:179-200`, SRC-175/178/179 extraction reports and the held XC capture before selecting this outcome. Southern Nevada SNTCCCA sheets publish real school/program/name/mailbox fields: SRC-175 XC capture line 104 binds Mountain View Christian / DeeDee Nellis / a published mailbox; line 2 states the head-coach contact purpose. It is incorrect to call these sheets name-only or mailbox-free. Their historical extracted CSVs are not a registered live acquisition arm or a proof-bound current-run frontier. No current capture, executed national obligation or current-tenure admission is claimed here. SRC-175/178/179 share sheet sources and are not independent corroborators.

The bead's 2026-10-07 observation establishes a FinalForms school-level coach-name landing, with no emails and `noindex,nofollow`; do not repeat the older claim that Nevada has no coach landing. That observation is supplied evidence, not a network command executed by this reviewer. FinalForms remains unqualified for a mailbox arm; the noindex signal is not bypassed. NIAA's separate historically empty `niaa-coaches` capture remains part of the unchanged, verbatim work-order refusal reason; it does not characterize every Nevada source.

Unqualified alternatives: CCSD/Washoe staff pages lack an executed proof-bound frontier; Nevada private-school directories are school discovery rather than qualified current contacts. Raw sheets retain inconsistent/absent names and short school labels; no inferred role, corrected mailbox, current appointment or population coverage is created.

## Capture integrity observed this round

`sha256sum research/sources/coach-coverage-bundle-20261004/extract/SRC-175/captures/sntccca-xc-sheet.csv research/sources/coach-coverage-bundle-20261004/extract/SRC-175/captures/sntccca-track-sheet.csv research/sources/coach-coverage-bundle-20261004/probes/sources/SRC-198/robots/mshsaa.org.txt research/sources/state-assoc-southcentral/samples/tx-tf-school-codes-2026.txt research/sources/state-assoc-midatlantic/samples/vpa-home.html` exited 0. Nevada hashes:

- XC: `b54ec8d48de4d2fe0c0f19d08b33432cc8d2957164d28a619739999a721aef96`.
- Track: `ec901de764836630ad3e4c9f82e4b504da5f1e3505a83db7c8cab12f523deb68`.

## Initial execution and limits

`env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-service -E 'test(coach_lane_nv) or test(coach_lane_mo) or test(coach_lane_tx) or test(coach_lane_vt)'` exited 0: 18 passed, 949 skipped; nextest `a6dfd184-17f9-4401-aad6-ea0213d10cc5`. This baseline exercises recorded refusals and test-only acceptance semantics, not the new persistence scenario.

`env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-service -E 'test(coach_lane_nv_basis_acknowledges_held_contact_and_name_surfaces)'` exited 1 before test execution: concurrent `admission/tests.rs:259,260` used `wait_ticket()?`, whose `DepartureReason` does not implement `StdError`. This is not a semantic red for Nevada.

`env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-crawl -E 'test(coach_contacts::) or test(registry::tests::artifact_adapters_declare_no_fetchable_host) or test(school_sites::tests::queue_parsing_keeps_valid_rows_and_counts_rejections)'` exited 1 before tests: 21 census-store record-cutover compile errors, including absent UnitState/PublishState variants and Digest/string mismatches. Main notified; no out-of-scope repair attempted.

The new per-state scenario uses the real Fjall `CoachLanes` table and journal: flush, drop writer, reopen and decode, rebind three times with exact unchanged state, save, drop, reopen again; one row/receipt and zero contacts must remain. This is offline persistence/replay acceptance, not native Restate restart evidence. No fake semantic red is assigned to existing correct refusal behavior.

## Focused adapter execution after store-library cutover

`env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-crawl -E 'test(coach_contacts::) or test(registry::tests::artifact_adapters_declare_no_fetchable_host) or test(school_sites::tests::queue_parsing_keeps_valid_rows_and_counts_rejections)'` exited 0, Moon `3095475d`, nextest `9bd4b51a-4206-46e8-a0b1-7ba816c6e8d2`: **27 passed, 1242 skipped**. The shared artifact path rejects raw unverified CSV, tampered evidence, unmatched extra rows and oversized records; the source registry confirms no fetchable host; the school queue preserves valid records and counts rejects. These are focused scaffolding tests, not a qualified Nevada network collector.

The Nevada basis command rerun after Main's store-library split signal still exited 1 before test execution (Moon `d98ff858`): service spawn referenced removed `permits`, support's StoreError match was nonexhaustive, and admission tests referenced removed WorkClass fields/private Charged fields. None were Nevada compile diagnostics. Main identified their owners; basis remains unchanged until a real semantic red can execute.

`env -u CI tools/moon-local run pipeline:fmt` exited 1 (Moon `1fdd4164`) with concurrent out-of-scope formatting differences and one Missouri bind formatting difference. The owned Missouri correction was applied surgically; no global formatter wrote sibling files.

Store dependency audit: each new acceptance scenario uses only `Store::open`, `replace`, `scan`, `journal_done`, `journal_payload`, `journal_keys`, `flush` and `walk_table`. It uses neither leases nor input/build/unit/publish records. Thus the store-record API cutover requires no API migration in these tests.

Behavior specification: **Given** the real state-specific blocked obligation and exact refused-source reason with no acceptance, **when** the real store writer closes and reopened persisted state is replay-bound three times then persisted/reopened again, **then** the full state stays value-equivalent as JSON, exactly one refused entry and durable obligation/journal key remain, no coaches appear, and the lane stays owed. The test exercises the registry/store integration boundary, not the live Restate service.

## Acceptance attempts after service-quiet signal

`env -u CI tools/moon-local run pipeline:check` exited 1 (Moon `022dab65`): `census-crawl/src/arbiter/collect/run.rs:270` had an unclosed delimiter from its function opener at line 252. The assigned coach files were not diagnosed. Main notified; no arbiter edit attempted.

`env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-service -E 'test(coach_lane_nv_basis_acknowledges_held_contact_and_name_surfaces)'` exited 1 (Moon `9d001201`) on the same dependency syntax error before any Nevada test executed. Neither this nor preceding compiler failures is a semantic red.

After Main confirmed the arbiter save was complete, the same Nevada semantic command exited 1 again (Moon `06867010`) before execution: `census-report/src/export/dataset/frozen/reader.rs:144` attempted to re-export private `window::EncodedWindowLimit` (`E0365`). The dependency owner was notified through Main. This is another compile blocker, not an observed Nevada assertion failure.

The explicitly requested `env -u CI tools/moon-local run pipeline:check` retry also exited 1 (Moon `c47a177e`) with that same report-library private re-export; neither service nor per-state assertions executed. Required gate remains unsatisfied at this point.

After Main removed that stale re-export, the Nevada semantic command still could not execute (Moon `bf169657`, exit 1): frozen `cursor.rs:42` called private `refusal::retain`, producing `E0603`. Main was notified.

`env -u CI tools/moon-local run pipeline:check` then progressed past frozen-report compilation but exited 1 (Moon `2a2c2993`) on concurrent ingest changes: `ingest/recorded.rs:126` referenced nonexistent root `census_crawl::MAX_RECORDED_WORK`, and `ingest/recorded/admission_tests.rs:29` required unsupported `HandlerError::to_string()`/Display. No assigned coach file was diagnosed; Main was notified with both exact loci.

Owned-file budget command `wc -l crates/census-service/src/restate_services/wire/coach_lanes/nv.rs crates/census-service/src/restate_services/wire/coach_lanes/mo.rs crates/census-service/src/restate_services/wire/coach_lanes/tx.rs crates/census-service/src/restate_services/wire/coach_lanes/vt.rs` exited 0: NV 263, MO 223, TX 226, VT 260 lines, each below 300. Persistence scenarios are bounded, independent temporary-store executions; their extracted save helpers check the refusal contract before both writes.

## Adversarial disposition of the speculative metadata check

The substring-only basis test was removed before any successful execution. Requiring the literal strings `FinalForms`, `SNTCCCA` and `mailboxes` would invent a defect: the existing basis accurately describes the historically empty `niaa-coaches` capture and the unregistered CSV frontier, without asserting that all Nevada landing pages are empty or that no published mailboxes exist. Its absence of newer source names is not a contract violation. Production basis and the verbatim work-order refusal therefore remain intentionally unchanged. The source qualification above records the newer nuance without rewriting historical evidence.

There is no observed semantic red and no production fix to certify. Verification will exercise existing Nevada refusal behavior and the new real-store replay acceptance; adding an acceptance scenario is not a bug-regression claim.

The focused crawl adapter command was re-executed against the later tree and again exited 0: Moon `53c6bd18`, nextest `b8981af7-37a0-4768-a5ec-6e8ca173c105`, **27 passed, 1242 skipped**. This is the latest observed adapter-scaffolding result, still not a state-specific live collector.

## Final required acceptance round

The named ingest type-annotation prerequisite was observed landed at `ingest/recorded.rs:109` as `Ok::<_, crate::restate_services::JobError>(...)`; verification then proceeded without an out-of-scope edit.

`env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-service -E 'test(coach_lane_nv)'` exited 0: **6 passed, 1007 skipped**, Moon `2dd7b57e`, nextest `4705c63f-2ce7-42f7-b3e2-1b0347c2b04e`. This includes `coach_lane_nv_refusal_survives_reopen_and_replay_without_acceptance`: actual table/journal persistence, two reopen boundaries, three replay bindings, exact original state/reason/kind, one row/receipt, zero contacts and still owed.

`env -u CI tools/moon-local run pipeline:check` exited 0 (Moon `4b883170`): `Finished dev profile ... in 0.16s`. Earlier observed compile blockers no longer prevented the required workspace/all-targets check.

Final `wc -l` on the same four owned files exited 0: NV 254, MO 223, TX 226, VT 260. No production basis, source reason, arm or acceptance actor was changed.

Additional `env -u CI tools/moon-local run pipeline:fmt` exited 1 (Moon `bcb2024e`): workspace differences in sibling emitter/store/admission/report/xtask files. Searching the full formatter output for `coach_lanes/(nv|mo|tx|vt)\\.rs` returned no matches. No global formatting writes were attempted; Main was notified. This additional gate is unresolved, not passed.

Per-state result: **existing explicit engineering refusal re-verified; replay durability proved; not armed, not owner-accepted, still Blocked/owed**. All named acceptance commands ran successfully in this final required round, but workspace formatting prevents an unqualified shop-wide certification. No native Restate restart, real owner acceptance, live network collection or current-run contact coverage is claimed.
