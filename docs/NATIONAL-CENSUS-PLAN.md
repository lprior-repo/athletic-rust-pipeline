# Fresh national census — delivery and acceptance plan

Status: binding unfinished delivery contract, **not execution evidence**. This is the only active
plan. [ARCHITECTURE.md](../ARCHITECTURE.md) owns mission, engineering policy and semantics
(§8–9), and [ADRs](adr/README.md) own decisions.
[Dated verification](VERIFICATION-EVIDENCE.md) records what actually ran.

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
| 8 — Assurance | Required tests/proofs/faults/security/mutation/load/backup restoration have actual evidence |
| 9 — Release | Validated seal binds tested build, run, artifacts and coverage limits; no unresolved blocker |

## 8. F01–F15 acceptance ledger

These IDs denote corrections, not the separately dated source-audit findings in the evidence ledger.

| ID | Required acceptance |
|---|---|
| F01 | Exact public mailbox binds to permitted current role; no fabricated address |
| F02 | Homonyms, transfers and reversible cross-source identities preserve attribution and contradictions |
| F03 | Cohort evidence cannot establish identity by itself |
| F04 | Partial/budget-limited work resumes under the same logical identity with apply-once effects |
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

All 24 canaries from the superseded delivery brief remain permanent behavior-test obligations.
Historical mark-shape cases exercise explicit maintenance readers, not a fresh-run import path.

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
| 19 | Kani selects zero harnesses/wrong package | Proof gate fails |
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
PASS. The release requirement retains 12 mandatory proof kernels; the current
[xtask](../xtask/README.md) wrapper enumerates eight names, so that narrower invocation does not
establish the twelve-kernel requirement. [tools/gate.sh](../tools/gate.sh) and xtask own proof verdict
taxonomy, property/fuzz/mutation/security/async gates and command procedures;
`xtask`'s perf commands own measured benchmark baselines. License enforcement is excluded by owner
direction, not advisory, security, provenance or cargo-vet checks.

The current blocking themes are end-to-end durable stage integration, evidence/receipt atomicity,
identity/cohort/contact correctness, exact mark precision and affiliation, complete source obligations,
recoverable shared export inputs, bounded derivation, full artifact verification and atomic promotion.
A completed large export is timing/output evidence, not proof of those invariants. Historic seals,
parser replays and narrower fixture passes do not certify the fresh national run.
