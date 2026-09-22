# ARCHITECTURE.md

The binding architecture and standards for this repository. §-numbers are stable: other documents and
commit messages reference them, so edit a section's content without renumbering it.

## 1. Mission

Build the richest defensible nationwide recruiting census of U.S. Class-of-2027 high-school Track &
Field and Cross Country athletes: discover them across all qualified sources, reconcile duplicate
identities, collect their available athletic histories, calculate comparable PRs, identify every
event they contest, resolve their current high school and its public professional coaching contacts,
retain source profiles, and export a recruiter-friendly workbook with auditable evidence.

The run may take many hours or days. It must be durable, resumable, restart-safe, rate-aware,
bounded, idempotent, observable, decomposable into independent work, ergonomic for coding agents, and
capable of retaining partial progress indefinitely. A crash, reboot, browser failure, source outage,
429, model outage, parser defect, or machine restart must not force the census to start over.

## 2. Scope

- **Geography (§2)**: all 50 states plus the District of Columbia are valid
  `UsJurisdiction` values. The census run covers the **48 continental states plus D.C.; Alaska,
  Hawaii and the territories exist as values but are never run or counted**. Territories are added
  later as explicit jurisdictions, never silently.
- **Cohort (§3)**: `GraduationYear(2027)` is the durable target, never "junior". Grade is
  time-scoped evidence (`GradeObservation { grade, academic_year, source }`) — `2025-2026 → Grade 11`
  and `2026-2027 → Grade 12` both support the same canonical cohort. No source-specific query
  parameter (`JR`, `11`, `SR`, `12`) becomes a cohort fact without season-aware interpretation.
- **Population (§4)**: every source-verifiable Class-of-2027 athlete in high-school TF or XC
  discovered through the qualified source graph — boys and girls, cross country, indoor and outdoor
  track, all legitimate events including relays where membership is evidenced. The workbook is a
  projection, not the population contract.

## 3. Pipeline

```text
                 NATIONAL SOURCE GRAPH
                          |
                  SOURCE DISCOVERY
                          |
                 durable Restate work
                          |
        +-----------------+------------------+
        |                 |                  |
      meets             athletes           schools
        +-----------------+------------------+
                          |
                       evidence
                          |
                         Fjall
                          |
                  deterministic merge
                          |
                 ambiguous identities
                          |
                   local Qwen review
                          |
                  canonical census
                          |
                  recruiting workbook
```

Athletic.net is one source. MileSplit is one source. State associations, timing companies, meet
systems and school directories are sources. The canonical census exists independently of all of them.

## 4. Crate layout (§17)

```text
crates/
    census-domain/     pure types and rules; no tokio, fjall, reqwest, chromiumoxide, restate, xlsx, llama
    census-store/      Fjall keyspaces, journals, snapshots, migration, backup/restore
    census-crawl/      source adapters, fetchers, per-origin admission, browser supervisor
    census-reconcile/  normalisation, deterministic scoring, conflict detection
    census-review/     local Qwen identity-review lane
    census-report/     coverage, bests, PR projection, workbook export
    census-service/    CLI + Restate workflows + bootstrap/supervision
xtask/                 the agent-facing verbs
```

`census-domain` operates entirely on explicit values. Parse external data once at the boundary:
external shapes become `Raw*` types, validation produces domain enums (for example
`RawGraduationValue` → `GraduationEvidence`), and no `String` stands in where the domain knows
something more precise (`CanonicalAthleteId`, `SourceAthleteId`, `GraduationYear`, `AcademicYear`,
`RawMark`, `NormalizedMark`, …).

## 5. Workflow backbone (§7-§9)

```text
NationalCensus
  +-- JurisdictionCensus(<each run jurisdiction>)   (the 48 + D.C.)
  |     +-- SourceDiscovery / SchoolDiscovery / MeetDiscovery / AthleteDiscovery
  |     +-- ResultAcquisition / CoachDiscovery / Reconciliation / GapAnalysis
  +-- MeetWorkflow / AthleteWorkflow / SchoolWorkflow / CoachWorkflow / IdentityReviewWorkflow
  +-- NationalReconciliation / NationalCoverageAudit / WorkbookExport / ArtifactVerification
```

Identities (§8) are deterministic and stable across retries:

```text
jurisdiction:{state}:{season}:{revision}      source-sweep:{source}:{state}:{season}:{revision}
meet:{source}:{source_meet_id}:{revision}     athlete:{source}:{source_athlete_id}:{revision}
school:{source}:{source_school_id}:{revision} review:{evidence_digest}:{policy_revision}
```

**§9 retry model**: one retry owner (Restate), maximum three automatic attempts per failed external
operation, transport performs one attempt. Never stack Restate × HTTP-helper × adapter retries.
Retry exhaustion is evidence (`OperationTerminal`): `Complete`, `NotFound`, `Incomplete`,
`RateLimited`, `HumanRequired`, `RetryExhausted`, `SourceUnavailable`, `PolicyBlocked`. A source
failure is never equivalent to `NO_MATCH`.

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

Fjall is the system of record; PostgreSQL is not part of this census. Keyspaces:

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

```text
candidate discovery → deterministic normalisation → hard contradiction detection
→ deterministic scoring → high confidence ? match : AI review
```

Each ambiguous case gets one stable `ReviewCaseId`; identical evidence packages reuse prior reviews;
both GPU servers process different jobs at once. The model receives a compact structured packet (no
raw HTML), returns `SamePerson | DifferentPerson | InsufficientEvidence`, and Rust adjudication stays
authoritative: AI can never create a match across a hard deterministic contradiction, and poor model
confidence means `REVIEW`.

## 9. Census flow (§46-§48)

A source inventory → B school census → C meet census → D bulk result acquisition → E athlete
discovery → F cohort verification → G athlete enrichment → H coach enrichment → I cross-source
reconciliation → J AI ambiguity review → K gap sweep → L final reconciliation → M export.

Gaps (`MissingGraduationEvidence`, `MissingPerformanceHistory`, `MissingCoach`, `MissingSchool`,
`MissingProfile`, `ConflictingIdentity`, `MissingEventContext`, `MissingPRSupport`) drive a targeted
second pass — this is one census, not a weekly-update architecture. Completion is not "the HTTP queue
is empty": the root workflow finishes only when every work item is terminal, and
`Acquiring → Complete` is not a legal transition.

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

The census may be sealed only when every configured jurisdiction has terminal sweeps; every source
object has a terminal acquisition state; every potential Class-of-2027 athlete has a terminal cohort
decision; every identity candidate has a terminal deterministic or AI-assisted decision; every
unresolved conflict and every retry-exhausted operation is explicitly retained; all successful
evidence is durable; performance and PR calculations are reproducible; every workbook athlete maps to
canonical stored evidence; and workbook counts, source coverage and run metrics all reconcile against
Fjall with a passing final export verification.

## 13. Current state and migration

The code today lives in `crates/midwest-census` (a single crate holding the store, the source
adapters, the identity lane, the report/workbook writers, the CLI and the Restate services) plus
`crates/census-domain` and `crates/g1-audit`. Deps already wired: fjall 3.1.10, restate-sdk 0.12,
reqwest 0.13, tokio, thiserror, serde. The migration onto §4 proceeds in waves — contract and
inventory, store, crawl, reconcile/review/report, service, then the national run — with the gates
green and a pushed commit at the end of every wave. `docs/migration/module-map.md` holds the
file-level cut; `docs/adr/README.md` holds the decisions frozen so far.
