# AGENTS.md — repository working instructions

Read this file, then [ARCHITECTURE.md](ARCHITECTURE.md) before changing the repository. The forward
product is the full [October 11 spec v2](docs/specifications/athletic-recruiting-end-vision-spec-v2.md)
reusable public-source recruiting database under [ADR-038](docs/adr/ADR-038-selected-profile-reusable-production-state.md).
The legacy fresh Class-of-2027 49-jurisdiction census remains an explicit qualification/national
contract. Excel is output, not population intake. Architecture adoption is not runtime acceptance.

## Startup — every session and after context reset

Invoke these skills before further investigation or editing. If unavailable, report the missing
skill rather than silently proceeding:

1. `rust-contract` — domain/type contracts before proof, tests or implementation.
2. `holzman-rust` — bounded resources, checked arithmetic, complete errors and measured performance.
3. `async-rust-reviewer` — spawn ownership, cancellation, replay and drain accounting.
4. `scott-ddd-refactor` — constrained types, exhaustive states and explicit transitions.

## Developer commands — Moon only

Moon is the **only** repository build, test, check, lint, formatting, benchmark and gate entrypoint.
Run from the repository root through `tools/moon-local`; never invoke bare `moon`, Cargo,
`cargo xtask`, `tools/gate.sh` or fault/backup wrappers directly. Cargo and those wrappers are
internal Moon task implementations. Translate skill/example commands into the tasks below.
The installed Moon version is 2.2.4; the launcher selects/fingerprints the pinned Rust toolchain
before cache lookup. [xtask/README.md](xtask/README.md) owns the complete command/task reference.

```sh
tools/moon-local run pipeline:tests
tools/moon-local run pipeline:tests -- -E 'binary(restate_kill_restart)'
tools/moon-local run pipeline:check
tools/moon-local run pipeline:lint-src
tools/moon-local run pipeline:fmt
tools/moon-local ci --force --summary detailed
env -u CI tools/moon-local run pipeline:build-portable
env -u CI tools/moon-local run pipeline:xtask -- --help
env -u CI tools/moon-local run pipeline:gate -- --release
```

Manual tasks (`build*`, `xtask`, `gate`, `bench`, `fetch`, `fmt-write`, `g1-audit`, `durability`
and `backup-drill`) are excluded from automatic CI; unset `CI` even when its value is `false`.
Use `pipeline:fetch` to populate locked dependencies, `pipeline:fmt-write` only when intentionally
formatting owned source, and `pipeline:bench -- --bench <target> -- <Criterion-args>` for measurements.
The optimized portable binaries live in `target/moon-portable/x86_64-unknown-linux-gnu/release/`.
Runtime census/Restate/admin commands still execute the actual binaries; they are not Cargo builds.
The passing Moon iteration CI is narrower than `pipeline:gate -- --release`; never substitute it
for release, proof, security or all seventeen fault obligations.


## Local Restate deployment lifecycle

The teardown/start procedures below describe isolated fresh census/qualification deployment.
Reusable production must retain its enrolled installation, durable source-work identities and
qualified evidence across sessions; never follow the fresh-store procedure as an annual reset.
Current single-run binding and unsupported-profile refusals remain until Main explicitly versions,
integrates and qualifies the production lifecycle. Profile registration/selection does not enroll
or migrate a store. Preserve historical IDs, manifests, artifacts, receipts and seals.

No container runtime is used here. The node is the native binary at `/home/lewis/bin/restate-server`
with one config per run (`var/<run>/restate.toml`, conventional loopback ingress 18095 and admin
19095). `census-serve` owns the Fjall store: one process owns one root, and a second opener failing on
the lock is correct, not a defect to work around.

Tearing down a previous run, in this order:

1. `kill -TERM <census-serve pid>` and wait for exit. Read the drain certificate it appends to
   `var/<run>/serve.log` (`drained: accepted=… completed=… cancelled=… timed_out=… aborted=…
   panicked=…`).
2. `kill -TERM <restate-server pid>` for each node and wait; the node config's `shutdown-timeout`
   bounds that wait.
3. Confirm nothing listens on the run's ports and that no `restate-server` or `census-serve`
   process remains.
4. Preserve `var/<run>/`, the node's `base-dir` and every artifact under `var/<run>/out/`. Never
   SIGKILL, never delete another run's durable state, and never reuse its store directory: a fresh
   census needs a new store path, a new endpoint port and a fresh registration.

Starting a fresh run:

1. Write `var/<run>/restate.toml` from the previous run's config with new node/cluster names and a
   new `base-dir`, start `restate-server --no-logo -c` it, and wait for the `Admin:` and
   `HTTP Ingress:` banner lines.
2. After `env -u CI tools/moon-local run pipeline:build-portable`, start
   `target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve --listen 127.0.0.1:<port> --data-dir var/<run>
   --max-concurrent <n> --drain-timeout <seconds> --browser-profile var/<run>/browser-profile
   --browser-executable /usr/bin/chromium --browser-headless`.
3. Register it: `curl -X POST http://127.0.0.1:19095/deployments -H 'content-type: application/json'
   -d '{"uri":"http://127.0.0.1:<port>/"}'`, then confirm all twelve services appear in
   `GET /deployments`.
4. Submit the run with `census-service national --ingress http://127.0.0.1:18095/ --detach` and
   observe it through the admin query API and `census-service open-work`, never by opening the store
   from a second process.

## Ownership

[Architecture §4](ARCHITECTURE.md#4-crate-layout-17) defines crate responsibilities. Main owns
workspace layout, public domain/serialized contracts, Fjall schema revisions, Restate workflow
contracts, migrations and final integration. Propose changes to those interfaces; Main applies them.
Adapter owners own their adapter, fixtures and source report. One owner retains a slice through
implementation, caller migration and verification.

Do not overwrite unrelated local work. Coordinate all writers; synchronize incoming changes while
preserving user edits. The database permits one owning process; use the documented ingress path
beside a serving census rather than opening its store from another process.

## Implementation discipline

The binding engineering rules are [Architecture §10](ARCHITECTURE.md#10-engineering-standards-37-43),
including zero project-code comments, constrained domain types, no project-owned unsafe or
input-triggered panics, complete outcomes, size budgets and Rust-only pipeline logic.

Reuse the existing implementation and conventions. Freeze shared interfaces before parallel work.
Migrate all callers and remove superseded execution paths; do not add compatibility shims, mirrored
types, duplicate business rules or source-specific identity exceptions. Persisted historical shape
handling must follow its explicit migration contract, not an improvised fallback.

## Delegation

Keep 2–3 useful workers during implementation when independent work exists; never create filler.
The owner-directed allocation is one `gpu5090-coder` and two `deepseek-flash` workers.
Keep the coding job coherent and non-overlapping; at most one coding job per GPU. Main owns shared
contracts, identity acceptance and durability. Flash handles specific investigation and fixture
repair, not production coding or repeated architecture audits.

**Hard agent policy:** never spawn `luna-*` or the unnamed/default `task` worker (its default model
is forbidden). Always name an allowed agent explicitly: `gpu5090-coder`, `gpu3090-coder`,
`gpu5090-reviewer`, `gpu3090-reviewer`, `deepseek-flash`, `scout`, `reviewer`, `security-reviewer`,
`sonic` or `sol-reviewer`. The RTX 3090 coder and reviewer are active again by owner direction
2026-10-12; their ancestor `.omp/agents/` definitions were re-enabled. Keep at most one coding job
per GPU. Workers may not edit outside ownership, change interfaces silently or reactivate finished
peers.

The owner has authorized `sol-reviewer` for SOL-only delivery. Its repository definition is
`.agents/sol-reviewer.md`; OMP discovers the existing definition in the ancestor `.omp/agents/`.
Use its explicit `openai-codex/gpt-6.1-sol:high` selection for requested SOL-only work; do not launch
the older Luna, Qwen or DeepSeek selections as substitutes. Caller-assigned file ownership and the
no-build/no-test rule during concurrent edits still apply.

## Delivery loop

1. Read the owning contracts and relevant implementation; state scope, exclusions and acceptance.
2. Assign exclusive files/interfaces, then implement one coherent end-to-end slice.
3. Compile and run focused checks after integration, before dependent work expands. Do not defer
   every check until all F01–F15 work is finished or weaken a failing gate.
4. Exercise the real changed surface: CLI, native Restate, source parser or artifact readback as
   applicable. Give every discovered consumer-visible defect a regression.
5. Run the required release gates before claiming delivery.

A qualification slice is not the national census. Staffing changes do not reduce scope. Measure
verified completions and escaped defects, not agent activity or files changed. Accuracy, data-loss
and recovery failures block release; simulated faults and passing builds do not certify unexercised
behavior.

## Adding work

- **Adapter:** use `env -u CI tools/moon-local run pipeline:xtask -- new-source <name>` with
  captured fixtures under `crates/census-crawl/tests/fixtures/<name>/`.
- **Workflow:** use the existing logical identities and retry ownership; never mint a new job
  because an attempt failed.
- **Benchmark:** follow `xtask/README.md`'s measurement and perf commands; no optimization claim
  without a measurement.
- **Fixture:** deterministic and isolated; no external network or real Restate unless explicitly
  exercising the separately documented native integration lane. Never commit private workbook data.

## Handoff (§15)

Use: **TASK / OWNERSHIP / DO NOT MODIFY / INPUT CONTRACT / OUTPUT CONTRACT / ACCEPTANCE / HANDOFF**.
Report changed files, exact commands and observed results, and remaining blockers.
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

Use Beads (`bd`) for durable task tracking in repositories that include it. Use the `beads` skill at `.agents/skills/beads/SKILL.md` (project install), or the same skill from your global agent install, for Beads workflow guidance, then use the `bd` CLI for issue operations.

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
