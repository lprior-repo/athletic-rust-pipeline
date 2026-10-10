# Fresh national census — delivery and acceptance plan

Status: binding unfinished delivery contract, **not execution evidence**. This is the only active
plan. [ARCHITECTURE.md](../ARCHITECTURE.md) owns mission, engineering policy and semantics
(§8–9), and [ADRs](adr/README.md) own decisions.
The dated verification ledger (removed 2026-10-09) records what actually ran.

## 1. Run boundary

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
| `dragonfly-coach-directories` | One ancestry-unique commit `4d0b962c` is patch-equivalent in main. NC/GA/IN adapter, fixtures/goldens and callers are present; main admission/robots behavior is stronger. No production merge needed. Dirty untracked tests/helpers/fixtures retained. |
| `port-python-to-rust` | No commits ahead; dirty worktree includes the missing `SchoolAddress` CLI variant/store-free route. Selectively restore under `q2w`; current main address/geocode/publication library is stronger than the dirty port. Preserve all remaining edits/captures. |
| `engine-buildout` | No commits ahead; dirty parallel engine models/store facade are unwired and overlap authoritative contracts. Reject a wholesale engine merge. Qualify only admission/halt propagation under `o99`; adapt useful behavioral cases to current APIs, preserving originals. |
| `holzman-5090` | No commits ahead; dirty direct-snapshot bests path is superseded by `ExportDataset`/`Derivation`. Selected-scan helper has no current production consumer. Preserve delta/examples; no speculative performance port. |
| `port-piaa` | No commits ahead; dirty coach/robots changes are older than main's current admission, synchronized school batch and guarded origin path. Preserve delta; no whole-file replacement. |
| `holzman-enforcement` | No commits ahead; untracked report examples remain. Retain until evidence/helper disposition is explicit. |
| `research/captures-20260929` | Two ancestry-unique stash-style commits at `bba998e2`; preserve reference, index snapshot and all capture/fixture evidence. Old code is not a merge candidate; synthetic captures do not establish live qualification. |
| `closeout-python-port`, `holzman-3090` | Merged with clean tracked worktrees. Potential retirement only after ignored/artifact preservation checks and owner-approved scope. |
| `feature/port-acquisition-features`, `piaa-merge`, `restate-spine-alignment`, `strip-inline-comments` | Merged, no linked worktree. Candidates for approved local-ref cleanup; do not delete remote refs implicitly. |

Ten linked worktrees were inspected: main and three other clean tracked worktrees, plus six dirty
worktrees. Whole merges of the two unmerged implementation branches are unnecessary/regressive.
Selective missing callers are planned; preservation is not complete merely because a ref exists.
No national completion or all-17 fault acceptance was established by this code/branch analysis.

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
Executed per-canary coverage from 2026-10-05/06 — each carrier, its observed result, and the
canaries that still have no carrier — is recorded in the "Named regression canaries §9 — executed
coverage" section of the verification ledger (removed 2026-10-09).

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
PASS. The release requirement retains 12 mandatory proof kernels, and no harness implements any of
them: the [xtask](../xtask/README.md) command list and `tools/gate.sh` carry no proof, Kani or Verus
command, no kernel registry and no proof verdict taxonomy, no `kani::proof` or Verus source is checked
in, and neither invocation enumerates the eight kernels named as missing in
the verification ledger (removed 2026-10-09), so a release invocation does not establish the
twelve-kernel requirement and cannot fail for its absence. Moon is the only repository developer
entrypoint: `env -u CI tools/moon-local run pipeline:gate -- --release`. Its internal `tools/gate.sh`
and xtask carry the lint/type, all-feature test (property tests and `loom` async models included),
security, feature-powerset, benchmark-presence and measured-performance lanes; no gate lane executes
the `fuzz/` targets; the twelve-kernel reconciliation and the proof verdict taxonomy remain open with
beads `7w5` and `psx`.
`xtask`'s perf commands own measured benchmark baselines. The whole-workspace mutation lane was
retired by owner decision (`5rtr`), so mutation assurance is no longer required; scoped
`cargo mutants` runs remain an unrequired procedure. License enforcement and cargo-vet are
excluded by owner direction; advisory, security and provenance checks remain required.

The current blocking themes are end-to-end durable stage integration, evidence/receipt atomicity,
identity/cohort/contact correctness, exact mark precision and affiliation, complete source obligations,
recoverable shared export inputs, bounded derivation, full artifact verification and atomic promotion.
A completed large export is timing/output evidence, not proof of those invariants. Historic seals,
parser replays and narrower fixture passes do not certify the fresh national run.
