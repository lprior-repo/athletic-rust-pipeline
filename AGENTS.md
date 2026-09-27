# AGENTS.md — working instructions for this repository

Primary instruction document for any person or coding agent working here. Read this first, then
`ARCHITECTURE.md`, then `docs/adr/README.md`.

## What this repository is

A Class-of-2027 high-school Track & Field / Cross Country recruiting census over the run scope of
ADR-009: the 48 contiguous states plus D.C. A qualified
source graph feeds durable acquisition, acquisition produces evidence, evidence lands in Fjall, a
deterministic merge proposes canonical identities, and the local Qwen lane advises only where Rust
cannot resolve uncertainty. Rust adjudicates; models do not establish identity. The result is
projected into an Excel workbook a recruiter can filter. The workbook is a projection; the durable
evidence system is the census.

## Required delivery contract

- Build a production-grade, fully Rust pipeline that processes every real row from both input
  workbook sheets. Preserve the original workbook unchanged, retain stable source-row identities,
  and reconcile every input row to an explicit outcome, including unresolved `REVIEW`.
- Discover athlete information across as many relevant sources as can be found: Athletic.net,
  MileSplit, DirectAthletics, relevant TFRRS records, AthleticLIVE, state associations, timing
  companies, meet organizers, school and team websites, and linked results in HTML, structured
  data, PDFs and spreadsheets. Continuously discover additional relevant sources through school,
  team, meet and results links, subject to bounded work and source policy.
- Acquire team rosters and meet results once and reuse them across applicable athletes. Maintain a
  shared local evidence index so workbook rows do not repeat searches, downloads, parsing or
  unchanged model reviews.
- Track source coverage, pagination, retrieval failures, unsupported formats and unfinished
  discovery explicitly. Incomplete coverage must never become a definitive no-match result or a
  claim of exhaustive coverage. Source failure is not evidence of athlete absence.
- Verify Class-of-2027 identity and distinguish Track & Field from Cross Country participation.
  Return source profile links, verified marks and traceable evidence.
- Resolve cross-source identities using corroborated school, location, graduation-year, season
  and participation evidence. Never merge on name alone or count syndicated copies as independent
  corroboration. Preserve provider-specific athlete IDs and URLs, conflicting observations and
  provenance for every accepted fact; identity decisions must remain reversible.
- Deduplicate performances without losing source references. Compare marks only within compatible
  events and conditions. Distinguish source-declared PRs from best observed marks when coverage is
  incomplete; retain historical school/team affiliation and legitimate rounds, heats and attempts.
- Use one canonical implementation per responsibility and one shared workflow. Source adapters own
  acquisition and format differences, but all sources feed the same domain types, identity rules,
  performance normalization, review process and output logic. Remove legacy compatibility,
  duplicate business logic, ad hoc exceptions and competing execution paths. Model legitimate
  source differences and athlete edge cases explicitly.
- Perform reliable discovery, extraction, normalization, matching, contradiction detection and mark
  calculation deterministically before invoking AI. Clean deterministic matches bypass AI.
- Treat the local Qwen llama.cpp servers on the RTX 5090 and RTX 3090 as scarce review capacity.
  Unresolved identities and cross-source conflicts receive independent reviews from both servers,
  using the permitted full local row and prepared, source-linked evidence. Acceptance requires
  sufficient corroboration, satisfaction of deterministic rules and reviewer agreement. Agreement
  cannot replace evidence; unresolved uncertainty remains `REVIEW`.
- Keep admissions data and model review local. Do not expose private workbook rows to cloud models,
  development-agent prompts, public logs, committed fixtures or remote services.
- Use locally hosted, non-Docker Restate with its Rust SDK for durable orchestration, idempotent
  operations and crash recovery. Restate is the sole retry owner; transport performs one attempt.
  The repository's stricter three-total-attempt ceiling remains within the requested maximum of
  three retries after the initial attempt. Preserve every source row, candidate, observation,
  decision and exhausted failure under stable logical identities.
- Export accepted identities, graduation evidence, sport classifications, source links, verified
  marks, coverage status and review reasons. Publish snapshot-bound workbook and audit artifacts
  atomically; never replace a valid published bundle with a partial one.
- Existing F01–F15 corrections, named regression acceptances and all 17 durability scenarios remain
  delivery obligations. A change in staffing or sequencing does not narrow the requested scope.


## Read first

1. `ARCHITECTURE.md` — pipeline, crate boundaries, invariants, and the binding standards (§-numbered:
   §9, §55 and §70 resolve there; the §38/§49/§56 family comes from the mission brief, which is not in
   this tree, and this file restates those standards where they bind).
2. `docs/adr/README.md` — decisions that may not be silently re-litigated.
3. `docs/migration/module-map.md` — the module inventory and the crate cut being executed.
4. `crates/census-service/README.md` and `xtask/README.md` — the two crate-root READMEs that exist;
   keep additional architecture, domain and operating rationale in separate documentation, not code.
5. `docs/OPERATIONS.md` — operations runbook; successor to the superseded `HANDOFF.md`.
6. `docs/FJALL_BACKUP.md` — Fjall backup and restore procedures.
7. `docs/deployment-lifecycle.md` — deployment and lifecycle management.

## Commands

| Command | Purpose |
| --- | --- |
| `cargo xtask gate [-- <gate args>]` | every lane of `tools/gate.sh` — the four build gates (§55), the scans, the debt ratchet, and each optional tool lane that is installed; it runs them all and reports the failing set |
| `cargo xtask contract` | the eight workspace contract checks, adapter registration and the documented set among them |
| `cargo xtask census-status [--ingress [<ORIGIN>] \| --store <DIR>]` | current store state, counted by the serving census (`Census/status`) or read from the store directly |
| `cargo xtask coverage [--ingress [<ORIGIN>] \| --store <DIR>]` | coverage summary (§49 fields) |
| `cargo xtask export [--ingress [<ORIGIN>] \| --store <DIR>]` | workbook export (`Workbook/run`) |
| `cargo xtask source-check <name>` | alias of `source-test`: the census-service tests covering one source |
| `cargo xtask source-fixture <name>` | lists the captures under the crate's `tests/fixtures/<name>/` (read-only) |
| `cargo xtask source-test <name>` | offline fixture test for one source |
| `cargo xtask replay <name>` | deterministic offline parser replay |
| `cargo xtask new-source <name>` | scaffold a new adapter |
| `cargo xtask bench -- parser` | parser benchmarks; filters after `--` are forwarded to `cargo bench` |

`census-status`, `coverage` and `export` default to the serving census: the flag-free form and
`--ingress [<ORIGIN>]` (default `http://127.0.0.1:18095/`) submit the matching Restate handler and are
safe to run beside `census-serve`; origins must be loopback HTTP with no path or credentials.
`--store <DIR>` opens the store in-process instead, so it requires `census-serve` to be stopped — the
store is single-writer, and the lock error it returns while the census serves is the correct answer,
not a bug.

Binary: `./target/release/census-service` — the `census-service` bin of `crates/census-service`; its
`serve` subcommand prints the `census-serve` command line that runs the endpoint against this store.
Store root: `var/census-service` (the `--data-dir` default); the delivered census of ADR-009 lives
in `var/midwest-census` (the store the seal, the workbook and the §58 verification were built from).
**The store is single-writer**; lanes that write must serialize.

## Crate ownership

| Crate | Owns | May not |
| --- | --- | --- |
| `athleticnet-browser` | persistent headed profile, tab pool, CDP request/response capture, challenge classification, retry-after header classification, `BrowserError` failure vocabulary | retry (belongs to the caller), solve challenges, spoof headers, handle logins, proxy rotation |
| `census-domain` | pure types, cohort/grade evidence, event canonicalisation, PR ordering rules | depend on tokio, fjall, reqwest, chromiumoxide, restate, xlsx, llama clients
| `census-store` | Fjall keyspaces, journals, snapshots, migrations, backup/restore | know about HTTP or parsers
| `census-crawl` | source adapters, fetchers, admission, browser supervisor | write canonical entities
| `census-reconcile` | identity normalisation, deterministic scoring, conflict detection | call models
| `census-review` | the local Qwen identity-review lane | decide identity (it advises; Rust adjudicates)
| `census-report` | coverage, bests, PR projection, workbook/export | mutate evidence
| `census-service` | CLI, Restate workflows, bootstrap and task supervision | bypass the store's durability rules
| `xtask` | the agent-facing verbs above | hold business logic

Main (the architect) owns: workspace layout, domain public contracts, serialized public types, Fjall
schema revisions, Restate workflow contracts, migrations, and final integration. Adapter owners own
their adapter directory, its fixtures, and its `research/sources/<source>/` report. Propose shared
contract changes; Main applies them.

## Coding standards (binding)

The standards in `ARCHITECTURE.md` and the following requirements are binding:

- **Zero code comments, permanently.** Do not add or retain comments in project-owned source,
  tests, examples, benchmarks or generated project-code templates. This includes ordinary line and
  block comments, Rust doc comments (`///`, `//!`, `/** */`, `/*! */`) and documentation attributes
  used to move prose back into code. Express intent through names, types, functions, errors and
  tests. Maintain design rationale separately. A blocking lexical check must enforce this rule;
  comment-like bytes inside strings or captured source evidence must not be mistaken for comments.
- Apply Scott Wlaschin's type-design principles: validated newtypes, private fields, checked
  constructors, exhaustive algebraic data types and explicit state transitions. Separate raw
  observations, validated evidence, candidate identities and accepted matches. Make illegal states
  unrepresentable wherever practical.
- Apply Holzmann-style discipline: bounded work, simple control flow, checked arithmetic, explicit
  resource limits and complete error handling. Forbid project-owned unsafe code and input-triggered
  panics. Keep `#![forbid(unsafe_code)]` and `#![deny(unused_must_use)]` workspace-wide.
- Files stay within 300 lines; production functions within 60 logical lines, hot paths within 25.
  Decompose long orchestration into named stages rather than suppressing checks.
- Use `thiserror` inside production crates, `anyhow` only at CLI/composition boundaries, and
  structured `tracing` in production paths. Preserve async outcomes and supervise shutdown with
  explicit drain accounting.
- Cache only immutable successes; quarantine poisoned objects and retain failures rather than
  silently falling back. Admission is per remote origin and counts physical requests, not workflows.
- Optimize measured end-to-end throughput on the 16-core/32-thread, 128-GB machine through streaming,
  shared caches, bounded parallelism, buffer reuse and minimal copying, allocation, contention and
  repeated I/O. Use Toyota Production System principles: eliminate rework, constrain work in
  progress, expose defects immediately, apply backpressure and improve the measured bottleneck.
  Performance claims require measurements; more agents or files changed are not throughput proof.
- Rust only: no Python and no shell scripts as pipeline steps.

Relevant architecture references: §37 (panic/error discipline), §38 (size budgets), §39 (errors),
§42 (supervised shutdown), §43 (async outcomes), §44 (tracing), §55 (gates), §56 (forbid/deny),
§61-§62 (cache policy).

## Source policy

Robots and per-origin admission are honored. No CAPTCHA, authentication, or paywall circumvention.
Athletic.net is acquired through the headed persistent-profile browser lane (§26-§28) with a
`HumanRequired` handoff when a challenge appears. Never collect athlete personal contact data; never
infer GPA (§36). A §69 stop condition for one source is persisted and the census continues elsewhere.

## Delivery workflow and release gates

1. Synchronize incoming `main` changes while preserving local work and coordinating every writer.
   Freeze shared interfaces before parallel implementation.
2. Keep 2–4 useful subagents once implementation is underway. The requested four-worker allocation
   is one `gpu5090-coder`, one `gpu3090-coder` and two `deepseek-flash` evidence workers. Main scopes
   and schedules their work; never create filler assignments to maintain activity. Keep coding to
   coherent, non-overlapping slices. One owner retains a slice through implementation, caller
   migration and verification.
3. Main handles shared contracts, identity acceptance and durability. Use local Qwen for bounded
   caller migrations and fixture work, and Flash for specific evidence collection, not repeated
   architecture audits. Workers must not invent mirrored types, edit outside ownership, silently
   change interfaces or reactivate finished peers.
4. Compile and run focused tests after each integrated slice, before dependent work expands.
   The previous “finish all F01–F15 before any checks” restriction is superseded. Existing defects
   are blockers to repair, not permission to defer verification or weaken a gate.
5. Finish one working end-to-end workbook-to-evidence-to-output path first, then extend coverage
   through the same implementation. Measure verified slices completed and defects escaping review,
   not agent activity or files changed.
6. Make testing a release gate. Exercise every defined state transition, source contract, acceptance
   rule, boundary and recovery path. Every discovered defect gains a regression test.

Required release evidence includes:

- Adversarial identity fixtures, cross-source conflicts, syndicated duplicates, schema changes,
  pagination gaps, incompatible marks and complete input/output reconciliation.
- Property tests, parser fuzzing, concurrency and cancellation, duplicate delivery, fault injection,
  crash recovery, retry exhaustion and malformed model responses.
- Critical-logic mutation testing, representative load tests, throughput and resource measurements,
  strict formatting and Clippy, dependency auditing, and adversarial security and async-Rust review.
- Real execution of the named durability scenarios against isolated local-disk scratch state.
  Simulated success, a narrowed test or a passing build does not prove the intended fault.
- Exercise changed CLI, Restate and export paths; verify generated artifacts against durable
  evidence and confirm the original workbook remains unchanged.

Unresolved accuracy, data-loss or recovery failures block release. Record exact commands, observed
results, coverage limits and remaining blockers. Never claim completion, exhaustive coverage or a
valid seal without the corresponding evidence.


## How to add work

- **Adapter**: `cargo xtask new-source <name>`, then captured fixtures under
  `crates/census-crawl/tests/fixtures/<name>/` — flat captured bytes, the scaffolded `README.md`
  contract, and an optional provenance note beside them — offline test via
  `cargo xtask source-test <name>`. Answer the §13 report questions in
  `research/sources/<name>/SOURCE_REPORT.md`.
- **Workflow**: identity per §8 (`jurisdiction:{state}:{season}:{revision}` and friends). Never mint a
  new logical job because an HTTP call failed. Contract changes go through Main.
- **Benchmark**: §57 list only; no optimisation without benchmark evidence.
- **Fixture**: offline, deterministic, no network, no Restate required.

## Handing work back

**Agent-spawn policy (hard rule).** Never spawn the `luna-*` agents, and never spawn the default
`task` agent without naming a model — the harness default resolves to `gpt-5.6-luna`, which is
forbidden here. Allowed workers: `deepseek-flash` (read-mostly probes and evidence), `gpu5090-coder`
and `gpu3090-coder` (local Qwen coding lanes, confined to assigned files), `scout` (read-only
research), `reviewer` / `security-reviewer` (read-only review), `sonic` (mechanical edits). Always
name the agent explicitly.

Use the §15 packet: TASK / OWNERSHIP / DO NOT MODIFY / INPUT CONTRACT / OUTPUT CONTRACT / ACCEPTANCE /
HANDOFF. Report changes, the exact commands you ran with their observed results, and remaining
uncertainty. Do not commit — Main commits.
