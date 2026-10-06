# Native durability harness

[The fault catalog](../../docs/NATIONAL-CENSUS-FAULTS.md) owns all seventeen required scenarios,
phase-boundary subcases and exact recovery oracles. This file owns harness invocation and maps the
current wrappers to implementation; it is not a duplicate acceptance contract or execution ledger.

## Prerequisites and isolation

Build the intended binaries through `env -u CI tools/moon-local run pipeline:build-portable`
and record their identities. All repository build/test/fault invocations go through Moon.
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
env -u CI tools/moon-local run pipeline:build-portable
mkdir -p var/durability-scratch
SCRATCH_STORE="$PWD/var/durability-scratch" RESTATE_SERVER_BIN=<verified-server-path> \
  env -u CI tools/moon-local run pipeline:durability

SCRATCH_STORE="$PWD/var/durability-scratch" \
  env -u CI tools/moon-local run pipeline:durability -- scenario-01-endpoint-kill
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
| 02 | `restate-kill-during-fanout` | Feature-enabled endpoint holds a reserved teams source (`jurisdiction:VT:2026-27:2/teams/milesplit`), SIGKILLs native Restate mid-fan-out, restarts from the same base-dir and requires every pre-kill invocation id to survive; the boundary directory must be a process-owned 0700 directory (the script chmods it) and the reservation marker appears within seconds of submission; skips without the feature-enabled endpoint, chromium, curl/python3/ss or free ports |
| 03 | `reboot-with-full-census` | Explicit skip: isolated machine reboot missing |
| 04 | `rolling-upgrade` | Explicit skip: distinct V1→V2 upgrade missing |
| 05 | `http-error-taxonomy` | Explicit skip: real browser HTTP fault-server scenario missing |
| 06 | `no-duplicate-evidence` | Explicit skip: reached production commit/lost-ack injection missing |
| 07 | `domain-dedup` | Explicit skip: concurrent cross-workflow source-unit scenario missing |
| 08 | `global-budget` | Explicit skip: multi-endpoint physical origin-budget scenario missing |
| 09 | `disk-full-fjall` | Private 64-MiB tmpfs with fail-closed cap/private-mount inspection; actual commit ENOSPC, exact acknowledged records/receipts, unchanged replay and new recovered writes; conditional namespace/tool skips |
| 10 | `disk-full-restate` | Owned processes/private bounded mount; demands an actual OS disk-full log and acknowledged-work recovery |
| 11 | `parent-exit` | Explicit skip: `ATHLETIC_FAULT_HTTP_EXIT` seam missing |
| 12 | `cross-midnight` | Explicit skip: isolated clock fault missing |
| 13 | `ai-review-failures` | Runs `census-review` HTTP transport tests; does not prove advice-checkpoint crash recovery |
| 14 | `seal-refuses` | Empty-store CLI refusal; does not exercise every unmet acceptance item |
| 15 | `full-backup-restore` | Runs service `backup_restore` integration tests; not Restate recovery |
| 16 | `golden-census-determinism` | Runs exact `rebuilding_the_fixture_store_reproduces_semantics_not_publication_identity` regression and retains its log; not full frozen real-capture/advice replay |
| 17 | `recovery-tests` | Runs service `recovery` suite; verify the actual reached batch window against the catalog |

Scenario 10 also uses `restate-enospc-probe.sh`; its current helper prerequisites include Python.
That existing harness implementation is not permission to implement census pipeline logic in Python.
Do not convert a missing tool or inaccessible fault seam into simulated success.

Scenario 09 creates a fresh store child within its private capped mount. The probe rejects
preexisting stores, inherited host mounts, missing size/isolation evidence and non-ENOSPC
failures. It reserves 8 MiB before the fault and releases only that owned file afterward,
then verifies exact cold readback, unchanged receipted replay and a new atomic write across
another reopen. The wrapper uses a 300-second TERM-only deadline and retains `probe.out`
and a cold `preserved-store` under its printed `EVIDENCE:` directory before namespace exit.
Set `SCRATCH_STORE` to an existing owned local-disk parent to preserve this evidence; no
scratch cleanup deletes it. These tmpfs results do not establish power-loss/reboot recovery.

The separate `qualification_native_vm` example's source-reservation reboot lane
requires an explicitly feature-enabled endpoint:

```sh
env -u CI tools/moon-local run pipeline:build -- --release -p census-service \
  --features native-fault-injection --bin census-serve --example qualification_native_vm
```

Those feature-enabled artifacts are under `target/moon-build/x86_64-unknown-linux-gnu/release/`;
stage them into the guest's documented deployment paths rather than assuming `target/release`.

The feature is disabled by default. The owned guest supervisor sets
`CENSUS_NATIVE_SOURCE_BOUNDARY` only on its endpoint, with private configuration
under `/srv/qualification`. A matching original operation and reserved attempt
publishes a complete, no-overwrite, identity-bound marker before acquisition and
holds for at most 60 seconds; expiry fails the injection rather than continuing
normally. The host must still prove the original parent is owed, its original
child is active and awaited, its deployment pin and journaled registration match,
and its actual durable reservation exists. Configuration and marker are verified
byte-exact after reboot. This reached seam proves reserved pre-acquisition work,
not an HTTP request, response or parse in flight, and does not certify all
scenario-03 phase boundaries or a full national census.

The host exercises reboot before the independent natural-midnight lane, so a guest-only clock
injection cannot roll back on reboot and confound that fault. It retains both original Sweep
and source recovery outcomes before propagating either error. Recovery polls original invocation
status and captures full bookended journals/inspections on completion or the final bounded check,
instead of duplicating them at every poll. The 32 MiB artifact limit, 300 checks and one-second
interval are unchanged. A still-unfinished parent remains a failure with its exact obligations,
not source acquisition or national PASS.

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
