# Fresh national census — implementation master plan

Status: binding target and delivery plan, not a claim of completed implementation or verification.
[ADR-013](adr/ADR-013-fresh-national-source-census.md) records the owner's correction and supersedes
workbook-driven intake and the former geographic restriction. [ARCHITECTURE.md](../ARCHITECTURE.md)
retains the binding numbered standards; [architecture.md](architecture.md) describes implementation.

## 1. Product and fresh-run boundary

Discover the Class-of-2027 high-school Track & Field / Cross Country population across all 50 states
plus D.C. Include source-supported boys' and girls' XC, indoor and outdoor participation, legitimate
events and evidenced relay membership. Do not infer participation, graduation year, GPA or identity
from a matching name. Never collect athlete personal contact information.

There is no input workbook, admissions dataset, supplied recruit list or original-sheet accounting.
Excel is generated output. Public result spreadsheets are source documents, not a seed population.
Population and coverage come from public source discovery, including schools and programs that have
not yet yielded an accepted athlete. A subset run is qualification evidence, not national completion.

Use a fresh named store and unused durable run revision/namespace, explicitly bound to each other.
A new directory with old Restate keys is not fresh: replay can skip work using old completions.
Preserve existing stores and exports. Do not import their evidence, identities, receipts, counts or
seals. Reuse code, captured parser fixtures and qualified public source entry points; acquire this
run's evidence afresh. Once acquired, reuse its immutable captures and effects across subjects and
recovery. "From scratch" does not mean rewriting the working Rust stack or repeatedly fetching it.

## 2. One architecture, not another engine

Keep the existing nine crates, native non-Docker Restate, Fjall, Tokio and both local Qwen servers.
Keep one implementation per responsibility; migrate every caller and delete superseded paths.
No replacement root package, acquisition crate family, custom runtime, ORM or generic workflow DSL.

```text
51-jurisdiction run specification + qualified public source registry
  -> discovery frontier and school/program/season obligations
  -> origin-admitted acquisition -> immutable captures
  -> provider-owned observations -> validated evidence
  -> candidate generation -> contradiction checks -> Rust adjudication
                                      ^ bounded local Qwen advice
  -> versioned decisions and retained gaps -> one snapshot projection
  -> verified atomic output generation -> evidence-backed seal
```

`census-domain` owns pure constrained types and rules; `census-crawl` owns source parsing, discovery
and single-attempt acquisition; `athleticnet-browser` owns headed capture and challenge vocabulary;
`census-store` owns durable transactions and snapshots; `census-reconcile` owns deterministic identity;
`census-review` obtains advice; `census-report` projects; `census-service` composes and supervises;
`xtask` exposes existing operations. Adapters never accept canonical identities themselves.

## 3. Domain contracts before caller migration

Separate raw wire DTOs, captured observations, validated claims, identity candidates and accepted
links. Private newtypes and checked constructors carry provider identity, jurisdiction, cohort,
season, event, performance conditions, source object, capture, decision and operation identities.
Deserialization must run the same checks. Do not serialize invalid states around constructors.

Use explicit state enums rather than flags and nullable lifecycle fields. Keep cohort membership,
person identity, school affiliation, sports participation, coach appointment and source coverage
separate. A grade needs its academic-year context; today's clock cannot reinterpret old evidence.
A school affiliation is time-scoped, not the athlete's timeless current school.

Provider IDs retain their namespace and actual ownership scope: athlete-global, meet-local or
sport-local as applicable. Missing IDs produce capture-scoped subjects, not name hashes masquerading
as stable people. Canonical links and reversals are versioned decisions; observations are immutable.
A source's copied/syndicated descendants share a corroboration family, not independent votes.

Pure functions handle normalization, cohort rules, contradictions, admissibility, result comparison
and coverage reductions. Network, store, clock, spawning and model capabilities stay in the shell.
Expected failures are typed: invalid input, unsupported schema, partial parse, access restriction,
human required, rate limit, transport failure, exhausted retry, corrupt capture, budget stop,
identity conflict, insufficient evidence, durability failure and publication failure stay distinct.

## 4. Nationwide source graph and coverage obligations

Create obligations for every required jurisdiction before acquisition. Discover schools/programs
from public association, school and team directories independently of successful athlete matches.
Qualify Athletic.net, MileSplit, DirectAthletics, relevant TFRRS, AthleticLIVE, state associations,
timing providers, organizers and school/team sites. Follow relevant linked HTML, structured data,
PDFs and public spreadsheets. Retain provenance for newly discovered sources and their access policy.

Prefer shared rosters and meet/results acquisition over repeated athlete-name searches. Use major
meets and qualifiers to discover participation, then rosters, linked profiles and earlier seasons
to fill cohort and history gaps. Track discovered links, visited source units, pagination, seasons,
unsupported formats and truncation. New sources extend the frontier; qualification is not a one-off
fixed list. Bound each activation and persist its cursor; reaching a budget is not finishing a source.

Track jurisdiction, source, school/program and season obligations with explicit pending, acquired,
partial, quarantined, access-blocked, human-required, retry-exhausted and budget-limited outcomes.
A state with zero accepted athletes is not covered by definition. Unknown source denominators stay
unknown. A failed or unavailable source is never negative identity evidence or `NO_MATCH`.

## 5. Immutable acquisition evidence

A capture records request method, canonical request identity including relevant body digest,
non-secret representation context, final URL, status, actual acquisition time, media type, byte
length, body digest and immutable storage reference. Published/effective dates are separate from
fetch time. A URL alone cannot identify a POST response or prove a parsed fact.

Durably write and verify captured bytes before committing references to them. Commit observations,
progress and effect receipt together, then acknowledge. A crash can leave an unreferenced capture;
it must not leave acknowledged observations pointing at missing bytes. Quarantine corruption with
a typed finding. Cache only immutable successes; failed captures may be retained as evidence of
failure, never replayed as successful acquisition. Cache reads do not advance source freshness.

Every material claim resolves to a captured locator: provider object/result ID, HTML location,
structured-data path, PDF page or spreadsheet sheet/row. Parsing revisions and rejected locators are
retained. Partial rosters keep valid rows while retaining the rejected rows and unfinished obligation.
A parser count or successful HTTP status does not prove the required immutable archive exists.

## 6. One durable application path

Evolve the existing `NationalCensus`/`JurisdictionCensus` and service handlers into one path through
discovery, acquisition, reconciliation, review, gap resolution and publication. CLI entry points
submit/status that application; they do not maintain a second business implementation. Existing
batch-only index/review/gap paths must join it before the target can be called implemented.

Distinguish immutable run semantics from execution controls. Cohort, jurisdiction set, source and
policy revisions define the logical run. Pagination/work-per-activation/concurrency budgets control
execution. Increasing a smoke-run budget must resume retained work, not mint duplicate people or
pretend replaying a completed workflow changes its immutable input. Main must define the durable
resume/control transition and migrate current wire contracts explicitly.

Distinguish run, logical source unit, physical attempt, capture, ingest effect, review case and export
generation IDs. Retries retain logical IDs. Receipt lookup precedes an effect; reusing an operation
ID with different content is a typed mismatch, not an overwrite. A newly fetched capture is distinct
from replaying the same effect. Journal activity alone cannot make a partial unit complete.

Restate is the sole automatic retry owner: three total attempts, not three retries after an initial
attempt. HTTP, browser, adapter and model transports each perform one attempt. Preserve the budget
across restart; exhaustion remains evidence while independent sources continue. Persist Retry-After
and challenge state. No private retry loop or new workflow key may reset the ceiling.

## 7. Admission, boundaries and supervision

Measure admission at physical remote requests. Share one origin budget across endpoints, workflows,
tabs and related Athletic.net surfaces; more workers must not multiply it. Honor robots, authorized
origins, redirects and content-size limits. Revalidate redirect destinations; protect local/private
network boundaries. Browser and HTTP acquisition retain their distinct supported policies.

Athletic.net uses the headed persistent-profile lane. Challenges enter `HumanRequired`, close new
admission and drain issued work. No CAPTCHA solver, authentication/paywall bypass, identity spoofing,
proxy evasion or direct-HTTP fallback to circumvent a challenge.

Own every task in a bounded region. Backpressure precedes spawning; queues are bounded accelerators,
not durable authorities. No lock crosses an await. Offload bounded blocking/CPU work to supervised
workers, not detached tasks. At each suspension an effect has a reserve/commit/rollback or replay
contract. Separate returned errors, cancellation, timeout, abort and panic.

Shutdown stops intake, drains/finalizes owned work, persists unfinished obligations, then reports
mutually exclusive completed/failed/cancelled/timed-out/aborted/panicked/still-running outcomes.
The sum reconciles with accepted work; no running task is silently called completed. Successful
shutdown leaves no unowned work; unfinished business remains durably resumable in Restate.

## 8. Identity and both local Qwen lanes

Candidate discovery is high-recall retrieval, not acceptance. Name, grade, school text, a score or
model agreement alone cannot prove identity. Require policy-defined corroboration and retain
contradictions, source ownership and observation time. Transfers are time-scoped affiliations;
different schools at different times are not automatically different people.

Resolve deterministic cases in Rust first. Ambiguous cases go to both approved local Qwen servers
(5090 and 3090) with bounded structured evidence, claim IDs and contradictions. No cloud fallback,
private workbook or admissions data. Bind advice to evidence digest, policy and model revisions;
retain each response and reuse valid completed advice. Do not silently truncate decisive evidence.

Models return `SamePerson`, `DifferentPerson` or `InsufficientEvidence` with evidence references.
Rust checks the evidence binding, admissibility and required agreement. Invalid output, disagreement,
insufficient corroboration or exhausted model attempts stays `REVIEW`; agreement cannot defeat a
hard contradiction. Changed material evidence invalidates the affected decision/advice, not arbitrary
unrelated work. Models cannot invent marks, graduation evidence, affiliations or email addresses.

## 9. Performances, affiliations and coach contacts

Identify result instances by provider result ID or a qualified meet/event/round/heat/attempt/subject
key. Unknown conditions are not wildcards. Deduplicate repeated references while retaining legitimate
heats, rounds, attempts and relay memberships. Preserve the affiliation on the historical result;
never replace it with a present-day school. Club and unattached participation remain explicit.

One event/comparison contract governs PRs everywhere: timing, wind, distance, units, surface and XC
course/context compatibility. Keep declared profile PRs separate from best observed performances.
Report the history/conditions limitations. Workbook and sidecars must agree on exact winner IDs,
values and evidence, not merely the number of rows.

Research public school/program coach appointments by role, sport, side and season. Current explicit
appointments outrank former staff even when the former coach has an email. Unknown side is not both
sides; another sport is not TF/XC. Bind each published mailbox to exact source text and its person or
program role. A consumer-domain address can be legitimate when the institution publishes it. Reject
mailbox construction from name/domain patterns and unrelated addresses on the same page.

Persist contact-research attempts, including empty, inaccessible and failed results. No coach row
means no published contact, not proof that research never ran. Keep identity and contact decisions
separate so invalidating a mailbox does not erase otherwise valid athlete evidence.

## 10. Store, projections and publication

Fjall remains single-writer. Plan sequence allocation under the write lock; commit data, progress and
receipts with the required durability before publishing in-memory counters or acknowledgements.
Counter exhaustion, poisoned state and failed durability cannot silently produce usable writes.
Use the actual logical table catalog, not a hard-coded historical table count. Legacy import is an
explicit maintenance operation for old stores, never normal startup for this fresh census.

Use one MVCC snapshot and one shared projection for all output formats. Writers do not reimplement
cohort, identity, contact, coverage or PR policy. Stream bounded batches instead of constructing
multiple full-corpus vectors. Retain the snapshot only for the required projection lifetime.

Stage every generation's workbook, machine-readable/audit outputs and manifest together. The
manifest binds run/snapshot/scope/cohort/policy revisions, paths, byte lengths, hashes, record IDs,
coverage totals and PR winners. Flush, independently verify, then atomically switch the published
generation reference. Retain the previous valid generation on failure. Generation identity is not
a calendar date; stale concurrent exporters cannot overwrite a newer accepted generation.

Workbook sheets expose accepted athletes/cohort, separate TF/XC evidence, source profiles, observed
marks/PRs, schools, public coaching contacts, coverage, conflicts, review and run metrics. Raw source
counts, scoped observations and accepted Class-of-2027 counts are separate, with all limits visible.

## 11. Completion and measured efficiency

ADR-011 remains: terminal gaps, conflicts and exhaustion can be retained findings, but unresolved
work and unverified publication block sealing. A seal certifies the declared run and obligations;
it is not proof of every athlete or document on the Internet. Do not hide incomplete history,
unknown denominators or unsupported sources behind a national label.

Measure useful verified records per physical request, capture/parse/advice reuse, completion rate,
latency, CPU scaling, peak process RSS and peak disk use including staging, old generations and MVCC
retention. Use the actual 16-core/32-thread, 128-GB workstation and approved GPU lanes. Bound pages,
payloads, candidate sets, queues, batches and model prompts. No allocator, parallelism or layout
rewrite without a representative baseline and correctness-preserving benchmark comparison.

## 12. Dependency-ordered delivery

Each slice owns its domain contract, caller migration and focused executable acceptance. Main owns
public types, schemas, Restate contracts and integration. Use 2–4 useful bounded agents, at most one
coding job per GPU; do not parallelize competing writers or disguise staffing limits as scope cuts.

| Stage | Deliverable and exit evidence |
|---|---|
| 0 — Scope | ADR-013, fresh-run contract, all 51 scope members and migration inventory; preserve old data. |
| 1 — Entry | Fresh store/run binding, qualified source roots and per-jurisdiction/program obligations; native national entry point. |
| 2 — Evidence | One real source-object discovery/capture/parse/atomic-ingest path, exact provenance and partial-result behavior. |
| 3 — Decisions | Source-owned subjects, time-scoped cohort/affiliation, reversible deterministic acceptance and both local advice lanes. |
| 4 — Vertical proof | Real discovery-to-census-to-output qualification run; restart and increased execution budget resume exact remaining work. |
| 5 — Breadth | All named source families, linked document formats, all 51 jurisdictions, history and persisted public-contact research. |
| 6 — Publication | Shared snapshot projection, exact PR/contact parity, independent verification and atomic output generations. |
| 7 — National run | Fresh source-discovered acquisition across all 51; independently reconcile obligations, decisions, findings and artifacts. |
| 8 — Assurance | All F01–F15 checks, all 17 native faults, security/async review, hostile-input and measured resource gates; repair failures. |
| 9 — Release | Verified final generation and seal, reproducible evidence and operations docs; Main commits/pushes only verified delivery. |

Do not wait for a broad refactor before exercising the first vertical path. Do not claim that path
is national completion. Remove temporary probes after preserving their evidence and durable fixes.

## 13. F01–F15 acceptance — none dropped

| ID | Required consumer-visible evidence |
|---|---|
| F01 | Exact public mailbox, role and school/source binding; reject invented address, wrong role, wrong institution and unrelated page address. |
| F02 | Homonyms remain distinct; accepted links have corroboration and a reversible decision history. |
| F03 | Cohort evidence is not identity proof; contradictory graduation/grade evidence remains visible. |
| F04 | Partial acquisition and increased execution budgets resume the same logical work without duplicate effects or false completion. |
| F05 | Comparable performance conditions and exact PR-winner parity across workbook and machine-readable outputs. |
| F06 | Current, program/sport/side-specific coaching appointments; missing email never promotes a former or unrelated coach. |
| F07 | Explicit source/program/season and history obligations, including unavailable and truncated coverage. |
| F08 | Historical result affiliations preserved through transfers, clubs and unattached participation. |
| F09 | Genuine rounds, heats and attempts retained; repeated references do not create false result conflicts. |
| F10 | Durable contact-research attempts distinguish not attempted, empty, failed and completed research. |
| F11 | Honest national denominators and gaps; no failure-to-absence conversion or empty-state coverage fiction. |
| F12 | Source-discovered national athlete/result outputs with evidence-linked profiles, participation and auditable accepted decisions. |
| F13 | Raw, scoped and accepted-cohort counts separated; run scope and acquisition/publication limits disclosed. |
| F14 | Generation manifests verified and published atomically; no date collision, partial bundle or stale-export overwrite. |
| F15 | Measured bounded memory/disk/concurrency and useful acquisition throughput; no benchmark-free efficiency claim. |

## 14. All 17 native fault scenarios

Required, not executed by this document. Each scenario records the actual injection point, owned
processes, exact command/exit/status, before/after oracle, recovery result and cleanup. An unreached
fault, skipped scenario, simulated error substituted for the real fault, or unmeasured invariant is
not PASS. Preserve genuine terminal gaps; never count them as successful acquisitions.

The complete numbered [native fault catalog](NATIONAL-CENSUS-FAULTS.md) defines all 17 injections,
their exact recovery obligations and isolation rules. It is part of this plan, not optional work.

## 15. Quality gates and current implementation gaps

Run the four workspace gates in ARCHITECTURE §11, contract checks, the zero-comment lexical gate,
dependency/license/security audits and async review. Exercise CLI/native Restate/export behavior,
not just unit tests. Keep focused regressions for real consumer-visible failures, adversarial
identity/contact/result fixtures, proptest and parser fuzzing; use mutation on acceptance rules and
Loom on the actual shared concurrency kernel rather than a disconnected replica.

Current scope code still declares 49 jurisdictions. Batch-only application stages still need
integration; capture metadata is not proof of an immutable archive; partial-resume and whole-bundle
publication require end-to-end evidence. Historical Midwest/49-jurisdiction exports and seals do not
satisfy this fresh 51-jurisdiction run. These are delivery obligations, not claims fixed by this plan.
