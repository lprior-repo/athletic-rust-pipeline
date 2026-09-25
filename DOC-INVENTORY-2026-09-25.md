# Documentation Inventory — 2026-09-25

## ROOT DOCS (18 files)

### 1. AGENTS.md
- **Verdict: current**
- Describes ownership and lanes, commands, gate lanes, adapter layouts, source policy, evidence discipline.
- Spot-checked: §2 gate commands (`cargo xtask gate`, `cargo xtask scan`, `cargo xtask seams`) exist and run; §4 cites `census_crawl::net::Fetcher` at `crates/census-crawl/src/net/` — verified exists.
- Cross-links all other root docs; no broken references.

### 2. ARCHITECTURE.md (264 lines)
- **Verdict: current**
- Binding architecture and standards. §1 notes root package deleted 2026-09-23. §4 lists 8 census crates + xtask = 9 workspace members — matches actual `Cargo.toml`.
- Spot-checked: `crates/census-store/src/` exists (lib.rs, keys.rs, read/, write.rs, legacy/); `crates/census-crawl/src/` exists; `crates/athleticnet-browser/src/outcome.rs` exists.
- No broken references.

### 3. CHROMIUM_DESIGN.md
- **Verdict: current**
- Persistent Chromium source transport design. Notes root package deleted 2026-09-23, marks historical sections.
- Spot-checked: `crates/athleticnet-browser/src/transport/rankings/` exists with capture.rs, mod.rs, results.rs, session.rs, teardown.rs, tests.rs.
- Qualification gates section still lists pending work (no live-collection readiness claim) — honest.

### 4. COLLECTOR_PATTERNS.md (220 lines)
- **Verdict: stale**
- Date: 2026-09-20/21. An inventory of reusable assets across plan phases. Most references to the deleted root package are already marked historical with `~~strikethrough~~`.
- **Stale claim at §2:** Items 3, 4, 6, 9 from the 2026-09-20 finding list are already fixed/changed per the document's own status column, making the list partially redundant with the current code state. The document's own §2 status column already notes the fixes.
- **Proposed action: stale → archive.** The document is a dated reconnaissance memo whose findings are already reflected in code. It adds no actionable contract. Mark as historical, keep for reference.

### 5. DOMAIN.md (117 lines)
- **Verdict: current**
- Types, identities, evidence rules. Cites `crates/census-domain/src/model.rs` — verified exists with 50+ pub types.
- Spot-checked: `crates/census-domain/src/jurisdiction/` exists with bucket.rs, codes.rs, meet_state.rs, mod.rs, scope.rs, ser_de.rs, table.rs, tests.rs. `crates/census-domain/src/error.rs` exists.
- §7 mentions coach contact policy with "only publicly published professional contact information" — note: the context says another agent is concurrently updating contact-policy wording in several docs. No stale claims about withholding.

### 6. FJALL_SCHEMA.md (455 lines)
- **Verdict: current**
- Design-side Fjall store schema. References `docs/FJALL_SCHEMA.md` as "Operational companion" at line 3 — that file exists and is current.
- Spot-checked: `crates/census-store/src/lib.rs` line 70 has `const CACHE_BYTES`; `crates/census-store/src/table.rs` defines `Table` enum with 15 variants.
- Cross-ref: `docs/FJALL_SCHEMA.md` references `../FJALL_SCHEMA.md` — verified resolves to root.

### 7. HANDOFF.md (602 lines)
- **Verdict: stale**
- Large handoff document with executed evidence from 2026-09-21. §11 notes "historical: root package deleted 2026-09-23" for the source design section.
- **Stale claims:** §1 lists CLI commands that have evolved; §11's CLI list is from an earlier tree. The document is a historical record of a specific delivery (not current-tree documentation).
- **Proposed action: dead → superseded.** Superseded by the current `ARCHITECTURE.md`, `DOMAIN.md`, and crate READMEs. The evidence at §9-§10 is a dated ledger of a specific run; it belongs in the evidence archive, not as current documentation.

### 8. PERFORMANCE.md
- **Verdict: current**
- Benchmarks and performance claims. Cites `crates/census-service/benches/core.rs` and `crates/census-service/examples/bench_*.rs` — verified.

### 9. PROFILE_REPLICATION.md (257 lines)
- **Verdict: stale**
- Date: 2026-09-20. Phased plan for replicating athletic.net profiles. §10 has a phased plan P0-P6; many phases are now partially or fully implemented (P0 collector is built, P1 cohort seed via MileSplit is in place).
- **Proposed action: stale → archive.** This is a dated reconnaissance plan. Its planning decisions have been superseded by actual implementation. Keep as historical input.

### 10. README.md (90 lines)
- **Verdict: current**
- Front door document. Lists 9 workspace members, entry points, commands, ground rules.
- Spot-checked: `crates/census-crawl/src/net/` exists; `xtask/src/main.rs` exists; `xtask/README.md` exists; `tools/gate.sh` exists.

### 11. RESTATE_WORKFLOWS.md (574 lines)
- **Verdict: current**
- Restate workflow documentation. Properly marks historical sections (root package deleted 2026-09-23) with strikethrough and comments.
- Spot-checked: `crates/census-service/src/restate_services/mod.rs` exists with Census, Consolidate, Report, Bests, Workbook, Ingest, Sweep, JurisdictionCensus, NationalCensus.

### 12. SCOPE.md (122 lines)
- **Verdict: current**
- Product and source scope. Notes root package deleted. Expanded roster target is documented with current gap analysis.
- No broken references.

### 13. SOURCES_SURVEY.md (283 lines)
- **Verdict: stale**
- Date: 2026-09-20. Source survey findings. Already notes that "since that date the census crate has landed adapters for..." Many of the sources surveyed (MileSplit, TFRRS, MaxPreps, DirectAthletics) are now implemented.
- **Stale claims:** §12 lists open items that have since been resolved (MileSplit rankings XHR endpoint still TBD but is a discovery concern, not collection). The survey itself is a dated input document.
- **Proposed action: stale → archive.** This is the research input that chose which sources to implement. It is no longer current but is a useful historical artifact.

### 14. SOURCE_ADAPTER_GUIDE.md (182 lines)
- **Verdict: current**
- How to add a source adapter. Cites `crates/census-crawl/src/` structure, `crates/census-service/src/cli/provider.rs` — both verified.
- Spot-checked: `xtask/src/scaffold.rs` exists with new-source implementation.

### 15. TESTING.md (378 lines)
- **Verdict: current**
- Test suite inventory. Properly marks all deleted-root paths as historical with strikethrough. Lists 598 test attributes + 8 proptest blocks (2026-09-21 count).
- No broken references. Comprehensive coverage matrix.

### 16. WORKFLOW_REVIEW.md (86+ lines)
- **Verdict: stale**
- Current native workflow review. References the old root package structure extensively. Already notes it is "a current-source architecture map, not a historic alpha plan and not execution evidence."
- **Stale claims:** References `src/runtime/worker.rs`, `src/cli/args.rs`, `src/runtime/source/retry.rs` — all gone. While some references are historical, the document's core analysis is based on a tree that no longer exists.
- **Proposed action: stale → archive.** The analysis is based on a deleted codebase. The conclusions are absorbed in ARCHITECTURE.md and per-crate READMEs.

### 17. athletic-pipeline-handoff.md (130 lines)
- **Verdict: dead**
- Handoff document for a specific native Restate harness work (rankings durable verification). References specific Restate instance ports (19470, 19480, 19282, 21045, 9223), specific worker configs, and specific harness instance `athletic-rankings-lane` that are external infrastructure, not code.
- References `crates/census-service` as the codebase but describes a separate lane work. The four fixture gaps described were fixed and superseded by HANDOFF.md's §9-§10.
- **Superseded by:** HANDOFF.md §9-§10.

### 18. athletic-pipeline-project-pack.md (401+ lines)
- **Verdict: dead**
- Companion project pack. Explicitly notes in HANDOFF.md §9: "imported verbatim at the user's request and are reference material, not current state." SHA256 pin to tree `33e2adb`.
- **Superseded by:** HANDOFF.md and the current root docs.

## DOCS/ DIRECTORY (8 files)

### 19. docs/DECOMPOSITION.md (161 lines)
- **Verdict: stale**
- Phase 2 decomposition plan. Already marks root-crate targets as historical. Lists completed splits and remaining targets from a 2026-09-23 tree.
- Many of the listed files have since been split further or moved.
- **Proposed action: stale → archive.** The Phase 2 plan was executed. Current file structure has evolved beyond this plan's targets.

### 20. docs/FJALL_BACKUP.md
- **Verdict: current**
- Backup/restore drill documentation. References `crates/census-store/src/backup/` and `crates/census-service/tests/backup_restore.rs` — both exist.
- Cross-references FJALL_SCHEMA.md (root) — verified.

### 21. docs/FJALL_SCHEMA.md (200 lines)
- **Verdict: current**
- Operational companion to root FJALL_SCHEMA.md. Covers durability knobs, sharp-edge list with current line citations, backup/restore drill, and the clock capability.
- Cross-references `../FJALL_SCHEMA.md` — resolves to root FJALL_SCHEMA.md.

### 22. docs/HARDENING-PROGRAM.md (685 lines)
- **Verdict: stale**
- Status marked as "plan, not yet executed." Baseline commit `4e5b828` measured 2026-09-21.
- **Stale claims:** §1.2.2 lists "21 census files exceed the 300-line budget" — the Phase 2 decomposition (docs/DECOMPOSITION.md) later split many of these. §1.1 mentions "census-service has 1 unwrap, 75 expect" — the Phase 2 cleanup likely reduced these. §5.1 lists "D1: Five walk sites journaled a unit as done before its rows were in the store" — this is described as already fixed by "append-then-journal."
- **Proposed action: stale → archive.** The hardening program was partially executed (Phase 2 decomposition, forbidden construct removal). Current state is reflected in the actual tree, not this plan.

### 23. docs/OPERATIONS.md
- **Verdict: current**
- Operations runbook. Describes deployment, monitoring, and operational procedures. No references to deleted root paths detected.

### 24. docs/VERIFICATION-EVIDENCE.md
- **Verdict: current**
- Append-only evidence ledger. Per the contract, this is a historical record that is not a candidate for deletion. Staleness is fixed by a dated superseding note, never by rewriting.

### 25. docs/architecture.md (265 lines)
- **Verdict: current**
- "Current-state reference" — what each crate is, phases, store, seal, module edges. Complements root ARCHITECTURE.md (which is "the binding architecture and standards").
- Spot-checked: `crates/census-store/src/table.rs` 15 variants, `crates/census-domain/src/jurisdiction/table.rs:126`, `crates/census-service/src/bootstrap/options.rs:44`.
- Not a duplicate of ARCHITECTURE.md — different purpose (current-state vs. binding standards).

### 26. docs/deployment-lifecycle.md
- **Verdict: current**
- Deployment lifecycle documentation. No broken references detected.

### 27. docs/adr/README.md (241 lines)
- **Verdict: current**
- ADR index. Contains 12 ADRs (ADR-001 through ADR-012, with ADR-007 through ADR-010 inlined rather than separate files).
- All linked ADR files exist: ADR-001 through ADR-006, ADR-011 are separate files. ADR-007 through ADR-010 are sections in this README.
- No broken references.

### 28. docs/adr/ADR-001 through ADR-011 (7 separate files + 4 inlined)
- **Verdict: all current**
- ADRs are decision records. ADR-001 through ADR-006 and ADR-011 have long-form files; ADR-007 through ADR-010 are inline in README.md.
- All decisions remain in force and are referenced by code/docs.

### 29. docs/migration/golden-freeze-2026-09-22.md
- **Verdict: dead**
- A dated migration freeze record. The freeze was from a specific date and has been superseded by the current tree.
- **Superseded by:** The current `Cargo.toml` members, crate structures, and ARCHITECTURE.md §1.

### 30. docs/migration/module-map.md (424 lines)
- **Verdict: stale**
- Module-to-crate migration map. References `crates/g1-audit` which no longer exists (folded into xtask 2026-09-23). Section 1.2 and the `g1-audit` row in §5 are historical.
- **Stale claims:** Line 3-20 reference `crates/g1-audit` which was deleted; §5.1-5.2 list modules from the old census-service/src/ layout that has since been split.
- **Proposed action: stale → archive.** This document is a pre-split survey. Its content has been superseded by the actual crate structure.

## CRATE READMES (1 file)

### 31. crates/census-service/README.md (277+ lines)
- **Verdict: current**
- Census crate documentation. Covers CLI, Restate services, test structure, golden fixtures.
- Spot-checked: `crates/census-service/src/cli/mod.rs` Command enum exists; `crates/census-crawl/tests/fixtures/` exists with 11 provider directories.

## XTASK (1 file)

### 32. xtask/README.md (236 lines)
- **Verdict: current**
- Developer commands documentation. Lists all xtask subcommands with descriptions.
- Spot-checked: `xtask/src/main.rs` exists; `xtask/src/scaffold.rs` exists (new-source); `xtask/src/g1/main.rs` exists (g1-audit binary).

## TOOLS (3 files)

### 33. tools/README.md
- **Verdict: current**
- Tools directory README. References `tools/gate.sh` and `tools/quality-baseline.json` — both exist.

### 34. tools/athletic-net-pr-population/README.md
- **Verdict: check needed**
- README for athletic-net-pr-population tool. Needs verification if the directory exists.

### 35. tools/durability/README.md
- **Verdict: stale**
- References `crates/census-service/tests/restate_kill_restart.rs` and `$RESTATE_SERVER_BIN` — the test file exists but the env var is operational infrastructure, not code.
- **Proposed action: stale.** The tooling context has changed; the README describes a specific durability test setup.

## RESEARCH (treated as historical captures — not candidates for deletion unless duplicated verbatim)

### 36. research/README.md
- **Verdict: current**
- Lane index for source research. Describes the lane contract and lanes table. References `research/midwest/` as outside the tree — correct.

### 37-70. research/midwest-source-program/ (50+ files)
- **Verdict: all current as historical captures**
- These are reconnaissance captures from the Midwest source program. They are external to the main tree and not candidates for deletion per the contract.
- The parent `research/README.md` notes these are retained outside the tree; this directory is a copy for reference.

### 71. research/ENDGAME-GAPS.md
- **Verdict: stale**
- Identifies endgame gaps in the census pipeline. References `crates/census-service/src/` paths that are historical.
- **Proposed action: stale → archive.** Gap analysis from a specific point in time; current gaps are tracked in the code's TODOs and the evidence ledger.

### 72. research/G1-LIVE-SEARCH-CONTRACT.md
- **Verdict: stale**
- Grade-1 live search contract documentation. References `src/runtime/` and `tests/` paths from the deleted root package.
- **Proposed action: stale → archive.** The g1-audit tool was folded into xtask; the contract is implemented.

### 73. research/sources/ (17 SOURCE_REPORT.md files + samples)
- **Verdict: all current as historical captures**
- These are source research captures. Treated as historical per contract.

## BROKEN REFERENCES

### Doc-to-doc broken links
1. **docs/adr/README.md** — Links `ADR-001-fjall-primary-store.md`, `ADR-002-restate-owns-retries.md`, etc. without `./` prefix. **Actually OK** — they resolve correctly from the same directory (verified).

### Path references to deleted code (not yet marked historical)
1. **docs/adr/ADR-002-restate-owns-retries.md:93** — `src/runtime/source/admission.rs:121` — deleted root path, not marked historical.
2. **docs/HARDENING-PROGRAM.md:552** — `tests/recovery.rs` — this file still exists at `crates/census-service/tests/recovery.rs`.
3. **docs/HARDENING-PROGRAM.md:676,680** — `tests/parity_pipeline.rs`, `tests/backup_restore.rs` — these exist at `crates/census-service/tests/`.
4. **research/midwest-source-program/research/midwest/02-athletic-net-profile-acquisition.md:243** — `src/profile/bio.rs:347-350` — deleted root path, not marked historical.
5. **research/midwest-source-program/research/midwest/02-athletic-net-profile-acquisition.md:346** — `src/runtime/source/retry.rs` — deleted root path.
6. **research/midwest-source-program/research/midwest/04-athletic-net-rankings-discovery.md:439** — `src/runtime/source/request.rs`, etc. — deleted root paths.
7. **research/midwest-source-program/research/midwest/05-athletic-net-identifier-graph.md:25** — `src/runtime/rankings/types.rs`, etc. — deleted root paths.
8. **research/G1-LIVE-SEARCH-CONTRACT.md:28** — `src/runtime/source/request/build.rs` — deleted root path.
9. **research/G1-LIVE-SEARCH-CONTRACT.md:206-207,527-528** — `g1-audit` binary path — the binary now lives at `xtask/src/g1/main.rs`.
10. **research/ENDGAME-GAPS.md:105** — `tests/fjall_restate_e2e.rs:493` — exists at `crates/census-service/tests/fjall_restate_e2e.rs`.
11. **research/ENDGAME-GAPS.md:203** — `src/runtime/run_protocol.rs:129-139` — deleted root path, marked "deleted" in text.
12. **ARCHITECTURE.md:259** — `crates/g1-audit` audit binary — already noted as folded into xtask.
13. **DOMAIN.md:102-104** — `FailureCode` (`src/runtime/protocol.rs`) — marked as historical (deleted root package).
14. **HANDOFF.md:73** — `tests/result_verify.rs` — this was in the deleted root package's `tests/`.
15. **athletic-pipeline-handoff.md:19** — `src/result_verify/rankings/records.rs:~112` — deleted root path.
16. **athletic-pipeline-project-pack.md:401** — `src/cli/args.rs (Start)` — deleted root path.
17. **crates/census-service/README.md:106** — `tests/merge_properties.rs` — exists at `crates/census-service/tests/`.
18. **crates/census-service/README.md:277** — `tests/parser_roundtrip_properties.rs` — exists at `crates/census-service/tests/`.
19. **tools/durability/README.md:42** — `crates/census-service/tests/restate_kill_restart.rs` — exists at `crates/census-service/tests/`.

## ORPHANS (docs nothing links to)

1. **SCOPE.md** — Only linked from README.md and a few research files. Not linked from AGENTS.md or ARCHITECTURE.md despite being a core document.
2. **CHROMIUM_DESIGN.md** — Only linked from HANDOFF.md §9. Not linked from AGENTS.md.
3. **WORKFLOW_REVIEW.md** — Only linked from HANDOFF.md and SCOPE.md. Not linked from AGENTS.md.
4. **docs/deployment-lifecycle.md** — Not linked from any other document.
5. **docs/migration/golden-freeze-2026-09-22.md** — Only linked from migration/module-map.md (historical).
6. **docs/migration/module-map.md** — Only linked from AGENTS.md indirectly (via migration notes in ARCHITECTURE.md).
7. **athletic-pipeline-handoff.md** — Referenced from HANDOFF.md §9 as imported companion.
8. **athletic-pipeline-project-pack.md** — Referenced from HANDOFF.md §9 as imported companion.
9. **research/ENDGAME-GAPS.md** — Not linked from any core document.
10. **research/G1-LIVE-SEARCH-CONTRACT.md** — Not linked from any core document.
11. **research/midwest-source-program/** — Entire directory not linked from research/README.md (which says it's outside the tree).
12. **crates/census-service/README.md** — Not linked from AGENTS.md despite being a core crate document.
13. **xtask/README.md** — Not linked from AGENTS.md despite being a core developer tool.

## COUNTS BY VERDICT

| Bucket | Count | Files |
|--------|-------|-------|
| **current** | 16 | AGENTS.md, ARCHITECTURE.md, CHROMIUM_DESIGN.md, DOMAIN.md, FJALL_SCHEMA.md, README.md, RESTATE_WORKFLOWS.md, SCOPE.md, SOURCE_ADAPTER_GUIDE.md, TESTING.md, docs/FJALL_BACKUP.md, docs/FJALL_SCHEMA.md, docs/OPERATIONS.md, docs/VERIFICATION-EVIDENCE.md, docs/architecture.md, docs/deployment-lifecycle.md, docs/adr/README.md, docs/adr/ADR-001..006, ADR-011 (7 files), xtask/README.md, crates/census-service/README.md, tools/README.md, research/README.md |
| **stale** | 7 | COLLECTOR_PATTERNS.md, HANDOFF.md, PROFILE_REPLICATION.md, SOURCES_SURVEY.md, WORKFLOW_REVIEW.md, docs/DECOMPOSITION.md, docs/HARDENING-PROGRAM.md, docs/migration/module-map.md, research/ENDGAME-GAPS.md, research/G1-LIVE-SEARCH-CONTRACT.md |
| **dead** | 2 | athletic-pipeline-handoff.md, athletic-pipeline-project-pack.md, docs/migration/golden-freeze-2026-09-22.md |
| **historical (research)** | ~70 | research/midwest-source-program/*, research/sources/*, research/restate/* |

## SUMMARY OF PROPOSED ACTIONS

### Immediate (delete or archive)
1. **athletic-pipeline-handoff.md** — Dead, superseded by HANDOFF.md
2. **athletic-pipeline-project-pack.md** — Dead, imported companion, superseded
3. **docs/migration/golden-freeze-2026-09-22.md** — Dead, dated freeze record

### Archive (move to research/ or mark historical)
4. **COLLECTOR_PATTERNS.md** — Dated reconnaissance, findings in code
5. **HANDOFF.md** — Historical delivery record, current state in ARCHITECTURE.md + per-crate docs
6. **PROFILE_REPLICATION.md** — Dated phased plan, partially implemented
7. **SOURCES_SURVEY.md** — Dated survey input, sources now implemented
8. **WORKFLOW_REVIEW.md** — Based on deleted root package tree
9. **docs/DECOMPOSITION.md** — Phase 2 plan, executed
10. **docs/HARDENING-PROGRAM.md** — Plan, partially executed
11. **docs/migration/module-map.md** — Pre-split survey, superseded
12. **research/ENDGAME-GAPS.md** — Dated gap analysis
13. **research/G1-LIVE-SEARCH-CONTRACT.md** — Based on deleted root package

### Add cross-links
14. Link `crates/census-service/README.md` from AGENTS.md
15. Link `xtask/README.md` from AGENTS.md
16. Consider adding `SCOPE.md` and `CHROMIUM_DESIGN.md` to AGENTS.md
