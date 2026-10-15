# Final-spec alignment review

| Control | Value |
|---|---|
| Status | Review retained; repairs in progress; **final-spec acceptance not established** |
| Owner / date | Main / 2026-10-14 |
| Scope | Nine workspace crates; redundancy, Rust discipline, architecture/ADR currency |
| Authority | [Spec v2](../specifications/athletic-recruiting-end-vision-spec-v2.md), [architecture](../../ARCHITECTURE.md), [ADRs](../adr/README.md) |
| Tracking | Review `cbxd`; unresolved findings `iilh`; [existing delivery plan](../NATIONAL-CENSUS-PLAN.md) remains authoritative |
| Provenance | Original review `59a80d451fba1ba95dffd46fb979d6f9d8935d02`; full-suite repair base `9745c809`, subsequently rebased onto `cdd1bd4f`; repository history identifies the repair revision, not a release certificate |

**Method:** architecture-description organization informed by
[ISO/IEC/IEEE 42010:2022](https://www.iso.org/standard/74393.html): stakeholder concerns,
views, decision rationale, correspondence and known inconsistencies. Not ISO conformity
or NASA certification. Concerns: recruiting accuracy, durable evidence, honest coverage,
minimal repeated work and maintainable ownership. The original read-only review covered
domain/report/reconcile/review, store/service and crawl/browser. Subsequent implementation and
execution evidence are recorded below; neither phase establishes national coverage, a complete
security audit or exhaustive dead-code proof.

The Downloads spec and repository spec are byte-identical:
`a3b687cc06952dbceaeda2f21300e4769acb96130570fc1174ff70210a09f11e`.
Architecture adopts the full reusable product; current runtime is narrower. Decisions,
implemented components and qualified behavior must remain separate evidence states.

## Findings register

Source paths below are relative to `crates/` at the original review revision. Static
counterexamples identify risks, not observed contamination of a live dataset. H = final-product
correctness/acceptance hold; M = bounded defect, contract gap or maintainability risk. No finding is waived.

| ID | Level / concern | Evidence | Disposition required |
|---|---|---|---|
| R1 | H — Participation/window eligibility | `census-report/src/bests/reduce.rs:79–103,233–241`; `bests/mod.rs:30–36`: athlete/event/numeric compatibility gate, no selected high-school participation/as-of interval. | Spec §§4,7: gate candidates with versioned participation and temporal evidence; retain rejection reasons. Counterexample: admitted athlete with a collegiate/post-graduation mark. |
| R2 | H — Current school | `census-domain/src/model/athlete.rs:69–90`; `census-report/src/report/derivation/population.rs:39–52`: geography uses one stored school, not dated affiliation resolution. | Spec §§4,8: bind accepted affiliation at cutoff into frozen input; preserve performance-time school separately. Domain affiliation candidates are not integrated publication evidence. |
| R3 | H — Permanent identity / splits | `census-domain/src/model/athlete.rs:7–47,135–151`; `model/identity_application.rs:107–116`; `model/identity_decision.rs:20–45`: candidate-derived IDs, least-member canonical projection, incomplete split/revocation/successor contract. | Spec §5: Main-owned stable public-ID and reversible decision contract; preserve historical IDs and export aliases. Do not remint old athletes. |
| R4 | H — Required event classes | `census-domain/src/model/recruiting_catalog/policy.rs:190–192` lists flat 300m/500m/600m/2000m/10000m/two-mile variants as missing. `model/event_specification.rs:8–16` lacks combined-event lineup/scoring version; performance contract lacks multis completion. | Spec §7: add explicit versioned classes; withhold unknown/incomplete multis. Current unresolved disposition is correct but incomplete. |
| R5 | H — Normalized dataset | `census-report/src/workbook/publication/manifest.rs:213–247` has convenience artifacts, not the eight required normalized CSV families. | Spec §13: athletes, best marks, schools, coach contacts, comparison classes, provenance, coverage and aliases from one frozen generation. SQLite remains optional, not a missing mandatory feature. |
| R6 | M — Tie evidence | `census-report/src/bests/reduce.rs:106–110`; `bests/selection.rs:8–15`; `bests/write.rs:8–39`: candidate count is not tied-winner count; context reducer retains an incumbent. | Spec §§7,13: distinct eligible/tied counts and deduplicated tied references. Passing winner tie-break tests do not establish those fields. |
| R7 | M — Repeated publication loads | `census-service/src/restate_services/national/publication.rs:118–140`; `restate_services/jobs.rs:106–145`; `census-report/src/export/dataset.rs:195–208`: Report, Bests and Workbook separately acquire datasets. | Share one bound generation for outputs advertised together. Separate snapshots can differ; **no mixed-revision defect inside the managed workbook bundle was demonstrated**. No optimization speedup claimed. |
| R8 | M — Duplicate workflow ordering | `census-service/src/cli/cycle.rs:145–178,218–238` and `restate_services/national/publication.rs:18–41`: offline/live/national sequences differ; live explicitly skips index derivation. | Architecture §5: one application-stage/completion contract with explicit online/stopped-store effects. Retain legitimate offline maintenance, not competing business rules. |
| R9 | M — Duplicate relay rule / ownership | `census-domain/src/model/event_ontology.rs:158–168` and `census-report/src/bests/events.rs:3–13` repeat six variants. Complete comparison policy lives in report `bests/key.rs:15–72`. | Use domain relay predicate; domain owns comparison semantics. No current relay disagreement or duplicate workbook winner reducer found. |
| R10 | M — Unchecked construction | `census-domain/src/model/athlete.rs:163–193`: public `new` uses debug assertions; `new_checked` exists. `census-crawl/src/athleticlive/map_rows/identity.rs:115–125` calls `new`. | Checked untrusted construction. Public empty-name input can panic in debug; adapter crash not demonstrated and upstream validation may prevent that particular path. Lexical scan is not whole-program panic proof. |
| R11 | M — Semantic receipt witness | `census-store/src/receipt.rs:17–25`: operation/digest/time/count/credit; `write_batch/commit.rs:52–105`: serialized receipt check and SyncData atomic batch. | Spec §9: explicitly bind accepted revision, commit sequence, output/version and outcome. Some callers encode meaning elsewhere; no local atomicity race alleged. Version, do not reinterpret old receipts. |
| R12 | H — Reusable lifecycle integration | `census-domain/src/model/source_acquisition/work.rs:13–25,152–228`; [capability boundary](../PROFILE-EXECUTION-V1.md): run-bound MileSplit attempts and enrollment-only refusal. | Spec §§6,9–12,14: attempts are not frontier/refresh/reparse/invalidation/annual reuse. Keep planned gaps and fail-closed refusal; no implicit reset/rebinding. |
| R13 | H — Discovery/UP qualification | `census-service/src/restate_services/meets_arms/prerequisites.rs:6–11` blocks Athletic.net without producer/frontier; profile compiler refuses source-backed regional priority/membership. | Spec §§3,4,19: qualify source-backed UP denominator and five-state path. Registration/armed dispatch/directory collection are not completed athlete discovery. |
| R14 | M — Browser resource accounting | `athleticnet-browser/src/transport/script.rs:21–52` caps decoded bytes, not separate encoded/wire bytes; base64 creates additional representations. | Spec §§10,12,16: account for wire/decoded limits and aggregate process/browser/storage envelope. Local task/body caps do not prove global RAM/disk ceilings. |
| R15 | M — Protocol maintenance duplication | Browser `src/outcome.rs` and crawl `src/net/bridge/wire.rs` separately express corresponding outcomes with different runtime representations and shared wire fixtures. | Evaluate one wire-contract owner or explicit boundary rationale. Not duplicate acquisition logic; do not create a dependency cycle merely to remove DTOs. |

**Retain, do not deduplicate blindly:** provider-specific ownership/label parsing,
Athletic.net profile versus whole-meet/per-event acquisition, immutable historical
policy fixtures, and independent artifact readback. Whole-meet wind/heat remain unknown
unless separately captured; this is source limitation, not permission to assign favorable
conditions. Shared exact integer ranking, surface/timing/wind classes, conflict fallback,
cohort contradiction handling, managed atomic workbook publication and owned task drain
are substantive components. None certifies the unfinished product.

## Requirement correspondence

| Spec | Current correspondence / limit |
|---|---|
| §§1–2,17 | Architecture and ADR-038 adopt reusable public-source product; fresh/historical contracts preserved. |
| §§3–4,19 | Registry, profile configuration and explicit source refusals; full source graph, UP fence and selected execution incomplete. R1–2,R13. |
| §5 | Conservative identity decisions/alias projection; permanent split/consumer contract incomplete. R3. |
| §§6,9–11 | Capture integrity and atomic local effects are components; general interpretation identity, A→B→A revisions, frontier, negative evidence, reparse and affected-key invalidation incomplete. R11–12. |
| §7 | Exact ranking and technical classes; eligibility, listed event variants, multis and tie inventory incomplete. R1,R4,R6. |
| §8 | Source-backed addresses/tenure rules; dated current-school integration incomplete. R2. |
| §§12,14,16 | Local bounds/configuration/backup APIs; global quotas, annual reuse and exact-build fault/resource qualification unestablished. R12,R14. |
| §§13,15 | Frozen workbook bundle exists; normalized snapshot incomplete. Query database is an optional downstream choice. R5,R7. |
| §18 | Existing delivery plan remains the implementation/acceptance authority; this review creates no competing roadmap. |

## Rust discipline: measured, not asserted

The strict scan checked 18,380 callables / 1,585 files; zero unresolved syntax sites.
It reported zero counted production unsafe/unwrap/expect/panic/indexing/as-cast sites,
but **failed nine handwritten callables above 60 logical lines**:

| Location (relative to `crates/`) | Logical lines |
|---|---:|
| `census-store/src/acquisition/transition/work.rs:9` register_acquisition_work | 66 |
| `census-store/src/acquisition/terminal.rs:132` reuse_acquisition_cache | 66 |
| `census-store/src/acquisition/read.rs:99` read_work | 77 |
| `census-store/src/acquisition/read/closure.rs:8` attempt | 64 |
| `census-store/src/acquisition/qualification.rs:75` declaration | 62 |
| `census-store/src/acquisition/qualification/inventory.rs:7` inventory | 94 |
| `census-store/src/acquisition/cache.rs:35` inventory | 82 |
| `census-store/src/acquisition/backing.rs:33` validate_metadata | 63 |
| `census-service/src/cli/national/run.rs:84` run_national | 79 |

Physical-file warnings: production `census-store/src/backup/integrity.rs` 668 lines;
example recovery boundary 309. Advisory counts: 1,693 callables above 25 lines; 138 above
five parameters. These are not new hard failures or reasons for mechanical context bags.

Power-of-Ten Rule 4 has observed failures; Rule 5/7 boundary enforcement has R10;
Rule 10 cannot pass while format/contract/size fail. Static loop termination (Rule 2),
post-init allocation freedom (Rule 3), worst-case process resource limits and universal
cancellation correctness were not proved. No measured allocation/layout/dispatch rewrite
is proposed. Read Holzman references: `nasa-jpl-standards.md`,
`runtime-performance-architecture.md`, `zero-cost-abstractions.md`,
`latency-throughput-playbook.md`; async references: `structural-correctness.md`,
`cancellation-safety.md`, `spawn-discipline.md` (each under its skill's `references/`).
All four repository startup skills were available and read; the spec's historical
missing-skill statement remains historical.

## Verification record

| Exact command | Observed result |
|---|---|
| `sha256sum /home/lewis/Downloads/athletic-recruiting-end-vision-spec.md docs/specifications/athletic-recruiting-end-vision-spec-v2.md` | Identical digest above. |
| `env -u CI tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src` | Check/lint passed; fmt failed at `census-service/src/cli/national/run.rs:145` wrapping; overall exit 1. No Rust formatting performed. |
| `env -u CI tools/moon-local run pipeline:xtask -- scan` | Exit 1; nine hard callable-size failures; counts above. |
| `env -u CI tools/moon-local run pipeline:xtask -- comments` | Pass: 2,871 Rust files, no comments. |
| `env -u CI tools/moon-local run pipeline:xtask -- panic-extraction` | Pass: 2,871 Rust files and three rendered templates. |
| `env -u CI tools/moon-local run pipeline:xtask -- seams` | Pass: no allowlisted module/crate violations. |
| `env -u CI tools/moon-local run pipeline:xtask -- domain-purity` | Pass: no banned async/I/O normal dependency. |
| `env -u CI tools/moon-local run pipeline:xtask -- contract` | Initial exit 1: empty documented-set and `http_retry_fixture` registration. AGENTS backtick reference repaired. Repeat exit 1: documented-set passed; fixture registration still failed; five known crate-edge deviations. No gate weakened. |
| `tools/moon-local run pipeline:tests -- -E 'test(bests) \| test(recruiting_catalog) \| test(affiliation) \| test(identity_application) \| test(spawn::)'` | 311 passed / 4,841 skipped; run `7c2267ff-4218-49da-b535-0b8cc9f2bac4`. Markdown escapes represent literal filter bars, not shell pipelines. |
| `env -u CI tools/moon-local run pipeline:bench -- --bench capture_export -- --sample-size 10 --warm-up-time 1 --measurement-time 1 --noplot` | Pass: retained-capture ingest → real temporary Store → workbook/CSV/JSONL independent readback; 185 canonical rows / 60 cohort members verified. No live fetch. |

Benchmark `pipeline/capture_export/captured_live_wiaa_co2027`: 10 samples / 20 iterations,
1 s warmup, estimated 1.1908 s collection; Criterion time interval
57.382–58.617 ms (estimate 57.991 ms), throughput 3.3437–3.4157 Kelem/s
(estimate 3.3798). Throughput counts the declared 196 input elements, not accepted
athletes. Optimized portable build. Short local sample, uncontrolled concurrent/cache
conditions; no before/after comparison, p99, allocation profiler, process-memory/disk
ceiling or annual/skew certification. Gnuplot unavailable; plotters fallback did not
prevent benchmark/readback execution.

**Not run:** full release/security, all 17 native faults, parser fuzz campaigns,
whole-workload resource/retention/restore, annual rollover, live UP/five-state collection
or approved production publication. Known gate failures already prohibit acceptance;
this review did not own native fault stores/processes. Focused passes do not waive holds.

## Document control and handoff

Documentation verification: 102 local Markdown targets across the six changed documents
exist; required startup skills and the contract gate's backtick architecture pointer are
present. This checks links/pointers, not semantic release acceptance.

Changed only startup/documentation/evidence: `AGENTS.md`, `ARCHITECTURE.md`, ADR-038,
ADR register, deployment lifecycle and this report. At review capture, AGENTS was 42 lines /
2,444 bytes, down from 296 lines; subsequent startup cleanup reduced it to 32 lines.
Native procedures were retained in the existing deployment owner; headed
Athletic.net and real drain accounting replace the misleading copied headless/default
assumptions. ADR-038 now distinguishes policy 18 and existing enrollment-only capability
from qualified reusable execution. Historical amendments and original spec are untouched.

TASK / OWNERSHIP: review and concise documentation / Main. DO NOT MODIFY: production,
live/historical durable state, unrelated work. INPUT CONTRACT: final spec and binding
architecture. OUTPUT CONTRACT: findings and observed evidence, no release certificate.
ACCEPTANCE: review delivered; `iilh` retains unresolved findings for owner disposition.
HANDOFF: use the existing plan for repairs and exact-build qualification. Review documentation
was subsequently published in `91066391` and startup cleanup in `e58ffa88`.

## Repair integration — 2026-10-14

Incremental repairs, not closure of the final-product findings. Main includes PRs #36–41 at
`cdd1bd4f`. Earlier restoration conflicts were reconciled; subsequent main updates and the PR41
rebase preserved local work without new conflicts. Preservation stashes and live/historical state remain intact.

### Implemented scope

- R10: fallible `CanonicalAthlete::new`; callers propagate `AthleteConstructionError`.
- R4/R5/R6: policy 20, checked combined-event completion, distinct eligible/tied inventories,
  eight normalized CSV families and artifact revision 3. Original policies 18/19 remain read-only
  under their original encodings/layouts. Source qualification and remaining lineage gaps stay open.
- R9: shared `EventKind::is_relay`; comparison keys, measures, catalog classification and tie policy
  now belong to `census-domain::model::performance_comparison`. Report callers migrated; no shim.
- Archive: finite 20 GiB default and attributed capacity-refusal logging. Controlled admission/replay
  is covered; no product capture-rate field or national-scale acceptance is claimed.
- Post-PR38/37 integration: explicit offline-store validation runs before store access without
  rejecting valid global-argument placement; build-job validation accepts ASCII decimal digits only.
- Browser correlation, bounded CSV parsing, historical-school tamper detection and module-size
  repairs are integrated. Bare 403/429 no longer revokes the challenge gate; explicit challenge
  evidence still does. Current real Chromium and transport evidence is recorded below.

### Executed evidence

| Command / run | Observed result | Qualification boundary |
|---|---|---|
| `pipeline:check` / `69a3c4a5` | Passed | After PR41 rebase |
| `pipeline:tests` / `248d03af` | 5,623 passed; four skipped | Pre-PR41 whole workspace; Nextest `7d6782e8-48ce-4b91-931e-f1d54f06d9c2` |
| Journal/conditional/lineage tests / `26c6a922` | 141 passed; 614 filtered | After PR41 fixture migration; Nextest `96576d73-dbf0-4e25-b933-2287e1c513b5` |
| `pipeline:lint-src` / `a4c92269` | Passed; three forced argument-count advisories | After PR41 rebase; no suppression or artificial context bags |
| Release lane `ratchet` / `a6c3d03f` | Passed; oversized production files 5 → 0; no metric grew | Pre-PR41 strict source/size measurement, not a full release pass |
| Release lane `panic-extraction` / `65a5e44f` | Passed | Pre-PR41: 3,136 Rust files, three rendered templates and all-target checks |
| Release lane `contract` / `c46399c6` | Passed with declared crate-boundary deviations | Pre-PR41: ten architecture checks, not complete target-architecture conformance |
| `pipeline:bench -- --bench capture_export` / `3855f844` | 185 canonical rows; 60 cohort members; authoritative XLSX/CSV/JSONL readback passed | Captured fixture corpus, not national coverage |
| Actual `store_smoke` | Valid athlete persisted; empty name returned `EmptyName`, exit 1 | Constructor/store behavior |
| Actual offline CLI without `--store` | Exit 1; isolated working directory remained empty | No implicit store opened |
| Real Moon check with `MOON_RUST_BUILD_JOBS=２` / `6b28de2e` | Wrapper exit 2; no compiler invocation | Expected invalid-input refusal; Moon reports exit 1 |

The first PR41 integration run passed 140 focused cases but hung in an existing snapshot fixture:
it held the append guard and called the newly locking public writer. The fixture now uses the
already-locked writer and owns the guard inside its scoped closure, preserving the reader boundary
and releasing it on errors. The case passed in 2.143 seconds; the 141-case rerun completed.

The distinct-ID combined-event counterexample failed in both arrival orders (`cf8a7c86`), then
passed after policy-20 context quarantine. The captured export estimate was 62.202 ms, with no
statistically detected change against the immediately preceding iteration. This does not reverse
the earlier 8.0856% regression against an older baseline or establish a performance improvement.

### Release blockers retained

Earlier release `a02e3509` timed out at 3,600 seconds during performance checks; audit passed,
while source and size checks failed. Those source failures were subsequently repaired and their
focused lanes passed above. The full release gate has not passed; focused passes do not replace it.

Earlier native durability `6e37d364`: 12 passed, six failed out of 18 (02/03/06/07/11/12).
Host scenarios encountered the live endpoint's valid VT origin lock. Scenario 03 booted and
recovered work but failed orderly guest drain (`qualification.service`: `exit-code`); scenario 12
had no successful producer to reuse. New pre-teardown diagnostics are implemented but unexecuted.
Retain `var/durability-scratch/run-TbiWLWFY`; never bypass locks, stop the live owner or reset its store.

Read-only admin inspection did not establish the paused `open_work` cause. `refused_sources`
counts planned refusals, not archive-capacity failures; team, result and contact gates are distinct.
`iilh`, `6fhk` and the owning feature beads remain open. No reusable-production, selected-profile,
national, full fault-catalog, global-resource or final-spec acceptance is certified.

### R5/R6 focused evidence repairs — 2026-10-14

Base `b4b6a485`. R5: normalized alias readback now checks `input_generation`
against frozen lineage, including a copied bundle whose manifest hashes were
recaptured after tampering. R6: the existing reducer retains sorted, deduplicated
supporting source IDs through a nonserialized accessor. Exported representative
IDs/counts and winner ordering remain unchanged. Repeated IDs with equal original
marks and different adjusted values retain the best converted value deterministically;
contested contexts discard supporting IDs but retain conflict marks.

Additional reference inventory is pre-admitted before reduction using the existing
256-MiB logical grouping budget, charging two class references per performance at
128 bytes plus ID length each. This is conservative accounting, including excluded
and replayed inputs, and may refuse large historical/current input sets. It is not
an allocator bound, measured RSS, or qualification of the historical 64-GB problem.

Exact cloud checks through Moon, with `CI` unset and `MOON_RUST_BUILD_JOBS=1`:
- `tools/moon-local run pipeline:fmt-write`: PASS, 7.447 seconds.
- Initial normalized/tie run: 56 PASS, 1 FAIL, 711 skipped; the copied-manifest test
  incorrectly used create-new while its copied manifest existed. Fixed only that
  temporary-fixture setup; original-bundle preservation assertions remain.
- Added duplicate-conversion regression before the correction: 0 PASS, 1 FAIL,
  768 skipped (Nextest `62ff154a-6b71-4387-ac3c-a25cad513fb4`).
- `tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-report
  -E 'test(workbook::publication::verify_sidecars::normalized::) |
  test(bests::tests::ties::) | test(bests::tests::adjustment::) |
  test(bests::reduce::support::)' --no-capture --no-tests fail`: PASS, 70 tests,
  0 failures, 701 skipped, 35.420 seconds test time; Nextest
  `d035d2fd-367e-42cf-8c25-45320daf93e8`.
- Unchanged base all-target/all-feature `pipeline:check`: PASS, 4m59.164s.
  Changed-tree all-target check: PENDING. Strict lint/full release, representative
  memory, live acquisition and national census: NOT RUN for this change.

Changed-surface independent review caught and verified the duplicate-ID ordering
repair and checked the logical budget, conflict behavior and serialization boundary.
Functional-core, DDD, Holzman boundedness and adversarial-test review applied; no
async/lock boundary changed. Complete skill/release qualification is not claimed.
R5 acceptance mapping: normalized/athletes.rs plus tests/generation.rs. R6 mapping:
bests/reduce.rs, reduce/{reports,support}.rs, selection.rs and supporting/adjustment
regressions. R5 still needs complete metadata and broader positive fixtures. R6's
context-based duplicate-count policy needs identity justification; normalized
provenance still enumerates exported tied IDs rather than every supporting source ID.
Neither review item is fully closed by these focused repairs.

### Focused R7/R8 routing and ingress package — 2026-10-14

Prepared on `87178d3a` from the ordered Rust/Cargo deltas `95b27801`, `87e07200`,
`1d7a46cd`, `201b285a`, `4e54e825`, `9b266252`, `728bb79b`. The focused report
package is a prerequisite: existing `388bb367`/`a041adcf` test-only follow-ups
retain the artifact6 / 22-artifact / nine-normalized-family assertions. No report,
identity, acquisition-snapshot, resource-guard or shutdown implementation is replaced.
New standalone/national commands use the existing managed publication path;
`LegacySeparateV1` remains explicit historical replay compatibility, with old wire
omission semantics retained. Observer failures preserve the original invocation;
only validated completed Final output permits the existing bounded drive continuation.

Prior cloud results below belong to their recorded source trees, not this rebase:

| Surface / source | Passed / failed / skipped | Retained Nextest run |
|---|---|---|
| Managed routing, `87e07200` | 126 / 0 / 1033 | `75c67ac5-0ae6-459e-afa8-d6fc3df6ba62` |
| Offline/cutoff/roster integration, `87e07200` | 16 / 0 / 0 | `d4fe605c-8024-4cf0-85bd-0afb6676ed2c` |
| Attachment/resume, `1d7a46cd` | 28 / 0 / 1142 | `975b45d7-351d-419c-a4b0-4a4e0af8d1f4` |
| Managed/replay selector, `201b285a` | 75 / 0 / 1120 | `2f6311cd-d6a2-4c5f-9a0b-2114c5d07f86` |
| Bounded ingress, `4e54e825`, 23:13 UTC | 27 / 0 / 1505 | Exit 0; 1.291s tests |
| Drive ownership, `9b266252` | 110 / 0 / 1098 | `4a35badd-d7dd-40de-a2b9-d01d1576207a` |
| HTTP/2 retry ownership, `728bb79b` | 2 / 0 / 1013 | `3335e2f7-f442-4e5f-9de0-3f9b9d3eca87` |

The HTTP/2 REFUSED_STREAM fixture observed three pinned-default transmissions versus
one with explicit no-retry policy; it does not establish duplicate provider effects.
Body admission is bounded, while earlier header/network allocations and global peak
RSS remain unqualified. Known response status/invocation and original transport causes
are retained; H2 header overflow is not guessed from opaque transport failures.
This package has patch/static checks only. Current-main compilation, focused tests,
all-target/source lint and release gates are PENDING. Genuine historical Restate
endpoint replay, native fault windows, provider E2E and representative peak memory
remain NOT QUALIFIED. Incoming R9–R15 evidence below is preserved unchanged.

### Subsequent R9–R15 working-tree evidence

These exercised repairs follow checkpoint `8005d33f`; they are not a full release certificate.

| Surface | Executed evidence | Boundary |
|---|---|---|
| R9 comparison ownership | `tests-scoped` / `2d0135aa`: 851 focused passes; `bench -- --bench capture_export -- --test` / `2b80b844`: 185 canonical rows, 60 cohort members, authoritative XLSX/CSV/JSONL readback | Captured corpus; no speed or national-coverage claim |
| R10 fallible construction | Actual `store_smoke` seed; empty name exit 1 / `EmptyName`; independent reopen/readback / `0acad89c` verified the exact athlete, school and 2027 claim | Isolated synthetic fixture, not population intake |
| R14 browser boundary | Actual Chromium / `5087be2e`: normal 200, 33,554,433-byte refusal, bare-403 recovery, 429/Retry-After recovery, explicit challenge; clean manager drain | Loopback transport smoke, not source entitlement |
| R14 unavailable accounting | Actual masked-proc smoke first exposed exit 0 after resource drain; two CLI-policy regressions / `42ad8a63` passed; actual repeat exited 1 with accepted=completed=3 | Engine accounting failure, not whole-group qualification |
| R13 parser/registry and R14 cgroup | `tests-scoped` / `ef0459d9`: 62 passes, including real 755-row official text, 1-MiB admitted registry payload, pinned reads, corruption refusal and cold backup/restore | Official PDF has 40 physical pages, 20 blank trailing pages; no UP/sport inference |
| R14 strict CLI | Build / `b4a699f6`; actual unbounded-group refusal exited 1 before creating the store; separate finite 1-GiB/zero-swap systemd unit admitted and exited 0 after SIGTERM, accepted=completed=3 | Isolated endpoint only; Restate/browser/global disk envelope unqualified |
| R15 runtime/wire ownership | `tests-scoped` / `9085a8a0`: 14 producer/consumer and policy cases passed after removing three incidental tests | ADR-007 preserves the intentional effect-boundary representations |
| R14 measurement integrity | Failing-before `14d85808`: two failures; passing-after `b27340b9`: three passes; final integrated parser/registry/guard run `2240ea2c`: 65 passes | Observed peak regression, peak below current usage and changed hard limits fail closed |
| Rebased checkpoint `5bb37152` | `f6e30786`: 5,901 passes, four skipped; check `c4acb64b`, lint `d105b28d`, fmt `570eee66` passed | Three existing forced argument-count advisories; not a full release pass |
| User snapshot PR `0ce5f405` | Check `fae3c990`, fmt `790f93b8` and ten snapshot/budget cases `0751b399` passed | Focused follow-on integration, not another full-workspace/native certificate |
| Native reboot/midnight | `3245a342`: scenarios 03 and 12 passed, zero failed/skipped; candidate build `6cbc8b8a` at `5bb37152` | Original reserved invocation recovered; two FULL200 HTTPS acquisitions bracketed guest midnight. Not full national/all HTTP subphases or newer-main qualification |
| Actual enrollment/registry API | `run-scoped` / `3500a5d2`: official-text cells/pages, pinned regional reads, unchanged evidence generations, cold backup/restore and exact repeat readback passed | Isolated public-text/synthetic-configuration smoke; harness/output fingerprints retained under `var/review-r9-r15-20261014/` |
| Actual CLI help | Documented command first failed with exit 1 (`916c0065`); after repair, locked-store regression `6393cc0d` and actual `--help` command `5648cf2a` passed | Help exits 0 before runtime/store acquisition; malformed flags remain nonzero |

The real official-text case failed with `MissingHeader` before repair (`bca4278b`): the PDF contains
blank trailing physical pages. Admission still counts every physical page before trailing layout
normalization; malformed interior pages remain errors. The original capture is retained under
`crates/census-crawl/tests/fixtures/mhsaa_enrollment/`, SHA-256
`9a68ae75835a3e69b7a8abc68cef77e7dadb716d8ae9aceaf2a90bffc596300a`.
It is enrollment research, not qualified pipeline capture or complete five-state discovery.

Strict-mode smoke evidence: `var/r14-memory-mode-C9wsAWJm/finite-smoke.json`.
Browser and constructor harness/readback evidence: `var/review-r9-r15-20261014/`.
Native evidence: `var/review-native-RDHKbzIf/native-vm-session-g5Zdspyr/native-vm`; preserved machine
`/home/lewis/.cache/cen-native-vm/run-y2AWu8g7/machine/qualification-certificates.json`.
R9, R10 and R15 are closed with scoped evidence. R11 semantic witnesses, R12 reusable execution,
R13 qualified denominators, R14 whole-workload resources, latest-build/full-fault coverage and the
full release gate remain open. The R11 draft is incomplete and unapplied. No live installation changed.

### Focused R4/R5/R6 publication packaging — 2026-10-14 23:17 UTC

This package takes the committed report artifacts and three event-policy model files
from cloud source `a041adcf`, relative to published base `9d4e03a0`. It retains one
canonical workbook path. Four existing service best-results writers only gain the
explicit selection-policy argument; managed-publication routing commit `95b27801`,
its wire/workflow changes, and pending source edits are outside this package.

Observed source-tree evidence: with CI unset, the command
`MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-report --lib -E 'test(bests::) | test(export::dataset::frozen::) | (test(workbook::publication::) & !test(workbook::publication::crash_tests::))' --no-capture --no-tests fail`
passed 601 tests, 0 failed, 276 skipped; Nextest
`7af28a54-b435-4ca4-a298-3082b84f352f`, 196.603s tests / 5m10.460s Moon total,
exit 0. Retained log: `/tmp/report-transition-20261014-2304.log`, SHA256
`ad096d336b8d00efbf9792ba05cc6b398b126f37b98f2d757561f7e7ea1a8cec`.
This is evidence for that source tree, not execution of this focused package.

R4 now retains bounded combined-event exclusion reasons; R5 includes normalized
row-count commitments and populated independent readback fixtures; R6 carries all
supporting provenance and explicit policy21 result-group inventories in artifact6,
with historical policy/artifact readers retained. R4/R5/R6 remain acceptance-open:
the deterministic policy21 equal-value provenance tie repair is not included or
qualified here. Focused-tree tests, that tie repair, changed-tree all-target check,
strict source lint/full release, and native Mac validation are NOT RUN. Actual
publication crash-window cases, representative peak memory, and final runtime
acceptance remain pending. Earlier Mac cleanup-harness 10-pass evidence covered its
earlier candidate only and does not qualify actual publication crash recovery.
The full specification is unchanged (SHA256
`a3b687cc06952dbceaeda2f21300e4769acb96130570fc1174ff70210a09f11e`).

Policy21 deterministic provenance follow-up: cloud commit `94f89039` passed 29 focused
identity/result-group tests, 0 failed, 853 skipped; Nextest
`314538d0-bac7-4eb4-b28b-22df39f4c657`, 2.059s tests / 2m14.621s task.
Command: `env -u CI CARGO_INCREMENTAL=0 MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-report --lib -E 'test(bests::reduce::identity::) | test(bests::tests::ties::result_groups::)' --no-capture --no-tests fail`.
Same-ID equal-value provenance is stable across input order; historical policies retain
their previous behavior. Full-row admission is 1 MiB per row and 256 MiB per stage plus
reference accounting. This is no physical-performance count or peak-memory qualification.
Focused-tree all-target/lint and combined 606-case report rerun remain pending.

Focused-package rebase onto `87178d3a`: the incoming centralized comparison implementation
from `5bb37152` remains the sole owner of comparison keys, measures, labels and mark ties.
The report result-group and exclusion changes consume those domain APIs; report-local
comparison modules are not restored. Both prior review-history sections above are retained.
Formatting and patch checks do not qualify this rebased tree: compilation, focused report
tests, all-target/source lint, native crash windows and release acceptance remain pending.

### R2 Athletic.net result-capture timestamps, October 15 00:37 UTC

`athleticnet/meet/collect{.rs,/walk.rs}` now commits each interpreted meet's school
and athlete observations with its results capture's actual `fetched_at`. Canonical
meet evidence retains the separately fetched meet timestamp. Partial mapped rows
commit under their own capture without completion markers, and the resolved-school
cache clears after commit so subsequent captures preserve school provenance.
No refresh policy, shared schema or completion-journal format changed.

PASSED: five offline acquisition tests, zero failed, 1,530 skipped; 0.727s tests /
129.967s Moon, exit 0. Three new regressions cover separate capture times for shared
subjects, partial rows and partial-then-success; cached replay now checks raw source
observations. Initial test build FAILED at link with signal 7 while disk was full;
no tests ran. Removed only reproducible generated test artifacts and consolidated
byte-identical clean research checkout copies through reversible sparse checkout;
the identical command then passed. Historical source files and Git objects remain.

```sh
env -u CI CARGO_INCREMENTAL=0 MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-crawl --lib -E 'test(athleticnet::meet::tests::acquisition::)' --no-capture --no-tests fail
```

Static review and formatting PASSED. Current all-target/lint and native Mac gates
PENDING. Source coverage, live acquisition and representative peak RSS NOT RUN.

### R2 immutable archive preservation on cache hits, October 15 00:45 UTC

`net/execute/fetch.rs` now preserves verified ordinary cache hits through the existing
canonical replay/archive helper before returning or counting a hit. `net/cache/archive.rs`
selects exact expected metadata before encoding/loading, rereads once under existing
archive/cache fences, and preserves original acquisition time and historical metadata.
Changed/damaged pairs remain misses; encoding and unsupported-envelope errors remain
typed refusals. Refresh, adopted occurrence and conditional-304 routes retain their
existing meaning. The ordinary caller releases its prior body before the fenced reread.

RED: eight new archive regressions ran, three passed and five failed before repair
(Nextest `1b0b94f4-35e1-4cc0-9ee2-26a1574e1509`). Two incorrectly named filter attempts
FAILED with zero selected tests; an intermediate broad filter ran seven, six passed and
one failed. These are not pass evidence. GREEN: 84 cache/fetch/adopted/304 regressions
passed, zero failed, 1,459 skipped; Nextest `44049a1e-1c17-4b84-9762-c44e88f66269`,
7.570s tests / 145.190s Moon, exit 0.

```sh
env -u CI CARGO_INCREMENTAL=0 MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-crawl --lib -E 'test(net::execute::fetch::tests::) | test(net::cache::) | test(actual_304_) | test(adopted)' --no-capture --no-tests fail
```

Formatting/static correctness review PASSED. Synchronous archive locks/hash/fsync still
need owned blocking composition for uncovered async callers; no latency or cancellation
claim is made. Full changed-tree format/check/lint and Mac validation PENDING. Live
acquisition and representative peak-RSS qualification NOT RUN.
