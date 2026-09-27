# AGENTS.md — repository working instructions

Read this file, [ARCHITECTURE.md](ARCHITECTURE.md), then the
[decision register](docs/adr/README.md) before changing the repository. The product is a fresh,
public-source-discovered Class-of-2027 TF/XC census; Excel is output, not population intake.

## Startup — every session and after context reset

Invoke these skills before further investigation or editing. If unavailable, report the missing
skill rather than silently proceeding:

1. `rust-contract` — domain/type contracts before proof, tests or implementation.
2. `holzman-rust` — bounded resources, checked arithmetic, complete errors and measured performance.
3. `async-rust-reviewer` — spawn ownership, cancellation, replay and drain accounting.
4. `scott-ddd-refactor` — constrained types, exhaustive states and explicit transitions.

## One authoritative home per subject

| Subject | Read |
|---|---|
| Product scope, crate boundaries, safety/source policy and release requirements | `ARCHITECTURE.md` |
| Decision rationale and supersession | `docs/adr/README.md` and its individual ADRs |
| Active delivery order, F01–F15 and named regression acceptance | `docs/NATIONAL-CENSUS-PLAN.md` |
| The 17 required native fault scenarios | `docs/NATIONAL-CENSUS-FAULTS.md` |
| Domain representations and semantic gaps | `DOMAIN.md` |
| Adapter integration and source qualification | `SOURCE_ADAPTER_GUIDE.md`, `research/README.md` |
| Physical storage and snapshots | `FJALL_SCHEMA.md` |
| Implemented durable handlers and replay boundaries | `RESTATE_WORKFLOWS.md` |
| Operating the census | `docs/OPERATIONS.md` |
| Backup/restore and deployment | `docs/FJALL_BACKUP.md`, `docs/deployment-lifecycle.md` |
| Test/gate and benchmark procedures | `TESTING.md`, `PERFORMANCE.md` |
| Dated command evidence and limitations | `docs/VERIFICATION-EVIDENCE.md` |
| Developer verbs and native entry points | `xtask/README.md`, `crates/census-service/README.md` |

Keep implementation facts, target requirements and historical results distinct. Edit the owning
document instead of creating a second architecture, handoff, status or execution plan. ADRs explain
why; references explain current APIs; evidence records what actually ran. Preserve unique source
captures and audit evidence. Scraped Restate references are external material, not project policy.

## Ownership

[Architecture §4](ARCHITECTURE.md#4-crate-layout-17) defines crate responsibilities. Main owns
workspace layout, public domain/serialized contracts, Fjall schema revisions, Restate workflow
contracts, migrations and final integration. Propose changes to those interfaces; Main applies them.
Adapter owners own their adapter, fixtures and source report. One owner retains a slice through
implementation, caller migration and verification.

Do not overwrite unrelated local work. Coordinate all writers; synchronize incoming changes while
preserving user edits. The database permits one owning process; use the documented ingress path
beside a serving census rather than opening its store from another process. Historical stores,
exports and seals are preserved and are not fresh-run acceptance evidence.

## Implementation discipline

The binding engineering rules are [Architecture §10](ARCHITECTURE.md#10-engineering-standards-37-43),
including zero project-code comments, constrained domain types, no project-owned unsafe or
input-triggered panics, complete outcomes, size budgets and Rust-only pipeline logic. Rationale goes
in documentation, never source comments or documentation attributes.

Reuse the existing implementation and conventions. Freeze shared interfaces before parallel work.
Migrate all callers and remove superseded execution paths; do not add compatibility shims, mirrored
types, duplicate business rules or source-specific identity exceptions. Persisted historical shape
handling must follow its explicit ADR/migration contract, not an improvised fallback.

## Delegation

Keep 2–4 useful workers during implementation when independent work exists; never create filler.
The requested four-worker allocation is one `gpu5090-coder`, one `gpu3090-coder` and two
`deepseek-flash` evidence workers. Keep GPU coding jobs coherent and non-overlapping; at most one
coding job per GPU. Main owns shared contracts, identity acceptance and durability. Flash handles
specific evidence collection, not repeated architecture audits.

**Hard agent policy:** never spawn `luna-*` or the unnamed/default `task` worker (its default model
is forbidden). Always name an allowed agent explicitly: `gpu5090-coder`, `gpu3090-coder`,
`deepseek-flash`, `scout`, `reviewer`, `security-reviewer` or `sonic`. Workers may not edit outside
ownership, change interfaces silently or reactivate finished peers.

## Delivery loop

1. Read the owning contracts and relevant implementation; state scope, exclusions and acceptance.
2. Assign exclusive files/interfaces, then implement one coherent end-to-end slice.
3. Compile and run focused checks after integration, before dependent work expands. Do not defer
   every check until all F01–F15 work is finished or weaken a failing gate.
4. Exercise the real changed surface: CLI, native Restate, source parser or artifact readback as
   applicable. Give every discovered consumer-visible defect a regression.
5. Update the owning docs and dated evidence. Record exact commands/results, tested scope and
   remaining blockers. Run the required release gates before claiming delivery.

A qualification slice is not the national census. Staffing changes do not reduce scope. Measure
verified completions and escaped defects, not agent activity or files changed. Accuracy, data-loss
and recovery failures block release; old evidence, simulated faults and passing builds do not
certify unexercised behavior.

## Adding work

- **Adapter:** use `cargo xtask new-source <name>`, captured fixtures under
  `crates/census-crawl/tests/fixtures/<name>/`, and
  `research/sources/<name>/SOURCE_REPORT.md`; follow `SOURCE_ADAPTER_GUIDE.md` for qualification.
- **Workflow:** use the logical identities and retry ownership in `RESTATE_WORKFLOWS.md`;
  never mint a new job because an attempt failed.
- **Benchmark:** follow `PERFORMANCE.md`; no optimization claim without representative evidence.
- **Fixture:** deterministic and isolated; no external network or real Restate unless explicitly
  exercising the separately documented native integration lane. Never commit private workbook data.

## Handoff (§15)

Use: **TASK / OWNERSHIP / DO NOT MODIFY / INPUT CONTRACT / OUTPUT CONTRACT / ACCEPTANCE / HANDOFF**.
Report changed files, exact commands and observed results, evidence limits and remaining blockers.
Workers do not commit; Main owns verified integration and landing.
