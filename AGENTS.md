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

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:970c3bf2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Agent Context Profiles

The managed Beads block is task-tracking guidance, not permission to override repository, user, or orchestrator instructions.

- **Conservative (default)**: Use `bd` for task tracking. Do not run git commits, git pushes, or Dolt remote sync unless explicitly asked. At handoff, report changed files, validation, and suggested next commands.
- **Minimal**: Keep tool instruction files as pointers to `bd prime`; use the same conservative git policy unless active instructions say otherwise.
- **Team-maintainer**: Only when the repository explicitly opts in, agents may close beads, run quality gates, commit, and push as part of session close. A current "do not commit" or "do not push" instruction still wins.

## Session Completion

This protocol applies when ending a Beads implementation workflow. It is subordinate to explicit user, repository, and orchestrator instructions.

1. **File issues for remaining work** - Create beads for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Handle git/sync by active profile**:
   ```bash
   # Conservative/minimal/default: report status and proposed commands; wait for approval.
   git status

   # Team-maintainer opt-in only, unless current instructions forbid it:
   git pull --rebase
   bd dolt push
   git push
   git status
   ```
5. **Hand off** - Summarize changes, validation, issue status, and any blocked sync/commit/push step

**Critical rules:**
- Explicit user or orchestrator instructions override this Beads block.
- Do not commit or push without clear authority from the active profile or the current user request.
- If a required sync or push is blocked, stop and report the exact command and error.
<!-- END BEADS INTEGRATION -->

<!-- BEGIN BEADS CODEX SETUP: generated by bd setup codex -->
## Beads Issue Tracker

Use Beads (`bd`) for durable task tracking in repositories that include it. Use the `beads` skill at `.agents/skills/beads/SKILL.md` (project install) or `~/.agents/skills/beads/SKILL.md` (global install) for Beads workflow guidance, then use the `bd` CLI for issue operations.

### Quick Reference

```bash
bd ready                # Find available work
bd show <id>            # View issue details
bd update <id> --claim  # Claim work
bd close <id>           # Complete work
bd prime                # Refresh Beads context
```

### Rules

- Use `bd` for all task tracking; do not create markdown TODO lists.
- Run `bd prime` when Beads context is missing or stale. Codex 0.129.0+ can load Beads context automatically through native hooks; use `/hooks` to inspect or toggle them.
- Keep persistent project memory in Beads via `bd remember`; do not create ad hoc memory files.

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.
<!-- END BEADS CODEX SETUP -->
