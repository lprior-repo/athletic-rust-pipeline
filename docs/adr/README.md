# Architecture decision records

Short records of decisions that are shared or expensive to reverse. Agents read these before
proposing architectural change; a superseding decision gets a new record with a new number rather
than an edit that hides the old reasoning. Content here is normative and referenced from
`AGENTS.md` and commit messages.

---

## ADR-001 — Fjall remains the primary store

**Decision.** The census lives in Fjall (version-pinned), in the keyspaces of `ARCHITECTURE.md` §7.
No PostgreSQL, no separate SQL layer.

**Why.** The downstream interface is Excel, not interactive SQL; the workload is an append-heavy
evidence log with prefix scans and snapshots, which Fjall serves without an operational server. A
relational store would add a process to keep alive across a multi-day, crash-tolerant census for no
query capability we use.

**Consequences.** Analytical queries are projections built in `census-report`, not SQL. Schema
changes are migrations with an explicit revision, never silent rewrites.

---

## ADR-002 — Restate owns retries

**Decision.** Exactly one retry owner: the Restate workflow. Maximum three automatic attempts per
failed external operation. The transport performs one attempt per call.

**Why.** Layered retries multiply silently — Restate × HTTP helper × adapter can turn one logical
fetch into 27 wire requests and get an origin to block us. Durable ownership also means a retry
reuses the same workflow identity rather than minting a new logical job.

**Consequences.** Adapter code must not loop on failure; it reports a terminal outcome
(`ARCHITECTURE.md` §5). Retry exhaustion is retained as evidence, and a source failure is never
`NO_MATCH`.

---

## ADR-003 — GraduationYear is the cohort identity

**Decision.** `GraduationYear(2027)` is the durable cohort. Grade is time-scoped evidence
(`GradeObservation { grade, academic_year, source }`), never a canonical fact on its own.

**Why.** Sources publish "JR", "11", "SR", "12" relative to a season they do not always state.
Storing the raw token as identity corrupts the cohort every summer; storing year plus season-scoped
grade evidence keeps both observations supporting one canonical athlete.

**Consequences.** Cohort verification answers "is this athlete Class of 2027", with the grade
observation recorded as supporting evidence. Source query parameters are `Raw*` values until
interpreted.

---

## ADR-004 — Meet-first result ingestion

**Decision.** Acquire whole-meet result payloads wherever a source permits, and derive athlete
identities from those payloads. Athlete-profile acquisition is enrichment, not the primary
result-discovery strategy.

**Why.** One meet payload carries hundreds to thousands of performances with meet, event, mark,
round and school context; the same data through per-athlete profiles costs orders of magnitude more
requests, and the efficiency metric is verified useful records per physical request (§10, §45).

**Consequences.** Ordering is meet value first (championship and qualifier meets before the
remainder), and the per-origin admission budget is spent on payloads, not navigation.

---

## ADR-005 — AI cannot override contradictions

**Decision.** The local Qwen lane may return `SamePerson | DifferentPerson | InsufficientEvidence`
over a compact structured evidence packet. Rust adjudication remains authoritative: AI can never
create a match across a hard deterministic contradiction, and low model confidence yields a review
state rather than a match.

**Why.** Identity is a correctness property: a wrong merge contaminates every downstream record,
while an unresolved case is an honest, retained state. GPU inference is also the scarcest resource,
so it is spent only where deterministic evidence is genuinely ambiguous.

**Consequences.** Every ambiguous case has one stable `ReviewCaseId` derived from the evidence
digest and policy revision, so identical packages reuse prior verdicts. Contradictions are listed
explicitly in the packet; models never see raw HTML.

---

## ADR-006 — Excel is the recruiter query layer

**Decision.** The published product is a workbook with the `Athletes`, `PRs`, `Performances_*`,
`Coaches`, `Schools`, `Meets`, `Sources`, `Coverage`, `Conflicts`, `Review` and `Run Metrics` sheets.
Fjall remains the evidence system the workbook is verified against.

**Why.** Recruiters filter in a spreadsheet; the durable store must stay append-oriented and
auditable. Treating the workbook as a projection keeps both honest and makes §70's reconciliation a
comparison of two countable things.

**Consequences.** Every workbook row maps to canonical stored evidence, counts reconcile by command,
and `Performances` partitions to respect Excel's row limit.

---

## ADR-007 — Crate cut and migration order

**Decision.** `crates/census-service` is decomposed into `census-store`, `census-crawl`,
`census-reconcile`, `census-report` and `census-service`; `census-domain` absorbs the pure types;
`census-review` is new. Migration waves: contract → store → crawl → reconcile/review/report →
service → national run. Each wave ends with the four gates green and a pushed commit.

**Why.** The single crate mixes pure rules with I/O, which is what makes the domain untestable
without a network and lets adapters reach into store internals. The cut follows the dependency rule
in `ARCHITECTURE.md` §4 rather than the file tree.

**Consequences.** Clean cutover: callers are migrated, no re-export shims are left behind. The CLI
binary is renamed to `census-service` in the service wave, with the systemd units and docs updated in
the same commit.

---

## ADR-008 — Rust-only tooling

**Decision.** No Python and no shell scripts as pipeline steps. Research scripts that were mirrored
into `research/` are removed from the repository; their durable artefacts (samples, schemas, reports,
CSVs) remain. Agent ergonomics come from `cargo xtask` verbs.

**Why.** A second language in the repo means a second toolchain to keep working, a second place for
logic to hide, and pipeline behaviour that cannot be unit-tested or fuzzed. The functions those
scripts performed (`merge_coach_fragments.py` → `merge-coaches`, `xlsx-dump.py` → the workbook
writer) already have Rust equivalents.

**Consequences.** New tooling is an `xtask` verb or a subcommand; the working corpus that produced
the research stays outside the repository as a read-only cache.

---

## ADR-009 — Run scope: the 48 contiguous states plus D.C.

**Decision.** `UsJurisdiction` models every state and D.C. The census run requires the 48 contiguous
states plus the District of Columbia — forty-nine jurisdictions. Alaska and Hawaii are valid values
that the run never requires: every value outside the run scope is representable, never run, and never
silently counted in coverage. Territories and freely associated states are not modelled at all, so
`"PR"` fails to parse instead of widening coverage; adding one is an explicit domain change.

**Why.** The census is a national Class-of-2027 recruiting product, and a model that silently omits
values is worse than one that carries them and states the run scope, because coverage denominators
stay defensible.

**Ruling, 2026-09-22 — the run scope is the 49, and no code path narrows it.** The scope is the
forty-nine entries of `UsJurisdiction::CENSUS_SCOPE`
(`crates/census-domain/src/jurisdiction/table.rs:126`, length asserted in
`.../jurisdiction/tests.rs:59`), and `is_in_census_scope` derives from that same constant
(`crates/census-domain/src/jurisdiction/codes.rs:26`), so no second, driftable excluded-list can
disagree with it. An earlier amendment that narrowed the run to a Midwest subset was reversed the
same day and survives nowhere: the scope is the 49 above. No code path narrows it —
`require_census_scope` (`codes.rs:16`) refuses a jurisdiction outside the scope at the national run
(`crates/census-service/src/restate_services/national.rs:73`) and at the CLI boundary
(`crates/census-service/src/cli/mod.rs:261`), and the report path filters instead of assuming
(`crates/census-service/src/report/projection.rs:111`).

**The one jurisdiction default in the tree, and why it is not a scope.** A gather command given
neither `--states` nor `--all-states` covers Wisconsin alone
(`crates/census-service/src/cli/mod.rs:235`) — a one-state quick test, so a subcommand can be
exercised without the scope's request load. It is a convenience, never the run scope: `--all-states`
selects `CENSUS_SCOPE` (`cli/mod.rs:234`), an explicit `--states` list is validated against the scope
(`cli/mod.rs:261`), and the operational path is the service, which opens every jurisdiction in the
scope on its own (`crates/census-service/src/restate_services/open_work.rs:102` sets
`scope = CENSUS_SCOPE`). Restriction flags have no default at all: an empty restriction stays empty
(`cli/mod.rs:248`). Source expansion keeps its own ordering (Wave F of the audit patch program) —
which states' sources are researched and added first — and that ordering is never the run scope.

**Consequences.** Coverage reports name their denominator and their run scope; adding a jurisdiction
means flipping it into the run set, not editing the type. A census built under a different run scope
publishes fewer rows and must be re-exported before it can seal, because the seal compares
the workbook's own coverage sheet against the classifier's rows.

---

## ADR-010 — Migrate the corpus, never rebuild it

**Decision.** The existing Fjall corpus (millions of athletes, tens of thousands of schools, meets
and coaches) is migrated into the new keyspaces in place, with a recorded revision, rather than
re-acquired from sources.

**Why.** Partial progress is the point of the durability requirements: re-acquisition costs days of
source traffic we have already spent, and the evidence is source-identical after migration. Rebuild
is only correct if the migration cannot carry a keyspace honestly — and that would be recorded as its
own decision.

**Consequences.** The store migration runs before the national wave, is verified by count
reconciliation on both sides, and keeps the single-writer rule: nothing else writes while it runs.
