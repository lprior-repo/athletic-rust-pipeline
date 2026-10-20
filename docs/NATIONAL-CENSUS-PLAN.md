# Recruiting database — delivery and acceptance plan

Status: binding unfinished delivery contract, **not execution evidence**. This is the only active
plan. [ARCHITECTURE.md](../ARCHITECTURE.md) owns mission, engineering policy and semantics
(§8–9), and [ADRs](adr/README.md) own decisions.
[Verification evidence](VERIFICATION-EVIDENCE.md) records observed results; removed ledgers do not certify current behavior.

## 1. Run boundary

[ADR-038](adr/ADR-038-selected-profile-reusable-production-state.md) makes the complete October 11
spec-v2 reusable selected-profile product the forward target, beginning with UP qualification and
Class-of-2027 MI/MN/IA/IL/CO delivery. The fresh 49-jurisdiction requirements below remain the
explicit national/isolated-qualification lane and keep their historical meaning. This remains
the sole implementation/order/acceptance plan; the ADR does not add a parallel plan, enroll
production, authorize migration or certify a currently unsupported runtime.

Deliver the Class-of-2027 high-school TF/XC census from public-source discovery across exactly
49 jurisdictions: 48 contiguous states plus D.C. `UsJurisdiction::ALL` models 51 locations;
`CENSUS_SCOPE` is the run denominator. The proposed ADR-013 expansion was withdrawn on 2026-09-27.

Create a fresh named store bound to an unused durable run namespace/revision. Preserve historical
corpora without importing their athletes, observations, decisions, receipts, coverage or seals.
No admissions workbook, seed list or previous export supplies the population; public result
spreadsheets are captured sources. Historical migration is separate maintenance, not a current
legacy-import API or a fresh-run prerequisite. Excel is output.

Extend the existing nine-crate implementation. Do not create a replacement engine, new scheduler,
shadow domain model or parallel CLI. First prove one complete source-to-output path, then breadth:

```text
49-jurisdiction run -> source obligations -> captures -> validated claims
  -> candidates -> Rust decisions (+ dual local advice when needed)
  -> immutable export dataset -> verified atomic bundle -> qualified seal
```

### Selected-source qualification position — 2026-10-14

Research and component tests are not a qualified denominator. School membership, enrollment,
sport assignment and actual competition remain distinct. Preserve year/category, cooperative
programs and access refusals; unknown is not zero. The five-state target does not discharge
the separate 49-jurisdiction obligation.

| Stratum | Primary source | Unqualified boundary |
|---|---|---|
| MI / UP | [2026–27 MHSAA enrollment](https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/26-27-Enrollment-List.pdf); [UP boys XC list](https://my.mhsaa.com/Sports/Boys-Cross-Country/School-Division-List-2026-27-UP) | Enrollment is not UP/sport membership. Named-list handlers are robots-disallowed; complete membership remains unproved. |
| MN | [2026–27 boys XC assignments](https://www.mshsl.org/api/competitive-sections-map/get-csl-teams/698/124); [year dictionary](https://www.mshsl.org/api/competitive-sections-map/get-years) | Year 698 is 2026–27; sport-page year 232 is stale. Assigned teams/co-ops are not competition; actual-client access and qualified capture remain owed. |
| IA | [Fall 2026 qualifying assignments](https://www.iahsaa.org/cross-country-2026-state-qualifying-assignments/) | Preserve boys-only exceptions and both associations. Current-year TF and unavailable girls-source evidence remain owed. |
| IL | [2026–27 boys 1A XC assignments](https://www.ihsa.org/data/ccb/1assign.htm) | Inspected TF assignments are 2025–26, not 2026–27. API robots access was unresolved; full membership/co-op joins remain unproved. |
| CO | [Official directory access policy](https://schools.chsaa.org/robots.txt) | Named AI agents are disallowed. No current-year membership or sport denominator was established; do not bypass or infer zero. |

The [capability boundary](PROFILE-EXECUTION-V1.md) distinguishes current components from runtime
acceptance. Retained public-source research must still pass policy, capture, interpretation,
identity binding and complete-frontier qualification before it can discharge an obligation.

## 2. Discovery and coverage obligations

Represent school/program/season and source obligations independently of accepted athletes. Discover
schools, teams, schedules, rosters, meets and linked results through all relevant qualified provider
families. Reuse shared source objects across athletes; keep profiles as bounded evidence/history
fill-in. Registration or an applicability row is not an executed source sweep.

Persist pagination/cursors, redirects, unsupported media/schema, access stops, parser quarantine,
exhaustion and unfinished discovery. Model successful-empty, partial, blocked, human-required,
exhausted and complete-with-gaps separately. Never turn zero accepted athletes or an unreached
jurisdiction into complete coverage. Continue unrelated permitted work after a source stop.

## 3. Capture and external-effect durability

Archive immutable bytes before publishing claims/progress. Capture identity binds source instance,
method, canonical request including POST body and representation context, URL/status, actual fetch
time, media type, byte length, digest and archive locator. Distinguish event, source publication,
acquisition and run-as-of dates. URLs/digests alone are not retained evidence.

Retain valid rows and rejected locators from partial parses with parser/schema revision. Quarantine
corruption; do not silently refetch/overwrite an object or cache a failed attempt as success.

Use one store apply-once contract under existing write synchronization. Atomically persist bounded
observations, sequence/count metadata, source progress and a reusable effect receipt. Bind the
receipt to store/run lineage, source instance, logical object/page, capture/parser revision,
destination schema/table, stable batch ordinal and full payload digest. Changed content under the
same operation ID is a typed conflict; successful-empty pages still have receipts.

Partition oversized objects deterministically with row, record and byte bounds and a manifest.
Keep completed chunk boundaries stable through restart. Retain receipts while a legitimate Restate
replay can use them; do not confuse effect receipts with a second task scheduler. Prove the real
commit/lost-ack window using physical observations, not only merge-on-read canonical counts.

## 4. Durable orchestration and resource boundaries

Run discovery, acquisition, parse/apply, deterministic reconciliation, local review, gap handling,
export and verification through the existing durable application. Parent completion requires each
child's declared terminal state. Large bodies/batches remain in immutable evidence storage; Restate
carries bounded references, not a workflow per performance or an entire state's rows per result.

Keep immutable run semantics distinct from execution budgets. A larger budget resumes owed work;
it does not change completed step meaning, erase exhaustion or mint replacement jobs. Restate
owns three total external attempts; transports make one. Account conservatively for an uncertain
network attempt across a crash, without claiming exactly-once HTTP or adding another scheduler.

Apply shared per-origin physical admission across endpoints/workflows/tabs, Retry-After, redirect
SSRF checks, response/parse limits and browser human handoff. Own and bound async/blocking tasks;
stop intake, drain/finalize and persist exact mutually exclusive outcomes on shutdown.

## 5. Identity, marks, affiliations and contacts

Implement the raw/validated/candidate/accepted separation [ARCHITECTURE.md](../ARCHITECTURE.md)
§8–9 defines, and constrained construction/deserialization. Provider ownership, independent source
families, temporal cohort,
contradictions and reversible decisions survive every consumer. Names/scores/cohort alone never
accept identity. Review packets preserve structured attribution; token-bag hashing is not equivalent.

Clean deterministic decisions bypass AI. Ambiguous cases require independent, evidence-bound advice
from both approved local GPUs and Rust adjudication. Failed/malformed/disagreeing/insufficient advice
remains `REVIEW`; decisive conflicting evidence cannot be silently truncated to fit a packet.

Retain exact source mark units/precision, comparison context, historical team, rounds/heats/attempts,
relay-versus-split meaning and source-declared PR versus observed best. Share one compatible-key and
tie policy across all consumers. Ambiguous historical units require explicit versioned maintenance
and quarantine, never magnitude-based guessing during fresh acquisition.

Resolve current coaching role, sport, side/category and affiliation from time-scoped public evidence.
Bind the exact permitted mailbox to the eligible role; do not fabricate patterns or treat unknown as
both. Persist successful-empty, failed/blocked and never-attempted contact outcomes distinctly.

## 6. Snapshot-bound derivation and publication

Derive all report/workbook/audit outputs from one captured input generation. Current independent
`Store::scan` snapshots and live sequence numbers are not a durable restart boundary. Freeze inputs
in a recoverable form with store/run/schema/policy lineage; derived writes must not invalidate their
own source generation. Reuse one bounded immutable export dataset rather than rebuild decisions
and cohort/PR/contact joins per sheet.

Keep no-performance athletes, accepted aliases versus candidate statuses, unknown joins, Core versus
All evidence filtering and geographic/cohort scope distinct. Use performance-team/date for historical
affiliation, not present athlete school. Measure scan/decode/sort/render/write phases; selected reads
that traverse all keys are not point indexes. Add persistent indexes only for measured query need;
workbook order may require bounded sorting regardless of athlete-keyed access.

Stage workbook, sidecars, audit and manifest together. The manifest binds run/input generation,
scope/cohort/as-of, schema/policy revisions, exact record/partition identities, counts, lengths,
digests, coverage and PR winners. Flush and independently verify before atomic fenced promotion.
Failed/stale exporters retain the previous valid bundle. Define spill, MVCC, receipt and old-generation
retention with explicit resource budgets; no naked live-directory copy or unbounded memory cache.

### Workbook publication fencing and recovery

The workbook publisher persists a request keyed by canonical publication namespace, frozen input
generation and recipe. Its expected `current` root is captured on the first attempt and is never
refreshed by a failed compare-and-set. `current` must be an owned relative
`generations/<lowercase-sha256>` symlink selecting a real, independently verified directory.
Rendering, artifact hashing, semantic readback, bottom-up file/directory finalization, immutable
no-replace installation and generation-directory fsync run outside both publication fences. Open
directory read pins retain the verified bundles while the publisher makes its small fenced writes.
Managed bundles receive full frozen-input, semantic sidecar and workbook verification before
installation. Publication and recovery pins recheck their exact manifest, inventory and artifact
digests against the immutable generation identity without reopening a second complete dataset.
Standalone readback and sealing still perform semantic verification.

The lock order is **namespace publication lock → append/input fence → Intent SyncData commit →
same-filesystem atomic symlink rename → publication-directory fsync → Published and receipt
SyncData commit**. No path acquires those locks in reverse order. The input fence binds lease epoch,
store/run lineage, frozen-input token and recipe; it compares the captured evidence/derived
generations and the small run-manifest binding rather than hashing tables while locked. This is a
conservative aggregate-generation fence: a dedicated audited per-input-table semantic-revision API
is not claimed. The full input digest is checked against a captured snapshot before rendering and
again after rendering, outside both fences.
The run-manifest digest is also captured outside both locks. `Store::bind_run` / `run::write`
permit only the initial absent-to-bound transition and refuse changes to an existing binding.
Consequently the append-fenced check compares that preflight digest with the persisted request and
checks existence of the existing `run-manifest` meta key: it detects a first binding racing preflight
without rereading or hashing even a large run manifest while fenced. This depends on that immutable
binding contract; a future mutable run API must supply its own audited revision rather than weakening
the guard.
The frozen-input token binds store identity, semantic input generation, run and cohort, not the
physical snapshot sequence. Publication bookkeeping may advance that sequence without changing
the input. A render interrupted before Intent can therefore rebuild from the same semantic input
on restart without refreshing the persisted expected root. A Requested record must pass the input
fence before its first Intent even when a verified desired bundle is already visible.
Publication writes do not change the input counters. The measured `critical_section_us` tracing
field includes lock acquisition, recovery's small state writes and publication fsync/receipt costs;
these operations are not described as free.

Intent is **not** a success receipt. The gn1j output/receipt coupling is satisfied by durable Intent,
explicit uncertainty and the recovery matrix, not by claiming that Fjall and the filesystem share
one fused transaction. After a successful rename, any directory-fsync or receipt failure returns
`PublicationDurabilityUncertain`, retains Intent and both generations, and never rolls back a
visible verified bundle. A successful reply names the actual receipted bundle through its canonical
generation workbook path, including historical idempotent success.

Recovery independently validates and pins retained roots outside the locks, then checks the pointer
and active record again under the namespace lock before any other publisher proceeds:

| Durable state and selected root | Typed result and policy |
|---|---|
| Intent; desired verified bundle selected | Fsync publication directory, commit Published + receipt; `DesiredPublished` |
| Intent; expected root still selected | Recheck input/lease; retry the same Intent (`ExpectedRetried`) or mark it `Superseded` (`ExpectedSuperseded`) |
| Intent; another verified root selected | Do not overwrite it; mark the historical Intent Superseded; `AnotherRootSelected` |
| Pointer missing, corrupt or not owned | Validate retained roots, preserve all evidence; `RecoveryRequired`; operator must explicitly repair the pointer, no automatic choice |
| Published receipt; later verified root selected | `HistoricalSuccess`; never restore the historical root |
| Published receipt; receipted bundle missing/corrupt | Persist an integrity incident, preserve the bad bytes and old accepted output; `IntegrityIncident`; publication and sealing remain blocked until explicit operator remediation |

The first filesystem qualification is Linux on a local Unix filesystem supporting safe no-replace
directory installation, exclusive/shared file locks, relative symlinks, same-filesystem atomic
rename and file/directory fsync. Unsupported filesystem operations fail closed. Network filesystems,
cross-filesystem publication and Windows have no copy, delete, or no-op-fsync fallback. Cold backups
must include publication requests, active intents, receipts, retained bundles and integrity-incident
evidence; the new records live in the existing Fjall meta keyspace, and directory evidence remains
inside the publication root.

### Required workbook information

These are semantic requirements, not a claim that today's sheet/column layout implements them:

| Projection | Required content |
|---|---|
| Athletes | Stable accepted ID, names, source profiles, source-backed graduation/cohort decision, current evidenced school/location, category, TF/XC/indoor/outdoor observations, events/best summary, history coverage, conflicts/review and snapshot identity; no-performance athletes retained |
| PRs | Athlete + compatible context, exact value/unit and display mark, supporting performance/source-claim IDs, date/meet, timing/wind/specification, policy, ties, source-declared versus observed-best status and completeness |
| Performances | Declared cohort denominator; athlete/source subject, meet/event, team at performance date, raw/exact normalized mark, conditions/status/round/heat/attempt/place, result/capture locators and identity-decision revision; relay team versus individual split explicit |
| Contacts | Current TF/XC coach/AD and preferred permitted contact, exact mailbox/role/program binding, resolution state, source/date/freshness and school athletics URL |
| Supporting detail | Coaches, Schools, Meets, Sources, Coverage, Conflicts, Review and Run Metrics; complete normalized profile/evidence detail when a cell cannot hold it |

Observed, not-observed and unknown participation must not imply the same fact. Any optional publicly
reported recruiting GPA retains scale/source/date; never infer GPA or import private admissions data.
Use filters, frozen headers, useful widths and explicit units. Detect Excel row/column/cell/hyperlink
limits before publication and partition **every** potentially oversized sheet without dropping IDs,
rows or provenance. Validate hyperlinks and write hostile formula-prefixed text safely, including CSV.

### Independent readback oracle

Read the actual delivered files, not only intermediate JSON or the exporter's own row builder.
Share validated primitive/schema contracts, not the projection logic under test. Verify every row
using bounded sorted keys/exact multisets and partition manifests; samples/totals are insufficient.

1. Each accepted athlete ID appears once in the primary cohort projection.
2. Withheld/unresolved candidates are accounted for outside accepted identities.
3. Every performance has valid athlete/source/event/meet references and historical affiliation.
4. Every PR has exact admissible support in a compatible performance or explicit source claim.
5. Rejected/ambiguous values never contribute to calculated bests.
6. Every cohort acceptance has source-backed graduation evidence.
7. Every contact mailbox is permitted, current-role-backed and traceable.
8. Source failures and unfinished/terminal gaps reconcile with coverage.
9. All 49 planned jurisdictions have correct statuses; unknown location is separately named.
10. Exact partition records, counts, lengths and hashes reconcile with the manifest.
11. This run consumed no seed workbook: verify source-discovery obligations, not fictional input rows.

Repeat required verification against the restored backup. Equal totals with different ownership or
units fail. Distinguish execution conformance from real-world coverage: an approved terminal access
gap may be honest, while an unwired stage, skipped required scenario or failed artifact oracle is an
engineering blocker. Never remove failed sources from the declared manifest to obtain a seal.

## 7. Ordered stage exits

| Stage | Required exit before dependent expansion |
|---|---|
| 0 — Run contract | Fresh store/namespace bound; 49-jurisdiction scope, immutable semantics and acceptance fixed |
| 1 — Source entry | One real public discovery path with explicit bounded obligations/cursors |
| 2 — Durable evidence | Capture-to-claim provenance and actual apply-once commit/lost-ack recovery |
| 3 — Decisions | Typed deterministic identity/cohort/mark/contact rules plus evidence-bound dual review |
| 4 — Vertical proof | One source-to-published-output slice passes exact independent readback and restart |
| 5 — Breadth | Extend the same implementation across provider families/jurisdictions; record limitations |
| 6 — Publication | Shared recoverable input generation, bounded derivation and fenced atomic bundle |
| 7 — National result | All 49 obligations terminal; exact populations/coverage and retained findings reconcile |
| 8 — Assurance | Required tests/proofs/faults/security/load/backup restoration have actual evidence |
| 9 — Release | Validated seal binds tested build, run, artifacts and coverage limits; no unresolved blocker |

Coaches-sheet distinct counts name the entity. A count of schools that have at least one coach
row is distinct schools, not distinct coach names or distinct coach IDs. The workbook publishes
those three figures separately.

### 7.1. Beads execution graph — 2026-10-01

`athletic-rust-pipeline-7lx` owns the complete product target: one accepted canonical Class-of-2027
athlete with all supported compatible event PRs, current eligible coaching contacts, school/address
and source evidence across the contiguous 48 states plus DC. Candidates and source-backed subjects
without results remain accounted for separately; a name match or two agreeing models cannot
manufacture admissible identity evidence. The reported Adelyn Spann duplicates, unmapped boys
110 m hurdles event and six-state workbook filter are diagnostic inputs, not permission for
writer-only deduplication or invented national coverage.

The planning reconciliation created 60 concrete Beads contracts, including one per native fault,
and reused the existing defect/qualification issues. Every new contract passed the installed
planner CUE schema. This validates plan structure, not production behavior or a time estimate.
Beads holds ownership, input/output contracts, exclusions, executable acceptance and handoff.
The component labels below organize work; the prerequisite graph and the ordered exits above
determine execution. Broad source qualification slices must expose any newly demonstrated
adapter/caller gap as a concrete owned obligation before implementation expands.

Unfinished audit children no longer wait on the audit parent that itself waits for their completion.
Grouping uses labels/nonblocking relations. Actual task prerequisites use `blocks`; this installed
Beads build rejects epic-to-task `blocks`, so the product and directory milestones use `tracks`
and remain blocked until their tracked acceptance completes. `1qp` is the executable release gate.
Historical notes and closed evidence are preserved. Stale in-progress claims were reset to open,
not declared fixed. Superseded queues include `cni` → `6yj` and the already
implemented coach-admission issue `6yj.13` → `gqk`.

All IDs in the following tables have prefix `athletic-rust-pipeline-`.

| Component | Concrete work and Beads |
|---|---|
| Identity | `oxy`: exact duplicate/source-owner inventory; `t1b`: admissible same-provider application; `46r`: positive cross-source corroboration ([ADR-022](adr/ADR-022-admissible-cross-source-corroboration.md)); `61k`: stable/reversible aliases and contradiction checks; retain WIAA acceptance `6yj.2` and source-cohort ownership `6yj.12` |
| Events and PRs | `pmv`: reported 110 m hurdles ontology; `7ok`: exact compatible winners across projections; `947`: rounds/relays/historical affiliation; retain wrapped-header acceptance `6yj.14` and closed retained-mark evidence `1yx` |
| Coaches, schools and addresses | `8bs`: implementation-versus-executed-family inventory; `7vk`: source-backed school mapping ([ADR-023](adr/ADR-023-school-link-mapping.md), delta over landed [ADR-021](adr/ADR-021-school-address-join-durable-stage.md)); `rh3`: current eligible program/contact join ([ADR-024](adr/ADR-024-coach-contact-tenure-admission.md); gaps `2b1` emission, `0hx` claim binding, `df2` research outcomes, `jb2` cell traceability); `q2w`: missing school-address CLI; `3c5`: directory-to-census address join; `vxz`: remaining corpus integration; `vxz.1` and `9rj`: explicit live-qualification blockers |
| Durable application | `6ly`: journal reconciliation/application; `a81`: native independent dual-model review; `hf2`: complete bounded attributed packets/advice reuse; `5jv`: discovery frontier/gaps; `sbv`: capture/apply-once chunk boundaries; `w4j`: aggregate physical admission; `o99`: selectively qualify engine admission precedence |
| Source breadth | `omk`: qualification/registry/durable-dispatch matrix; `loa`: MileSplit first vertical source; `afi`: headed Athletic.net; `3bo`: relevant TFRRS/DirectAthletics; `ead`: AthleticLIVE/Hy-Tek/RaceDay/linked files; `616`: regional associations; `9tq`: official school/contact/history links; `o8x`: existing Arbiter caller/applicability integration |
| Frozen publication | Retain `6yj.10`: recoverable inputs and fenced atomic bundles; `45a`: one accepted athlete with joined details; `csv`: independent complete artifact readback; `mtg`: every-sheet limits/partitions and hostile text |
| Fresh national result | `99o`: actual source-to-output vertical qualification with S01/S06 recovery before breadth; `ai4`: separate fresh 49-jurisdiction run binding; `usr`: execute all qualified source obligations; `7x8`: exact national coverage/history/contact denominators |
| Assurance | `tqu`: every named canary (1–18, 20–24) plus suffix fixpoint; `37q`: hostile parser/persisted-shape boundaries; `nno` (closed 2026-10-06: mutation assurance retired with the gate lane); `7w5`: reconcile twelve-kernel requirement with eight-kernel wrapper and prior no-expansion direction; `g42`: security/async ownership; `710`: representative phase/resource measurements |
| Existing defect exits | `cj6`: framed tuples; `98i`: checked confidence deserialization; `grl`: browser schemes; `psx`: proof verdicts; `8b7`: Criterion/fail-closed comparisons; `zi3`: actual adapter source-test selection; `2yq`: drain ownership; `06o`: symlinked restore parents; `1ia`: batch append/replace ordering; `trs`: full loaded native-recovery diagnostic; `c5g`: remaining owning-doc consistency |
| Release and repository | `zr8`: isolated native fault resources; `6yj.11`: all-17 fault gate; `6yj.7`: comparable baseline blocker; `6yj.9`: final integrated independent review; `6yj`: consolidated assurance gate; `1qp`: national gate/seal/user bundle; `h0b`: preserve/extract qualified dirty work and owner-approved branch retirement |

The first useful work is current-main acceptance for P0 `cj6`, alongside `oxy`, `pmv`, `q2w`,
`8bs`, `7vk` and the distinct ready foundational defects. Landed-looking fixes require their exact
consumer acceptance before closure; do not repeat the old failure solely to confirm it. Main owns
identity acceptance and shared domain/store/Restate contracts. SOL-only coding uses Main or the
explicitly authorized `sol-reviewer` selection with exclusive files, never older model substitutions.
No source breadth or national-run activity substitutes for the joined vertical output.

### 7.2. Individually tracked native faults

The fault catalog remains authoritative for injection boundaries and required subcases. These
Beads do not narrow it, certify historical runs or replace real native faults with unit simulations.
`zr8` must establish owned bounded filesystems, isolated clock/VM resources and reached-boundary
instrumentation; no shared workstation reboot, host clock change or shared GPU-server termination.

| Scenario | Bead | Required real surface |
|---|---|---|
| S01 | `ew2` | Endpoint SIGKILL, same invocation, all required capture/apply/ack boundaries |
| S02 | `8o5` | Native Restate SIGKILL during active fan-out |
| S03 | `9j7` | Actual isolated persistent-disk VM reboot with different boot IDs |
| S04 | `zzn` | Distinct compatible V1/V2 binaries and in-flight handover |
| S05 | `lwx` | Physical 429/Retry-After/500 and browser challenge outcomes |
| S06 | `3ct` | Capture/effect/receipt/lost-ack boundaries and content conflicts |
| S07 | `yxq` | Concurrent workflows requesting one logical source unit |
| S08 | `5oi` | Multiple endpoints sharing one aggregate origin budget |
| S09 | `ob8` | Real ENOSPC on the owned bounded Fjall filesystem |
| S10 | `9ir` | Real ENOSPC on the separate owned Restate filesystem |
| S11 | `ahz` | Parent stop with async/blocking workers and exact drain accounting |
| S12 | `9bk` | Actual isolated midnight crossing with stable run semantics |
| S13 | `lie` | Owned proxy around real model HTTP and advice-persistence crashes |
| S14 | `3as` | Each unmet seal item and interrupted publication promotion |
| S15 | `66h` | Nonempty cold backup/restore on a store that has published (`out/publication/current` present), corruption and interrupted destinations |
| S16 | `5gn` | Real frozen capture/advice replay, quarantine and interrupted export |
| S17 | `6gg` | Reached in-flight worker batch and deterministic partial-page chunks |

### 7.3. Branch/worktree disposition — main `02d13802`

The owner's cleanup response requested inspecting branch contents first. No branch/worktree was
deleted, merged, cherry-picked, committed or pushed in this planning pass. Ancestry alone cannot
authorize removal of uncommitted work, ignored artifacts or capture evidence. `h0b` owns any later
preservation, qualified extraction and explicitly approved retirement.

| Local branch | Observed disposition |
|---|---|
| `coach-acquisition-rust` | Eleven non-patch-equivalent commits; scanner/labels/dates/drain already harvested or superseded. Main has Arbiter acquisition code but lacks qualified CLI/Restate callers and four-state applicability: selectively adapt under `o8x`. Do not merge identity/proof relocation, old docs or branch vet exemptions. Clean linked worktree retained. |
| `dragonfly-coach-directories` | One ancestry-unique commit `4d0b962c` is patch-equivalent in main. NC/GA/IN adapter, fixtures/goldens and callers are present; main admission behavior is stronger. No production merge needed. Dirty untracked tests/helpers/fixtures retained. |
| `port-python-to-rust` | No commits ahead; dirty worktree includes the missing `SchoolAddress` CLI variant/store-free route. Selectively restore under `q2w`; current main address/geocode/publication library is stronger than the dirty port. Preserve all remaining edits/captures. |
| `engine-buildout` | No commits ahead; dirty parallel engine models/store facade are unwired and overlap authoritative contracts. Reject a wholesale engine merge. Qualify only admission/halt propagation under `o99`; adapt useful behavioral cases to current APIs, preserving originals. |
| `holzman-5090` | No commits ahead; dirty direct-snapshot bests path is superseded by `ExportDataset`/`Derivation`. Selected-scan helper has no current production consumer. Preserve delta/examples; no speculative performance port. |
| `port-piaa` | No commits ahead; dirty coach changes are older than main's current admission, synchronized school batch and guarded origin path. Preserve delta; no whole-file replacement. |
| `holzman-enforcement` | No commits ahead; untracked report examples remain. Retain until evidence/helper disposition is explicit. |
| `research/captures-20260929` | Two ancestry-unique stash-style commits at `bba998e2`; preserve reference, index snapshot and all capture/fixture evidence. Old code is not a merge candidate; synthetic captures do not establish live qualification. |
| `closeout-python-port`, `holzman-3090` | Merged with clean tracked worktrees. Potential retirement only after ignored/artifact preservation checks and owner-approved scope. |
| `feature/port-acquisition-features`, `piaa-merge`, `restate-spine-alignment`, `strip-inline-comments` | Merged, no linked worktree. Candidates for approved local-ref cleanup; do not delete remote refs implicitly. |

Ten linked worktrees were inspected: main and three other clean tracked worktrees, plus six dirty
worktrees. Whole merges of the two unmerged implementation branches are unnecessary/regressive.
Selective missing callers are planned; preservation is not complete merely because a ref exists.
No national completion or all-17 fault acceptance was established by this code/branch analysis.

### 7.4. Spec-v2 implementation handoff — 2026-10-16

The owner's Downloads specification and the repository copy are byte-identical:
SHA256 `a3b687cc06952dbceaeda2f21300e4769acb96130570fc1174ff70210a09f11e`.
First delivery remains UP qualification followed by Class-of-2027 MI/MN/IA/IL/CO.
The separate fresh 49-jurisdiction qualification is not discharged by either.

Planner session `athletic-spec-v2-readiness-20261016` validated 33 work contracts:
20 new atomic issues and 13 updated existing owners. Beads holds the live claims,
dependencies, exclusive files and acceptance criteria; this table is a routing index,
not a second task list. Planning/schema validation is not runtime acceptance.

All IDs below have prefix `athletic-rust-pipeline-`.

| Spec boundary | Implementation beads | Existing acceptance owner |
|---|---|---|
| Funded identity history, lifecycle bootstrap, atomic graph and semantic revisions (§§5,9,12,14) | `1itf`, `m9ew`, `2002`, `ukkb` | `lzue`, `c945`, `rod5` |
| Frozen proof replay, bounded derivation and workbook generation (§§4,7,8,12,13) | `gnp5`, `t3rc`, `qxit` | `53hl`, `tmdl`, `5lg2` |
| Current acquisition, CLI/Restate callers and native qualification (§§3,6,9,10,16) | `notj`, `w5id`, `769a` | `rod5`, `61w4` |
| Current-main identity, roster, V4, replay-race and parallel-selection regressions (§§5,7–9,12,14,16) | `uz3a`, `pqtb`, `6jkb`, `lzue.1`, `lzue.2` | `lzue`, `rod5`, `iilh` |
| Fenced refresh, offline reparse, indexed no-change and annual reuse (§§6,10,11,14) | `6urc`, `gqqc` | `rod5` |
| Source-backed five-state/UP obligations and actual publication (§§3,4,18,19) | `ck33`, `2ao9` | `1d9h`, UP pilot `99o`; national `ooat` remains separate |
| Capability/document correspondence (§§16–18) | `twwx` | `iilh` |

Archive visibility stays with `6fhk`; OHSAA tenure/replay with `smps`; WIAA
fragment/counter authority with `56ry`. MN staff `098i` and IA Bound `zp10` already
have adapters; their remaining work is qualification/receipts and historical-failure
attribution, not duplicate implementation. MI refusal/school-site work stays with `2gqi`.

The implementation checkout is the registered repository root on `main`.
Verified repairs through `8b15baf9` are recorded in `VERIFICATION-EVIDENCE.md`;
their results do not qualify another worktree or the current-only migration.
The preserved `integration/current-v2-main-20261016` worktree is unmerged and
unqualified: reference/preservation only, not a production editing target or a
wholesale merge candidate. Its prototype receipt-intake SDK is absent from main.
Confirm actual main interfaces before proposing or applying any migration.

The new Domain `public_identity_history::funding` producer port is consumed by
Service `Reservation::into_history_memory`, which retains a genuine host permit.
It does not fund the existing identity projection or history preparation yet;
`1itf` retains that work. The separately tested borrowed JSON cursor is preserved
outside production until typed-codec integration, not shipped as an unused parser.

The older `fix/current-authority-fixtures-20261016` repair overlaps landed R1 work;
do not merge it wholesale or mistake its failing snapshot for current-main status.

Agents start with `bd ready`, inspect `bd show <id>`, then claim one exclusive
boundary. Main freezes shared/persisted contracts and integrates after source
quiescence. No worker runs builds mid-flight, weakens an oracle, changes source
identity to fit a fixture, resets a live store or substitutes a historical binary.
Closure requires the owning exact-build behavior and dependent acceptance lanes.

### 7.5. Latest qualified checkpoints — 2026-10-20

Latest immutable release checkpoint: `b1a63afc12401f5ab20201afaefc3c1f254f31fb`,
tree `358799949642f37d8551d396660278e66ad54fb2`. Controlled50→withdraw51,
unchanged original-A replay and optimized native readback pass. The unrelated NCES
receipt now lives outside the byte-identical frozen roster-kernel module.
Generation09 retry03 saved the real workbook, then was interrupted with SIGTERM
after identifying repeated full-dataset hashing inside row verification (`tzu2`).
The retained diagnostic XLSX and bundle are explicitly unverified; structural
readback confirms all Review records,55,344 coaches and zero selected athletes/PRs.

Integrated source `052b0b0d` passes canonical static gates, all386 adjacent cases
including unchanged original-A replay, controlled50→51 publication, zero-comments,
trusted AST and module seams. National per-row rehash repair (`tzu2`) is next.

Prior integrated static checkpoint: `ddd07ff76f9750a460967471649813da3f1d98b7`,
tree `005e50d310e668b5f7722c1c74741b5602edd381`. Its exact format/check/lint,
201-case report/crawl queue, 27 owner boundaries and original-A replay passed.
Deployed source `d6f6c68ad281cd8e2db8504c4e5732454b76ea2d`, tree
`949a6095327882682b32f4b7136507b02e2c0da4`, adds test-only refusal cases:
four focused cases and portable build passed; predecessor static passes are not
relabelled as this build's passes. Genuine Class2026 cold lifecycle is exercised;
current geographic eligibility and national coverage remain separate obligations.

Caller-authority source: `cc3cb4cf04b113a04c14e1457fbcaa0e2b0530e9`, tree
`6760c4d41e037a10e681be44f79e4f32e3dde6b7`.

Selected-generation source: `dde1949bab82bdde01feb9778fa9960400e8c41b`, tree
`22b56642570a0cb7aa6b1d0b29aa1bb527162c03`. The broader cross-crate checkpoint remains
`e4a207ef05fc0c9f37909132620caa2a58b9c44a`, tree `8ad9927b5aa1cecdf30a31e8fefae83662757592`.
Each row owns its tested build; predecessor passes do not transfer automatically.
Current roster kernels are 12/13; original 2–11 identities and A artifacts remain retained.
[Exact receipts and artifacts](VERIFICATION-EVIDENCE.md#genuine-calendar-revocation-and-cohort-bound-reopen--2026-10-19).

| Requirement | Production entrypoint | Exercised evidence | Tested source | Result / artifact | Next dependency |
|---|---|---|---|---|---|
| A: prior static and structural checkpoint | Canonical Moon format/check/lint; AST, comments, panic and contract guards | `381f88ad`, `2519a875`, `6e52424b`; `3684572c`, `3dcfa7d1`, `33357c78`, `46b7e906` | `d11dcfa4` | PASS; configured advisories and known deviations remain | Later checkpoint identified above; remaining production acceptance |
| R1: controlled eligibility/calendar withdrawal | Public reviewed source → selector → workbook/normalized → cold replay | `57459025` | `a598a5b32` | PASS: 49 excluded, G0=50, withdrawal G1=51, retained proofs and reopen | Genuine Class2027 population; controlled data is not coverage |
| R1: genuine calendar lifecycle | Public acceptance/revoke/original replay → G0/G1/G2 and artifact reopen after source deletion | `534626e7`; owner boundaries `c7403723` | `d11dcfa4` | PASS: original 13.57/2026-05-15 retained, current affiliation Unknown; artifact digests in ledger | Genuine Class2027 population; remaining `d7hp` acceptance |
| R1: review admission atomicity | Actual public prepare/apply with rejected approvals and unchanged owner state | `ac420b35` | `d6f6c68ad` | PASS: four cases cover opaque approval, stale head, foreign Store and six physical-source scope faults | Remaining genuine-source acceptance; no blanket R1 closure |
| R2: historical kernels and current roster consumers | Original three-case A → independently authenticated changed-build B; complete preserved roster queue | `6693e62f`; `7495a7f3` through `79ee9e58` | A `1ab843c4`; B `e4a207ef` | PASS for exercised cases; original A SHA256 `8897f37e…c35908` unchanged | Exact accepted-claim withdrawal `v80o`; broader population |
| R2: original-A changed-build replay | Original retained three-case A → current B and optimized CLI | `0efc0fc9`, Nextest `c6a32768…`; private `releases/b1a63afc/release.json` | A `1ab843c4`; B `b1a63afc1` | PASS; original retention digest and all workbook bytes unchanged; no guard/digest repin | Exact member withdrawal and genuine population |
| R3/R7: selected service consumers | Reserved workbook handler, actual CLI, isolated native Restate | `2501cc57`, `892a4313` | Same production source as `e4a207ef`, precommit client run | PASS for controlled 50→51, split successors, retry/reopen and native restart/revocation; retained workbook hashes in evidence | Remaining `agv2` cohort `6pb6`, receipt `e7lx` and native completed-child/seal acceptance |
| R3/R7: selected National generation | National workbook dispatch → real builder; CLI capability preflight | `4b292d9a`, `10c69620`; actual rebuilt CLI smoke | `dde1949b` | PASS: unchanged G0 path/bytes, withdrawn-G0 refusal, explicit G1=51 and cold G0=50; workbook hashes in ledger | Native completed-child/seal acceptance and remaining `agv2` dependencies; SDK adapter is not a national run |
| R3/R11: explicit caller authority | CLI resume and existing-invocation attachment | `367b4bec`, `9de2ee20`; actual CLI before/after | `cc3cb4cf` | PASS: generation mismatch before resume submission; store/profile mismatch before attachment, including failed-send fallback; omitted flags/budgets preserved | Remaining `agv2` native, cohort and receipt acceptance; no new acquisition identity |
| R3/R8: explicit current startup | Atomic graph-V3/capture-V2 owner; actual CLI/profile/publication and endpoint refusal/cleanup | `d804dd74`, `09162a53` | `d11dcfa4` | PASS: current/historical guards, opaque roster approval/replay; unreviewed affiliation stays Unknown; enrollment-only refusal precedes recovery | Fresh national submitted on `d6f6c68ad`; coverage unqualified, remaining `agv2` acceptance |
| Recovery and owned-process cleanup | Authoritative Requested-state dispatch; graceful native fixture shutdown | `897e8daa`, `892a4313` | `e4a207ef` | PASS; `8v2q` and `lxy2` closed, no SIGKILL fallback | Remaining R12/fault/resource acceptance |
| R2: exact member withdrawal | Existing pure `WithdrawsClaim` reducer and sole roster authority head | Admission-contract review only | — | NOT IMPLEMENTED; parser admission/capture issuance is not accepted membership | Domain-owned authenticated target; no Store→Crawl cycle, `v80o` |
| C/D: complete reusable product | R1–R15, independent discovery and comparison | Spec-linked live Beads | — | NOT QUALIFIED; no national acceptance workbook | UP/five-state/49-state coverage, remaining consumers, reuse/recovery and representative resource evidence |
| D: actual national feedback | Original Restate/serving owner; read-only `Census/status` | Private `diagnostic-live-status-during-a598-build.json` | Live `d6f6c68ad` | 1,630,891 raw athletes, 86,914 source meets, 3,568 canonical marks; NOT accepted-identity or coverage counts | Verified workbook, documentary acceptance, access/engineering gaps; no reset or bypass |
| R5: frozen normalized readback | Eight-family independent readback, grouping and durable input jobs | `88e7a1c8`; initial16 failures repaired by captured-policy catalog binding | `a598a5b32` | PASS80/80; production historical/current validation unchanged | Actual national artifact contents and manifest verification |
| R13: Utah capture URI | Offline native `provider uhsaa` with controlled original-shaped input | `c26c1291`; private `diagnostic-uhsaa-cli-01/{before-27731275,after-a598a5b3}.json` | `a598a5b32` | PASS: rows0/errors1 → rows1/errors0, both requests0; strict ownership/address rules retained | Genuine population/deployment; adapter repair is not statewide coverage |
| R14/D: national diagnostic publication | Same generation09/profile/archive; immutable CLI under16GiB/zeroSwap | Private retry03-unverified receipt and structural readback; XLSX SHA `ba1692b7…11320e` | `b1a63afc1` | INTERRUPTED after3256.642s; peak15,339,282,432B/swap0; real diagnostic saved, no verified publication | `tzu2`: eliminate per-row full-dataset hashing, rerun complete readback; full R14 remains open |
| R12: source cooldown/progress replay | SDK journal, Store reopen, fresh-clock gate and explicit access-evidence withholding | Slot FAIL `96428f30` → PASS `3ebfeea9`; setup FAIL `19f88db9` → PASS `fc274fdc`; full adjacent386/386 `7ecbbffd` | `c4045273`, integrated/tested `052b0b0d` | Known progress survives snapshot failure; failed setup cannot consume or settle an attempt; cached slots replay without acquisition | Broader R12 reuse/fault acceptance; no original live-journal upgrade |

Prior identity, catalog, tie and managed-route closures remain scoped evidence.
Neither checkpoint is blanket national acceptance.

## 8. F01–F15 acceptance ledger

These IDs denote corrections, not the separately dated source-audit findings in the evidence ledger.

| ID | Required acceptance |
|---|---|
| F01 | Exact public mailbox binds to permitted current role; no fabricated address |
| F02 | Homonyms, transfers and reversible cross-source identities preserve attribution and contradictions |
| F03 | Cohort evidence cannot establish identity by itself |
| F04 | Partial/budget-limited work resumes under the same logical identity with apply-once effects; a national submission re-drives owed jurisdictions through stage-resilient `pass` calls and reports each unfinished identity under `owed` instead of publishing it as done |
| F05 | Exact mark/condition normalization and compatible PR winners agree across every projection |
| F06 | Contacts distinguish current/former role, TF/XC and side/category; unknown is not both |
| F07 | Source/program/season obligations and history remain explicit even with no accepted athletes |
| F08 | Historical performance affiliation survives transfers |
| F09 | Deduplication retains legitimate rounds, heats, attempts and provenance |
| F10 | Contact attempt state distinguishes empty, failed/blocked and never attempted |
| F11 | Denominators include unresolved/unfinished coverage; source failure is not absence |
| F12 | Fresh source-discovered population reconciles to generated outputs, not seed-workbook accounting |
| F13 | Raw observations, scope-filtered identities and accepted-cohort counts remain separate |
| F14 | Same-generation atomic publication fences stale writers and retains prior valid artifacts |
| F15 | Representative measured resource/throughput gates fail closed; no invented speedup |

## 9. Named regression canaries

All canaries from the superseded delivery brief remain permanent behavior-test obligations. The
brief and this table share the numbering 1–18 and 20–24, with 19 absent in both, so the table below
names 23 canaries.
Historical mark-shape cases exercise explicit maintenance readers, not a fresh-run import path.
Historical per-canary claims from removed ledgers are not current evidence.
[Verification evidence](VERIFICATION-EVIDENCE.md) owns dated execution; every canary below remains
an obligation regardless of its earlier carrier.

| # | Input/fault | Required outcome |
|---|---|---|
| 1 | Old JSON `TimeSeconds: 60` versus versioned centiseconds `60` | Writer version determines units, never integer appearance |
| 2 | `3-0.75` versus `4-0.00` | Four feet is better |
| 3 | `5-4.00` versus `5-4.25` | Quarter-inch distinction retained |
| 4 | `0.5` versus `0.50` inch | Exact equality |
| 5 | Compatible times differing in the third decimal | Distinction retained until explicit comparison/reporting policy |
| 6 | Same school/name/class/category, different students | No automatic collapse |
| 7 | Same student across transfer | Admissible identity, history and result ownership preserved |
| 8 | Provider IDs `123` in different namespaces | No cross-provider collision |
| 9 | Add a smaller-sorting cluster candidate | Stable public ID or atomic alias transition |
| 10 | A–B and B–C proposed, A contradicts C | No unchecked transitive merge |
| 11 | Swap which subject owns graduation year 2027 | Review evidence identity changes |
| 12 | Reorder an unordered fact set | Review evidence identity unchanged |
| 13 | Fjall commit succeeds, acknowledgement lost | One physical application, reusable receipt |
| 14 | Successful page has zero athletes | Successful-empty receipt, not permanently owed |
| 15 | Same operation key, changed table/payload | Typed conflict, no silent reuse |
| 16 | Empty derived-snapshot rebuild | Stale materialized rows removed |
| 17 | Benchmark parser returns no cases | Performance gate fails |
| 18 | A baseline benchmark is missing | Performance gate fails |
| 20 | S06 is skipped | Release gate fails |
| 21 | Unreached jurisdiction has empty applicability | Unresearched, not verified empty |
| 22 | 4×400 team names athlete without split | Not an individual 400 m PR |
| 23 | Source text starts `=`, `+`, `-` or `@` | Safe text, not executable spreadsheet content |
| 24 | Newest output is failed/partial | Never published |

Retain the discovered repeated-suffix `normalize_name` fixpoint regression as well. No duplicate
counts-only fixture substitutes for these consumer-visible boundaries.

## 10. Fault, proof and release evidence

[The fault catalog](NATIONAL-CENSUS-FAULTS.md) owns all 17 named native scenarios and their required
phase-boundary subcases. Use owned isolated native non-Docker infrastructure, actual production
paths and reached injections. A skipped, unselected, simulated substitute or unmeasured lane is not
PASS. Release retains twelve mandatory proof kernels; source checks and ordinary test results do
not substitute for their verifier certificates. Kernel reconciliation and proof-verdict taxonomy
remain open under `7w5` and `psx`; removed-ledger references do not discharge either obligation.
Moon is the only repository developer entrypoint:
`env -u CI tools/moon-local run pipeline:gate -- --release`. Preserve its source/type, all-feature
test, security, feature-powerset and measured-performance obligations. Separately execute the
required parser fuzz campaigns; passing narrower fixtures does not qualify hostile-input coverage.
`xtask`'s perf commands own measured benchmark baselines. The whole-workspace mutation lane was
retired by owner decision (`5rtr`), so mutation assurance is no longer required; scoped
`cargo mutants` runs remain an unrequired procedure. License enforcement and cargo-vet are
excluded by owner direction; advisory, security and provenance checks remain required.

The current blocking themes are end-to-end durable stage integration, evidence/receipt atomicity,
identity/cohort/contact correctness, exact mark precision and affiliation, complete source obligations,
recoverable shared export inputs, bounded derivation, full artifact verification and atomic promotion.
A completed large export is timing/output evidence, not proof of those invariants. Historic seals,
parser replays and narrower fixture passes do not certify the fresh national run.

## 11. Coach-lane obligation semantics — 49 rows

The run scope has exactly one durable coach-lane obligation row per jurisdiction, per season and per
revision, stored in the derived-map table `coach_lanes` (`Table::CoachLanes`) under the row id
`coach_lane:<state code>:<season>:<revision>`. A lane is not a source sweep, a source outcome or a
contacts stage. It is the per-jurisdiction performance obligation to either produce per-school coach
role/tenure/mailbox evidence or to carry an owner-accepted refusal reason, and it stays owed until
one of those two holds.

| Field group | Required content |
|---|---|
| Identity | Row id, jurisdiction, season, revision |
| Arm | Selected arm name and the recorded basis for selecting it |
| Status | One typed status from the lattice below |
| Refusal | Source reference, reason and observed date when a source outcome or a source plan refused |
| Acceptance | Actor, event and date of the explicit acceptance that closed the lane |
| Evidence | Per-school output/evidence digest, school and contact counts, observation date and optional source URL when armed |
| Resolution | Role, tenure and mailbox resolution state, retrieval date, freshness days and evidence detail |

The status lattice is total. `pending`, `blocked`, `failed`, `exhausted`, `successful_empty` and
`complete_with_gaps` are non-terminal and remain owed; `armed` and `accepted_refusal` are the only
terminal states. A row leaves the non-terminal set only by arming with per-school evidence or by
accepting a recorded refusal. A coarse contacts stage maps onto recorded, non-terminal statuses
(`complete`/`partial` to `complete_with_gaps`, `terminal`/`interrupted` to `blocked`, `exhausted` to
`exhausted`, `handler_failed` to `failed`) and never onto a terminal one. Source outcomes map the
same way: a completed run with zero retained rows records `successful_empty`, a terminal or
interrupted source records `blocked`, an exhausted source records `exhausted`, and a source plan
that refuses a source records `blocked` with the plan's refusal reason.

Closure rule. A merely recorded outcome cannot close a lane: a source that completes with zero
retained rows, a source plan that refuses a source, a blocked/exhausted/interrupted source, and a
coarse contacts stage each record a candidate status and, when available, a refusal — nothing more.
The durable jurisdiction object reports `coach_lane` under `owed` with the row's status and arm
until the row is armed or its refusal carries an explicit acceptance. An `armed` row requires
non-zero school and contact counts, a 64-hex output digest, a valid observation date and a valid
resolution state. An `accepted_refusal` row requires a recorded refusal with a non-empty reason plus
the acceptance actor, event and date, and carries no evidence.

Matrix requirement. Every run closes with exactly 49 rows, each `armed` or `accepted_refusal`; a
missing row, a duplicate row, a row outside `CENSUS_SCOPE`, a row that is still non-terminal, an
armed row without per-school evidence and a refusal without a reason each fail. This matrix is the
strongest form of "all 49 obligations terminal" in stage 7, and it is distinct from coverage or
population accounting: a closed lane is an obligation verdict, not evidence that a state's schools
or athletes were fully observed.

### 11.1. Group A coach-lane rows — implemented with held evidence, 2026-10-11

Per-state lane modules live in `crates/census-service/src/restate_services/wire/coach_lanes/`. `bind`
records each row's missing-source refusal in the durable `SourcePlan` refused set and returns the
obligation stored on `JurisdictionState::coach_lane`; an armed row must name a slug the plan sweeps,
and a capture older than 400 days at lane creation records a `blocked` refusal instead of arming.
These rows are implemented against held evidence with unit tests, not live-run evidence.

| State | Row | Arm | Evidence | Held capture | sha256 | Freshness |
|---|---|---|---|---|---|---|
| CA | `armed` (role derived, tenure unknown, mailbox published) | `home_campus` | 1717 schools / 6758 coach rows, all with email, 0 errors | `var/home-campus-CA-20261004/pass2.log` | `b1658ef1f62f9373fca3b4a1c4bd24b6c87a1e46818bea3ead435374c9da5cd4` | 7 days at 2026-10-11 |
| FL | `armed` (role derived, tenure unknown, mailbox published; capture store holds no request metadata, so the origin is unverified and no source URL is claimed) | `home_campus` | 880 schools / 3677 coach rows, all with email, 0 errors | `var/home-campus-CA-20261004/fl.log` | `34d59e4eb1529b93a64be617e3d84847838c61d9ae86fe2b4a89bbedba0129b9` | 7 days at 2026-10-11 |
| NY | recorded refusal, still owed until an owner acceptance exists | `milesplit` | work-order true-missing set recorded as `ny_missing_coach_sources` | — | — | — |
| TX | recorded refusal, still owed until an owner acceptance exists | `coach_contacts` | work-order true-missing set recorded as `tx_missing_coach_sources` | — | — | — |

The freshness bound is exercised at both ends: 2027-11-08 (400 days) still arms, 2027-11-09 (401
days) records the refusal. TX and NY close only through an explicit owner acceptance.

### 11.2 Coach-lane rows — MO, OK, IA, MA, CT (CoachArmsC) — held evidence, 2026-10-11

- CT `armed` — arm `ciac` (a wired teams arm and the plan's sweepable slug for CT); evidence is the retained capture `research/sources/coach-coverage-bundle-20261004/probes/ciac-fpsports/raw/directory-SchoolLevelID-1.adapter.body`, 673123 bytes, sha256 `a872654b3c9450a484c6f68ae75f9bd32c2523af0f7a2c118f8bb823f1cfc634`, retrieved_on 2026-10-04, 184 schools / 1033 coach rows, resolution published/current/directory-only (the directory publishes phone, not email); the row arms only while the capture is at most 400 days old (boundary 2027-11-08) and CT's remaining sources stay refused as `ct_missing_coach_sources`; module `wire/coach_lanes/ct.rs`, tests `coach_lane_ct_arms_on_the_retained_ciac_capture` and `coach_lane_ct_evidence_digest_is_the_sha256_of_the_capture` (the digest above is asserted byte-for-byte against the held file), `coach_lane_ct_arm_agrees_with_the_probe_manifest`, `coach_lane_ct_arm_is_wired_in_the_teams_arms_table`, `coach_lane_ct_true_missing_reason_is_the_work_order_line`, `coach_lane_ct_stale_capture_records_a_blocked_refusal_without_closing`.
- MO `blocked` with recorded refusal, terminal as `accepted_refusal` on the owner's explicit acceptance — arm `mtccca`; refusal `mo_missing_coach_sources` carries the work order's line for Missouri (`group-b.md:244`) verbatim; module `wire/coach_lanes/mo.rs`, tests `coach_lane_mo_records_the_work_order_true_missing_reason`, `coach_lane_mo_lane_records_the_refusal_without_closing_the_row`, `coach_lane_mo_owner_acceptance_closes_the_row`, `coach_lane_mo_lane_matches_the_shared_registry_signature`.
- OK `blocked` with recorded refusal, terminal as `accepted_refusal` on the owner's explicit acceptance — arm `ossaarankings`; refusal `ok_missing_coach_sources` carries the work order's line for Oklahoma (`group-b.md:272`) verbatim; module `wire/coach_lanes/ok.rs`, tests `coach_lane_ok_records_the_work_order_true_missing_reason`, `coach_lane_ok_lane_records_the_refusal_without_closing_the_row`, `coach_lane_ok_owner_acceptance_closes_the_row`, `coach_lane_ok_lane_matches_the_shared_registry_signature`.
- IA `blocked` with recorded refusal, terminal as `accepted_refusal` on the owner's explicit acceptance — arm `bound`; refusal `ia_missing_coach_sources` carries the work order's line for Iowa (`group-b.md:301`) verbatim; module `wire/coach_lanes/ia.rs`, tests `coach_lane_ia_records_the_work_order_true_missing_reason`, `coach_lane_ia_lane_records_the_refusal_without_closing_the_row`, `coach_lane_ia_owner_acceptance_closes_the_row`, `coach_lane_ia_lane_matches_the_shared_registry_signature`.
- MA `blocked` with recorded refusal, terminal as `accepted_refusal` on the owner's explicit acceptance — arm `mstca`; refusal `ma_missing_coach_sources` carries the work order's line for Massachusetts (`group-b.md:356`) verbatim; module `wire/coach_lanes/ma.rs`, tests `coach_lane_ma_records_the_work_order_true_missing_reason`, `coach_lane_ma_lane_records_the_refusal_without_closing_the_row`, `coach_lane_ma_owner_acceptance_closes_the_row`, `coach_lane_ma_lane_matches_the_shared_registry_signature`.

### 11.3 Group A coach-lane rows — MI, WA, AZ, VA, NJ, IN, LA (CoachArmsB) — held evidence, 2026-10-11

All seven rows record a `blocked` refusal and stay owed until an explicit owner acceptance exists; none arms, because no wired coach arm for these states retains per-school coach role/mailbox evidence and each held source is school-level, name-only or empty. Arms and refusal slugs: MI `coach_contacts` / `mi_missing_coach_sources`; WA `milesplit` / `wa_missing_coach_sources`; AZ `aia` / `az_missing_coach_sources`; VA `milesplit` / `va_missing_coach_sources`; NJ `home_campus` / `nj_missing_coach_sources`; IN `coach_contacts` / `in_missing_coach_sources`; LA `milesplit` / `la_missing_coach_sources`. Each refusal reason is the byte-exact `group-a.md` "True missing" line for that state, asserted against the work-order file by each module's `coach_lane_<st>_true_missing_reason_is_the_work_order_line` test, and each row is registered in `wire/coach_lanes.rs` with `kind: RefusalKind::EngineeringGap` so the slug also lands in the durable `SourcePlan` refused set.

- MI `blocked` with recorded refusal — arm `coach_contacts` (the registry's Michigan coach-mailbox surface, refused by the plan as an engineering gap); the held MHSAA bodies are school-level coaching-tab HTML and the MITCA probe rows are name-only, so no retained body carries per-school coach mailboxes; module `wire/coach_lanes/mi.rs`, tests `coach_lane_mi_records_the_true_missing_refusal_without_closing`, `coach_lane_mi_true_missing_reason_is_the_work_order_line`, `coach_lane_mi_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_mi_arm_is_not_claimed_sweepable_by_an_arming_lane`.
- WA `blocked` with recorded refusal — arm `milesplit` (the only Washington surface the plan sweeps with names); the held finalforms rows carry athletic-director names without coach emails and the association bodies are directory/state-partition links; module `wire/coach_lanes/wa.rs`, tests `coach_lane_wa_records_the_true_missing_refusal_without_closing`, `coach_lane_wa_true_missing_reason_is_the_work_order_line`, `coach_lane_wa_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_wa_arm_is_the_wired_milesplit_source`.
- AZ `blocked` with recorded refusal — arm `aia` (wired and sweepable for Arizona); the held AIA search JSON is school-level with no per-school coach rows, `extract/158-aia-json` is empty and the authenticated admin email directory is restricted; module `wire/coach_lanes/az.rs`, tests `coach_lane_az_records_the_true_missing_refusal_without_closing`, `coach_lane_az_true_missing_reason_is_the_work_order_line`, `coach_lane_az_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_az_arm_is_the_wired_aia_source`.
- VA `blocked` with recorded refusal — arm `milesplit`; the held VDOE and VISAA bodies are school lists only and the VHSL `qualified.json` body is partial; module `wire/coach_lanes/va.rs`, tests `coach_lane_va_records_the_true_missing_refusal_without_closing`, `coach_lane_va_true_missing_reason_is_the_work_order_line`, `coach_lane_va_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_va_arm_is_the_wired_milesplit_source`.
- NJ `blocked` with recorded refusal — arm `home_campus` (wired and sweepable for New Jersey); the held `var/home-campus-NJ-20261004/run.log` records processed 0 schools (0 coach rows, 0 with email, 1 error: http status 405 for the cifsshome directory endpoint), the NJSIAA bodies are AD and school rows and the GMC TCA extract is effectively empty; module `wire/coach_lanes/nj.rs`, tests `coach_lane_nj_records_the_true_missing_refusal_without_closing`, `coach_lane_nj_true_missing_reason_is_the_work_order_line`, `coach_lane_nj_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_nj_arm_is_the_wired_home_campus_source`.
- IN `blocked` with recorded refusal — arm `coach_contacts` (refused by the plan as an engineering gap); `extract/SRC-114/rows.csv` is a 108 B header-only body, the Indiana DOE extract is a school roster without coach columns and the IATCCC tap retained robots only; module `wire/coach_lanes/in.rs`, tests `coach_lane_in_records_the_true_missing_refusal_without_closing`, `coach_lane_in_true_missing_reason_is_the_work_order_line`, `coach_lane_in_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_in_arm_is_not_claimed_sweepable_by_an_arming_lane`.
- LA `blocked` with recorded refusal — arm `milesplit`; the held LHSAA registered-coaches extract (SRC-215, 2025-09-22) carries XC coach names without mailboxes for the completed season and the coaches-directory PDF route (SRC-212) is a documented text-extraction refusal; module `wire/coach_lanes/la.rs`, tests `coach_lane_la_records_the_true_missing_refusal_without_closing`, `coach_lane_la_true_missing_reason_is_the_work_order_line`, `coach_lane_la_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_la_arm_is_the_wired_milesplit_source`.

### 11.4 Group C coach-lane rows — OR, SD, UT, NV, VT (CoachArmsD) — held evidence, 2026-10-11

Modules live in `crates/census-service/src/restate_services/wire/coach_lanes/` and are registered in the shared
`LANES` table (`bind` records each row's missing-source refusal in the durable `SourcePlan` refused set with
`RefusalKind::EngineeringGap` and returns the obligation stored on `JurisdictionState::coach_lane`; an armed row must
name a slug the plan sweeps, and a capture older than 400 days at lane creation records a `blocked` refusal instead of
arming). These rows are implemented against held evidence with unit tests, not live-run evidence.

- SD `armed` — arm `bound` (a wired teams arm, `TEAMS_ARMS` → `TeamsArm::Bound`, and the plan's sweepable South Dakota slug); evidence `crates/census-crawl/tests/fixtures/bound/staff-yankton-sd-girls-xc.html`, 49,944 bytes, sha256 `3bdab3bd25bbc3ae5ff06c308922b7bd8480a303877abbfcd3aecf32cd5dc93f`, with the Yankton boys (49,884 B, `55f0a5027ed504c15a2003c8a0eb3de48cee35cff9f615c05ca9972f9d076ccc`), White River girls (49,705 B, `d8d0353e15399fae06426ff15d7ed27834b59be0f255003bc1d3eb42967064f5`) and Faith boys empty-staff (48,167 B, `c4a3d7b76a7d4bebd45ca04e039d6071b7f26f35191fed845ee57d37c76efcf2`) captures enumerated in the resolution note; 3 schools / 6 coach rows (Head Coach and Assistant Coach rows across the Yankton and White River pages), retrieved_on 2026-10-07 (the crawl cache records no fetch timestamp, so the documented offline replay date is claimed), resolution published/current/unpublished because the GoBound staff table publishes role and name but no mailbox; the capture's provenance table in `crates/census-crawl/tests/fixtures/bound/SOURCE.md` carries the same bytes and digests; the row arms only while the capture is at most 400 days old (boundary 2027-11-11) and the remaining work-order gaps stay refused as `sd_missing_coach_sources`; module `wire/coach_lanes/sd.rs`, tests `coach_lane_sd_arms_on_the_retained_bound_captures`, `coach_lane_sd_capture_digests_match_the_held_fixtures`, `coach_lane_sd_arm_is_wired_into_the_plan_and_the_engine_arm_table`, `coach_lane_sd_stale_capture_records_a_blocked_refusal_without_closing`, `coach_lane_sd_remainder_is_the_work_order_true_missing`, `coach_lane_sd_refusal_is_recorded_in_the_durable_plan_path`.
- UT `armed` — arm `uhsaa` (a wired teams arm, `TEAMS_ARMS` → `TeamsArm::UhsaaDirectory`, and the plan's sweepable Utah slug); evidence `crates/census-crawl/tests/fixtures/uhsaa/directory.html`, 484,909 bytes, sha256 `3ed900a99bd11aa577bf15fe975a0920748780756c94ec19f04c28a214152d41` (324 anchors over 162 distinct school ids), with the Alta (458,367 B, `6cf4845fab4f61e2a13322411764547d0e344b63e7ee55b71c97008a8e7418bf`), Murray (456,574 B, `fa37204cfce4e2db4db79d8c7742e52db1903f06bad2c519e435d55025f45d42`) and Herriman (456,554 B, `aba002e9700ac87449f8f65cbabc192769cb1fccc5912aa3200b2371628b6367`) profile captures enumerated in the resolution note; 162 schools / 12 per-sport TF/XC coach rows (five coaches, XC and track for boys and girls), retrieved_on 2026-10-04 per `crates/census-crawl/tests/fixtures/uhsaa/SOURCE.md`, resolution published/current/published (the profiles carry `mailto:` addresses); the row arms only while the capture is at most 400 days old (boundary 2027-11-08) and the remaining work-order gaps stay refused as `ut_missing_coach_sources`; module `wire/coach_lanes/ut.rs`, tests `coach_lane_ut_arms_on_the_retained_uhsaa_captures`, `coach_lane_ut_capture_digests_match_the_held_fixtures`, `coach_lane_ut_arm_is_wired_into_the_plan_and_the_engine_arm_table`, `coach_lane_ut_stale_capture_records_a_blocked_refusal_without_closing`, `coach_lane_ut_remainder_is_the_work_order_true_missing`, `coach_lane_ut_refusal_is_recorded_in_the_durable_plan_path`.
- OR `blocked` with recorded refusal, terminal as `accepted_refusal` on the owner's explicit acceptance — arm `coach_contacts` (the registry's Oregon coach surface, refused by the plan as an engineering gap: the 298-profile corpus lives in the prototype JSONL, not in a durable jurisdiction research frontier); refusal `or_missing_coach_sources` carries the work order's three Oregon true-missing bullets verbatim; module `wire/coach_lanes/or.rs`, tests `coach_lane_or_records_the_work_order_refusal_without_closing`, `coach_lane_or_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_or_accepted_refusal_closes_the_lane_with_the_explicit_event`, `coach_lane_or_registered_route_is_not_a_wired_arm`, `coach_lane_or_remainder_is_the_work_order_true_missing`.
- NV `blocked` with recorded refusal, terminal as `accepted_refusal` on the owner's explicit acceptance — arm `niaa` (Nevada has no registered coach-capable descriptor at all; `niaa-coaches.html` was captured at 0 B and the Southern Nevada coach CSVs are extract artifacts); refusal `nv_missing_coach_sources` carries the work order's three Nevada true-missing bullets verbatim; module `wire/coach_lanes/nv.rs`, tests `coach_lane_nv_records_the_work_order_refusal_without_closing`, `coach_lane_nv_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_nv_accepted_refusal_closes_the_lane_with_the_explicit_event`, `coach_lane_nv_arm_route_admits_no_coach_lane`, `coach_lane_nv_remainder_is_the_work_order_true_missing`.
- VT `blocked` with recorded refusal, terminal as `accepted_refusal` on the owner's explicit acceptance — arm `vpa` (no registered coach-capable descriptor; the held VPA bodies parse as tournament/scoreboard/alignment navigation and the only coach-ish lane is the 4.8 KB school-sites probe); refusal `vt_missing_coach_sources` carries the work order's three Vermont true-missing bullets verbatim; module `wire/coach_lanes/vt.rs`, tests `coach_lane_vt_records_the_work_order_refusal_without_closing`, `coach_lane_vt_refusal_is_recorded_in_the_durable_plan_path`, `coach_lane_vt_accepted_refusal_closes_the_lane_with_the_explicit_event`, `coach_lane_vt_arm_route_admits_no_coach_lane`, `coach_lane_vt_remainder_is_the_work_order_true_missing`.

The freshness bound is exercised at both ends for the two armed rows: SD 2027-11-11 (400 days) still arms and 2027-11-12 (401 days) records the refusal; UT 2027-11-08 (400 days) still arms and 2027-11-09 (401 days) records the refusal. OR, NV and VT close only through an explicit owner acceptance (`accepted(season, revision, observed_on)` = record then `CoachLaneAcceptance { actor: "census-ops", event: "jpx4.accepted_refusal" }`); recording a refusal alone leaves the row owed.

### 11.5 Coach-lane qualification correction — 2026-10-12

NY, AZ, CT and FL retain nonterminal, named work-order refusals until explicit owner acceptance or a real
qualified arm. CT's phone-only CIAC directory and FL's aggregate home_campus log do not supply per-school
current role/tenure/mailbox proof; initial plan binding must not arm from either. CA never arms from the
2026-10-04 pass2.log totals (1,717 schools / 6,758 email rows) or any historical count: those are capture
evidence, not fresh-run closure evidence, so CA records the nonterminal `ca_missing_coach_sources` refusal
with the historical path, hash and deficiency retained in `arm_basis`. Capture counts and historical hashes
remain preserved but are never fresh-run closure evidence. Qualification and reopen commands live in the
bead-close handoffs; current source limitations live in the owning `SOURCE_REPORT.md` files.

## 12. Refactoring and line-reduction plan — 2026-10-18

This section is the canonical refactoring plan within the existing delivery plan, not a
replacement product specification. Main owns shared contracts and integration. Finish the
currently failing source-to-output acceptance slice before starting a broad consolidation.
R1 eligibility-to-PR, R2 dated current affiliation and R3 reversible public identity remain
separate acceptance obligations; an audit, refactor or passing component test closes none.

### Evidence and honest reduction budget

The retained `athletic-rust-removal-audit-20261017.zip` audit reports **677,932 physical
first-party Rust lines in 4,635 files**, plus **111,199 generated vendor lines** excluded
from first-party savings. Its pinned main is `a2f3979ed0b6a7b7d430b296318a2c334138dfec`,
source-equivalent commit `50b02cf25534acd25b5fcd79b3229ebebb66253a`, tree
`c656422ca29b2032f0ee7db7249ea03fc8c94b0e`. Archive SHA-256:
`563f09eda97bd21329c0762d3ac7c081f0164540dc474ef2134412da7a9c90c7`.
Its `AUDIT-SCOPE.md`, `START-HERE.md` and `conditional-subtotal-qa.md` distinguish
source inspection from runtime proof; builds, tests and refactor qualification were not run.
These are historical measured physical counts, including blanks/comments, not a fresh
measurement of this checkout or executable statements.

The later audit's overlap-adjusted **conditional net estimate is 5,700–13,285 lines**,
including a 20–30-line seed-compatibility test reserve. This is neither achieved deletion
nor a guaranteed positive lower bound: remaining correctness, compatibility and test costs
can consume it. It does **not substantiate a 100,000-line removal commitment**. Do not add
older bounded-audit estimates, overlapping helper totals, vendor code or whole rewrite
footprints to it. Revalidate the pinned candidates against the current tree before assigning
work; unpriced larger redesigns remain unpriced.

### Design rule and bounded delivery order

Use DDD ownership and a **pure functional core / imperative shell**: deterministic domain
rules consume explicit values and return decisions; Store owns atomic persistence, Crawl
owns source interpretation, Service owns durable orchestration, and Report owns projections.
Give each business operation one canonical implementation. Thin adapters may differ in
transport/effects, but must not duplicate policy. A wrapper that leaves two competing
engines underneath is not consolidation. Avoid generic frameworks for merely similar code.

| Order / owner | Small first slice and inspected candidate boundary | Required proof before retirement |
|---|---|---|
| 0 / Main + lane owners | Stabilize one genuine-source eligibility → PR → publication path, dated affiliation, and reviewed identity correction. | Preserve rejected/unknown outcomes; verify current and frozen consumers, exact publication readback and bounded full-operation resource usage. Controlled fixtures alone do not establish genuine-source closure. |
| 1 / Service | One equivalent coach-lane construction pair under `crates/census-service/src/restate_services/wire/coach_lanes.rs` and `coach_lanes/`; use a closed descriptor and one pure policy operation only where semantics match. | Identical durable obligation IDs, refusal reasons, freshness and replay outcomes. Keep provider/state-specific rules and independent expected values; expand to other states only after the first slice passes. |
| 2 / Crawl | Trace one checked directory row from `crates/census-crawl/src/ohsaa/checked.rs` through its actual consumer; retire a second interpretation only if still present on the current tree. | Parse-to-end, ownership, partial/blocked outcomes, source bytes and timestamps remain authoritative. Already repaired paths earn no new deletion credit. |
| 3 / Store | Compare `crates/census-store/src/write.rs` and `write_batch/journal.rs`; share only equivalent fallible journal encoding, retaining distinct direct and batch commit shells. | Exact old-producer bytes, chunk boundaries, reserved namespaces, hash checks, crash/reopen behavior and atomicity. Coordinate incoming borrowed-decoder/physical-row work before touching these files. |
| 4 / Service + Report | Route one CLI/service publication operation through the canonical application operation; reuse one immutable generation rooted at `crates/census-report/src/export/dataset.rs`. | Equivalent selected PRs, eligibility exclusions, ties, normalized outputs and workbook readback. Preserve offline publication, logical job keys, source authority and retry ownership; measure memory rather than infer improvement from fewer lines. |
| 5 / owning crate | Consolidate narrowly duplicated test arrangement or pure mechanics after the production keeper is proven. | Preserve each negative, cancellation, tamper, historical and physical-state scenario. Independent expected-output/hash/readback oracles must not be generated by the implementation they test. |

The audit prices the entire coach-lane operation at only **435–835 conditional net lines**,
not its 5,866-line rewrite envelope (`START-HERE.md`). Other rows above are bounded starting
points, not newly estimated savings. Public identity command-shell consolidation and broader
archive/export redesigns follow only after their distinct approval, revocation, re-fencing,
receipt and historical-consumer obligations are demonstrated; do not combine them with the
first small slice.

### Per-slice acceptance, measurement and rollback

1. Record base commit, exact owned paths, callers, canonical keeper, retired path, preserved
   invariants and independent acceptance cases. Reconcile concurrent ownership before editing.
2. Characterize the existing genuine and adversarial behavior. Change one complete operation
   through its consumer; migrate all current callers and remove its superseded execution path
   in the same reviewed change. Preserve required historical semantics explicitly, without
   routing old artifacts through today's policy or silently relabeling versions.
3. Run repository Moon format/check/lint and focused tests, then applicable full/release and
   fault gates before claiming qualification. Record exact revision, commands, exit status,
   skipped prerequisites and remaining acceptance gaps in `VERIFICATION-EVIDENCE.md`.
   Include source-deleted frozen replay, wrong-anchor/tamper refusal and no-mutation checks
   where the slice touches authority or publication. Keep resource limits finite and measured.
4. Re-measure the same tracked first-party Rust population at base and result, separately
   reporting production, tests, generated/vendor and physical/nonblank counts. Report gross
   removed, replacement added and **net** lines; do not count formatting compression, moving
   code across crates, macros/generated tables, deleted tests or duplicated overlap as savings.
   Record complexity, allocation and latency changes separately from line counts.
5. Keep each slice independently revertible before deployment. Preserve original captures,
   IDs, receipts, historical exports and stores. For persisted-state changes, require an
   explicit versioned migration/reader and tested recovery plan before rollout; a Git revert
   alone cannot undo durable effects. Refuse unsupported state without changing its bytes.

Do not remove source/identity checks, independent verifiers, historical evidence or negative
coverage to reach a quota. Main may approve a measured finite resource-limit adjustment;
that is a separately evidenced operational change, not a refactoring saving or policy waiver.
