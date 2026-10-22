# Working instructions

Read `ARCHITECTURE.md` first: binding product, ownership, Rust and release rules.
Final goal: [spec v2](docs/specifications/athletic-recruiting-end-vision-spec-v2.md)
under [ADR-038](docs/adr/ADR-038-selected-profile-reusable-production-state.md).
One current runtime; no legacy execution or shims. Acceptance requires evidence.

## Runtime source access

Do not add runtime robots.txt fetches, parsing, allow/disallow enforcement, freshness
gates or crawl-delay policy. Historical robots captures, bindings and receipts remain
immutable evidence, not current authority. Preserve authentication, CAPTCHA, paywall
and access-denial stops, configured pacing/rate limits, SSRF/redirect checks and secrets.
No global bypass, denial retry, alternate transport or implicit history rewrite.

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
- Up to 12 useful, nonoverlapping subagents. RTX 5090/3090 are primary coders,
  one coding job per GPU. Muse/DeepSeek Flash support; Sol only as needed.
- One Fjall owner; inspect live state via ingress/admin. Preserve historical data,
  IDs, receipts and artifacts. No SIGKILL, implicit migration, annual fresh reset or
  replacement keys to evade exhaustion. Follow [deployment](docs/deployment-lifecycle.md).

[ADRs](docs/adr/README.md): decisions. [Plan](docs/NATIONAL-CENSUS-PLAN.md): work.
[Capabilities](docs/PROFILE-EXECUTION-V1.md): support. [Operations](docs/OPERATIONS.md):
runtime. [Evidence](docs/VERIFICATION-EVIDENCE.md): observed results, not promises.
