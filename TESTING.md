# Testing and release evidence

This is the current test-authoring and verification guide, not a pass certificate. Dated commands,
results and limits belong in [VERIFICATION-EVIDENCE.md](docs/VERIFICATION-EVIDENCE.md).
[ARCHITECTURE.md](ARCHITECTURE.md) owns engineering policy; the
[national plan](docs/NATIONAL-CENSUS-PLAN.md) owns acceptance and its 24 named regression canaries.

## Commands and gate modes

Run from the workspace root with the pinned toolchain. The workspace has nine member crates;
there is no root application package or root-package test suite.

| Purpose | Command |
|---|---|
| Developer gate | `cargo xtask gate` |
| Slow performance and mutation lanes too | `cargo xtask gate -- --full` |
| Release gate; missing required tools fail | `cargo xtask gate -- --release` |
| Workspace contract checks | `cargo xtask contract` |
| All-feature behavior suite | `cargo nextest run --workspace --all-features` |
| Cargo fallback, including doc tests | `cargo test --workspace --all-features` |
| Crawl adapter tests directly | `cargo test -p census-crawl --lib` |
| Store tests directly | `cargo test -p census-store --lib` |
| CLI report-checker tests | `cargo test -p census-service --bin census-service cli::qa_reports` |
| Process recovery integration target | `cargo test -p census-service --test recovery` |
| Store schedule models | `cargo test -p census-store --features loom --lib` |
| Service schedule models | `cargo test -p census-service --features loom --lib` |

`tools/gate.sh` is the executable lane list: formatting, compilation, rustdoc, tests, strict
production Clippy, lexical comments, source/size scans, domain purity, module seams, integrity,
debt ratchet and installed dependency/assurance tools. Domain purity checks that
`cargo tree -p census-domain --edges normal` carries no banned serialization, I/O or async crate;
canonical digests and the identity and contact-proof rules that hash them therefore live in
`census-store` ([ADR-017](docs/adr/ADR-017-canonical-encoding-ownership.md)). Inspect its actual
summary; an optional-tool `SKIP` in developer mode does not establish coverage. `--release` enables
the heavy lanes and rejects missing tools. Nextest does not execute doc tests; ignored tests require
explicit invocation. The four build gates alone do not prove the separate proof, source, fault and
artifact obligations.

`--update-baseline` measures and rewrites the debt baseline, then returns before later lanes;
it is not a release pass. `--allow-increase` is an explicit debt waiver, not a repair. Use
[PERFORMANCE.md](PERFORMANCE.md) for measurement and baseline semantics rather than copying counts
into this guide. A stale deleted-target fingerprint can make `cargo geiger` fail before producing
an unsafe-code report; diagnose that artifact or use an isolated target directory, not a blanket
claim that the code passed.

`cargo xtask source-test <name>` currently routes through census-service. It does not prove that
all extracted census-crawl adapter tests ran. Use the owning crate's tests and
[SOURCE_ADAPTER_GUIDE.md](SOURCE_ADAPTER_GUIDE.md) for the source qualification contract.

## Test and fixture ownership

| Owner | Behavior under test |
|---|---|
| census-domain | Validated values, source ownership, cohort interpretation, mark units, identity transitions |
| census-store | Atomic batches/receipts, replay conflicts, observation history, snapshots, migrations, backup/restore |
| census-crawl | Captured format parsing, source IDs, provenance, admission, pagination, explicit empty/failure outcomes |
| census-reconcile | Candidate generation, contradictions, accepted application, split/replay behavior |
| census-review | Subject-bound packets and advice, transport limits, malformed/contradictory replies, cache identity |
| census-report | Compatible PRs, scope/cohort filtering, historical affiliation, workbook cells and artifact reconciliation |
| census-service | CLI, durable handler boundaries, retry exhaustion, process supervision, whole-path recovery |
| athleticnet-browser | Request/response attribution, challenge handoff, pool admission and shutdown |
| xtask | Scanner/gate behavior, command dispatch, benchmark parsing and completeness checks |

Unit tests live with their owning module, moving to a child test file when needed for the size
budget. Cross-module/process tests live under the owning crate's `tests/`. Existing service parity
fixtures/goldens remain service-owned; parser captures live under
`crates/census-crawl/tests/fixtures/`. Standalone fuzz targets and retained public seeds live under
`fuzz/`; a compiled target is not a fuzz execution result.

- Keep captures verbatim, public and source-linked. Record capture date, URL, media type and what the
  fixture proves beside it. Synthetic malformed cases must be labelled synthetic. Never commit
  private workbook rows, credentials or athlete personal contact data.
- Default tests are offline. Use isolated temporary stores, generated workbook fixtures and fixed
  dates/seeds. Do not mutate live `var/` state or depend on current exported workbooks.
- Loopback fixture servers and isolated local Restate/browser instances are explicit integration
  dependencies, not permission for outbound source requests. Record prerequisites and teardown.
- Test observable boundaries, rejection paths and state transitions. Assert typed errors where
  available; assert wording only when it is the operator-facing contract. Do not add tests of
  implementation text, copied defaults or mock forwarding.
- Preserve failing-before/passing-after regressions for discovered defects. Updating a golden
  requires explaining the semantic difference; `GOLDEN_UPDATE=1` is not an acceptance oracle.
- New adapters need deterministic parser cases and an acquisition-path exercise proving the expected
  records, coverage/failure disposition and request accounting, not only a non-empty parse result.

## Mandatory proof kernels

Proofs execute the same production kernel used by the application, not a duplicated algorithm.
Factor a small pure kernel when needed. A stub is an explicit trust boundary requiring justification
and a concrete integration test. Do not change behavior under `cfg(kani)`, weaken assertions or
silently narrow input domains to obtain a verdict. Assumptions need reachable rejection and boundary
cases. Unsupported operations, panic, timeout, OOM and no verdict are non-passes.

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

A proof of SHA-256 internals is not needed for stable ID formatting or domain separation. Golden
serialization, collision detection and keyed-identity tests serve those contracts; Kani cannot prove
cryptographic collision resistance for truncated IDs. Keep the claim on the small state machine
rather than forcing an unrelated hash/Unicode library through a bounded solver.

Define a fast mandatory tier and a broader bounded tier. Set and commit per-harness resource limits
after measuring a successful baseline; bound solver concurrency alongside the crawler and GPUs.
The proof manifest distinguishes `verified`, `counterexample`, `timeout`, `oom`, `unsupported`,
`not_selected`, `not_run` and `tool_failure`, with raw logs, versions, harness selection, assumptions
and bounds. Exit zero with zero or wrong harnesses is not success. A pinned compiler/serde/Kani
failure needs its concrete reproducer and supported integration repair, not a generic environment
waiver.

## End-to-end release boundary

Exercise the real CLI, Restate handlers, export and independent readback against isolated state.
Unit/property tests, bounded fuzzing, mutation tests, schedule models, load measurements, security
review and async review complement each other; none replaces a reached crash or a full artifact
comparison. See the [17 fault scenarios](docs/NATIONAL-CENSUS-FAULTS.md) for required boundaries and
[durability runbook](tools/durability/README.md) for implemented drivers and prerequisites.

For every claimed scenario, retain the exact command, input generation, reached fault signal,
exit status, raw result, recovered state and remaining work. A skipped case, random kill that missed
the boundary, sampled verifier or old seal cannot qualify the fresh census. All 49 jurisdictions
must have honest coverage dispositions, and publication must reconcile with its frozen generation
under the national plan's readback checks. No seed workbook is an input to that fresh run.
