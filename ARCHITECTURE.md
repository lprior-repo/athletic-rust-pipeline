# ARCHITECTURE.md

The binding architecture and standards for this repository. §-numbers are stable: other documents and
commit messages reference them, so edit a section's content without renumbering it.

## 1. Mission

Build a fresh, source-discovered recruiting census of U.S. Class-of-2027 high-school Track & Field
and Cross Country athletes across the 48 contiguous states plus D.C. Discover the population through qualified
public sources, reconcile identities, collect athletic histories, calculate comparable PRs, retain
contested events and source profiles, resolve time-scoped school affiliation and public coaching
contacts, and generate recruiter workbooks with auditable evidence. No workbook supplies the
population; admissions matching and original-input-row reconciliation are not this product.

The run may take many hours or days. It must be durable, resumable, restart-safe, rate-aware,
bounded, idempotent, observable, decomposable into independent work, ergonomic for coding agents, and
capable of retaining partial progress indefinitely. A crash, reboot, browser failure, source outage,
429, model outage, parser defect, or machine restart must not force the census to start over.

**The architectural rule.** Restate owns what work must happen. Fjall owns what was observed.
Deterministic Rust owns what the evidence means. AI may advise only where deterministic evidence is
genuinely ambiguous. Excel is a projection of that truth, never the truth itself.

**The identity rule.** No canonical identity decision may destroy source identity or provenance
required to reverse that decision later.

**The fresh-run rule.** ADR-013 requires a new store and unused durable run namespace/revision.
Preserve historical stores and artifacts; do not import their observations, identities, receipts,
coverage or seals into the new census. Reuse maintained code, fixtures and qualified source entry
points, then acquire fresh evidence. Reuse that run's captures and completed work during recovery.

**The root package is gone.** The acquisition-side root crate, `athletic-rust-pipeline`, was deleted
on 2026-09-23 once the census crates owned its work: this workspace now holds only the nine members of
§4 plus the package-less `fixtures/` directory, and no root `src/`, `tests/` or `benches/` exists. The
root manifest's header records the same fact, and §4's `acq-*` note says what became of that pipeline
— documents that cite this section for the deletion mean this paragraph.

## 2. Scope

- **Geography (§2)**: the census scope is the **48 contiguous states plus D.C. — 49 jurisdictions**;
  Alaska, Hawaii and the territories are outside it, per [ADR-009](docs/adr/README.md). ADR-013's
  51-jurisdiction target was withdrawn by owner direction on 2026-09-27. `UsJurisdiction::ALL`
  models all 51 states and `UsJurisdiction::CENSUS_SCOPE` is the 49 the run and every coverage or
  seal denominator count; a jurisdiction in `ALL` but outside the scope parses and is refused,
  never silently counted. Adding one requires an explicit domain decision.
- **Cohort (§3)**: `GraduationYear(2027)` is the durable target, never "junior". Grade is
  time-scoped evidence (`GradeObservation { grade, academic_year, source }`) — `2025-2026 → Grade 11`
  and `2026-2027 → Grade 12` both support the same canonical cohort. No source-specific query
  parameter (`JR`, `11`, `SR`, `12`) becomes a cohort fact without season-aware interpretation.
- **Population (§4)**: every source-verifiable Class-of-2027 athlete in high-school TF or XC
  discovered through the qualified source graph — boys and girls, cross country, indoor and outdoor
  track, all legitimate events including relays where membership is evidenced. The workbook is a
  projection, not the population contract.
- **Inputs and outputs**: public source discovery is the population contract. Published result
  spreadsheets may be captured as evidence, like PDFs; an operator workbook or recruit list may not
  seed the census. Excel is output only. Subset runs are labeled qualification runs.

## 3. Pipeline

```text
51-jurisdiction run + qualified public source graph
  -> school/program/season discovery obligations
  -> origin-admitted durable Restate acquisition
  -> immutable captures + provider-owned observations in Fjall
  -> validated claims + candidate identities + contradiction checks
  -> deterministic Rust adjudication
       ambiguous cases -> both local Qwen lanes -> evidence-bound advice -> Rust
  -> reversible identity/cohort/affiliation decisions + retained gaps
  -> one snapshot-bound projection
  -> independently verified atomic workbook/audit generation
  -> evidence-backed census seal
```

Athletic.net is one source. MileSplit is one source. State associations, timing companies, meet
systems and school directories are sources. The canonical census exists independently of all of them.

## 4. Crate layout (§17)

```text
crates/
    census-domain/     pure types and rules; no tokio, fjall, reqwest, chromiumoxide, restate, xlsx, llama
    census-store/      Fjall keyspaces (entities, journal, meta, receipts), snapshots, migration,
                       backup/restore
    census-crawl/      source adapters, fetchers, per-origin admission, browser supervisor
    census-reconcile/  normalisation, deterministic scoring, conflict detection
    census-review/     local Qwen identity-review lane
    census-report/     coverage, bests, PR projection, workbook export
    athleticnet-browser/ persistent Chromium transport: tab pool, CDP capture, challenge detection, 429 cooldown
    census-service/    CLI + Restate workflows + bootstrap/supervision
xtask/                 the agent-facing verbs
```

Paths in older documents that read `crates/census-service/src/store/`, `.../report/`, `.../net/` or
`.../workbook/` predate this split: `store/` is `census-store` (`lib.rs`, `keys.rs`, `read/`,
`write.rs`, `legacy/`, `backup/`), `report/` and `workbook/` are `census-report`, and `net/` is
`census-crawl`. Symbol names remain the authority; the module paths moved, the schema did not.

### The `acq-*` family

§17 also names `acq-*` crates for the acquisition pipeline. Those crates do not exist: the pipeline's
root package (`athletic-rust-pipeline`) was deleted once the census crates owned its work, and what
survived of it is library code rather than a package — `crates/athleticnet-browser` carries the
persistent Chromium transport, and the census crates carry the rest. The `acq-*` split is therefore
superseded rather than unstarted.

`census-domain` operates entirely on explicit values. Parse external data once at the boundary:
external shapes become `Raw*` types, validation produces domain enums (for example
`RawGraduationValue` → `GraduationEvidence`), and no `String` stands in where the domain knows
something more precise (`CanonicalAthleteId`, `SourceAthleteId`, `GraduationYear`, `AcademicYear`,
`RawMark`, `NormalizedMark`, …).

## 5. Workflow backbone (§7-§9)

The catalog below describes the current implementation, not completion of ADR-013. The target
integrates discovery, acquisition, reconciliation, review, gap resolution and export under one
durable application path. Batch-only stages must join it; do not create a parallel engine.

```text
NationalCensus (implemented workflow: fans out JurisdictionCensus, folds NationalFailure rows, merges once via Consolidate)
  +-- JurisdictionCensus(<each run jurisdiction>)   (currently 49; ADR-013 migration pending; teams / rosters / meets / results)
  |     +-- meets stage routes via Ingest (<slug>_<state>, ISO-week window); teams / rosters / results write the store directly inside ctx.run
  |     +-- SourceDiscovery / SchoolDiscovery / MeetDiscovery / AthleteDiscovery,
  |         ResultAcquisition / CoachDiscovery / Reconciliation / GapAnalysis — NOT Restate workflows;
  |         offline batch (collect, provider, index, review, import-coaches)
  +-- MeetWorkflow / AthleteWorkflow / SchoolWorkflow / CoachWorkflow / IdentityReviewWorkflow — NOT Restate workflows; offline batch
  +-- Consolidate / Report / Bests / Workbook (implemented workflows: journaled blocking jobs) replace
      NationalReconciliation / NationalCoverageAudit / WorkbookExport / ArtifactVerification as named workflows
  +-- Census (implemented service: status, open_work, seal) + Sweep / Ingest (implemented) + BrowserSession (implemented object, bound only when --browser-profile serves a lane)
```

Implemented means bound in `restate_services::build_endpoint` under the struct name as the wire
name: `Census`, `Consolidate`, `Report`, `Bests`, `Workbook`, `Ingest`, `Sweep`,
`JurisdictionCensus`, `NationalCensus`, `BrowserSession` (`restate_services/mod.rs`). Anything not
in that list — Discovery, ResultAcquisition, CoachDiscovery, Reconciliation, GapAnalysis as
workflows; Meet/Athlete/School/Coach/IdentityReview workflows; NationalReconciliation,
NationalCoverageAudit, WorkbookExport, ArtifactVerification as named workflows — does not exist as
a Restate workflow. Index, review cases, §47 gaps and coach imports are offline batch over the
store the service holds; they are durable work, not durable invocations. See
`RESTATE_WORKFLOWS.md` §2.2 for the service table and `docs/OPERATIONS.md` for the batch chain,
which does not route yet.

Live `teams`, `meets` and `collect` with an ingress origin drive the full
`JurisdictionCensus/run` for each named state — every stage the object still owes — not one stage
in isolation (`cli/gather.rs` via `cli/live.rs::drive_states` and `jurisdiction_request`). The
workflow fetcher carries `authorized_hosts` (wire default empty); the plan still refuses
browser-transport sources by name when no lane is configured (`plan::BrowserLaneState::of`).

Identities (§8) are deterministic and stable across retries:

```text
jurisdiction:{state}:{season}:{revision}      source-sweep:{source}:{state}:{season}:{revision}
meet:{source}:{source_meet_id}:{revision}     athlete:{source}:{source_athlete_id}:{revision}
school:{source}:{source_school_id}:{revision} review:{evidence_digest}:{policy_revision}
```

**§9 retry model**: One retry owner (Restate), max three attempts per failed external operation; transport performs one. Never stack retries across layers. Heavy jobs (`Consolidate`, `Report`, `Bests`, `Workbook`, `Sweep` prune/report, `Census` seal) run as journaled blocking jobs inside `ctx.run` with `max_attempts(1)`; the invocation-level `max_attempts = 3` is the only retry budget. Exhaustion parks the invocation with `on_max_attempts = pause`, except `JurisdictionCensus` which uses `on_max_attempts = kill` so `NationalCensus` folds the failure as a `NationalFailure` row and continues. Store-backed services declare 1h inactivity + 1h abort via `limits::census_service()`; `BrowserSession` keeps the SDK defaults. See `docs/adr/ADR-002-restate-owns-retries.md` for the full retry policy contract, exhaustion semantics, and `FailureCode`/`AccessBlockKind`/`FetchError` error taxonomy.

## 6. Admission and browser state (§10, §26-§28)

`SourceAdmissionPolicy` is per origin: `maximum_in_flight`, `target_rate`, `burst`, `retry_policy`.
Athletic.net shares one parent budget across rankings, profiles, Bio/history, meets, results and
browser tabs; launching more workflows must not multiply traffic. Physical requests visible to the
origin are the measurement. Tabs are execution lanes, not rate-limit budgets.

```text
Ready --429--> Cooldown --successful evidence--> Ready
Ready --challenge--> HumanRequired --manual resolution--> Ready
```

A challenge closes new admission; issued requests drain. No CAPTCHA automation, no browser-identity
evasion, no direct-HTTP fallback intended to circumvent Cloudflare.

## 7. Store (§29-§31)

Fjall is the system of record; PostgreSQL is not part of this census. Target logical collections
(not one physical keyspace per collection; implemented storage is cataloged in `census-store`):

```text
canonical:      athletes schools coaches meets performances
source:         source_athletes source_schools source_meets source_results
relationships:  athlete_performances athlete_schools school_athletes school_coaches meet_performances
evidence:       source_observations graduation_evidence document_receipts raw_documents
reconciliation: identity_candidates review_cases canonical_links conflicts
coverage:       source_coverage jurisdiction_coverage collection_snapshots
```

Composite keys are deterministic and sortable (`SourceSystem | SourceAthleteId`,
`AthleteId | Date | PerformanceId`, `MeetId | PerformanceId`, `SchoolId | Sport | Season | CoachId`,
`Jurisdiction | SourceSystem | SourceObjectId`). No index is created merely because Excel might
filter on a field.

## 8. Identity review (§32-§34)

Candidate discovery is not acceptance. Deterministic normalization and hard-contradiction checks
precede adjudication; only genuinely ambiguous cases receive advice from both approved local Qwen
servers. Models return `SamePerson | DifferentPerson | InsufficientEvidence`, bound to retained
evidence and policy/model revisions. Rust alone accepts a link. Agreement or a score cannot replace
corroboration or override a hard contradiction; unresolved uncertainty remains `REVIEW`.
See `docs/adr/ADR-005-ai-cannot-override-contradictions.md`.

## 9. Target data and publication contracts

Raw observations, validated claims, candidates and accepted links are separate constrained types.
Cohort membership is not person identity. Provider IDs retain their true ownership scope; missing
IDs cannot become name-derived canonical people. Grade, affiliation, participation and coaching
appointment are time-scoped facts. Syndicated sources are not independent corroboration.

Archive captured bytes durably before publishing references. A URL, body hash or parse count alone
is not an archive. Commit observations, source progress and effect receipts atomically; acknowledge
only after durability. Partial parses retain valid rows and rejected locators without closing the
unfinished obligation. Retry replay and a legitimate new capture have different effect semantics.

Use one snapshot and one projection for workbook and audit outputs. Preserve historical result
affiliation, real rounds/heats and comparable PR conditions. Bind public coach mailboxes to exact
institution/role evidence. Verify exact record IDs, PR winners and coverage, then atomically publish
the whole generation; a failed or stale exporter cannot replace a valid bundle.

Detailed contracts, dependency order, F01–F15 and all 17 native fault scenarios live in
[`docs/NATIONAL-CENSUS-PLAN.md`](docs/NATIONAL-CENSUS-PLAN.md). They remain required until exercised.

## 10. Engineering standards (§37-§43)

```text
no unsafe; no production recursion; no careless unwrap/expect; no panic from input
no ignored Result; no silent fallback; no unchecked boundary conversion
no unchecked arithmetic where correctness matters
no unbounded loop; no unbounded queue; no unbounded spawn
```

Every loop over external data has a bound or a demonstrably finite iterator. Production functions
stay within 60 logical lines (hot paths 25); long orchestrations decompose into named stages. Error
types are `thiserror` enums inside production crates; `anyhow` lives only at CLI/composition edges.
Async outcomes are not flattened (§43) and a panic never becomes a generic source error. Every
spawned task belongs to a supervisor whose shutdown accounts for accepted, completed, cancelled,
timed-out, aborted, panicked and remaining work — success requires `remaining = 0` or a persisted
unfinished Restate workflow (§42). Observability is structured `tracing` with the standard field set
(§44). Per-source metrics (§45) are recorded and the efficiency metric is
**verified useful records per physical request**.

## 11. Quality gates (§55-§58)

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Fuzz-feature build failures are fixed or assigned an explicit fuzzing lane; they never disappear
silently from CI. `#![forbid(unsafe_code)]` and `#![deny(unused_must_use)]` are workspace policy, and
the quality ratchet records remaining historical debt without permitting increases. Benchmarks (§57)
cover parsing, ingestion, merge, scoring, PR calculation, Fjall batch writes and prefix scans, report
and XLSX generation, and AI case preparation — no optimisation without benchmark evidence. The
verification program (§58) requires unit, property, fuzzing, failure-injection, replay, concurrency,
Restate recovery, browser record/replay, Fjall backup/restore, golden-corpus, workbook-verification,
dependency/license audit, async review, security review and adversarial identity review; architecture
documents are not test evidence.

## 12. Acceptance (§70)

The national census may be sealed only when all 51 required jurisdictions have terminal declared
discovery/acquisition obligations; every potential Class-of-2027 athlete has a terminal cohort
decision; every identity candidate has a terminal adjudication; and every conflict, gap and
retry-exhausted operation is retained. Unresolved work and unverified publication block sealing;
terminal findings may remain under ADR-011. All successful evidence must be durable, performance
and PR calculations reproducible, and workbook records, coverage and metrics reconciled against
the same Fjall snapshot. A seal certifies the declared run, not exhaustive knowledge of the Internet.

## 13. Current state and delivery order

The nine-crate split exists; `docs/migration/module-map.md` records its historical cut. Do not
restart it or resurrect the deleted root pipeline. `docs/architecture.md` records implementation
details; §5 names the current durable handlers and batch-only gaps.

ADR-013 changes the mission, not the code by declaration. First migrate the 49-jurisdiction scope
and bind a fresh store/run; establish source/program obligations; prove one real discovery-to-capture-
to-census-to-output path; then expand sources, history and contacts across all 51. Integrate the
same durable application, shared projection and atomic publication before the national acceptance
run. Execute all F01–F15 and 17 fault obligations, reconcile the final generation and seal, then
Main commits verified delivery. Exact stage exits are in `docs/NATIONAL-CENSUS-PLAN.md`.

Historical `var/midwest-census` data, old exports and prior seals are preserved, not imported or
counted as completed work for the fresh national run. Documentation is not runtime evidence.
