# Handoff — Class-of-2027 Nationwide Recruiting Census (2026-09-25)

## Goal

Complete the Class-of-2027 nationwide recruiting census delivery: seal the census over the rebuilt workbook via Restate, verify the full durability harness with evidence-backed pass results, update documentation, commit the integrated tree, and push origin/main.

Repository: `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline`

## Current State

**Git:** `HEAD` = `origin/main`, 0 ahead, 0 behind. Last commit on main is `ee1a296 (push)`.

**Uncommitted work:** 33 modified files (+1424 -1602) + 6 untracked files. These are NOT on main.

**Untracked files:**
- `crates/census-review/src/model_transport_tests.rs`
- `crates/census-service/tests/exporter_kill_restart.rs`
- `crates/census-store/examples/`
- `tools/durability/restate-enospc-probe.sh`
- `tools/durability/scenario-17-recovery-tests.sh`
- `agent:/` (session artifacts)

**Services running:**
- Restate admin: 19095, ingress: 18095
- Census service: port 19103

## Completed (18/24)

### Integration & Code Quality
- 8 splitter agents' siblings integrated (census-domain fixed_mark/records, census-report bests+meta+recruiting, census-store write_batch, census-service seal/workbook+sweep, census-crawl raceday/table)
- `cargo fmt --all` + `cargo check --workspace --all-targets` green
- `cargo xtask scan`: 0 files >300 lines, 0 functions >60 lines
- `tools/gate.sh` full pass: 1,343 tests passed / 3 skipped, strict clippy 0 diagnostics
- All gate lanes: fmt, check, doc, scan, integrity, purity, seams, deny, audit, vet, machete, geiger, powerset, bench presence all PASS

### Fixes Applied
- `census-service seal/workbook/reconcile.rs`: `saturating_sub` replaced unchecked subtraction (arithmetic_side_effects)
- ratchet PASS after clippy fix
- Handwritten fixed-mark numeric conversion replaced with `rust_decimal` (preserving stored integers and legacy rounding)
- Million-conversion benchmark: float-to-Decimal conversion verified 1,000,045 inputs with zero mismatches (2.25ms vs 2.81ms)
- `ModelClient::adjudicate` body-read failures now preserve `ModelError::Request` and original `reqwest::Error` (7 HTTP failure scenarios pass)

### Workbook
- Offline rebuild: `census-service workbook --grad-year 2027 --out var/midwest-census/out/census-service-2026-09-25.xlsx` completed in 1,635.32s
- Output: 78,073,748 bytes, 623,509 athlete rows, 9,958 PR rows, 32,031 coach rows
- 11 sheets confirmed: Athletes, PRs, Performances_001, Coaches, Schools, Meets, Sources, Coverage, Conflicts, Review, Run Metrics
- `census-service verify --store var/midwest-census --workbook out/*.xlsx` exits 0
- Verified §54 sheets: Schools (meta/schools.rs), Conflicts (split data_quality_sheet), Review (wire sheet list)

### Durability Evidence
- **Mid-batch SIGKILL recovery:** Worker killed after 512/4096 meets; restart processed 3,584 remaining with zero duplicates. 9 recovery cases passed in 10.12s.
- **Restate crash recovery:** Process SIGKILL during paused workflow; restart preserves invocation ID, resumes correctly. 2 Restate crash cases passed in 125.65s.
- **Isolated Restate ENOSPC:** Real RocksDB ENOSPC via high-entropy bodies; restart preserves completed workflow ID, HTTP 409 on replay. Passed in 6.15s.
- **Backup drill:** `tools/ops-backup-drill.sh var/midwest-census` — verifies consistent restoration (not line counts).

### Test Artifacts
- 17 durability scenario scripts (`tools/durability/scenario-*.sh` + `scenario-17-recovery-tests.sh`)
- `tools/durability/run.sh` executes all scenarios, prints table, exits nonzero on any FAIL/SKIPPED
- `crates/census-service/tests/recovery.rs` — mid-batch worker kill ladder (4096 meets, 64 commits)
- `crates/census-service/tests/restate_kill_restart.rs` — Restate process crash + restart
- `crates/census-service/tests/exporter_kill_restart.rs` — untracked
- `crates/census-review/src/model_transport_tests.rs` — HTTP failure transport (untracked)

## Remaining Work (6/24)

### 1. Re-seal census over rebuilt workbook
- **Status:** Pending. Workbook rebuilt but seal workflow not run against new data.
- **Action:** Run `census-service seal --grad-year 2027 --store var/midwest-census` via Restate ingress (or direct binary). Needs season=2026, revision=8, 53 source objects.
- **Risk:** Seal depends on workbook data being consistent; verify store state before sealing.

### 2. §59 Durability harness — evidence-backed pass
- **Status:** Partially repaired. Last `tools/durability/run.sh` returned **exit 1**: 6 PASS, 0 FAIL, 11 SKIPPED.
- **Current scenarios (17 total):**
  1. `scenario-01-endpoint-kill.sh` — endpoint crash mid-workflow
  2. `scenario-02-restate-kill-during-fanout.sh` — Restate kill during fanout
  3. `scenario-03-reboot-with-full-census.sh` — full census reboot recovery
  4. `scenario-04-rolling-upgrade.sh` — Restate version rollback
  5. `scenario-05-http-error-taxonomy.sh` — HTTP error classification
  6. `scenario-06-no-duplicate-evidence.sh` — idempotent observation ingestion
  7. `scenario-07-domain-dedup.sh` — domain-level dedup
  8. `scenario-08-global-budget.sh` — source admission rate limiting
  9. `scenario-09-disk-full-fjall.sh` — Fjall ENOSPC
  10. `scenario-10-disk-full-restate.sh` — Restate ENOSPC (PASSED in isolation)
  11. `scenario-11-parent-exit.sh` — parent process exit propagation
  12. `scenario-12-cross-midnight.sh` — midnight boundary
  13. `scenario-13-ai-review-failures.sh` — AI review failure handling
  14. `scenario-14-seal-refuses.sh` — seal rejects inconsistent state
  15. `scenario-15-full-backup-restore.sh` — backup/restore drill
  16. `scenario-16-golden-census-determinism.sh` — deterministic census re-run
  17. `scenario-17-recovery-tests.sh` — untracked, new recovery test
- **Key constraint:** All scenarios must FAIL if a real fault did not occur — no mocked fault, no generic-error-as-ENOSPC.
- **Constraint:** `SCRATCH_STORE` must be off tmpfs (local disk) for durability scenarios.
- **Action:** Run full harness against current code + store state; diagnose each SKIP/FAIL; fix scenarios that are too loose.

### 3. Repair unsafe durability scenarios
- **Status:** Pending. Some scenarios may be passing via insufficient fault injection or false positives.
- **Action:** Review each passing scenario for: real fault vs. synthetic; deterministic outcome; no generic errors masquerading as real faults.
- **Evidence requirement:** Every PASS must be backed by observable proof (journal state, entity counts, HTTP response codes).

### 4. Update docs/VERIFICATION-EVIDENCE.md
- **Status:** Pending. Current document covers Phase 6 evidence (integration gate, decimal conversion, Restate crash, model HTTP failures).
- **Action:** Append sections for: seal results, updated durability harness results, workbook rebuild metrics, library cutover proof, backup drill verification.
- **Constraint:** Evidence must be factual and reproducible — no architectural assertions without test output.

### 5. Commit the integrated tree
- **Status:** Pending. 33 modified files + 6 untracked files need commit.
- **Staged changes include:**
  - `cargo.lock` — dependency updates
  - `census-domain/Cargo.toml` — rust_decimal addition
  - `census-domain/src/model/fixed_mark.rs` and subs — decimal conversion
  - `census-review/` — model transport tests
  - `census-service/tests/recovery.rs`, `restate_kill_restart.rs` — durability tests
  - `tools/durability/` — 17 scenario scripts + README
  - `tools/ops-backup-drill.sh` — backup drill
  - `docs/VERIFICATION-EVIDENCE.md` — evidence documentation
  - `supply-chain/` — audit lock files
- **Action:** `git add -A && git commit -m "..."`

### 6. Push origin/main
- **Status:** Already done. `HEAD` = `origin/main`, 0 ahead/0 behind.
- **Note:** After commit in #5, will need `git push origin main`.

## Critical Constraints

- No `unsafe`, no production recursion, no careless `unwrap`/`expect`/`panic` from input
- No ignored `Result`
- File size: no files >300 lines, no functions >60 lines
- Only Fjall + Restate + census-service binaries (no PostgreSQL)
- GPU inference only on ambiguous identity cases (Qwen 3.6 on RTX 5090 + RTX 3090)
- Census service on port 19103, Restate admin on 19095, ingress on 18095
- All durability scenarios must fail if real fault did not occur

## Verification Standards

- §58: Unit tests, property tests, parser fuzzing, failure injection, deterministic replay, concurrency tests, Restate recovery tests, browser record/replay, Fjall backup/restore, parser golden corpus, workbook verification, dependency audit, license audit, async review, security review, adversarial identity review
- §64: Source fixture isolation (`cargo xtask source-test <source>` works without Restate/internet)
- §67: `cargo xtask gate` is the master quality command
