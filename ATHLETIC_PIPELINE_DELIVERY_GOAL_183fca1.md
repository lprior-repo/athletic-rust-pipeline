# ATHLETIC-RUST-PIPELINE
# National Class-of-2027 census: final delivery contract

**Revision:** GOAL-2026-09-24-01  
**Repository:** `lprior-repo/athletic-rust-pipeline`  
**Reviewed branch:** `main`  
**Pinned review commit:** `183fca13dd6d48165d04c76fd56648998e6b432f`  
**Commit time:** September 24, 2026, 15:30:53 UTC / 10:30:53 a.m. America/Chicago  
**Prior review baseline:** `2fbb2f7b91e662640edfa104087a92ec527641bc`  
**Product:** One-shot, resumable nationwide recruiting census, not a weekly synchronization service.  
**Run scope:** 48 contiguous states plus District of Columbia.  
**Cohort:** Graduation year 2027.  
**Architecture:** Rust + native Restate + Fjall + persistent Chromium where required + two existing local review-model servers + audited XLSX exports.

**Authority of this document.** Sections labelled REQUIREMENT define the requested target. Statements labelled OBSERVED describe the pinned source inspected for this review. Statements labelled REPORTED describe repository evidence or a previous execution, not a new execution. Proposed commands, types, files and tests are implementation requirements, not claims that those things already exist. Source references are indexed at the end of this document.

**Review limitation.** This review retrieved current GitHub source and CI metadata and examined the changed high-risk paths. It did not execute the workspace, Kani, a live Restate deployment, a Fjall migration, model requests, the national census or an XLSX verifier. This document is an execution contract and source-based audit, not a successful release certificate. Existing user workbooks and HAR files were not modified or submitted to external models.

### Navigation

| Read next | Sections |
|---|---|
| [Objective, success and current audit](#1-paste-ready-governing-objective) | 1–4 |
| [Architecture and agent loop](#5-freeze-the-architecture-clarify-responsibilities) | 5–7 |
| [Correctness, migration, identity and durability](#8-p0-make-current-quality-evidence-trustworthy) | 8–20 |
| [Kani, testing and measured performance](#21-kani-is-a-release-requirement-not-a-directory-of-intentions) | 21–24 |
| [National sources, browser and coaches](#25-national-source-coverage-finish-the-product-not-just-the-fan-out) | 25–27 |
| [Snapshots, workbook, verifier and seal](#28-run-manifests-and-immutable-snapshots) | 28–31 |
| [Task waves, canaries and handoff](#32-implementation-waves-and-dependency-order) | 32–42 |
| [Pinned source index](#43-source-and-evidence-index) | 43 |

---

## 1. Paste-ready governing objective

Complete the existing `athletic-rust-pipeline` implementation without another architecture rewrite. Preserve the current census crate family and virtual workspace. Repair the release-blocking correctness, migration, proof, measurement and durability gaps documented below. Then execute a one-shot, restart-safe Class-of-2027 Track & Field / Cross Country census across the 48 contiguous U.S. states and D.C., using the union of qualified sources and retained evidence rather than any single provider or seed workbook as the population definition.

Deliver a new recruiter workbook and accompanying evidence package. Each accepted canonical athlete must have a stable identity, cohort evidence, current-school attribution or an explicit unresolved state, city/state evidence, every observed event, all acquired performances, defensible comparable bests, source-reported PR claims kept separately, public athletic profile links, and a current professional coach/contact outcome. Missing results, inaccessible sources, incomplete histories, unverified school affiliations and missing professional contacts must remain visible. Never fabricate a field merely to make a row appear complete.

Deterministic Rust owns ingestion, parsing, normalization, source identifiers, school matching, cohort interpretation, candidate generation, contradiction detection, mark arithmetic, comparison, PR reduction, evidence persistence, caching, coverage and export. AI may advise only on genuinely unresolved identity questions after the evidence is normalized and hard contradictions have been computed. Exactly one configured local model owns each review case. Use both existing model servers across different cases. The deterministic adjudicator owns acceptance; the model does not.

Use Restate as the durable execution backbone and Fjall as the application evidence system of record. Keep source operations bounded, checkpointed and individually accountable. The approved default is three total automatic external attempts per logical operation, including the first attempt. Distinguish this budget from ordinary replay of already journaled work. Do not multiply it through nested transport, adapter, handler or supervisor retries. Respect source access signals and human-required browser states.

Apply TigerBeetle-inspired engineering discipline through precise units, private validated types, stable persistent formats, bounded work in progress, atomic bounded batches, reversible identity decisions, fault injection, proof of critical kernels, and measured performance. Do not claim equivalence to TigerBeetle's assurance level. The standard here is evidence for this implementation, this commit, these bounds and this workload.

Continue the engineering loop until the final acceptance manifest proves completion, or a specific safety, access, ownership or missing-evidence condition requires operator intervention. An ordinary defect is work to repair, not a reason to mark the goal complete. A permanently inaccessible source is an explicit collection gap, not permission to bypass it. A review that lacks evidence is a retained unresolved outcome, not a forced match.

## 2. Success criteria

The following conditions jointly define delivery. A percentage estimate such as “98% complete” is not a substitute for them.

1. The exact release commit builds and passes its configured quality, compatibility, security, dependency and license gates. Required CI checks complete successfully for that commit. A baseline JSON file or a passing ancestor is not evidence for current `HEAD`.
2. Legacy Fjall observations and source artifacts remain readable without unit ambiguity or silent precision loss. The mark migration is versioned, reversible, tested against actual historical representations and performed first on an independently verified copy. Original evidence remains intact.
3. One exact, shared mark interpretation and comparison kernel serves acquisition, PR reduction and export. It preserves the source precision necessary to distinguish legitimate performances. Imperial and metric observations of the same compatible mark reconcile correctly. Invalid or non-comparable results never acquire a numeric PR.
4. Source objects and source rows remain distinct from canonical people. A same-name collision cannot destroy two source identities. A transfer can resolve to one person while preserving school history and each performance's original source owner. Every applied identity merge can be reversed from retained evidence.
5. Every live source operation has a durable owner, a bounded request contract, an explicit terminal disposition and a reusable identity. Restarting workers or Restate does not silently lose operations, reset exhausted budgets or multiply completed logical effects.
6. Applying a source batch to Fjall is idempotent across the crash-after-commit/before-acknowledgement boundary. Observations, accounting metadata and the application receipt commit atomically. A replay returns the recorded receipt without appending duplicate observations.
7. All mandatory Kani harnesses are discovered from the actual build, execute production kernels, and produce successful verification results within recorded bounds. Missing harnesses, zero selected proofs, timeout, OOM, unsupported verification, skipped runs and missing verdicts are not passes.
8. Performance tooling measures the processes and benchmark contracts it claims to measure. Empty benchmark results, missing baseline cases, malformed metrics and incompatible environments fail comparison. Approved performance and memory budgets hold on the designated reference workload and machine.
9. All 49 admitted jurisdictions receive an explicit source/discovery plan. An unresearched state is never equivalent to a successfully empty state. Every acquired or blocked source contributes a named coverage disposition.
10. The final workbook is generated from one sealed, versioned snapshot and independently checked against that snapshot. The primary export includes all approved source evidence, with any core-only view offered separately and clearly labelled.
11. Every emitted athlete has a contact outcome: suitable public professional coach email, public professional AD/department fallback, name-only, unattempted, not found, stale or conflicting. Unresolved identity rows are visible in review output and are not labelled accepted people.
12. Backup and restore are actually executed with the required ownership boundary, the restored store is independently reopened and verified, and the workbook/evidence manifest identifies exact hashes, versions, policy revisions, run identity and source coverage.
13. All intended changes and evidence are committed and pushed through the agreed integration process. The working tree is clean except for explicitly preserved unrelated user changes, and `git rev-list --left-right --count origin/main...HEAD` is `0 0` after fetching the remote. That result is synchronization evidence, not a correctness test.

## 3. Scope and non-goals

The admitted codes are:

```text
AL AZ AR CA CO CT DE DC FL GA ID IL IN IA KS KY LA ME MD MA
MI MN MS MO MT NE NV NH NJ NM NY NC ND OH OK OR PA RI SC SD
TN TX UT VT VA WA WV WI WY
```

There are 49 entries: 48 states and D.C. AK, HI, Puerto Rico and other territories are not run targets. They may remain representable boundary values. Existing observations outside the run scope are preserved but excluded from the primary recruiting projection and its run denominator. `UNKNOWN` is an unresolved placement bucket, never another jurisdiction.

Use graduation year 2027 as the cohort identity. Grade 11 in academic year 2025–26 and grade 12 in 2026–27 are compatible evidence when the source contract uses that academic-year convention. Do not derive a student's actual grade progression from a calendar assumption alone. Reclassification, repeated grades and conflicting source labels stay evidence-bearing exceptions.

Include boys and girls categories as actually published, TF/XC, indoor/outdoor distinctions, individual events, supported combined events, all observed legitimate event variants, and relay participation when membership is evidenced. Do not silently exclude an encountered legitimate event because it is unfamiliar or absent from the current ontology. Any deliberate event exclusion belongs in the owner-approved RunManifest; preserve its source observation and named disposition.

Historical backfill is bounded by a declared source/year window and the target athlete's available high-school history. The run is not a promise to discover every real person in the country or every performance ever recorded. Population completeness requires a justified denominator independent of successfully issued requests.

Non-goals include PostgreSQL, distributed database infrastructure, Kubernetes, Docker, Redis, Kafka, a new `acq-*` crate family, a second parser for the same wire contract, an automatic weekly scheduler, and a universal source framework that erases provider-specific semantics. Preserve useful existing code and remove verified duplication only after replacement coverage is demonstrated.

## 4. Current-main checkpoint: what is actually known

The last branch recheck for this document still resolved `main` to `183fca13dd6d48165d04c76fd56648998e6b432f`. Pin review, migration and test evidence to a commit, not a moving branch name. [R01]

### 4.1 Changes that should stay closed

The root/acquisition product has already been removed and the repository has a virtual workspace. The census crate split and the separate browser effect crate already exist. Do not assign another agent to recreate them.

`StoreBatch` already batches observations from multiple tables and completion metadata into one Fjall commit. The remaining work is application idempotency, correct accounting, bounded batch construction and production routing, not writing another batch framework. [R09]

The inspected meet handoff no longer prewrites the recorded rows locally before returning them for Ingest. This closes the old direct-local-write-plus-Ingest duplication pattern in that path. Do not keep the old recommendation open merely because an earlier audit found it. Check other production callers independently. [R10]

There is now a `kani` runner in `xtask`, and the performance runner has been split into modules. These are implemented surfaces whose correctness and execution evidence must be verified, not absent features to recreate. [R06, R07, R08]

### 4.2 Current release status

GitHub Actions run `36020764525` is associated with the exact reviewed SHA and completed with `failure`. Job `107715786629` failed at `Quality gate`; subsequent all-feature and no-default-feature gate steps were skipped. The log download failed with a credentials error in the review environment. Therefore the observed claim is **the quality step failed**, not an invented diagnosis of its compiler or test error. [R02, R03]

There is no basis in this review to declare the current commit release-ready, fully verified or ready for in-place mark migration. Previously reported census counts, Kani passes and backup runs belong to their recorded commits and snapshots. They are retained history, not current success evidence.

### 4.3 Source-audit findings

| ID | Severity | Observed source fact | Consequence and required disposition |
|---|---|---|---|
| F01 | P0 | Fixed mark wrappers are transparent integer newtypes while their comment claims unchanged decimal JSON. | Version and test persisted mark decoding before writing or migrating more corpus data. |
| F02 | P0 | Imperial PR conversion treats hundredths of an inch as inches in its formula. | Patch the single comparison kernel and add adversarial ordering tests immediately. |
| F03 | P0 | Ingest records its seen-operation receipt in Restate state after an external Fjall write. | Close the lost-acknowledgement window with a Fjall-atomic apply-once receipt. |
| F04 | P0 | The new S06 durability script explicitly skips the crash-injection boundary it describes. | Implement the scenario; its name and comments are not a passing test. |
| F05 | P0 | Review evidence normalization sorts words inside each textual fact. | Replace prose-bag hashing with structured fact hashing that preserves attribution. |
| F06 | P0 | Candidate and canonical singleton IDs still derive from school/name/class/gender. | Separate source/provisional identity from candidate grouping and canonical person assignment. |
| F07 | P1 | The fast Kani runner names proofs not established by the inspected domain wiring. | Resolve actual harness inventory and require nonzero, exact intended execution. |
| F08 | P1 | The performance parser accepts empty results; comparison iterates only returned current groups. | Reject empty/incomplete runs and compare the complete expected benchmark set. |
| F09 | P1 | RSS collection reads an invalid `/proc/self/status/VmHWM` path and can substitute zero. | Measure the correct process or cgroup and represent unavailable metrics as failure where required. |
| F10 | P1 | Source applicability explicitly maps twelve Midwest states and returns no entries elsewhere. | Complete the national applicability/discovery contract without falsely labelling empty plans complete. |
| F11 | P1 | StoreBatch still uses saturating row-count addition and buffers pages without a total byte budget. | Use checked semantic counters and enforce bounded work before allocation/commit. |

These findings are source-based. They are not claims that the reviewer executed the corresponding Rust tests. The code references and independently derived counterexamples below are the basis for the required patches. [R04–R16]

---

## 5. Freeze the architecture; clarify responsibilities

Keep the existing packages:

```text
athleticnet-browser
census-domain
census-store
census-crawl
census-reconcile
census-review
census-report
census-service
xtask
```

Keep `g1-audit` as its existing developer-tool binary rather than recreating a separate product architecture around it. The workspace manifest is the authority on current package names.

The operational flow is:

```text
RunManifest + approved SourceRegistry
                 |
          NationalCensus (Restate)
                 |
       jurisdiction/source/entity work
                 |
      bounded HTTP / persistent Chromium
                 |
    immutable source capture + parse receipt
                 |
        validated source observation batch
                 |
     Restate durable call -> Fjall apply_once
                 |
      source facts + durable application receipt
                 |
        deterministic reconciliation snapshot
                 |
   ambiguous cases only -> one assigned local model
                 |
       deterministic admissibility/finalization
                 |
        canonical recruiting read projection
                 |
   workbook + audit sidecars + independent verifier
                 |
       backup/restore evidence + release seal
```

Restate owns workflow execution, supported retries, durable calls, timers, status and recovery. Fjall owns raw evidence references, observations, source identity, canonical decisions, projections and application receipts. An application receipt is necessary because Restate's journal and Fjall's commit are separate durability domains; it is not permission to build a second task scheduler. [E01, E02]

`census-domain` owns effect-free types and pure decisions. It may not open files, read clocks, perform network calls or depend on runtime infrastructure. `census-crawl` owns source-specific discovery and boundary parsing. `athleticnet-browser` owns the persistent browser/session/CDP lifecycle, not athlete truth. `census-service` composes these into durable production routes. `census-review` owns local model transport and validated advice. `census-reconcile` owns applying evidence-backed identity decisions. `census-report` renders a frozen read model and never quietly repairs source facts.

Do not move files merely to improve a dependency diagram. Shared coverage or rendering metadata can cross an existing boundary when its authority is explicit. Conversely, an independent verifier must not call the exact exporter transformation and call the resulting agreement independent. Reuse public semantic specifications, not the same error-prone implementation twice.

## 6. Agent-facing repository contract

Maintain one short root `AGENTS.md` that points to this governing goal and current architecture ADRs. It should identify the run scope, active blockers, commands that actually exist, ownership rules, privacy rules and the evidence directory. Archive superseded goal dumps as historical material with a prominent non-normative banner. A file called `ENDGAME-GAPS.md` can inform this goal but does not replace explicit acceptance tests.

Each major crate should have a focused local guide explaining its permitted dependencies, input/output types, error taxonomy and tests. Do not make every source agent read the entire formal-verification program before editing a parser. Give the agent its bounded task packet, local guide, shared contract and representative fixtures.

Main owns public types, persistent schema changes, operation identity, run identity, migrations, shared source metadata, integration, final gates and release decisions. A source agent owns only its assigned adapter, fixtures, research report and colocated tests. A proof agent may propose changes to a production kernel but does not silently create a parallel model that behaves differently. A performance agent must preserve invariants and publish before/after evidence.

Use git worktrees or isolated branches for parallel implementation. Record the merge base in every handoff. Do not let agents race edits into a shared checkout, overwrite unrelated user changes, run mutating migrations against the same Fjall directory, or each launch a full workspace gate while the tree is being edited. Integration tests run on a coherent assembled commit.

Up to thirty independent research or fixture tasks can be useful. Thirty simultaneous editors of the same Rust contracts are not. Actual concurrency is limited by disjoint ownership and measured build/resource pressure, not the available agent count.

## 7. How the agent should loop

The goal runner follows a measured repair loop, not an unbounded retry loop:

```text
inspect HEAD and active task ledger
 -> choose the highest-priority ready task
 -> reproduce/record the failure on the intended production path
 -> implement the smallest invariant-preserving change
 -> run the task's focused tests/proofs/benchmarks
 -> inspect the diff and migration implications
 -> integrate on a coherent commit
 -> run affected global gates
 -> update evidence and task state
 -> repeat
```

A new commit invalidates only the evidence it can affect, but the final release suite must run against the final commit. Do not reset every task to open because one source adapter changed. Do not carry a passing mark-parser result across a changed serialization contract.

Every task has a status from this vocabulary: `not_started`, `reproduced`, `in_progress`, `implemented_unverified`, `verified`, `blocked`, `superseded`. Every execution has an outcome from `passed`, `failed`, `skipped`, `timed_out`, `out_of_memory`, `unsupported`, `not_run`. These are not interchangeable. A document edit can close a documentation task; it cannot close a runtime durability task.

There is no arbitrary global development-cycle cap in this goal. There are still strict per-operation attempt budgets, run resource limits and escalation rules. Repeating the same failing attempt without a new hypothesis, code change or changed external condition is not useful progress. Persist the blocker, report the attempted remedy and continue independent reachable work.

Do not weaken tests, increase tolerances, suppress diagnostics, reduce required coverage, silently shrink the cohort, delete difficult fixtures or reinterpret `SKIP` as success to finish the loop. A policy change requires a recorded owner decision and a versioned manifest, never an agent's private shortcut.

---

## 8. P0: make current quality evidence trustworthy

Begin with the exact reviewed tree, or explicitly record a newer HEAD and a delta analysis. Run the configured quality command in a clean, isolated integration worktree. Capture stdout, stderr, exit status, toolchain, lockfile hash, source SHA and feature set. If CI failed, retrieve logs with the user's available authorized tooling or reproduce the failing command locally; do not guess the cause from a red status. [R02, R03]

Required baseline commands include:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Run additional supported feature combinations using the repository's own documented gate. A fuzz/Kani feature that needs a distinct toolchain can have a distinct explicit lane; it cannot disappear from verification because the ordinary build succeeds. Compilation, rustdoc, license/advisory results and source-scanner outputs must all name the commit that produced them.

The current quality-baseline file is a policy/debt artifact, not a live execution result. In particular, a zero forbidden-cast count in an earlier baseline does not excuse numeric casts or `unwrap` calls visible in newly changed production code. The inspected imperial parser contains both an unchecked arithmetic path and an `unwrap` on integer conversion; it must be fixed rather than hidden from scanning. [R05]

The source scanner must cover all first-party compiled production targets, including developer tooling used by release gates. It must not mask the remainder of a file merely because an inline `#[cfg(test)]` module appears before later production items. Prefer compiler/AST-aware scope handling; test nested comments, raw strings, inline tests and production code following tests. Vendor-generated code can be separately classified, but first-party runtime code cannot be labelled generated to escape the standard.

Keep the requested 300-line file / 60-line function budgets on first-party production code, with the current narrower hot-kernel guidance. Do not split cohesive data tables into layers of forwarding methods just to improve a count. If an existing explicit policy permits a structural exception, its evidence must be visible and authorized; do not invent new exceptions to pass the final gate.

Acceptance for this work: the final SHA has green required CI and a complete local gate evidence record; missing tools and skipped mandatory lanes fail release; no stored historical PASS is presented as a new run.

## 9. P0: repair persisted mark compatibility before touching live data

The inspected `fixed_mark.rs` uses `#[serde(transparent)]` integer wrappers, which serialize as their inner integer representation. The comment claiming unchanged decimal JSON is unsupported by that implementation. Serde documents transparent behavior as forwarding to the contained field. [R04, E03]

The migration must cover at least these forms:

```json
{"TimeSeconds":10.94}
{"TimeSeconds":60}
{"TimeSeconds":1094}
```

They cannot be distinguished safely by asking whether the JSON number has a fractional part. In legacy data, `60` can mean sixty seconds. In a centisecond schema, `60` means six-tenths of a second. The same bytes have different semantics. **Do not infer the writer version from the magnitude or apparent realism of a result.**

Choose one explicit durable contract. The preferred option is a versioned observation envelope whose mark payload has a named unit and exact integer value, while legacy decoders interpret the old version according to its documented unit. Another acceptable option is a carefully tested custom compatibility codec with unambiguous writer metadata. Keep the serialized version independent from Rust enum debug spelling and independently version raw captures, source observations and canonical projections.

Inventory actual writers before migration: historical release SHA, schema version, source capture date, table, count, payload digest and any run manifest. Determine whether the live corpus contains both old decimal rows and unversioned scaled-integer rows. A row without sufficient writer provenance is quarantined as `AmbiguousLegacyUnit`; it is not guessed, deleted or treated as an impossible athletic mark. Reconstruct it from already retained raw evidence when available.

Migration procedure:

1. Freeze new writes that could mix incompatible mark encodings. Do not stop user-owned services without the required agreement; arrange a controlled drain or use an already consistent backup.
2. Obtain a consistent backup and verify it before any migration. Copy into a new candidate store directory. Preserve the original path and bytes.
3. Run a read-only inventory and write a migration manifest with per-table counts and semantic units. Do not silently mutate metadata while claiming a read-only audit.
4. Convert a bounded candidate batch using version-specific readers. Retain original raw bytes or immutable references, old identifier, writer version and exact interpretation.
5. Compare every source performance's source identity, numerical value, unit, precision, status and ownership before/after. Recompute PRs from the converted source facts, not a cached pre-migration summary.
6. Reopen the candidate store, run integrity, replay and full export verification, then drill restoring it. Only promote after the acceptance record passes.
7. Promotion is an explicit ownership-controlled operation. Keep rollback material and make old/new reader compatibility precise. Do not let an old worker reopen the newly versioned store without a compatible reader.

Preserving byte-identical old JSON is not itself the requirement; preserving meaning, evidence and recoverability is. If the wire format deliberately changes, say so and version it. Never satisfy a byte-parity test by leaving a one-hundred-fold unit error untouched.

## 10. P0: one exact numeric kernel, not a second report parser

The imperial comparison function currently uses this relationship:

```text
inches_hundredths = decimal_inches × 100
millimetres = (feet × 7620 + inches_hundredths × 635 + 12) / 25
```

The second term uses hundredths as whole inches. The correct dimensional treatment must divide that component by an additional factor of one hundred or use a common exact subunit. A simple source-derived counterexample is decisive:

| Published mark | Correct distance | Inspected formula's integer output |
|---|---:|---:|
| `3-0.75` | 933.450 mm | 2,819 mm |
| `4-0.00` | 1,219.200 mm | 1,219 mm |

The formula therefore selects the shorter jump as the better result. These numbers are arithmetic derived from the inspected implementation, not a claim that the Rust test suite was executed during this review. Add this exact case to the production comparison tests before changing the code. [R05]

Do not repair only the coefficient and leave three competing normalization paths. Put the exact decimal/imperial parser and unit conversion in a single reusable deterministic kernel. Crawl uses it once at the boundary, the stored comparable mark records its interpretation, and report compares that value. Report must not reinterpret the raw string differently to work around coarse canonical storage.

Preserve source notation, source precision and interpreted value separately. Appropriate internal types may be `TimeMicros`, `DistanceNanometres`, a signed wind unit, and an exact points representation. Select their ranges from the qualified corpus and source contracts. Nanometres comfortably represent decimal inch fractions that a micrometre scale may round; alternatively, use a checked exact rational representation for uncommon values. The selected unit must represent accepted source values without an undocumented rounding policy.

Do not claim that integers automatically make the system faster. Their immediate benefits here are exactness, explicit units and a tractable proof surface. Measure actual parse, comparison, allocation and serialization costs before publishing a speed claim.

Parser requirements include decimal scale validation, hour/minute/second fields, finite source bounds, sign handling, permitted whitespace, allowed source suffixes, fractions, metric units and explicit rejection of incompatible shapes. `0.5` and `0.50` must represent the same inch fraction. A third fractional digit is either represented exactly under the declared contract or rejected/retained as unsupported; it is not silently truncated. Negative time, negative distance, oversized values, NaN/Infinity legacy values and malformed strings never construct accepted comparable marks.

Provide checked arithmetic helpers instead of infallible `f64 -> i32` constructors. Parse digits directly into bounded integers where possible. If a legacy float channel must be read, its conversion is a separate fallible compatibility boundary with explicit finite/range/precision checks and recorded policy. Public tuple fields must not bypass those checks.

Acceptance tests must exercise production entry points with metric/imperial equivalence, adjacent fractional marks, sub-centisecond times, all non-finish states, conversion bounds, round-trip precision, old wire versions and ordering. Add metamorphic tests that increasing a compatible positive field mark cannot make it worse and decreasing a compatible positive time cannot make it worse. A golden fixture that merely expects the previous wrong number is not an oracle.

## 11. PRs, events and participation: the recruiting contract

Define a `ComparableEventKey` that records the attributes necessary for comparison rather than using a display label alone. Depending on the discipline, these include sport, distance, indoor/outdoor context, hurdle height/spacing, implement mass, relay structure, combined-event component and any source-specific rules that affect equivalence. Missing necessary specification is an explicit `NotComparable` reason. It is not replaced by a default men's implement, standard hurdle height or assumed race distance.

Separate event participation from a valid performance. DNS, DNF, DQ, foul, no height and no mark are legitimate observations. They can support participation without supporting a numerical PR. Retain statuses rather than dropping them from the athlete's event universe or turning them into zero. Keep team points separate from an athlete's combined-event score and from the measured performance itself.

Relay handling must distinguish the squad result, evidenced membership, leg order, actual split when published, and the team's time. A runner's membership in a 4×400 relay does not establish an individual 400 m PR equal to one quarter of the relay time. A published split is a different observation with its own timing/comparability classification. Preserve the parent relay result and source membership proof.

Cross-country bests require particular care. Store distance, course and conditions when published. A lowest captured 5 km XC time can be presented as a best captured time at that distance under the chosen policy; it is not a course-adjusted ranking or proof of greater ability across different courses. Do not have AI invent equivalence or convert course difficulty into arithmetic.

Keep at least three distinct concepts in the model and workbook:

- `SourcePrClaim`: a provider labels a mark as a PR, with source/profile/result provenance.
- `CapturedComparableBest`: the deterministic minimum/maximum over compatible acquired performances under a versioned policy.
- `HistoryCoverage`: what was actually searched and acquired, including known gaps and inaccessible history.

A claim of complete-career PR needs evidence beyond the lowest value currently in the store. Most rows should instead use wording that matches the captured-history contract. Source disagreement is shown as disagreement. Two sites mirroring one timing feed do not count as two independent witnesses merely because their hostnames differ.

The best reducer should be a small pure function that works over a compatible event key and a validated mark. Lower wins for time, higher wins for supported distance/points. Define tie-breaking independent of arrival order, hash-map iteration order or source fetch speed. Prefer a stable result identity and explicit tie policy; retain all equal-best supporting results when useful to audit.

The event list in `Athletes` comes from all retained participation/performance evidence, including uncommon events. Do not reduce it to the handful of columns used for convenient recruiter filtering. The normalized `Events`/`PRs`/`Performances` outputs preserve the complete set even if the primary athlete row has a concise summary.

## 12. P0: preserve source people before resolving canonical people

The new `AthleteCandidateKey` type is a useful semantic distinction, but its current four-field hash is still used to mint a singleton canonical athlete. Two distinct people can share school, normalized name, graduation year and category. Merely naming that key “candidate” does not prevent them from being folded together before review. [R14]

Represent three roles separately:

```text
SourceAthleteIdentity
    provider namespace + provider athlete ID
    OR a source-local immutable row identity when no athlete ID exists

AthleteCandidateKey
    normalized attributes used to generate possible matches

CanonicalAthleteId
    stable local identity of an adjudicated person/cluster
```

A source without stable athlete IDs still needs a reproducible observation identity, such as document digest plus a validated row locator under a source contract. That row identity is not a fabricated provider athlete ID. Preserve the distinction in types and export provenance.

Every athlete-producing adapter must emit source-level evidence before any canonical merge. Every source performance must retain the source athlete or source-row attribution, source meet/result identity, original team attribution and document locator needed to reverse a wrong canonical decision. Do not rely on parsing a compound `source_key` string whose meaning varies by adapter.

A canonical cluster can merge or split over time without rewriting source facts. Store versioned decision records and memberships. A chosen public canonical ID must remain stable when a later observation happens to sort earlier than the current minimum candidate. If the implementation deliberately changes the public ID, it must write an explicit alias/redirect and update all projections atomically; a comment about minimum-member ordering is not sufficient protection.

Distinguish a cryptographic collision from a semantic collision. A 64-bit digest collision means different encoded keys produced the same digest. Two real people with the same four candidate fields are not a hash collision: they supplied the same key material. Both failure modes need tests, but a hash-collision detector cannot fix premature semantic merging.

High-confidence deterministic joins can use compatible source identities and corroborating evidence. A provider ID shared across observations is strong evidence, not authority to ignore hard contradictions, known ID reuse, invalid source parsing or mismatched namespaces. Different IDs at one provider are a reason to investigate, not unconditional proof of different people, because duplicate profiles can exist. Make the decision rules explicit and versioned.

For cluster joins, checking only one pair is not enough when an accepted edge creates a transitive merge. Before joining components A and B, verify that no retained hard contradiction exists anywhere across the resulting component under the supported policy. Keep complexity bounded through indexed component facts. Do not claim transitive identity safety from a few pairwise happy-path tests.

`census-review` can retain model or deterministic verdicts without directly editing canonical facts. `census-reconcile` must then apply admissible decisions into a versioned canonical read model that the report actually consumes. A `SamePerson` row sitting in a verdict table while export still emits both legacy candidates is incomplete integration.

For unresolved ambiguity, retain separate provisional identities and an explicit review state. Do not publish them as one accepted person, and do not silently drop them to improve the apparent match rate. The accepted recruiting sheet and the unresolved review sheet must reconcile back to the full discovered candidate universe.

## 13. P0: make review identity bind structured evidence

The inspected `CaseEvidence` implementation normalizes each textual fact by sorting its words. That is not a semantics-preserving transformation. These two statements have identical token bags:

```text
side_a grad_year 2027 side_b grad_year 2026
side_a grad_year 2026 side_b grad_year 2027
```

They attribute the same two values to opposite people. A strong cryptographic hash cannot recover attribution already erased by normalization. The resulting case identifier can reuse a decision made about materially different evidence. [R13]

Replace prose-based case identity with canonical structured facts. Each fact records a subject, predicate, typed object/value, source identity, observation/document reference and relevant temporal/measurement context. Polarity, units, side attribution and source provenance are part of the digest. Sort a set of independent fact records if order is semantically irrelevant; never sort tokens within a fact.

A robust review key binds:

```text
review family
ordered or canonicalized subject identities, under an explicit symmetry rule
identity policy revision
canonical structured evidence digest
source/parser interpretation revisions where they change meaning
```

The full local model packet must be derivable from exactly that evidence snapshot. Adding a contradiction, changing a source owner, changing a grade date or swapping a fact between candidates must create a new evidence version. Changing whitespace in a human explanation should not.

Use a typed `AthleteIdentityCase` instead of relying on a prose `detail` field to reconstruct candidate IDs later. Model explanations are retained text, not primary keys. Subject IDs and evidence refs are validated before dispatch. The assigned model cannot invent a supporting reference not in the packet.

Tests must include attribution swaps, sign changes, unit changes, negation, two source namespaces using the same local ID, changed school date ranges, a reordered set of genuinely unordered facts and a changed hard contradiction. Only the last case's exact evidence may reuse its cached decision.

## 14. Exact AI authority and model-lane behavior

The model verdict vocabulary is small: `same_person`, `different_person`, `insufficient_evidence`. Schema validation rejects unrecognized fields/values, references to absent candidates, malformed JSON, unbounded output and answers for another case. A model failure is `ReviewFailed` or `ReviewRequired`, not `NoMatch`.

The deterministic finalizer accepts a merge only when the hard-rule contract already permits it and the model advice is admissible. It must be impossible for `same_person` to override a hard contradiction. Separate evidence strength from numerical model confidence. A model-generated confidence percentage is not a calibrated probability of identity and must not be displayed as one.

Exactly one model assignment belongs to a review case. Use both existing local servers across different cases. The assignment is stable, versioned and persisted. If a server becomes unavailable, pause/retry its assigned work under the approved budget; do not silently send the same case to a second model as an unrecorded consensus/retry policy. An explicit reassignment is a recorded state transition, not another identity decision.

Inspect actual model IDs, server endpoints, quantizations, context limits, slot counts, schema support and hardware allocation before setting throughput. Do not assume that a 5090 is automatically allocated to a particular endpoint. Benchmark representative synthetic compact evidence packets without workbook PII first. Use the measurements to set concurrency, byte/token ceilings and timeouts. Keep user-owned model installations and services unchanged.

Prepare all deterministic facts before enqueueing. High-confidence cases bypass AI. A slow AI queue must not block deterministic matches or cause the crawler to accumulate unlimited review payloads. Persist case references, not huge duplicated evidence packages in every queue. Record request counts, tokens if available, model latency, rejected outputs, cache hits and wait time separately for each server.

An insufficient-evidence result is a valid terminal review disposition for a fixed evidence snapshot. It can remain in the recruiter review output. Do not leave it pending forever solely to force the global loop to keep asking the model the same question. New evidence or an explicitly changed policy can reopen it through a new versioned case.

## 15. P0: external idempotency at the Restate/Fjall boundary

Restate journals durable steps, but an external side effect can complete immediately before the worker crashes and before the step completion is durably observed. Re-executing that step can repeat the external operation. Restate's documentation explicitly requires attention to idempotency for such external effects. [E01]

The current Ingest implementation checks `seen_operations` in Restate object state, performs the Fjall append inside a durable step, then updates the seen set. That closes ordinary duplicate submissions after successful completion but does not by itself close the external commit/lost-acknowledgement window. It also grows a per-object set that is repeatedly loaded and serialized. [R07]

Implement one store entry point whose contract is conceptually:

```rust
apply_once(operation: ValidatedOperation, batch: ValidatedSourceBatch)
    -> Result<AppliedReceipt, StoreError>
```

The exact signature is a proposed contract, not code to copy verbatim. It must run under the store's existing single-writer synchronization. In one Fjall atomic durable batch, it writes the source observations, sequence/count metadata and an application receipt keyed by the complete operation identity. If the receipt already exists, it verifies identity/payload compatibility and returns the original outcome without appending observations again.

An application receipt should bind source instance, logical source object/page, capture or parser revision as relevant, destination table or multi-table schema, batch ordinal, full payload digest and schema revision. Hashing only the JSON rows is insufficient when the same bytes can represent different tables or sources. Same operation ID with a different digest is an invariant violation, not an upsert or a silent no-op.

The receipt is a record of application to this store, not a second Restate task queue. Restate still owns when work runs, retries, cancellations and durable workflow status. The store receipt exists solely to make its external effect idempotent across the two durability domains.

For an empty successful source page, write an explicit completion receipt that states zero observations and its source evidence. “No rows appended” must not mean “not done,” and “some rows exist” must not prove the source object's full pagination completed. Completeness belongs to the source contract and cursor/terminal state.

Bound receipt growth through a run retention policy tied to replay/idempotency retention. Do not garbage-collect receipts while a live or retained Restate invocation can legitimately replay them. A fresh store rebuilt from immutable evidence gets a new store identity; do not mix receipts from one store generation with another.

Acceptance requires the actual production Ingest handler to survive a controlled crash after the Fjall commit but before the acknowledgement/journal completion becomes reusable. Physical observation count, receipt count, source identities, hashes and canonical counts must remain unchanged on replay. Counting canonical rows alone is insufficient because merge-on-read can hide duplicated observations.

## 16. Restate granularity, retries and control-plane size

Use durable work units that match external operations: a source directory page, one roster page, a meet-result page, a bounded profile/history request, one local review case, one bounded source batch application, one export partition or one verification phase. Do not create a workflow per individual performance, and do not put an entire state's millions of rows into one durable step response.

National and jurisdiction workflows coordinate children and aggregate receipts/status. Large raw responses and observation batches live in immutable local evidence storage with bounded references passed through Restate. A `Recorded` value must have byte and row limits. If a page exceeds a request limit, partition it deterministically with stable ordinals and a manifest, not different chunk boundaries on each retry.

A parent reports a child as complete only after the child's declared unit reaches an explicit terminal state. Distinguish successful empty, complete with gaps, partial pagination, blocked, human required, exhausted and parser quarantine. An unsupported adapter registration is not an executed source sweep.

The external attempt policy is three total automatic attempts by default. Before implementation, write down exactly which Restate boundary owns the retry and what exception/status it sees. A handler replay that reuses a completed durable step is not another HTTP attempt; an unjournaled step that reissues HTTP is. Test with a fixture server's request counter and actual worker restarts instead of proving a text literal equals three.

Do not impose a blanket three-attempt cap on arbitrary parent workflow recovery and claim that this automatically bounds HTTP. A large handler can perform many child effects, and its retry policy is not the same thing as a per-request budget. Conversely, do not let changing workflow revision or clearing a cache reset an exhausted source operation without an explicit new approved collection decision.

If a strict physical-attempt upper bound must hold across a crash at the network boundary, reserve/account for the attempt durably before issuing it and count uncertain in-flight attempts conservatively. Implement that using supported Restate state/lifecycle capabilities, not a second general scheduler. The same rule must explain what happens when the durable reservation exists but the worker dies before sending. Honest uncertainty can reduce available retries; it cannot justify claiming exactly-once HTTP.

Respect `Retry-After` and explicit operator ceilings. A 429 is not automatically a CAPTCHA; a challenge is not automatically a transient 429. `HumanRequired` suspends the affected browser source lane, preserves progress and continues unrelated permitted work. Never create new IP identities, rotate credentials, replay challenge cookies outside the browser or retry a blocked endpoint under an alias.

## 17. P0/P1: replace named-but-skipped durability scenarios with executable evidence

`tools/durability/scenario-06-no-duplicate-evidence.sh` explicitly marks the commit-to-acknowledgement injection scenario as skipped. Running older generic store tests inside that script does not establish the missing production Ingest property. [R11]

Create a scenario manifest that enumerates each required hazard, its production entry point, fixture, failpoint, expected state, raw log and verdict. Every scenario must state whether it was actually executed. The release harness must reject any required scenario that is missing, skipped, timed out or only described in comments.

Minimum production-path matrix:

| Scenario | Required observation |
|---|---|
| Crash before source request | No response/evidence claim is fabricated; operation remains owed or conservatively accounted. |
| Crash after source response before immutable capture commit | Operation resumes with explicit uncertain external effect accounting. |
| Crash after capture commit before parser result | Cached capture is reused without unnecessary source traffic. |
| Crash before Fjall apply-once commit | No observation or receipt appears partially. |
| Crash after Fjall commit before Restate step acknowledgement | Replayed Ingest returns the same receipt; physical observations do not grow. |
| Crash after some deterministic chunks of one source page | Completed chunks are reused and remaining chunks finish with the same manifest. |
| Restart worker independently | Completed durable calls/effects are reused. |
| Restart native Restate independently with its durable directory | The same run identity resumes; no fresh empty deployment is substituted. |
| Duplicate concurrent submissions | One logical effect, consistent receipt and no lost source unit. |
| Same operation ID with changed payload | Typed conflict, no write and retained diagnostics. |
| One malformed source object | That object is quarantined; unrelated objects continue. |
| 429 and Retry-After | No early readmission; one shared origin budget. |
| HumanRequired | No automatic challenge retry; unrelated origins may continue. |
| Model malformed/timeout response | Review remains explicit, never a match/no-match fallback. |
| Export interrupted before promotion | No partially published final workbook; prior artifact remains intact. |
| Backup/restore interrupted | Original store and accepted backup remain recoverable. |

Use the actual production binary, native local Restate and controlled local fixture endpoints. Fixture-only models of the desired behavior are useful supplementary tests, not substitutes. Record seeds, counters, failpoint names, process IDs, exit codes, exact invocations and hashes.

Failpoints belong behind an explicit test/injection feature and must be unreachable in normal release operation. Do not rely on timing a random kill and assuming it hit the intended boundary. The scenario should signal that it reached the boundary, then the harness terminates the intended test process, restarts it and checks the invariant.

## 18. Fjall contract after StoreBatch

Keep one live process owner for the real Fjall directory. Serialize mutations through the store's existing boundary. Multiple tasks may prepare bounded validated batches, but no adapter opens another database handle to the same path and no independent verification process opens the active store as if it were a harmless read.

Preserve the three table semantics: append-only source/observation logs, complete derived snapshots and partial derived maps. Their counters and integrity rules differ. A derived snapshot replaced by an empty set must become empty; an early return on `records.is_empty()` must not retain stale rows. Add that regression test for every snapshot-like table. A partial map update should leave unnamed entries unchanged by design.

Separate `ObservationSequence`, `SequenceHighWater`, `PhysicalRowCount`, `LogicalEntityCount`, `BatchRowCount`, `EncodedByteCount` and `ReceiptCount`. A sequence reservation is not a row, and a canonical merged entity is not an observation. Do not infer exact counts from an approximate LSM length or from a sequence counter that can contain gaps.

Replace saturation in correctness arithmetic with checked operations. Saturating telemetry can be allowed if labelled and if it cannot affect acceptance. A saturated row count, attempt budget, buffer size or offset is a data-integrity failure, not a tolerable metric loss.

Validate batch row/byte/resource limits while building the batch, before unbounded allocation. A 20-million-row table cap does not bound a single 3 GB page or the total memory of concurrently building batches. Define and test maximum rows, encoded bytes, per-record bytes, pending completions and preparation time for each source-operation class.

The 20-million-observation cap must be reconciled with the expansive national product. Do not simply delete it, and do not claim a streaming reader requires a small total-table cardinality to be safe. Bound per-step scans and memory independently. If the corpus can legitimately exceed the current cap, implement a versioned partitioning or paged scan contract with exact continuation and accounting, or raise the declared finite bound after capacity testing. Both approaches need a migration and a no-loss test; neither is permission for unbounded work.

Point reads and prefix-grouped merged scans should serve entity lookups, review packets and export reductions. Avoid scanning every athlete merely to inspect one candidate. Stream heavy performance tables and retain only required reduced indexes. Derived caches must bind the source snapshot/reconciliation revision so an old PR cache cannot leak into a new identity projection.

On an uncertain commit failure or poisoned writer state, fail closed and recover through the documented store lifecycle. Do not roll sequence counters back speculatively while an actual commit might have reached disk. Strong idempotency receipts and reopen verification are safer than an invented rollback protocol.

## 19. Immutable evidence, cache and receipt hygiene

Retain sufficient source evidence to reparse and audit without another network call. A document store may be content-addressed files or an existing Fjall capability; choose the existing implementation that meets the contract rather than introducing a second blob database. Canonical rows should refer to a document/observation digest and locator instead of repeating a large response body in every entity.

A capture receipt identifies source instance, sanitized resource identity, request semantics, source operation, acquisition time, media type, encoded/decoded byte bounds, status, body digest, redaction policy and parser input version. The cache key distinguishes materially different methods, bodies, query parameters, seasons and source objects. Temporary transport failure is not a durable negative identity fact.

Credentials must not become durable evidence. Omit cookies, Set-Cookie, authorization headers, bearer/session tokens and sensitive query parameters from routine logs and receipts. Some source bodies contain request tokens as well; a header-only redactor is insufficient. Retain only the fields required for evidence, or sanitize token-bearing metadata under a documented policy before durable capture. If a raw original must be held for an explicitly justified local forensic need, keep it access-controlled and out of remote-agent fixtures, exports and ordinary logs.

Bind receipts to whether the bytes are original or sanitized. Do not call a digest of redacted bytes the digest of the original response. A separately retained original digest is useful, but the verifier must know which content it can reproduce. Use keyed correlation only where genuinely required; hashing every secret is not a substitute for omitting it.

Cache success semantics follow the source contract. A static historical result file can often be replayed; a changing profile may need an explicitly scoped capture time. This one-shot census still needs a capture window and source-change accounting. It does not need a weekly refresh scheduler.

## 20. Async ownership and resource control

Every spawned task belongs to an explicit owner. The browser actor, source worker regions, parser pool, store writer, review lanes and export tasks each support cancellation and drain. No detached `tokio::spawn` may outlive the region that owns its observations or receipt acknowledgement.

A drain report must reconcile admitted work into disjoint outcome categories: completed, failed, cancelled before effect, cancelled with uncertain in-flight effect, timed out, panicked, handed back to durable Restate ownership, and still unaccounted. Do not count a cancelled task as both completed and aborted. `remaining = 0` alone is insufficient if tasks were removed from the ledger without an outcome.

Tokio owns bounded asynchronous I/O. CPU-heavy parsing, normalization, hashing and large serialization work use appropriately bounded worker capacity. Synchronous disk work must not block a runtime worker on a hot async path. `spawn_blocking` is not itself a cancellation protocol; long-running work needs bounded chunks or cooperative cancellation checks at safe commit boundaries.

Separate capacities for HTTP origins, browser tabs, parser CPU, batch preparation, store commit, each model server and export. A single global semaphore should not make a slow GPU starve source metadata requests. A large network concurrency setting must not produce unlimited CPU jobs or in-memory recordings downstream.

Bound queues by both count and estimated/encoded bytes where payloads vary substantially. Tune using observed queue occupancy, wait time and memory, not a desire to occupy every CPU thread. There is no performance requirement to use all 128 GB of RAM or keep all hardware at 100% utilization. The target is maximum verified useful throughput under explicit source and resource ceilings.

Instrumentation must preserve privacy and minimize hot-path cost. Record source operation ID, stage, elapsed time, outcome, retry reason, receipt reuse, parser revision, bytes, records and queue delay. Sample or aggregate high-volume metrics where appropriate; never suppress accounting necessary for the seal.

---

## 21. Kani is a release requirement, not a directory of intentions

The repository now has a Kani runner. The inspected fast-mode list names several harnesses whose definitions are not established by the inspected domain wiring, which includes the older grade-year, ID and publishing modules. This is an inventory/integration gap to verify, not grounds to announce that all named proofs ran. [R06, R12]

Discover proofs using the actual supported Kani inventory command for the pinned toolchain, including the package that owns each harness. Compare the discovered inventory to a committed proof manifest. A mandatory harness missing from the build, selecting zero proofs, or executing a similarly named test in another package is a failed verification lane. Kani's documented list facility provides a machine-readable basis for this check. [E04]

The manifest must record harness name, owning package, production functions exercised, claimed invariant, symbolic inputs, assumptions, unwind/size bounds, cover conditions, stubs, expected outcome, time/memory budget and tier. It must state what is not proved. A bounded proof of a pure state transition is not proof that the HTTP client, filesystem or Fjall behaves according to the model.

Mandatory proofs execute the same production kernel used by the application. Do not duplicate its algorithm in a test module and prove the duplicate. Factor a small pure kernel out of effectful code when necessary, then test that the production path actually calls it. Any stub is an explicit trust boundary: justify why it preserves the property, and separately test the concrete integration it replaces.

Do not hide solver limitations by changing production behavior under `cfg(kani)`, weakening an assertion, narrowing input domains without justification or treating unwinding assertions as optional. Assumptions need non-vacuity evidence, especially boundary values and rejection paths. Unsupported operations, panic, timeout, OOM and no-verdict are all non-passes for mandatory proofs.

### 21.1 Mandatory proof kernels

| Kernel | Required property | Concrete integration complement |
|---|---|---|
| Validated grade/year constructors | Accepted inputs are exactly the declared range; invalid values cannot enter. | Actual source season fixtures and migration tests. |
| Cohort derivation | Grade/date/academic-year mapping follows the explicit source interpretation; no overflow. | JR/SR historical source pages and current-year roster fixtures. |
| Exact mark conversion | Valid scaled components convert without overflow or undocumented loss. | Full text-parser fuzz/property tests and imperial/metric regression corpus. |
| Best-of-compatible-performances | Idempotent, associative and commutative under a fixed tie policy. | End-to-end PR export from multiple source orders. |
| Identity finalizer | Any hard contradiction blocks `Match`; insufficient evidence cannot promote itself. | Adversarial same-name/transfer source fixtures and actual review path. |
| Evidence snapshot identity | Typed attribution is preserved; semantically changed fact records differ before hashing. | Case-cache migration and packet digest tests. |
| Canonical link transition | No self-link; no admitted cycle; replay of the same decision is idempotent. | Persist/apply/split/re-export integration tests. |
| Batch accounting | Counts, sizes and high-water calculations do not wrap or exceed accepted bounds. | Fjall failpoint and concurrent submission tests. |
| Attempt state | A terminal/exhausted operation cannot issue another automatic attempt under the same budget. | Native Restate plus fixture-server physical request counters. |
| Run scope | Exactly the declared 49 jurisdictions, no duplicates, AK/HI outside the run. | Planner and export scope integration tests. |
| Contact policy | No denied/private contact class enters the recruiter projection. | Published directory fixtures and actual XLSX checks. |
| Seal reducer | Missing/failed mandatory evidence cannot produce `Sealed`. | Real artifact hash, CI and restore evidence ingestion. |

A proof of SHA-256 implementation details is not necessary to establish stable ID formatting or domain separation. Golden serialization tests, collision detection and keyed-identity construction tests serve those contracts more directly. Kani cannot establish cryptographic collision resistance for truncated IDs. Avoid forcing mandatory proofs through large hash/Unicode libraries when the actual safety claim concerns the small surrounding state machine.

### 21.2 Proof tiers and execution evidence

Define a fast mandatory tier that reliably finishes on the development machine and a broader tier for deeper bounded exploration. Do not promise arbitrary time limits before measuring the actual harnesses. Set per-harness resource limits after a successful baseline and commit them. Run a small number of solver processes at a time so a proof lane cannot exhaust the user's workstation while the crawler and GPUs are active.

The final proof manifest records all mandatory outcomes individually. It must distinguish `verified`, `counterexample`, `timeout`, `oom`, `unsupported`, `not_selected`, `not_run` and `tool_failure`. Store raw logs and tool versions. A runner's exit code is checked, but it is not the sole evidence if the runner could select zero harnesses.

If Kani cannot compile the current domain crate, diagnose the exact pinned compiler/serde/Kani compatibility problem. Capture a minimal concrete reproducer and fix the supported integration. Do not label it a generic “Kani environment issue” while the release simply omits the proof. The goal is a real, reproducible proof lane on the actual production core.

## 22. Property, fuzz, mutation and concurrency tests complement proof

Kani does not replace testing of rich source bytes. Keep deterministic property tests for normalization idempotence, exact serialization round trips, source-order independence, stable structured evidence identity, merge/reversal, batch partition equivalence and PR reduction algebra. Record generator bounds and seeds so failures can be reproduced and shrunk.

Fuzz every untrusted parser that can accept source bytes, saved evidence or exported artifacts: JSON, HTML/embedded state, result text, CSV, XML, archives, mark/date parsing, model output and source identifiers. A corpus containing only valid examples is inadequate. Include oversized strings, deeply nested shapes, duplicate fields, invalid UTF-8 where applicable, truncated responses, malformed escapes, contradictory fields and absent IDs. Preserve crashes and semantic failures as minimized regression fixtures.

A parser must not silently discard malformed rows while returning an apparently complete page. It can quarantine a row and continue, but the result count and coverage must account for that row. Fuzz or property tests should verify accounting alongside panic freedom: `observed = accepted + rejected + quarantined + explicitly unsupported` under the source's actual row contract.

Use mutation testing on small critical kernels where meaningful assertions should kill every dangerous mutation: hard-contradiction checks, exact unit conversion, attempt terminal states, application-receipt comparison, cohort derivation and seal acceptance. Do not demand a superficial universal mutation percentage across all network adapters. List surviving critical mutants with their meaning and fix tests or explicitly justify an equivalent mutant.

Use Loom or the existing concurrency model tools for the interleavings the project actually owns: writer admission, reservation ordering, task ledger transitions, source gate changes and shutdown. The model must retain the relevant production synchronization rules. Complement it with actual Tokio/Restate runtime tests. A proof that a toy counter increments does not establish correct browser or store ownership.

Do not remove mature old fixtures merely because the source architecture changed. Recover useful deleted exact-mark and input-validation tests from git history when they exercise current requirements. Port their contracts into the surviving implementation rather than resurrecting a legacy application.

## 23. P1: make performance tooling measure reality

The inspected benchmark runner parses custom `name=...` lines while launching Criterion with a JSON-output assumption, and can return an empty map without error. Its comparison loops over returned current groups, so a run that returns none can avoid every baseline comparison. Its environment mismatch path warns and continues. Its RSS wrapper reads `/proc/self/status/VmHWM`, which is not the documented file/field relationship and does not measure a benchmark process tree. [R08, R15]

Fix measurement before optimizing from its results. Query the actual pinned benchmark binary's supported CLI and output format. Parse the files or output it actually produces. Criterion's documented output/filter conventions are the starting point, not permission to invent a schema. If using Criterion's estimate artifacts, pin their schema and test the parser against captured artifacts from the installed version. [E05]

Require a nonempty, exact benchmark inventory. Compare baseline-required IDs against current IDs in both directions. Missing cases fail; unexpected cases require an explicit baseline update or separate classification. A benchmark with no declared throughput can still have a valid per-operation timing metric; do not silently skip it if timing is the required contract. Missing units or malformed numbers fail before comparison.

Validate every numeric metric: finite, in-range, semantically positive where required, and associated with an explicit unit and workload size. A tolerance of NaN must not disable all comparisons. A baseline throughput of zero must not create an undefined percentage. Missing peak memory is not zero bytes.

Record CPU, core topology, CPU affinity, memory, OS/kernel, Rust and relevant tool versions, release flags, dependency lock hash, workload/corpus content hash, cache mode and benchmark schema. The source SHA belongs in both records but is expected to differ when comparing an optimization. Hardware/toolchain/corpus differences that invalidate a comparison must fail or produce an explicitly non-comparable outcome, not a green regression result.

Use unique temporary directories and direct subprocess arguments. Do not interpolate user-controlled benchmark filters into `bash -c`. A shell wrapper is not necessary for most of the measurement path. If a shell script remains, argument handling and cleanup must be tested under spaces, quotes, interrupted runs and concurrent invocation.

Compile benchmark binaries before measuring runtime. Otherwise a memory or wall-time “regression” can simply measure rustc/linker work. For a direct process workload, measure the benchmark process using a supported OS facility. For a multiprocess workload, a dedicated cgroup can measure total scoped memory; record that it includes the cgroup's charged memory rather than calling it a single process's RSS. Do not claim `/proc/self` covers children. A benchmark's own maximum RSS, a process tree's maximum, and aggregate cgroup peak are different metrics.

Report raw observations, repetitions, median/distribution, workload size and pass/fail decision. A default five-percent threshold may be a starting policy for stable measurements, but characterize variance first. Do not loosen a threshold to accept a desired optimization. Large deterministic regressions should be caught with repeated runs; noisy results are `inconclusive` and rerun under controlled conditions, not automatically passed.

## 24. Performance engineering program

Optimize in the order indicated by measured cost, with correctness blockers repaired first. The major candidate areas are exact mark parsing, repeated source normalization, duplicated serialization, growing Restate payloads, multiple durable barriers, all-table scans for point lookups, string-backed identifiers, report lookup structures, and slow export verification.

Use three workload levels. Small fixtures run in normal CI and assert correctness plus broad resource bounds. Medium synthetic corpora exercise representative cardinality and skew. Large read-only replays use the retained real corpus or an approved privacy-safe equivalent on the 9950X3D/128 GB reference machine. Every workload specifies records, source distribution, skew, payload sizes, duplicate rate, conflict rate and sort order.

Minimum macrobenchmarks:

```text
source JSON/text decode -> validated observations
bounded multi-table apply_once, first application and duplicate replay
merged observation scan, including heavily re-observed entities
source-to-canonical identity reconciliation, including large candidate groups
PR reduction across exact event keys
review packet construction from point/prefix reads
performances export with spill/sort partitions
complete workbook generation
independent workbook verification
full cached rebuild without network/model calls
```

Measure wall/CPU time, records per second, peak scoped memory, bytes read/written, durable commit count, observation count, cache hits, request count where applicable, queue delay and model work. Use `perf`/allocation tooling on the measured hot path before changing representations. A warm page-cache run and a cold-storage run must be labelled separately.

The deterministic data plane should have substantial measured capacity above maximum admitted live-source ingress, with a committed target based on the observed workload. “Five-times headroom” can be an engineering budget once input volume is measured; it is not a fact about current code. In offline replay/export phases, the bottleneck may legitimately be CPU or disk. Do not claim network is always the bottleneck merely because live scraping is paced.

### 24.1 Compact identifiers and hot structures

Benchmark replacing `Id<T>`'s heap string with a compact fixed-width internal value while preserving typed roles and stable serialized text. Do not casually change the persistent ID width, prefix or hash material. A custom formatter may produce the old wire form without forcing every hot row to own a heap string.

Avoid claiming every newtype is automatically zero-cost. A transparent scalar wrapper often optimizes away, but parsing, allocation, serialization and dynamic dispatch choices determine real cost. Prove layout-sensitive expectations with tests where necessary and measure whole-path effects.

High-volume performance records should contain compact IDs, exact measured values, source refs and required flags. Large human-readable names, URLs and explanations belong in indexed source/entity records or sidecars. Resolve them at export. Do not duplicate a document URL and explanatory paragraph into millions of hot rows unless profiling shows the simplicity is worth the cost.

Benchmark shared immutable strings or source IDs where repeated values dominate allocations. Avoid a generic interning subsystem before evidence justifies it. Do not adopt SIMD, unsafe code, a binary codec or new storage layout because it sounds fast. There is ample ordinary optimization in streaming, batching and ownership first.

### 24.2 Acceptance for performance changes

Every performance patch must include the same-corpus before/after command, output digest or equivalent correctness comparison, runtime and memory measurements, environment and regression result. Smaller source code is not inherently faster; fewer allocations in one microbenchmark may lose at the full census workload.

Keep an explicit memory budget for parsing, batch preparation, identity indexing, PR reduction, spill partitions and export. A worker must refuse or split oversized work before exhausting shared RAM. Test skew: one athlete with a huge history, one provider page with a huge block, one common-name candidate group, and one export partition that receives most rows.

## 25. National source coverage: finish the product, not just the fan-out

The inspected applicability table is a twelve-state research mapping and yields no supplemental descriptors for other jurisdictions. The base national workflow may perform other common discovery, but this table does not establish a full 49-jurisdiction multi-source plan. Treat the unmapped jurisdictions as a researched-coverage obligation. [R16]

Produce a machine-readable source matrix for every admitted jurisdiction with separate columns for school universe, cohort discovery, meet discovery, result acquisition, historical profile enrichment, independent class evidence and coach/contact sources. Each cell identifies a real source/adapter, an access state and supporting evidence. `Unresearched`, `NotAvailable`, `Blocked`, `SupportedButUnwired`, `Qualified` and `Executed` are distinct states.

Use official school/state indexes, qualified national aggregators, timing-company archives, static results and public school athletics directories. A source may be useful for only one capability. Do not force every source to enumerate athletes or provide PRs. Data from a registration/index source can seed a known meet or athlete ID and eliminate expensive discovery calls elsewhere.

Acquire at the provider's natural bulk unit. One complete meet payload can supply many performances. Athlete profile/history calls fill explicit gaps rather than redundantly polling every known person. Existing Athletic.net athlete/meet IDs and external source aliases are already-paid discovery work; reuse them before broad browser searches.

A source that publishes one national or regional directory should be fetched once per actual resource, not once for each jurisdiction consuming it. Workflow identity must capture the real source resource and season, while jurisdiction views reference its receipt. This avoids turning a national fan-out into forty-nine copies of the same request.

Jurisdiction eligibility belongs to the athlete's evidenced school/placement policy, not automatically to the meet venue. An in-scope high school can compete in another state. Mixed-jurisdiction meets must preserve each team's actual school identity rather than assigning the venue state to every athlete. AK/HI remain excluded discovery targets; already acquired multi-jurisdiction evidence is filtered according to the run's explicit athlete policy without silently rewriting geography.

### 25.1 Source qualification packet

Each research/adapter agent submits a compact report with source URL, observation date, states/sports/years, enumeration path, stable IDs, pagination, terminal condition, schema/sample capture, grade interpretation, result precision, coach fields, access requirements, rate limits, background/browser cost and expected marginal coverage. Claims of source completeness must cite the actual contract or demonstrate bounded reconciliation against another universe.

Sample captures must be sanitized and bounded. Do not collect athlete contact information while investigating profile pages. Do not distribute workbook PII or token-bearing HARs to remote coding agents. A source's policy, login requirement or failure is recorded, not bypassed.

### 25.2 Parallel national exploration

Partition work into disjoint state/provider groups rather than one agent editing the shared registry for each discovery. Suggested research grouping covers Northeast, Mid-Atlantic/DC, Southeast, South Central, Great Lakes, Plains, Mountain and Pacific contiguous states, with a separate cross-provider audit. These groups are organizational work packets, not production geographic assumptions.

Main integrates source descriptors and applicability changes after evidence review. Prioritize marginal verified athlete/performance/contact coverage per request and engineering effort. Do not assume the next ten timing websites produce ten independent datasets; identify mirrors and upstream relationships.

Preserve existing Midwest coverage and use it as a regression corpus while extending the remaining jurisdictions. A national result that drops a working Wisconsin source to make the new architecture simpler is not progress.

## 26. Source admission and browser behavior

Keep one shared physical-origin/family budget across all adapters and Restate workflows using the same provider. A per-jurisdiction Fetcher must not multiply the effective source rate. Where a provider shares an upstream service across hostnames, the approved family budget takes precedence over a naive per-host limit.

The browser lane uses normal persistent headed Chromium and Chrome-owned authentication/storage. Bootstrap through the normal application. Reuse qualified request lanes when they produce the same authorized request semantics; avoid unnecessary profile navigation and its background traffic. Do not rely on an experimental replay API or mutate request templates without a bounded qualification test covering correlation, redirects, freshness, response identity and cleanup.

Measure logical requests and physical origin requests separately, including background XHR/fetch traffic, redirects and bootstrap bursts. Observation of requests is not admission control. If the browser can emit uncontrolled background bursts, budget and constrain the operation accordingly rather than reporting a nominal one-request permit.

Cloudflare `HumanRequired` closes affected admission and preserves the paused frontier. Recovery needs successful normal browser evidence after the operator resolves the condition; a timer alone must not reopen a challenge-blocked lane. Provider 429 cooldowns use the documented response where available and persist the required next-admission state.

Use explicit maximum response bytes, decompressed bytes, request duration, page count, redirect count, query depth and cancellation behavior. A successful HTTP status is not a successful parse. A source returning an HTML challenge at a JSON endpoint is classified before parsing and never cached as an empty athlete history.

## 27. School and coach data is a temporal graph

Represent school source identities, canonical school identity, city/state, official URL, association identifiers and aliases separately. Same-named schools within a state remain distinct until corroborated. A school rename or cooperative team cannot be represented safely by name normalization alone.

Coach records need an assignment: school/team, sport, category where applicable, role, season or effective interval, observation date, source and confidence/freshness state. A current head TF coach is not established by a five-year-old PDF. Preserve prior assignments and identify when the current assignment is unknown.

The preferred recruiting contact is a deterministic projection. For an athlete with TF evidence, prefer the applicable current TF head coach's public professional contact; for XC, the applicable XC contact. Multi-sport athletes retain both. If no suitable coach address is available, use a clearly labelled public professional AD or athletic-department fallback. Do not select an arbitrary alphabetically first coach merely because it has an email.

Use explicit outcomes: `CoachProfessionalContact`, `DepartmentOrAdFallback`, `CoachNameOnly`, `NotAttempted`, `NoPublicProfessionalContactFound`, `StaleEvidence`, `ConflictingAssignments`, `SourceBlocked`. The absence of an email is not evidence that no coach exists. Export the official source URL and observation/effective dates.

Apply the existing consumer-mailbox withholding policy unless explicitly revised by the owner. Do not weaken it quietly to raise contact coverage. Do not infer a role-published address from a naming pattern. Never harvest athlete personal email, phone or home address as part of this census.

GPA remains optional, source-backed recruiting evidence with reported scale and date where available. Do not obtain private school records, infer GPA from school, or have a model fill missing values. GPA is not an input to identity acceptance unless it is an explicitly justified non-sensitive corroborating fact and the product contract permits it; it is never a basis for an AI ranking of whom to recruit.

---

## 28. Run manifests and immutable snapshots

Create an immutable `RunManifest` before broad execution. It names the admitted jurisdictions, cohort rule, source registry revision, source plan, historical windows, capture cutoff, scope policy, identity policy, mark/comparability policy, schema revisions, parser revisions, retry policy, resource budgets and model configuration identities. It does not contain credentials.

A root run identity must bind the scope and meaningful execution plan. A materially changed source plan cannot silently reuse a completed old workflow and claim the newly added source ran. Preserve and reuse compatible child receipts where the underlying source operation is identical; do not force a full rescrape simply because one report policy changed.

Snapshot identity must bind the store generation, source observation boundary, canonical decision revision, review evidence revision, mark schema, filter policy and captured source plan. A workbook generated from a mutable live store is not a reproducible snapshot unless its read consistency is explicitly guaranteed. Prefer a stopped/immutable consistent snapshot for final publication where that is the simplest correct design.

Do not select the final artifact by “newest file modification time.” Use an exact manifest path and digest. A failed newer export must not displace the prior accepted workbook because its timestamp is later. Temporary files and superseded artifacts remain outside publication discovery, with their status preserved.

Primary recruiting export defaults to all approved source evidence. A `core` view that excludes Athletic.net-derived evidence is useful as an independence/coverage diagnostic, but it must not silently become the expansive recruiter product. Export both only with explicit labels and independent counts. A name such as `all_sources` is a filter choice, not proof that every possible website was scraped.

## 29. Recruiter workbook schema

Keep Excel as the product interface, not the database. It should permit a recruiter to filter by school, state, category, event, comparable best, recent evidence and coach contact without new network calls. Use stable IDs for joins across sheets and partition oversized detail tables deliberately.

### 29.1 `Athletes`

One row per accepted canonical Class-of-2027 athlete in the admitted school/geographic scope. Proposed fields:

```text
CanonicalAthleteId, Name, KnownNames, GraduationYear
CohortDecision, CohortEvidenceStrength, CurrentGradeEvidence
CurrentSchoolId, CurrentSchoolName, City, State, SchoolAssignmentState
Category, TFObserved, XCObserved, IndoorObserved, OutdoorObserved
AllObservedEvents, ComparableBestSummary, PerformanceCount, MeetCount
AthleticNetProfile, MileSplitProfile, OtherProfileReferences
HeadTFCoach, TFCoachProfessionalEmail, HeadXCCoach, XCCoachProfessionalEmail
AthleticDirector, ADProfessionalEmail
PreferredContactName, PreferredContactRole, PreferredContactEmail
ContactState, ContactSource, ContactObservedDate, ContactFreshness
SchoolAthleticsURL
ReportedRecruitingGPA, GPAScale, GPASource, GPAObservedDate
IdentityDecision, IdentityEvidenceStrength, SourceCount, IndependentSourceFamilies
HistoryCoverage, ConflictState, ReviewState, SnapshotId
```

Do not claim a grade or current coach that only follows from calendar arithmetic or stale evidence. A participation flag can use `Observed`, `NotObserved`, `Unknown` rather than a false binary where absence is not established. Clarify the workbook vocabulary so consumers do not read `NotObserved` as “does not compete.”

### 29.2 `PRs`

One row per athlete and comparable event/context. Include exact canonical value/unit, display notation, supporting performance IDs, date/meet, timing/wind/specification, comparison policy, source-reported PR observations, captured-best status, source disagreement and history completeness. Preserve ties and source links outside a single overly long cell when needed.

### 29.3 `Performances_*`

Every included source-backed canonical performance with athlete identity, source-athlete reference, meet/event, school represented at that performance, date, raw notation, normalized exact value/unit, timing, wind, specification, status, round/heat/place, source result IDs, document/evidence reference and identity-decision revision. Relay team result versus individual split must be explicit.

The performance sheet's population must be declared. Prefer details that join to the released athlete cohort. If a broader all-population performance archive is also supplied, separate it and name its denominator. Do not allow the seal to count cohort performances while the same field name in the workbook counts all athletes.

### 29.4 Supporting sheets and sidecars

Include `Coaches`, `Schools`, `Meets`, `Sources`, `Coverage`, `Conflicts`, `Review`, and `Run Metrics`. Add normalized profile/source-link detail where a cell limit would otherwise truncate provenance. The workbook may span multiple clearly named files if the payload is too large for one usable XLSX; the publication manifest must describe all partitions and their join IDs.

Set filters, frozen headers, useful widths and explicit units. Write source text as text, not spreadsheet formulas. Treat values beginning with `=`, `+`, `-` or `@` according to a tested safe-writing policy, including CSV sidecars. Validate hyperlinks before writing them. Never let untrusted source data introduce external workbook references or executable content.

The row/column/cell/hyperlink limits are properties of the pinned writer and target Excel format. Detect them before publication and partition all potentially large sheets, not just performances. Do not silently drop rows, shorten IDs or truncate event/provenance content. A summary cell can point to a complete normalized detail sheet.

## 30. Independent verification of the actual XLSX

The verifier reads the exact delivered XLSX files back from disk. Verifying an intermediate JSON object or recomputing an exporter checksum before writing does not establish the final artifact's correctness.

Use a frozen store/snapshot reader independent of the exporter's row-building code. It may share validated primitive types and the published schema contract, but it must independently check row identities, population filters, referential integrity, counts, contact states, event memberships, PR support and evidence requirements. Otherwise one shared projection bug can make exporter and verifier agree on the same wrong result.

For large tables, verify every row using streaming sorted keys, exact multisets, partition manifests and stable row digests. Random samples are useful exploratory checks but are not the final “every row” guarantee. Compare full key sets and duplicate counts, not only totals. Two different missing/duplicated rows can cancel in a count.

Mandatory checks:

```text
all accepted athlete IDs appear once in the primary cohort projection
all withheld/unresolved candidates are accounted for outside accepted identities
every exported performance has valid athlete/source/event/meet references
every PR is supported by at least one acquired comparable performance or explicit source claim
no rejected/ambiguous unit value contributes to a calculated best
every cohort acceptance has source-backed graduation evidence
every contact email is permitted, role-backed and traceable
all source failures and open/terminal gaps reconcile with the coverage view
all 49 planned jurisdictions appear with correct statuses; UNKNOWN is separately named
all partition counts and hashes reconcile with the publication manifest
all original input seed rows remain accounted for when that input contract applies
```

If the original two-sheet workbook remains an active seed source, preserve its SHA-256 and every real worksheet/row key and original field in an input accounting sidecar. Deduplicate enrichment work without losing duplicate source rows. If the national census has superseded workbook ingestion as a production input, state that explicitly; do not invent source-row checks for a workbook that was not consumed by this run.

Recompute required checks against the restored backup as well as the original snapshot. A restore with the same total row count but different PR ownership or mark units is a failed restore verification.

## 31. Backup, promotion and final sealing

Use a controlled, quiescent backup procedure unless the installed Fjall version provides a qualified consistent live snapshot mechanism that this implementation actually uses and tests. Draining, flushing and closing the sole writer is the simpler default. Do not copy a mutating database directory and call the result consistent.

Back up the database, immutable source capture references/content required for replay, operation receipts, schema/version metadata, run manifest, canonical decision history and artifact manifests. State exactly what the backup excludes, such as rebuildable temporary spill files. A backup that omits external raw evidence files is not a complete evidence backup if canonical rows depend on them.

Restore to a new directory, never over the live store during a test. Verify file manifests/digests, open with the intended compatible reader, run table integrity, compare logical/physical counts and receipt hashes, recompute selected semantic aggregates and run the full publication verifier against the restored snapshot. Record command, SHA, schema and every outcome.

A final `SealEvidence` document should identify the release commit, CI run, toolchain, lockfile, proof manifest, mandatory test outcomes, performance baseline/comparison, source coverage, run/snapshot IDs, mark/identity policy revisions, backup/restore evidence, XLSX partition hashes and verifier result. Use explicit typed states or a validated constructor for the seal. A free-form collection of booleans supplied by an agent is not sufficient proof.

Separate two concepts:

- **Execution conformance:** all required operations and verification lanes are accounted for and the product satisfies its declared contract.
- **Real-world coverage:** how much of the athlete/source universe was actually discovered and supported.

A source blocked under an approved policy can be a terminal collection gap while execution remains conformant. An unwired required production stage, skipped durability scenario, red CI, ambiguous mark migration or failed workbook verification is an engineering blocker. A retained `REVIEW` outcome is honest data, not necessarily an engineering failure, provided it does not appear as an accepted match.

Do not seal a run by removing required sources from its manifest after they fail. A materially reduced scope requires an explicit revised run/owner decision and a product label reflecting the reduction.

---

## 32. Implementation waves and dependency order

Do not launch another broad rewrite. Use the following order to reduce risk and avoid rework. Each task below also appears in the machine-readable ledger supplied with this document. A task is not verified merely because it appears in a later wave.

### Wave A: establish a trustworthy baseline and protect the corpus

**T01: Pin and reproduce current quality failure.** Record actual HEAD, tree state, CI status, toolchain and lockfile. Reproduce the failed quality command or obtain its authorized logs. Preserve evidence of the original failure. This baseline task closes when the failure is concretely reproduced or diagnosed with raw evidence and its repair tasks are linked. Passing release gates are required by T20 and T36; do not pretend the baseline is green or block all independent repair work waiting for it.

**T02: Audit legacy mark formats and freeze ambiguous writes.** Inventory actual historical observation shapes, writer versions and unit provenance. Identify whether unversioned old/new mark encodings coexist. Produce an executable compatibility corpus and migration plan. Do not update the live store until this task's safety conditions are satisfied.

**T03: Implement lossless versioned mark reading and migration.** Read all known old forms with their declared semantics and write an explicit new schema. Preserve original observations, raw values and version provenance. Ambiguous rows remain quarantined. Prove meaning-preserving conversion over an independently copied historical corpus.

**T04: Repair exact imperial comparison and unify mark interpretation.** First add the `3-0.75` versus `4-0.00` regression. Replace the report-local inconsistent parser with the shared exact kernel. Test fractional inches, metric equivalence, overflow, non-comparable statuses and source precision. No full-corpus PR publication until this passes.

**T05: Harden validated numerical types and deserialization.** Make constructors private/fallible, make units explicit, eliminate unchecked numeric casts and correctness saturation, and ensure serde cannot bypass the invariant. Add negative construction and full wire-version tests.

### Wave B: close durability gaps at the real effect boundary

**T06: Implement Fjall-atomic apply-once receipts.** Extend the existing StoreBatch implementation rather than replacing it. Bind receipts to the complete operation identity and payload. One commit contains observations, accounting and receipt. Duplicate application returns the same receipt; changed payload under the same ID fails closed.

**T07: Bound batch construction and recording.** Introduce row/byte/record/completion budgets before allocation. Partition large pages deterministically. Use stable chunk manifests and preserve raw evidence. Test one oversized row and many small rows separately.

**T08: Complete Ingest and source handoff routing.** The inspected meet prewrite path is already removed; verify the remaining teams/rosters/results/CLI routes. Route every production source effect through the intended durable application boundary. Avoid a second local append to make downstream reads convenient; await the single application receipt instead.

**T09: Implement S06 and the full crash matrix.** Use deterministic failpoints in the actual worker/Ingest path. Check physical observation and receipt counts after lost acknowledgement, not just canonical counts. A skipped scenario cannot satisfy release.

**T10: Validate retry ownership with source counters.** Prove three total external attempts under the chosen semantics across normal retry and restart. Verify no nested amplification, no automatic HumanRequired retry, and no budget reset by innocuous replay or cache reuse. Keep orchestration recovery distinct from external request attempts.

### Wave C: finish identity truth and review-cache safety

**T11: Preserve provisional source identity before candidate merging.** Wire every athlete-producing adapter through the source observation contract. A source-local stable ID or immutable source-row key must exist before grouping. Test same-name people with identical candidate fields.

**T12: Implement reversible canonical membership and projection.** Apply admissible decisions into the actual read model consumed by reports. Support aliases/redirects and split reversal without deleting source facts or moving performances by guess. Test transferred athlete, newly discovered smaller candidate and transitive contradiction.

**T13: Replace prose token-bag evidence hashes.** Introduce canonical structured facts and migrate case identities by policy revision. Test attribution swaps and sign/unit changes. Reuse only semantically identical snapshots.

**T14: Close the model-to-adjudicator contract.** One model per case, immutable evidence packet, validated candidate/evidence references, hard contradiction veto and terminal insufficient-evidence handling. Test actual local-compatible fixture HTTP replies, not only enum parsers.

**T15: Separate cohort evidence from identity confidence.** A grade observation must not by itself imply a high-confidence person match. Define independent typed cohort, school-assignment, identity and history states and export their meaning clearly.

### Wave D: make formal and empirical verification enforceable

**T16: Resolve the actual Kani inventory.** Enumerate the compiled harnesses and owning packages. Reconcile them with the fast/full runner manifests. Missing, duplicate, unintended or zero selected harnesses fail before execution.

**T17: Implement and run mandatory kernel proofs.** Cover exact marks, PR algebra, identity hard rules, structured evidence meaning, link transitions, batch arithmetic, attempts, scope and seal finalization. Record bounds, covers, stubs, versions and raw results. Do not count a `cargo test` result as Kani evidence.

**T18: Strengthen production parser fuzz/property lanes.** Include malformed/oversized data and exact accepted/rejected/quarantined accounting. Recover mature regression fixtures from history where appropriate. Tests must call the surviving implementation.

**T19: Exercise owned concurrency and critical mutants.** Use bounded interleaving models and real runtime scenarios. List surviving dangerous mutants; do not bury them under an aggregate score.

**T20: Make release gates fail on missing evidence.** Distinguish a convenient development gate from a strict release gate. Missing tools, skipped scenarios, zero proofs, empty benchmark output, missing verifier artifacts and stale commit evidence refuse release.

### Wave E: establish honest performance measurement and optimize

**T21: Fix benchmark capture and comparison.** Parse actual pinned Criterion artifacts, compare complete case sets, validate all units/numbers, reject incomparable environments, and eliminate fixed temp paths and unsafe shell interpolation.

**T22: Measure the right runtime and memory scope.** Compile first, then execute the intended benchmark binary. Use correct process/cgroup accounting and raw evidence. Missing measurements are not zero. Test runner failure behavior with synthetic malformed outputs.

**T23: Record representative reference baselines.** Use the designated workstation, known corpus digest, warm/cold modes, repetitions and peak-memory budgets. Do not fabricate measurements in the repository because a schema expects numbers.

**T24: Optimize the measured heavy paths.** Prioritize repeated scans, large recordings/journals, allocation-heavy identifiers/evidence, source normalization, store apply-once and export. Keep only measured improvements with invariant parity. Do not turn compact IDs into a destructive wire migration.

**T25: Capacity-test national limits.** Exercise skewed candidate groups, large histories, response/batch ceilings, spill partitions and the table-cardinality policy. No hard limit may allow writes that later normal reads cannot interpret safely.

### Wave F: finish nationwide source and recruiting coverage

**T26: Produce the complete 49-jurisdiction matrix.** Label unresearched gaps explicitly, identify real source capabilities and connect them to the existing registry. Separate common base discovery from supplemental qualified sources.

**T27: Bind remaining useful sources to natural durable units.** Meet-based, athlete-based, directory-based and source-page workflows should share resources where appropriate. Do not repeat one national directory for every state.

**T28: Qualify all-event/history acquisition.** Verify source precision, TF/XC history, relay membership and terminal pagination against captures. Keep excluded, unsupported and missing-result states rather than discarding rows.

**T29: Complete current school/coach/contact projection.** Add temporal assignment and explicit contact outcomes with official public provenance. Fill gaps through school/directory workflows without guessing addresses.

**T30: Implement gap-driven finishing sweeps.** After broad discovery, create explicit gap work for missing school/cohort/performance/contact/evidence. Reuse captured responses and successful operations. A missing field must not restart the whole source crawl.

### Wave G: publish only from verified immutable evidence

**T31: Freeze manifest/snapshot identity.** Bind run/source/parser/schema/mark/identity/model policies and the immutable observation boundary. Stop selecting outputs by modification time.

**T32: Finish all-source recruiter workbook generation.** Include normalized details, complete event coverage, PR claim distinctions, contact states, partition manifests and safe text handling. Preserve the independent core-only diagnostic without making it the default rich export.

**T33: Implement exhaustive actual-artifact verification.** Check every row/key and reference against the frozen snapshot independently of exporter projection code. Sampling is supplementary only. Verify source input accounting where those workbooks were actually consumed.

**T34: Execute cold backup/restore and semantic verification.** Restore to a new directory and compare counts, identities, units, receipts, source references and publication results. Do not overwrite the live store in a drill.

**T35: Run progressive real pilots and the complete run.** Start with a stratified small cohort, then expand only after correctness/resource gates. The final national run reuses durable work and reports all remaining source/identity gaps.

**T36: Seal, synchronize and hand off.** The final manifest must contain exact passing evidence for the final release SHA and snapshot. Publish all artifacts, retain blocked-source/readiness information and leave the repository synchronized without disturbing unrelated user work.

## 33. Regression canaries that must never disappear

The following small scenarios should become permanent tests. Their purpose is to catch classes of errors that a large happy-path corpus can miss.

| Canary | Expected property |
|---|---|
| Old JSON `TimeSeconds: 60` versus versioned centiseconds `60` | Writer version determines units; no heuristic based on integer appearance. |
| `3-0.75` versus `4-0.00` | Four feet is the better field result. |
| `5-4.00` versus `5-4.25` | Distinct quarter-inch marks remain distinguishable. |
| `0.5` inch versus `0.50` inch | Exact equivalent fractions. |
| Two compatible time results differing in the source's third decimal | Distinction is retained until an explicit comparison/reporting policy says otherwise. |
| Same school, same name, same class, same category, different students | No automatic canonical collapse. |
| Same student before and after a school transfer | One admissibly resolved person, preserved school history and source result ownership. |
| Source IDs `123` from two different namespaces | No cross-provider identifier collision. |
| Add a smaller-sorting candidate to an existing cluster | Public identity stays stable or receives an explicit, atomic alias transition. |
| A matches B and B matches C, but A contradicts C | No unchecked transitive merge. |
| Swap which side owns graduation year 2027 | Review evidence identity changes. |
| Reorder a genuine unordered fact set | Review evidence identity remains the same. |
| Fjall commit succeeds and acknowledgement is lost | One physical application, reusable durable receipt. |
| Successful source page with zero athletes | Explicit successful-empty receipt, not owed forever. |
| Same operation key, changed table or payload | Typed idempotency conflict, no silent reuse. |
| Empty derived-snapshot rebuild | Previously materialized stale rows are removed. |
| Benchmark parser returns no cases | Performance gate fails. |
| One baseline benchmark is missing in current results | Performance gate fails. |
| Kani selects zero harnesses or a wrong package | Proof gate fails. |
| S06 is skipped | Release gate fails. |
| An unreached state has an empty applicability list | State is unresearched/uncovered, not completely empty. |
| A 4×400 team result names an athlete but no split | Team time is not an individual 400 m PR. |
| XLSX source text begins with a formula prefix | It remains safe text, not executable spreadsheet content. |
| Newest output file is a failed partial export | It is never chosen as the published workbook. |

## 34. Real-data rollout and throughput expansion

The initial live pilot must be stratified, not simply the first hundred alphabetically sorted records. Include both categories, TF/XC, indoor/outdoor where available, same-name cases, transfers, profiles with no marks, uncommon events, relay participation, a source failure, a coach conflict and multiple jurisdictions/providers. A 100-row pilot cannot cover every event/category/state combination; record the coverage it actually exercises.

Use approximately 100, then 1,000, then 10,000 relevant athlete rows or equivalent bounded source units before full expansion. Include every admitted jurisdiction in the validation program, but do not pretend that one athlete from a state proves that state's source completeness. Pilot denominators and source coverage are separate evidence.

At each expansion, reconcile source objects, observations, candidates, canonical identities, performances, PRs, contact outcomes, review cases and workbook rows. Confirm cache/receipt reuse so a larger stage does not re-pay for work already completed. Compare source request amplification and queue/memory behavior under actual admitted traffic.

Do not expand past an unexplained false-positive identity, unit conversion error, missing observation, failed receipt replay, failed mandatory gate or unbounded resource growth. Stop the affected lane, preserve evidence and fix it. A polite source outage in one state does not require terminating unrelated work, but it must remain a named gap.

Measure useful verified output per source request, not merely scheduled tasks or open tabs. Measure review bypass rate and GPU latency separately from deterministic ingestion. A faster run that accepts the wrong person or quantizes away a better mark is a regression, regardless of its throughput number.

## 35. Exact agent ownership and handoff format

Before dispatching parallel agents, Main commits stable contracts for mark schema, source/provisional identity, canonical links, application receipts, run manifests and proof/performance evidence. Agents may propose contract changes but must not independently rewrite shared types in overlapping branches.

Suggested implementation lanes:

| Lane | Primary ownership | Dependency and boundary |
|---|---|---|
| Integration/quality | workspace, shared ADRs, CI, final task ledger | Main alone accepts shared contract changes. |
| Mark kernel/migration | domain mark modules, versioned readers, mark fixtures | Coordinate parser callers through declared interfaces. |
| Store/effect durability | store batch/receipt code, Ingest integration tests | Never mutate the user's live store while testing. |
| Identity/reconciliation | provisional identities, canonical links, evidence hashing | One owner for all public identity semantics. |
| Local review | model packet/schema/assignment integration | Cannot override deterministic rules. |
| Proof | actual Kani manifest/harnesses and bounded kernels | No alternate implementation under proof-only cfg. |
| Performance tooling | benchmark parser/runner/baseline schema | No baseline numbers without execution. |
| Source adapters | assigned provider/state directories and fixtures | No shared registry edits without Main integration. |
| Export/verifier | output schema and separately implemented verifier | Protect independent verification logic. |
| Operations | test deployment, crash harness, backup/restore | Only session-created test services may be managed automatically. |

A handoff must contain the task ID, base SHA, changed paths, design choice, required migrations, commands actually run, raw results, proofs/benchmarks affected, known risks and next dependency. “Tests pass” without commands and output is insufficient. “Implemented” does not mean “verified.”

Review agents should inspect actual diffs and counterexamples independently. Do not have the implementing agent generate an approval paragraph for itself and present that as an independent security, async or DDD review. A reviewer must cite concrete code/evidence and unresolved findings.

## 36. Commands: current surfaces versus required additions

The workspace already exposes developer-command infrastructure. At the pinned source, `xtask` includes Kani and performance command implementations. Their presence does not establish successful execution or the exact CLI flags a future agent should assume. Start by inspecting the built command's `--help` and the current manifest before issuing a verb.

The standard Cargo checks in Section 8 are required commands. Existing `cargo xtask gate`, census status, coverage, export, source test and replay surfaces should be reused and repaired rather than replaced with a parallel script collection.

The following are **required capabilities**, with proposed verb spellings where not already implemented:

| Capability | Suggested surface | Required behavior |
|---|---|---|
| Current goal state | `cargo xtask goal-status` | Read task/evidence manifests and report blockers without starting collection. |
| Strict release gate | `cargo xtask gate --release` or existing equivalent | Missing mandatory tools/results fails. |
| Actual proof inventory/run | existing Kani command with manifest-backed fast/full modes | Exact selected harnesses, owning crates and verdicts. |
| Benchmark record/check/profile | existing perf command family | Validated artifacts, correct memory/process scope, full expected case set. |
| Mark schema inventory | `cargo xtask migration audit-marks` | Read-only unit/version inventory, never guesses ambiguous rows. |
| Candidate-store migration | `cargo xtask migration migrate-marks` | New destination, source preserved, exhaustive compatibility evidence. |
| Durability scenarios | existing durability harness with a scenario manifest | Real production paths; skipped mandatory scenarios fail. |
| Source matrix | `cargo xtask source-matrix` | All 49 jurisdictions with capability/execution/gap states. |
| Exhaustive export verification | `cargo xtask verify-export` or existing verified command | Reads actual XLSX partitions against an exact frozen snapshot. |
| Backup/restore drill | existing backup drill command | Owns only test destination, proves semantic restoration. |

Do not paste a proposed command into a release checklist and check it off because it looks plausible. The implemented CLI must be discoverable, bounded, documented and tested. The accompanying bootstrap prompt explicitly requires this distinction.

## 37. Evidence manifest and automatic acceptance

Keep machine-readable evidence under a dedicated run/commit directory, for example `reports/verification/<sha>/<run-id>/`. This is a proposed organizational convention; migrate existing evidence carefully rather than deleting it.

Each execution record contains:

```text
evidence_id
requirement/task IDs covered
source commit and dirty-tree state
command argv and working directory
toolchain/tool versions and dependency lock digest
start/end timestamps
input/corpus/snapshot hashes
resource limits
exit status
outcome classification
raw stdout/stderr artifact references
produced artifact hashes
reviewer disposition where required
```

The acceptance engine must validate the evidence's schema and association with the release commit/snapshot. It must not trust a hand-entered `passed: true` for a missing log. Required artifacts must exist and match their hashes. A command that succeeded while exercising zero cases must be rejected when the contract requires a nonempty set.

Do not require every component to be rerun for an unrelated documentation change during iterative development. Do require the final release evidence to establish that the exact release configuration satisfies all mandatory gates. Versioned evidence provenance and explicit reuse rules make that distinction auditable.

Recommended top-level delivery state vocabulary:

```text
EngineeringBlocked
ReadyForPilot
PilotAccepted
CollectionRunning
CollectionTerminalWithDeclaredGaps
SnapshotVerified
RestoreVerified
ReadyToSeal
Sealed
```

These are proposed states, not a claim that the current implementation has them. The transition to `Sealed` must be a deterministic validated operation over the evidence manifest, not a narrative judgment.

## 38. Stop conditions and operator partnership

Pause the affected operation and ask for a specific intervention when credentials, source authorization or manual browser interaction are required. Continue independent permitted work. Never ask the user to re-confirm already supplied Athletic.net permission; ask only for the concrete missing runtime action or contract.

Stop all writes to the affected store when an integrity, migration, backup or restore verification failure could threaten evidence. Preserve the current database and diagnostic artifacts. Do not repair by deleting rows, rebuilding from scratch or restoring over the live path without authorization and a verified recovery plan.

Do not stop/restart user-owned model servers, browsers or Restate installations without the agreed maintenance boundary. Test crash scenarios against session-created services and copied stores. Record which processes/directories the harness owns and refuse to operate outside them.

A source's hard access block, automation prohibition or exhausted retry budget produces a terminal source disposition. It is not an instruction to try another IP, mutate fingerprints, guess tokens, bypass paywalls or omit the failure from coverage.

A blocking compiler/test/proof defect should be corrected, not escalated merely because the work is lengthy. Escalate when an explicit owner decision or unavailable capability is genuinely necessary. State the exact condition, evidence, attempted remedies and safe independent work that remains.

## 39. Final delivery package

Publish a new immutable artifact set with:

```text
run-manifest.json
source-matrix.json and human-readable source coverage
schema-and-policy-manifest.json
canonical-snapshot-manifest.json
recruiting workbook(s)
normalized PR/performance/profile sidecars where required
review/conflict/contact-gap outputs
workbook-verification.json
proof-manifest and mandatory proof results
quality/security/dependency/license results
performance-baseline/comparison and raw measurements
durability scenario manifest and raw results
backup-manifest and restore-verification.json
seal-evidence.json
operator-runbook.md
```

All filenames are proposed artifact roles. Use stable actual names in the implementation and include their exact paths/hashes in the publication manifest. Do not create empty files merely to satisfy a list.

The operator runbook explains how to inspect status without changing data, resume the same run, supply a manual browser resolution, replay cached captures under a new parser, review unresolved identities, verify an export, back up/restore safely and stop the session-owned services. It must not depend on a hidden conversation transcript or an agent's working memory.

The final handoff states what was achieved and what remains unknown. It names covered jurisdictions, source coverage, accepted/cohort-unresolved candidate counts, performance/PR coverage, contact outcomes, blocked sources and historical completeness limits. It does not inflate “source-verifiable census” into “every real athlete in America.”

## 40. Immediate next execution sequence

Start with an integration checkpoint, not another source crawl:

```text
1. Verify actual HEAD and preserve unrelated changes.
2. Reproduce the exact current quality failure.
3. Freeze ambiguous mark writes and inventory historical wire formats.
4. Add the imperial PR and legacy-unit regression canaries.
5. Repair/version the shared mark kernel and migrate a candidate copy.
6. Implement Fjall apply_once and the real commit/lost-acknowledgement test.
7. Replace token-bag review hashing with structured evidence identity.
8. Complete source-first provisional identity and applied canonical membership.
9. Validate the actual Kani harness set and run the required production-kernel proofs.
10. Repair perf capture/comparison, then measure rather than invent the baseline.
11. Expand the national source matrix and durable bindings in independent source lanes.
12. Run stratified pilots, full snapshot/export verification and restore drills.
13. Seal only the exact verified snapshot and release commit.
```

Research, fixture preparation and official coach-source discovery can proceed while the mark/store blockers are being repaired, provided they do not write incompatible data into the live canonical corpus. Architecture is no longer the primary problem. Correct semantics, verified recovery and truthful evidence are.

## 41. Superseded recommendations: do not re-open completed work

Do not recreate the root acquisition product or the `acq-*` family. Do not assign “extract census-store/crawl/review/report/service” as new tasks simply because an older memo says they are missing. Do not create a second StoreBatch because an earlier snapshot lacked one. Do not patch the meet handoff back into a local prewrite; its inspected current version deliberately removed that duplication. [R09, R10]

Do not assume all old functionality survived deletion merely because the architecture became smaller. Recover and port a specific useful test or exact parser contract from git history when a current requirement needs it. The objective is one surviving implementation with demonstrated behavior, not preservation of every historical subsystem or indiscriminate code deletion.

Do not present newly added Kani/perf files as newly passed Kani/perf programs. Their runner paths are implementation work; their actual nonempty results are verification work. The distinction applies to every artifact in the release package.

## 42. Governing engineering rules

Restate owns what work must happen. Fjall owns what was observed and which external application effects were committed. Deterministic Rust owns what the evidence means. AI provides bounded advice about unresolved identity. Excel is a versioned projection of the evidence, not the system of record.

A canonical identity decision may not destroy the source identity or attribution needed to reverse it. A type named “validated” must not be constructible from unchecked fields or unchecked deserialization. A fixed-point integer is not exact if conversion discards source precision or changes units without a version.

A performance change is complete only when its semantic invariants still hold and a reproducible measurement supports the claimed improvement. A fast proof that selected no harnesses is not a proof. A benchmark that parsed no measurements is not a baseline. A crash scenario that says SKIP is not recovery evidence.

A successful request is not a complete source. A complete source plan is not a complete population. A missing contact is not an invitation to guess. An unresolved identity is not a match. A passing old commit is not evidence for current main.

**Finish by producing evidence, not by producing another promise of evidence.**

---

## 43. Source and evidence index

This document uses the pinned repository as its implementation basis. It adds explicit requirements and source-derived counterexamples where the user requested a stronger final build goal. Paths below are source inspection references, not execution certificates. Official external documentation is cited only for framework/format semantics. The live agent must verify its installed versions and the exact source it actually builds.

### Repository references

**R01. [Latest main recheck and pinned commit](https://github.com/lprior-repo/athletic-rust-pipeline/commit/183fca13dd6d48165d04c76fd56648998e6b432f)**  
Observed branch SHA and commit timestamp; not a runtime gate result.

**R02. [GitHub Actions run for the reviewed SHA](https://github.com/lprior-repo/athletic-rust-pipeline/actions/runs/36020764525)**  
Observed completed failure; exact commit association was retrieved.

**R03. [Failed Quality gate job](https://github.com/lprior-repo/athletic-rust-pipeline/actions/runs/36020764525/job/107715786629)**  
Observed step failure and later skipped feature checks. Job-log retrieval failed with credentials error, so root cause was not inferred.

**R04. [Fixed mark representation](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/fixed_mark.rs)**  
Transparent integer serde wrappers, public scalar fields, legacy-wire claim and float conversion boundary.

**R05. [PR comparison and imperial conversion](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-report/src/bests/measure.rs)**  
Measure::value, parse_field_imperial and parse_hundredths; source-derived unit/order counterexample.

**R06. [Kani runner](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/xtask/src/kani.rs)**  
Fast/full/list surfaces and declared harness selection. Successful proof execution is not established by file existence.

**R07. [Production Ingest handler](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/ingest.rs)**  
Restate-side seen_operations state surrounding an external store effect; operation digest and lost-ack boundary.

**R08. [Performance benchmark capture](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/xtask/src/perf/bench.rs)**  
Benchmark output assumptions, RSS collection, temporary-file handling and empty result behavior.

**R09. [Existing multi-table StoreBatch](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-store/src/write_batch.rs)**  
Batching already exists; accounting, total bounds and application receipt are the next requirements.

**R10. [Current meet-stage handoff](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/meets_arms.rs)**  
Inspected take_recorded handoff no longer performs the old direct local prewrite.

**R11. [S06 crash/duplicate-evidence scenario](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/tools/durability/scenario-06-no-duplicate-evidence.sh)**  
Script explicitly skips the named crash-injection scenario; other tests do not substitute for it.

**R12. [Current domain Kani module wiring](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/kani/census_domain_wiring.rs)**  
Actual included harness modules must be reconciled with the runner inventory.

**R13. [CaseEvidence and review case identity](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/records.rs)**  
normalize_fact sorts tokens within text facts; role-preserving structured identity is required.

**R14. [Candidate and canonical athlete types](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/athlete.rs)**  
Four-field candidate key and singleton canonical mint behavior; source-person separation remains a requirement.

**R15. [Performance comparison](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/xtask/src/perf/compare.rs)**  
Current-result iteration, missing cases, environment handling and comparison acceptance.

**R16. [Source applicability](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-crawl/src/applicability.rs)**  
Twelve-state mapping and empty fallback. Does not establish a qualified 49-jurisdiction supplemental source plan.

**R17. [Repository endgame research](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/research/ENDGAME-GAPS.md)**  
Repository-reported research/gaps, not independently rerun measurements.

**R18. [Restate jobs and journal handoff](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/jobs.rs)**  
Source write helpers, recorded completion handoff and error boundaries.

**R19. [Jurisdiction pipeline integration](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/jurisdiction/pipeline.rs)**  
Stage/recording boundaries and posting flow; large durable payloads require explicit bounds.

**R20. [Grade-year Kani harnesses](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/kani/gradyear.rs)**  
Concrete current proof surface and assumptions/covers, separate from proof execution evidence.

### External primary documentation

**E01. [Restate Rust durable steps](https://docs.restate.dev/develop/rust/durable-steps)**  
External side effects and journaled durable-step behavior. Verify against the installed Rust SDK/runtime.

**E02. [Restate Rust error handling](https://docs.restate.dev/develop/rust/error-handling)**  
Step retries, terminal errors and invocation behavior; do not conflate handler recovery with physical request attempts.

**E03. [Serde container attributes](https://serde.rs/container-attrs.html)**  
Transparent wrappers serialize/deserialize through the inner field; comments do not change units.

**E04. [Kani harness listing](https://model-checking.github.io/kani/reference/experimental/list.html)**  
Machine-readable harness discovery; validate the installed command version.

**E05. [Criterion command-line options](https://bheisler.github.io/criterion.rs/book/user_guide/command_line_options.html)**  
Actual benchmark CLI/output contracts; inspect the pinned executable and artifacts.

**E06. [TigerBeetle testing practices](https://docs.tigerbeetle.com/coding/testing/)**  
Reference for deterministic testing and fault discipline, not a claim of equivalent assurance.

**E07. [TigerBeetle safety practices](https://docs.tigerbeetle.com/coding/safety/)**  
Reference for explicit bounds and invariants. Requirements in this document are adapted to this Rust/Restate workload.

### Review evidence limits

The CI failure is an observed GitHub Actions result. The mark, PR, identity, Ingest, applicability and runner findings are static source findings. The arithmetic and token-normalization counterexamples are independently derived examples, not new Rust/Kani test executions. No current-corpus row count, live throughput, GPU capacity, restored-store result or final workbook pass was measured in this review. The agent executing this goal must produce those artifacts itself.

When main advances, record the new SHA, inspect the relevant diff and reclassify each finding as still present, repaired-but-unverified, verified or superseded. Do not blindly replay an older patch list against a changing implementation. This goal's invariants remain binding even when paths move.

**End of governing contract.**
