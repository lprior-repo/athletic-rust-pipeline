# ARCHITECTURE.md

Binding product architecture and engineering standards. Numbered sections and the historical
§ references remain stable. Requirements below are not claims that the implementation passes them.
The [decision register](docs/adr/README.md) records rationale and supersession;
[the delivery plan](docs/NATIONAL-CENSUS-PLAN.md) owns unfinished work and acceptance.

## 1. Mission

Build a fresh, public-source-discovered Class-of-2027 high-school Track & Field / Cross Country
recruiting census. Discover athletes, reconcile identities, retain athletic histories and
source profiles, calculate comparable best marks, resolve time-scoped affiliations and public
coaching contacts, and publish recruiter workbooks backed by durable evidence.

Restate owns durable work. Fjall owns observations and decisions. Deterministic Rust interprets
and adjudicates evidence. Local models advise only on genuine ambiguity. Excel is a projection,
not the system of record or an input population.

A multi-day run must retain completed work across crashes, reboots, browser/model outages, source
failures and operator interruption. Partial progress is durable; a failure does not justify
restarting acquisition or manufacturing a new logical job.

[ADR-013](docs/adr/ADR-013-fresh-national-source-census.md) requires a fresh store bound to an unused
run namespace/revision. Preserve historical stores and artifacts without importing their population,
receipts or seals. Reuse maintained code, fixtures and qualified entry points; acquire this run's
source evidence afresh, then reuse its immutable captures and completed effects during recovery.

The former root acquisition package was removed on 2026-09-23. The virtual workspace below is the
only product implementation; do not recreate the root pipeline or a parallel acquisition engine.

## 2. Scope

- **Geography (§2):** 48 contiguous states plus D.C., exactly 49 jurisdictions under
  [ADR-009](docs/adr/ADR-009-census-run-scope.md). `UsJurisdiction::ALL` models 50 states plus D.C.;
  `UsJurisdiction::CENSUS_SCOPE` defines the run denominator. Alaska and Hawaii parse but are
  excluded; territories are not modeled. Qualification subsets are not a national delivery.
- **Cohort (§3):** graduation year 2027, not a timeless “junior” label. Grade and academic year are
  evidence; [§9](#9-target-data-and-publication-contracts) owns their required interpretation.
- **Population (§4):** source-verifiable boys' and girls' high-school XC, indoor and outdoor TF,
  including legitimate events and evidenced relay membership. Discovery includes programs that
  have not yet yielded an accepted athlete.
- **Inputs:** qualified public sources and linked HTML, structured data, PDFs and public result
  spreadsheets. No admissions workbook, operator recruit list or previous export seeds the census.
- **Outputs:** accepted identities, cohort and participation evidence, source profiles, verified
  marks, public coaching contacts, coverage and retained review reasons. Unknown denominators,
  unfinished discovery and incomplete histories remain visible; source failure is not absence.

## 3. Pipeline

```text
49-jurisdiction run + qualified public source graph
  -> school/program/season discovery obligations
  -> origin-admitted durable acquisition
  -> immutable captures + provider-owned observations
  -> validated claims + identity candidates + contradiction checks
  -> deterministic Rust adjudication
       ambiguous cases -> both local Qwen lanes -> evidence-bound advice -> Rust
  -> reversible identity/cohort/affiliation decisions + retained gaps
  -> shared snapshot-bound export dataset
  -> independently verified atomic workbook/audit generation
  -> evidence-backed census seal
```

Athletic.net, MileSplit, DirectAthletics, relevant TFRRS, AthleticLIVE, state associations, timing
providers, organizers and school/team sites are complementary sources, not independent products.
Relevant discovered links extend the bounded frontier. Shared rosters and meet results are acquired
once per capture/parse revision and reused across subjects; see [ADR-004](docs/adr/ADR-004-meet-first-ingestion.md).

## 4. Crate layout (§17)

| Crate | Owns | Boundary |
|---|---|---|
| `census-domain` | Pure constrained types, cohort/event/comparison rules | No async runtime, database, HTTP, browser, Restate, workbook or model clients |
| `census-store` | Fjall schema, transactions, receipts, snapshots, migration and backup | No source parsing or HTTP |
| `census-crawl` | Adapters, discovery, fetchers, admission and browser bridge | Produces observations; does not accept canonical identities |
| `athleticnet-browser` | Headed profile, CDP capture, tab pool, challenge/429 classification | One attempt; no challenge solving, spoofing, login handling or proxy rotation |
| `census-reconcile` | Deterministic normalization, identity scoring and contradiction checks | Does not call models |
| `census-review` | Approved local Qwen advice | Does not establish identity |
| `census-report` | Coverage, bests, shared export derivation and rendering | Does not mutate evidence |
| `census-service` | CLI, Restate workflows, composition and supervision | Does not bypass store durability |
| `xtask` | Developer commands and gate tooling | No census business logic |

Parse external representations once into validated domain values. Pure rules receive explicit
inputs; clocks, network, storage and spawning remain effect boundaries. Existing exceptions are
implementation work, not permission to create duplicate rules or mirrored public types.
[ADR-007](docs/adr/ADR-007-crate-boundaries.md) and [ADR-012](docs/adr/ADR-012-single-acquisition-plane.md)
record the completed crate cut. `fuzz/` is a separate cargo-fuzz workspace; `fixtures/` is shared data.

## 5. Workflow backbone (§7-§9)

The implemented handler/API catalog, logical keys and retry dispositions live in
[OPERATIONS.md](docs/OPERATIONS.md); [durable execution](docs/restate/durable-execution.md) is vendor
background, not the project's handler contract. Native, non-Docker Restate and its Rust SDK remain the
orchestrator. There is one application path; CLI and service surfaces must not implement competing
business workflows. Batch-only reconciliation, review and gap handling must join that path.

**Logical identity (§8):** distinguish run, source unit, physical attempt, capture, ingest effect,
review case and export generation. A retry keeps its logical identity. Reusing an effect ID with
different content is a typed mismatch. Increased execution budgets resume unfinished work; they do
not change the immutable meaning of an already completed workflow.

**Retry ownership (§9):** Restate alone owns at most three total automatic attempts per failed
external operation. HTTP, browser, adapter and model transports perform one attempt. No nested retry
loop or new workflow key resets the ceiling. Persist exhaustion and continue independent sources.
[ADR-002](docs/adr/ADR-002-restate-owns-retries.md) owns this decision.

A completed journaled step can replay its result. Wrapping a large blocking function in `ctx.run`
does not checkpoint its internal progress; resumable stages need explicit durable input and
completion boundaries. A queue is a bounded accelerator, never the authority for unfinished work.

## 6. Admission and browser state (§10, §26-§28)

Admission is per remote origin and counts physical requests. Share budgets across endpoints,
workflows and tabs, including related Athletic.net surfaces. Concurrency does not multiply a source
budget. Bound payloads, redirects, pages, tasks and queued work; revalidate redirect destinations
and protect local/private network boundaries.

Robots.txt is read for pacing only: a `Crawl-delay` published for `User-agent: *` paces that host,
while an absent, unreadable or non-200 robots.txt publishes no policy and never blocks a request. No
CAPTCHA, authentication or paywall circumvention, browser-identity spoofing, proxy evasion, cookie
extraction/replay, or direct-HTTP fallback intended to bypass a challenge. Athletic.net uses the
headed persistent-profile lane; the operator resolves challenges in that profile.
[OPERATIONS.md](docs/OPERATIONS.md#browser-lane) owns the runnable browser-lane procedures.

```text
Ready --429--> Cooldown --admitted retry--> Ready
Ready --challenge--> HumanRequired --operator resolution--> Ready
```

A challenge closes new admission and issued requests drain. A source stop (§69) is persisted while
other sources continue. Athlete personal contact data and inferred GPA are outside the product (§36).

## 7. Store (§29-§31)

Fjall is the sole evidence store under [ADR-001](docs/adr/ADR-001-fjall-primary-store.md).
[The store table registry](crates/census-store/src/table.rs) owns implemented logical tables;
[backup procedures](docs/FJALL_BACKUP.md) own physical storage and safe copying/recovery.
Do not infer a physical keyspace from a domain collection or an Excel column.

Preserve immutable observations and provenance needed to reverse any identity decision. Commit
observations, progress and effect receipts together; acknowledge only after durability. Persisted
schema changes require explicit versioned migration for affected stores, not silent reinterpretation.
A fresh run does not authorize deletion of historical evidence.

A live MVCC snapshot pins one read view, not a durable checkpoint that can be reopened from a bare
sequence number. Multi-table operations must use one captured view. Input-generation identity must
include store/run lineage and all semantically relevant revisions; writing derived output must not
invalidate its own input identity. Keep retention and garbage collection safe for active readers.

## 8. Identity review (§32-§34)

This section and [§9](#9-target-data-and-publication-contracts) own identity, cohort, affiliation,
source independence and comparison contracts;
[ADR-005](docs/adr/ADR-005-ai-cannot-override-contradictions.md) owns model authority.
Candidate retrieval is not acceptance. Name, school text, cohort, a score or model agreement alone
cannot establish identity. Provider ownership and hard contradictions survive every projection.

Resolve deterministic cases without AI. Only ambiguous cases receive independent advice from both
approved local Qwen servers (RTX 5090 and RTX 3090). Bind bounded structured packets and responses
to retained evidence, policy and model revisions. Preserve contradictory evidence; do not silently
truncate decisive facts. No cloud fallback or private admissions data.

Rust requires admissible corroboration and reviewer agreement for model-assisted acceptance. Failed
calls, malformed replies, disagreement or insufficient evidence remain `REVIEW`; agreement cannot
override a hard contradiction. Models do not invent marks, cohort evidence, affiliations or contacts.

## 9. Target data and publication contracts

This section defines required data semantics; [xtask/README.md](xtask/README.md#source-scaffolding)
defines adapter integration and [source reports](research/sources/) retain qualification evidence.
Every material claim must resolve to durable captured bytes
and a source locator. A URL, digest or parse count alone is not an archive. Partial parses retain
valid rows, rejected locators and the unfinished obligation.

All reports, workbook sheets and audit outputs use one immutable export dataset derived from one
input generation. Do not rebuild identity, cohort, contact, coverage or PR rules in writers. Keep
candidate statuses and accepted aliases distinct; athletes without performances remain in the
population, and unresolved joins remain explicit coverage outcomes. Core evidence scope is not
interchangeable with geographic scope or cohort eligibility.

Preserve historical result affiliation, rounds/heats/attempts, compatible comparison conditions,
source-declared PRs versus observed bests, and exact source references. Bound derivation, sorting,
rendering and staging by bytes/work, not only row counts. Persistent indexes require measured query
need; an athlete-keyed index does not inherently provide workbook display order.

Every athlete/event with an observed comparable numeric mark must have an observed best in its
compatible comparison class. Roster/profile-only athletes and no-result statuses retain explicit
missing-result coverage, never invented PRs or inferred individual relay splits. Preserve published
automatic/hand timing qualifiers when normalizing numeric times. An append-only correction may
refine a raw mark and unknown timing for the same natural identity, source owner, event and team;
it must not downgrade measured marks, override known timing or delete original observations.

Stage workbook, sidecars, audit and manifest as one generation. Bind lineage, input/policy/schema
revisions, scope, cohort, as-of time, record IDs, counts, hashes and lengths. Independently reconcile
against the frozen evidence, durably finalize the bundle, then atomically switch its publication
reference. Fence stale exporters; keep the previous valid bundle on failure. A file rename alone
is not multi-artifact atomic publication. XLSX byte equality alone is not semantic correctness.

## 10. Engineering standards (§37-§43)

- **Zero code comments:** no line/block/doc comments or prose documentation attributes in
  project-owned source, tests, examples, benchmarks or generated project-code templates. Intent
  belongs in names, types, errors and tests; rationale belongs in docs. A lexical gate must
  distinguish comments from captured evidence or string literals.
- **Type discipline:** validated newtypes, private fields, checked constructors/deserialization,
  exhaustive enums and explicit state transitions. Separate raw observations, validated evidence,
  candidates and accepted decisions; make illegal states unrepresentable where practical.
- **Safety (§37, §56):** workspace-wide `forbid(unsafe_code)` and `deny(unused_must_use)`; no
  input-triggered panic, careless `unwrap`/`expect`, unchecked arithmetic, ignored outcome,
  production recursion or silent fallback. Handle resource and counter exhaustion explicitly.
- **Size (§38):** files within 300 lines; production functions within 60 logical lines and hot
  paths within 25. Decompose named stages rather than suppressing checks. Dated evidence ledgers
  may retain longer command records; they are not production source.
- **Errors (§39):** `thiserror` in production crates; `anyhow` only at CLI/composition boundaries.
- **Async (§42-§43):** supervise every task, bound admission before spawning, avoid locks across
  awaits, and offload bounded blocking work. Cancellation is stop-intake, drain, finalize/persist,
  then report; distinguish domain error, cancellation, timeout, abort and panic.
- **Drain accounting:** accepted work reconciles with mutually exclusive terminal/still-running
  outcomes. Successful shutdown leaves no unowned task; unfinished business remains durable.
- **Observability (§44-§45):** structured `tracing` and exact resource/outcome accounting. The
  acquisition efficiency metric is useful verified records per physical request.
- **Caching (§61-§62):** reuse immutable successes; quarantine corruption and retain failures.
  Cache reads do not pretend source freshness advanced.
- **Efficiency:** measure the bottleneck on the 16-core/32-thread, 128-GB workstation. Use streaming,
  bounded parallelism, buffer reuse and minimal copying/contention/repeated I/O. Expose defects,
  constrain work in progress and apply backpressure. No performance claim without measurements.
- **Language:** Rust-only pipeline and tooling logic under [ADR-008](docs/adr/ADR-008-rust-only-tooling.md);
  operational/gate shell wrappers are not a second pipeline implementation.

## 11. Quality gates (§55-§58)

[tools/gate.sh](tools/gate.sh) and [xtask/README.md](xtask/README.md) own commands, gate semantics,
fixture rules and measurement procedures.
[VERIFICATION-EVIDENCE.md](docs/VERIFICATION-EVIDENCE.md) owns dated command results; a document,
passing build, skipped lane or old run is not acceptance evidence for a new delivery.

The release contract requires strict format/check/Clippy/tests, architectural and comment gates,
dependency advisories/provenance/cargo-vet, security and async review, adversarial identity/contact/
result fixtures, property tests, parser fuzzing, mutation of critical logic, concurrency/cancellation,
duplicate delivery, fault injection, crash recovery, retry exhaustion, malformed model replies,
representative load/resource measurements, and actual CLI/Restate/export execution. License
**enforcement is excluded by owner direction**; security checks are not waived.

The delivery plan retains every F01–F15 correction, named regression and all 17 native fault
scenarios. Unresolved accuracy, data-loss or recovery failures block release. Report exact commands,
exit/status, tested scope and remaining limits; never weaken a gate to manufacture a passing claim.

## 12. Acceptance (§70)

[ADR-011](docs/adr/ADR-011-census-seal.md) defines evidence-backed completion. Every required
jurisdiction/source obligation, cohort decision and identity candidate must have a terminal declared
outcome. Retain conflicts, terminal gaps and exhausted failures; open work, unsupported claims and
unverified publication block sealing. Unknown counts cannot become zero by default.

Accepted facts must be durable, marks reproducible and exact workbook records/PR winners/coverage
reconciled against the same frozen evidence. A seal certifies the declared run and its disclosed
limits, not exhaustive knowledge of the Internet. Historical exports and seals do not certify the
fresh run.

## 13. Current state and delivery order

The nine-crate split exists. [OPERATIONS.md](docs/OPERATIONS.md) inventories bound handlers;
[the table registry](crates/census-store/src/table.rs) defines implemented storage.
Sections 8–9 above state required semantics, not a certificate of their implementation.

[The national delivery plan](docs/NATIONAL-CENSUS-PLAN.md) is the only active repair/order/acceptance
list. Prove one working source-discovery-to-evidence-to-output path before scaling breadth; then
satisfy all 49 jurisdictions and all release obligations through the same implementation.
[OPERATIONS.md](docs/OPERATIONS.md) is the runbook, not a competing project plan.
