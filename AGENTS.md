# Working instructions

Read `ARCHITECTURE.md` first: binding product, ownership, Rust and release rules.
Final goal: [spec v2](docs/specifications/athletic-recruiting-end-vision-spec-v2.md)
under [ADR-038](docs/adr/ADR-038-selected-profile-reusable-production-state.md).
Architecture, configuration and enrollment do not certify runtime acceptance.

## Startup and commands

Every session/reset: read `rust-contract`, `holzman-rust`, `async-rust-reviewer`,
`scott-ddd-refactor` and `beads`; report missing skills. Run `bd prime`; use `bd`
for tracking and `bd remember` for memory, not markdown task/memory files.
Moon only: repository-root `tools/moon-local`; never bare Moon, Cargo or internal
wrappers. Unset `CI` for manual tasks. [Command reference](xtask/README.md).

```sh
tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
tools/moon-local run pipeline:tests
env -u CI tools/moon-local run pipeline:gate -- --release
```

## Execution boundaries

- Main owns shared/wire/schema/identity/workflow/migration contracts and integration.
  Assign exclusive files; preserve user edits; reuse rules, never duplicate workflows.
- One Fjall owner; inspect live state via ingress/admin. Preserve historical data,
  IDs, receipts and artifacts. No SIGKILL, implicit migration, annual fresh reset or
  replacement keys to evade exhaustion. Follow [deployment](docs/deployment-lifecycle.md).
- Useful workers only; one coder per GPU. Default: `gpu5090-coder` + two
  `deepseek-flash`. Allowed: both GPU coders/reviewers, `deepseek-flash`, `scout`,
  `reviewer`, `security-reviewer`, `sonic`, `sol-reviewer`; never Luna/default workers.
  SOL-only: `sol-reviewer` with `openai-codex/gpt-6.1-sol:high`. Workers stay within
  ownership; no builds/tests during concurrent edits; no worker commits.
- Exercise real behavior and applicable gates; focused/CI passes are not release,
  all-17-fault or national acceptance. No fabricated evidence or weakened gates.
- Handoff: TASK / OWNERSHIP / DO NOT MODIFY / INPUT CONTRACT / OUTPUT CONTRACT /
  ACCEPTANCE / HANDOFF; exact files, commands, results, blockers. No commit/push/
  Dolt remote sync without explicit current authority.

[ADRs](docs/adr/README.md): decisions. [Plan](docs/NATIONAL-CENSUS-PLAN.md): work.
[Capabilities](docs/PROFILE-EXECUTION-V1.md): support. [Operations](docs/OPERATIONS.md):
runtime. [Evidence](docs/VERIFICATION-EVIDENCE.md): observed results, not promises.
