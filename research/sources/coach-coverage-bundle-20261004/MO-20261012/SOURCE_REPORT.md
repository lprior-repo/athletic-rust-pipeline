# Missouri coach-route qualification — 2026-10-12

Executor: CoachLaneB / Sol / openai-codex/gpt-6.1-sol.
Bead: `athletic-rust-pipeline-0u77`. Scope: Missouri obligation module and this report. No live fetch or owner acceptance.

## Source decision

Retain `mo_missing_coach_sources` as `AccessBlocked`, under candidate arm `mtccca`. Registry binding already records the explicit refusal. The obligation is `Blocked`, not terminal, armed, accepted-refusal or verified-empty. jpx4.1 does not let a recorded refusal silently discharge coverage.

Read matrix and `state-work-orders/group-b.md:224-247`; inspect `probes/sources/SRC-198/FINDINGS.md` and `robots/mshsaa.org.txt`. The held robots file has two wildcard groups (lines 3 and 34); the latter has `disallow: /`. RFC 9309 group merging forbids the MSHSAA coach route. No content fetch is performed to bypass that refusal. The historical school listing provides a universe, not coach role/mailbox proof; live coach-page content remains unknown, not empty.

Unqualified alternatives, preserved in the exact work-order refusal: MTCCCA live membership has no registered proof-bound acquisition arm; moqualityschools and MOCSAA are unheld membership candidates; DESE/apps.DESE are school-discovery candidates. Prototype responses and historical contact totals do not certify a current national obligation.

## Retained source integrity

The five-file `sha256sum` command recorded in `../NV-20261012/SOURCE_REPORT.md` exited 0. SRC-198 `robots/mshsaa.org.txt`: `e565a7d99dd82e7063e7925a2a9e465c0c8c7d856548dfef12df1b6fc3a4e4ef`. This hash names the exact inspected path, not a different sweep copy or historical hash.

## Acceptance evidence and limits

The initial combined service command in the Nevada report exited 0, including all four Missouri baseline tests. Test-only acceptance actors exercise the domain API and are not actual owner decisions. New durability acceptance writes the real CoachLanes row and full jurisdiction state to a fresh temporary Fjall store, drops the writer, reopens, rebinds three times, writes replayed state, drops and reopens again; exact reason/kind, one row/receipt and zero contacts are asserted. Native Restate and the live run are untouched.

The initial focused adapter command recorded in the Nevada report was compile-blocked by concurrent census-store record migration; it did not execute tests. No semantic red is manufactured for the existing correct Missouri refusal.

## Focused adapter outcome

`env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-crawl -E 'test(coach_contacts::) or test(registry::tests::artifact_adapters_declare_no_fetchable_host) or test(school_sites::tests::queue_parsing_keeps_valid_rows_and_counts_rejections)'` exited 0: **27 passed, 1242 skipped**; Moon `3095475d`, nextest `9bd4b51a-4206-46e8-a0b1-7ba816c6e8d2`. This re-verifies the available artifact/queue scaffolding, not an MSHSAA or MTCCCA fetch. The store API audit and Given/When/Then persistence contract are recorded in the Nevada report; Missouri's assertions require `AccessBlocked`, not EngineeringGap.

The same focused adapter command was re-executed against the later tree: exit 0, **27 passed, 1242 skipped**, Moon `53c6bd18`, nextest `b8981af7-37a0-4768-a5ec-6e8ca173c105`. Global compile acceptance attempts and their sibling blockers are recorded in the Nevada report; they are not omitted from the handoff.

## Final required acceptance round

`env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-service -E 'test(coach_lane_mo)'` exited 0: **5 passed, 1008 skipped**, Moon `b40f3c05`, nextest `b37653bd-e404-47ba-8cc5-0a9f1f77e8e2`. The actual-store replay scenario passed, retaining the exact AccessBlocked reason, one refusal/obligation/journal key, no acceptance/evidence and zero contacts across two reopen boundaries and three replay binds.

`env -u CI tools/moon-local run pipeline:check` exited 0 (Moon `4b883170`). Latest adapter execution is the 27/27 run above. Additional `pipeline:fmt` exited 1 (Moon `bcb2024e`) only in out-of-scope files; full-output search found no NV/MO/TX/VT module diagnostics. Main owns resolution of that additional gate; no shared formatting write occurred.

Per-state result: **existing explicit access refusal re-verified; persistence/replay proved; still Blocked/owed, not armed or accepted-refusal**. Source access constraints are preserved; the passing tests do not grant permission to crawl MSHSAA.
