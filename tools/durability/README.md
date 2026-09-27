# Native durability harness

[The fault catalog](../../docs/NATIONAL-CENSUS-FAULTS.md) owns all seventeen required scenarios,
phase-boundary subcases and exact recovery oracles. This file owns harness invocation and maps the
current wrappers to implementation; it is not a duplicate acceptance contract or execution ledger.

## Prerequisites and isolation

Build the intended `census-service` and `census-serve` binaries and record their identities. Native
Restate tests use pinned server **1.7.10**, selected by `RESTATE_SERVER_BIN`, then the project install
under `$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/`, with test-specific fallback rules.
Obtain it from the [official release](https://github.com/restatedev/restate/releases/tag/v1.7.10),
verify the published checksum and check `--version`; never commit the binary.

Use owned scratch on local disk and isolated ephemeral services. Never reboot the shared host,
change its clock, fill production filesystems, kill shared GPU servers or open a live store with a
second owner. The limited tmpfs mounts used by ENOSPC injectors are deliberate fault media, not
power-loss/reboot evidence. Tests/scripts may not edit project sources or manifests.

## Invocation and configuration

```sh
mkdir -p var/durability-scratch
SCRATCH_STORE="$PWD/var/durability-scratch" \
BINARY="$PWD/target/release/census-service" \
SERVE_BINARY="$PWD/target/release/census-serve" \
RESTATE_SERVER_BIN=<verified-server-path> tools/durability/run.sh

tools/durability/run.sh scenario-01-endpoint-kill
```

Choose a fresh owned scratch root per evidence run. The runner exports `SCRATCH_STORE`, `TMPDIR`,
`BINARY`, `SERVE_BINARY`, `RESTATE_BINARY`, `RESTATE_SERVER_BIN`, `CORPUS_FIXTURE`, `ADMIN_PORT`,
`SERVICE_PORT`, `REPO_ROOT`, `BINARY_AVAILABLE` and `RESTATE_AVAILABLE`. Default scratch is under
`/tmp`; override it if that is tmpfs or shared. A scenario may additionally require namespace tools,
mount, curl, jq, timeout or the test harness's own prerequisites.

## Current wrapper map

Static implementation inventory, not observed PASS results. Files are `scenario-NN-<name>.sh` in
this directory. A narrower invoked test does not discharge the broader same-number acceptance item.

| # | Name | Current implementation/limit |
|---|---|---|
| 01 | `endpoint-kill` | Runs service `restate_kill_restart` integration tests |
| 02 | `restate-kill-during-fanout` | Explicit skip: native NationalCensus mid-fan-out fault missing |
| 03 | `reboot-with-full-census` | Explicit skip: isolated machine reboot missing |
| 04 | `rolling-upgrade` | Explicit skip: distinct V1→V2 upgrade missing |
| 05 | `http-error-taxonomy` | Explicit skip: real browser HTTP fault-server scenario missing |
| 06 | `no-duplicate-evidence` | Explicit skip: reached production commit/lost-ack injection missing |
| 07 | `domain-dedup` | Explicit skip: concurrent cross-workflow source-unit scenario missing |
| 08 | `global-budget` | Explicit skip: multi-endpoint physical origin-budget scenario missing |
| 09 | `disk-full-fjall` | Builds `census-store` ENOSPC probe in private 64-MiB tmpfs; conditional namespace/tool skips |
| 10 | `disk-full-restate` | Owned processes/private bounded mount; demands an actual OS disk-full log and acknowledged-work recovery |
| 11 | `parent-exit` | Explicit skip: `ATHLETIC_FAULT_HTTP_EXIT` seam missing |
| 12 | `cross-midnight` | Explicit skip: isolated clock fault missing |
| 13 | `ai-review-failures` | Runs `census-review` HTTP transport tests; does not prove advice-checkpoint crash recovery |
| 14 | `seal-refuses` | Empty-store CLI refusal; does not exercise every unmet acceptance item |
| 15 | `full-backup-restore` | Runs service `backup_restore` integration tests; not Restate recovery |
| 16 | `golden-census-determinism` | Runs selected `parity_pipeline` fixture test; not full frozen real-capture/advice replay |
| 17 | `recovery-tests` | Runs service `recovery` suite; verify the actual reached batch window against the catalog |

Scenario 10 also uses `restate-enospc-probe.sh`; its current helper prerequisites include Python.
That existing harness implementation is not permission to implement census pipeline logic in Python.
Do not convert a missing tool or inaccessible fault seam into simulated success.

## Verdicts and evidence

The runner executes every selected script, prints its output and emits a final table. A nonzero
child exit is FAIL. A `SKIPPED:` marker takes precedence over `PASS:`; no recognized success marker
also becomes SKIPPED. Exit 0 means all **selected wrappers** passed their implemented checks, not
that all national acceptance obligations were exercised. Any FAIL/SKIPPED returns 1; invalid scenario
selection returns 2. A single selected wrapper is not a full release run.

Record exact build/run IDs, commands, reached injections, owned process identities, physical request
counts, before/after exact-record oracles, logs, exit statuses and cleanup. Keep measured outcomes in
[VERIFICATION-EVIDENCE.md](../../docs/VERIFICATION-EVIDENCE.md). Release requires every mandatory
scenario and subcase, with no skip or substitute.
