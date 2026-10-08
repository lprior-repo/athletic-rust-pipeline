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
and protect local/private network boundaries. An authorized redirect still cannot cross from
direct HTTP into a browser-only provider; refuse it before dispatch rather than changing lanes.
Pre-follow and post-response admission use the same redirect decision: a destination may remain
on the original request's scheme, parsed hostname and effective port, or match an existing explicit
hostname grant. A same-host scheme or port change is not implicitly authorized. DNS/private-address
and browser-lane validation remain independent checks before dispatch.

Robots.txt is read for pacing only: a `Crawl-delay` published for `User-agent: *` paces that host,
while an absent, unreadable or non-200 robots.txt publishes no policy and never blocks a request.
Redirect admission is per origin, not per path: a same-origin hop is followed without a robots path
check, so the caller that admits a URL also admits its redirect destinations and must refuse a hop
into a login, authentication or otherwise disallowed path itself (record the chain and stop). No
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

The store names the schema it holds (`format.rs`: schema and key-format versions, creator and
creation date, and a migration resume marker) and refuses to open bytes it cannot interpret as
current, older but migratable, newer, or unknown. Migration is one explicit offline operation with
its own native verb, never a silent reinterpretation during open. Derived tables — source
identities, conflicts, review cases, coverage and pass snapshots — are partitioned by generation:
a pass materialises a whole generation in bounded row/byte batches and publishes it with one atomic
pointer flip plus its receipt, and readers see the current generation only. Reclaim of superseded
generations is budgeted and resumable. A review decision fences on the evidence generation rather
than on the global store sequence, so journal, receipt and derived writes cannot invalidate it while
a change to its inputs does; a staged generation whose evidence moved refuses to publish.
[ADR-026](docs/adr/ADR-026-derived-generations-and-store-schema.md) owns the rationale.

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
Acquisition retains the original request URL separately from the optional observed final response
URL. Historical absence means unknown, never an inferred redirect destination. Roster acquisition
requires consistent requested, observed and authoritative published-document team ownership;
foreign or ownerless captures remain archived without entering the accepted roster population.
Published owner declarations are decoded HTML tokens, not tag-shaped bytes in attributes,
comments, raw text, templates or embedded foreign namespaces. Ignore inert markup without
discarding contradictory authoritative declarations.
Result completion binds immutable owned/raw captures and the projected context. A changed capture
or projection cannot be suppressed by a bare meet/result-set identifier; identical row effects
remain idempotent and old captures and receipts remain preserved.
Acquisition manifests bind stable observed metadata separately from content-addressed byte
chunks. Identical bytes acquired at a different actual time or final URL retain separate manifests
sharing the same original chunks; cache replay alone does not create a new acquisition.
Partial interpretations and rejected locators are immutable and identical replay is a no-op.
Normalize typed set-valued entity evidence before hashing physical application witnesses;
do not reorder semantically ordered marks, rounds, heats or provider arrays.


Canonical schools may retain validated `SchoolPostalAddress` claims: published street/address,
exact association-school owner, source label, parsed capture URL/date and capture SHA256.
Parsed claim evidence uses the capture's actual `fetched_at`, not the adapter evaluation instant.
Replaying the same bytes does not advance address, school or coach evidence freshness.
`add_postal_address` checks that owner and jurisdiction against the school; publication refuses
foreign claims rather than joining by school text. The additive `postal_addresses` field defaults
to an empty vector for historical records and omits an empty vector on serialization. Absence is
unknown, not an invented address. Accepted-athlete publication retains source-owned claims for
each supported school affiliation; their presence does not prove which affiliation is current.
Qualified canonical postal metadata uses bounded, control/whitespace-free provider tokens, an
association source label matching the provider evidence namespace, syntactically valid
credential-free HTTP(S) capture URLs and real ISO calendar dates or bounded RFC3339 observation
timestamps in 1900–2100. URL syntax is not a second network source-qualification policy.
Source-label jurisdiction binds the school even when the address's state is absent. Canonical
claims deduplicate and sort deterministically; workbook/CSV columns retain aligned owner/capture
positions independently of input arrival order. Raw captures and distinct conflicting claims remain retained.


Owned structured results preserve published person, team, meet, result-set and event context.
Reject malformed, foreign, duplicate or mismatched ownership rather than replacing it with a name
lookup. Explicit complete counts, retained valid rows, rejected locators and refused/partial
captures remain distinct; successfully parsing some rows cannot certify a complete meet.
The MileSplit result-set bridge resolves schools only through unique positive `MilesplitSchool`
provider team IDs. Structured owner-bound rows supply names, cohort and marks; the raw document
supplies published meet date, sport and school year, with requested/captured/published/canonical
URLs bound to the same meet and result set. Raw names or grades never replace structured ownership.
Directly published graduation years are retained as typed `PublishedGraduation` claims with their
publisher source reference, separately from grade/year observations. No collector season is used
to invent a grade. Canonical cohort confidence considers both forms and retains contradictions;
historical missing typed claims remain unknown rather than being reconstructed from notes.
Accepted-alias publication unions both grade and published-graduation claims before cohort
derivation. Direct-year contradictions remain coverage conflicts even without grade observations;
published-year evidence contributes to cohort evidence counts without manufacturing grade rows.
Projection receipts are distinct from source-interpretation receipts and census acceptance.
Completed projection replay rebuilds the current accumulator without reappending committed effects.
Partial projection writes use `milesplit_result_set_effects_v1` table/content-digest application
witnesses, atomically committed with rows and disposition receipts. Unchanged partial replay retains
its unfinished obligation without duplicating physical facts; newly available exact school bindings
can add previously unresolved entities without rewriting unchanged source observations.
Partial mappings remain unfinished; source completeness may remain Unknown after projection.

All reports, workbook sheets and audit outputs use one immutable export dataset derived from one
input generation. Do not rebuild identity, cohort, contact, coverage or PR rules in writers. Keep
candidate statuses and accepted aliases distinct; athletes without performances remain in the
population, and unresolved joins remain explicit coverage outcomes. Core evidence scope is not
interchangeable with geographic scope or cohort eligibility.
Publication policy revision 6 requires typed direct graduation-year or compatible dated-grade
support for requested cohort membership after accepted-alias evidence union; unsupported or
contradictory members and unresolved result subjects are excluded from requested-cohort projections.
Same-context numeric conflicts disqualify that context from PR selection; a clean compatible
context may still supply the best. All-contested slots have no published best. Unfiltered archives
retain original observations. Older policy-1/2/3/4/5 frozen inputs are retained but refused, never
reinterpreted or overwritten; accepted-alias, postal-field and full-cell summary rules remain in force.
Independent readback compares every represented athlete row with its expected immutable position,
including displaced rows; population membership alone cannot certify cell/provenance equality.


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

A published recruiter contact requires one claim binding a school-scoped role and program to a
public mailbox from the same retained capture, plus an eligible current-tenure statement whose
school year equals the census run's season. Tenure evidence names the source URL, capture SHA256,
RFC3339 retrieval time and a bounded statement asserting that role and program. A mailbox or
listing that names no role, states a former role, or carries no season scope publishes `Unknown`
and no contact; retrieval alone never makes an appointment current. Successful-empty, blocked or
failed and never-attempted research remain distinct outcomes, and a published contact cell names
the selected coach identity, source and observation date.

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
  input-triggered panic, unchecked arithmetic, ignored outcome, production recursion or silent
  fallback. The complete `unwrap` family (`unwrap`, `unwrap_err`, `unwrap_unchecked`,
  `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`) and `expect`/`expect_err` calls or
  references are forbidden in all project-owned Rust, including tests, fixtures, examples,
  benchmarks, proofs and generated project code. Return complete typed outcomes; do not replace
  extraction with panics, assertions, waivers or fabricated defaults. Preserve domain-defined
  missing-value behavior with explicit matches or map combinators. Handle resource and counter
  exhaustion explicitly. The fatal owned-source lexical gate complements all-target Clippy.
- **Size (§38):** production source files within 300 lines. Handwritten production callables,
  including closures and handwritten project-macro callable templates, have a hard maximum of
  60 trusted logical lines; 25 lines is the preferred size and an advisory review threshold.
  More than five parameters is a cohesion-review warning, not a release failure. Do not introduce
  artificial context bags or mechanical extraction merely to satisfy an advisory count.
  Moon and release Clippy retain `too_many_arguments` as a forced review warning under otherwise
  fatal warnings; the trusted AST scan reports the five-parameter threshold. Arity diagnostics
  remain visible rather than being suppressed or converted into artificial context containers.
  Compiler/vendor-generated code is excluded from style-size certification, not from compilation,
  domain contracts, safety or behavior verification. Trusted AST measurement must resist statement
  packing and structurally distinguish production, tests and captured literals. Unmeasurable
  handwritten callable syntax fails closed; do not suppress or baseline away hard violations.
  Dated evidence ledgers may retain longer command records; they are not production source.
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

[Moon developer tasks](xtask/README.md) are the only repository command entrypoint;
[tools/gate.sh](tools/gate.sh) is their internal gate implementation. The developer reference owns
commands, gate semantics, fixture rules and measurement procedures.
[VERIFICATION-EVIDENCE.md](docs/VERIFICATION-EVIDENCE.md) owns dated command results; a document,
passing build, skipped lane or old run is not acceptance evidence for a new delivery.

The release contract requires strict format/check/Clippy/tests, architectural and comment gates,
dependency advisories/provenance, security and async review, adversarial identity/contact/
result fixtures, property tests, parser fuzzing, concurrency/cancellation, duplicate delivery,
fault injection, crash recovery, retry exhaustion, malformed model replies, representative
load/resource measurements, and actual CLI/Restate/export execution.
[tools/gate.sh](tools/gate.sh) `--release` executes the format, check, strict Clippy, doc, test,
zero-comment, architecture-contract, panic-extraction, production-scan/size-budget,
domain-integrity, purity, module-seam, debt-ratchet, dependency-advisory, feature-powerset,
bench-presence and pre-release performance lanes; it does not itself execute the native fault
scenarios or a parser-fuzz campaign, so a passing gate alone is not the whole release acceptance.
Those obligations remain separately executed and dated: the scenarios through the manual
`pipeline:durability` wrappers and the fuzz targets in the `fuzz/` workspace, with results recorded
in [VERIFICATION-EVIDENCE.md](docs/VERIFICATION-EVIDENCE.md). Giving the gate that coverage would
take a release-only lane invoking `tools/durability/run.sh` over the
[catalogued fault scenarios](docs/NATIONAL-CENSUS-FAULTS.md) and a bounded `cargo fuzz` smoke per
target. Mutation testing of critical logic was retired by owner decision
(bead `5rtr`, tombstone in [tools/gate.sh](tools/gate.sh)), so its absence is not a release
blocker. License **enforcement and cargo-vet are excluded by owner direction**; security checks
are not waived.

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
