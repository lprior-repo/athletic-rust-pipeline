# AGENTS.md — working instructions for this repository

Primary instruction document for any person or coding agent working here. Read this first, then
`ARCHITECTURE.md`, then `docs/adr/README.md`.

## What this repository is

A nationwide Class-of-2027 high-school Track & Field / Cross Country recruiting census. A qualified
source graph feeds durable acquisition, acquisition produces evidence, evidence lands in Fjall, a
deterministic merge proposes canonical identities, the local Qwen lane adjudicates only what Rust
cannot, and the result is projected into an Excel workbook a recruiter can filter. The workbook is a
projection; the durable evidence system is the census.

## Read first

1. `ARCHITECTURE.md` — pipeline, crate boundaries, invariants, and the binding standards (§-numbered
   so that references such as §9, §38, §70 resolve there).
2. `docs/adr/README.md` — decisions that may not be silently re-litigated.
3. `docs/migration/module-map.md` — the module inventory and the crate cut being executed.
4. Per-crate `README.md` files as the crates are extracted.

## Commands

| Command | Purpose |
| --- | --- |
| `cargo xtask gate` | the four build gates (§55) plus the line-budget scan, in order, stopping at the first failure |
| `cargo xtask census-status [--ingress [<ORIGIN>] \| --store <DIR>]` | current store state, counted by the serving census (`Census/status`) or read from the store directly |
| `cargo xtask coverage [--ingress [<ORIGIN>] \| --store <DIR>]` | coverage summary (§49 fields) |
| `cargo xtask export [--ingress [<ORIGIN>] \| --store <DIR>]` | workbook export (`Workbook/run`) |
| `cargo xtask source-check <name>` | adapter layout and fixture presence |
| `cargo xtask source-fixture <name>` | fixture set integrity |
| `cargo xtask source-test <name>` | offline fixture test for one source |
| `cargo xtask replay <name>` | deterministic offline parser replay |
| `cargo xtask new-source <name>` | scaffold a new adapter |
| `cargo xtask bench parser` | parser benchmarks |

`census-status`, `coverage` and `export` default to the serving census: the flag-free form and
`--ingress [<ORIGIN>]` (default `http://127.0.0.1:18095/`) submit the matching Restate handler and are
safe to run beside `midwest-serve`; origins must be loopback HTTP with no path or credentials.
`--store <DIR>` opens the store in-process instead, so it requires `midwest-serve` to be stopped — the
store is single-writer, and the lock error it returns while the census serves is the correct answer,
not a bug.

Binary: `./target/release/midwest-census` today, renaming to `census-service` when the CLI crate is
extracted — address it through the shared constant, never by inlining the name. Store root:
`var/midwest-census`. **The store is single-writer**; lanes that write must serialize.

## Crate ownership

| Crate | Owns | May not |
| --- | --- | --- |
| `census-domain` | pure types, cohort/grade evidence, event canonicalisation, PR ordering rules | depend on tokio, fjall, reqwest, chromiumoxide, restate, xlsx, llama clients |
| `census-store` | Fjall keyspaces, journals, snapshots, migrations, backup/restore | know about HTTP or parsers |
| `census-crawl` | source adapters, fetchers, admission, browser supervisor | write canonical entities |
| `census-reconcile` | identity normalisation, deterministic scoring, conflict detection | call models |
| `census-review` | the local Qwen identity-review lane | decide identity (it advises; Rust adjudicates) |
| `census-report` | coverage, bests, PR projection, workbook/export | mutate evidence |
| `census-service` | CLI, Restate workflows, bootstrap and task supervision | bypass the store's durability rules |
| `g1-audit` | the audit lane | be imported by production paths |
| `xtask` | the agent-facing verbs above | hold business logic |

Main (the architect) owns: workspace layout, domain public contracts, serialized public types, Fjall
schema revisions, Restate workflow contracts, migrations, and final integration. Adapter owners own
their adapter directory, its fixtures, and its `research/sources/<source>/` report. Propose shared
contract changes; Main applies them.

## Coding standards (binding)

- **§55** all four gates pass; **§56** `#![forbid(unsafe_code)]` and `#![deny(unused_must_use)]`
  workspace-wide, quality ratchet never increases, no suppression-based victory.
- **§38** files ≤ 300 lines, production functions ≤ 60 logical lines (hot paths ≤ 25); decompose
  long orchestrations into named stages.
- **§37** no unwrap/expect/panic reachable from input, no ignored `Result`, no silent fallback, no
  unchecked boundary conversion, no unbounded loop, queue, or spawn.
- **§39** `thiserror` inside production crates; `anyhow` only at CLI/application composition edges.
- **§44** structured `tracing` only — no `println!`/`eprintln!` in production paths.
- **§9** exactly one retry owner: Restate retries (3 attempts max), transport performs one attempt.
  Retry exhaustion becomes evidence; a source failure is never `NO_MATCH`.
- **§10** admission is per remote origin, measured on physical requests — never multiplied by
  spawning more workflows.
- **§43** do not flatten async outcomes; **§42** supervised shutdown with drain accounting.
- **§61-§62** cache only immutable successes; quarantine poisoned objects and keep going.
- **Rust only**: no Python, and no shell scripts as pipeline steps.

## Source policy

Robots and per-origin admission are honored. No CAPTCHA, authentication, or paywall circumvention.
Athletic.net is acquired through the headed persistent-profile browser lane (§26-§28) with a
`HumanRequired` handoff when a challenge appears. Never collect athlete personal contact data; never
infer GPA (§36). A §69 stop condition for one source is persisted and the census continues elsewhere.

## How to add work

- **Adapter**: `cargo xtask new-source <name>`, then fixtures per §64
  (`fixtures/<name>/{README.md,index.json,raw/,expected/}`), offline test via
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
