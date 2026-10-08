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
| 14 | `seal-refuses` | Empty-store CLI refusal names the jurisdiction, source-object and run-binding items with their exact details; does not exercise every unmet acceptance item |
| 15 | `full-backup-restore` | Runs service `backup_restore` integration tests; not Restate recovery |
| 16 | `golden-census-determinism` | Runs exact `rebuilding_the_fixture_store_reproduces_semantics_not_publication_identity` regression and retains its log; not full frozen real-capture/advice replay |
| 17 | `recovery-tests` | Runs service `recovery` suite; verify the actual reached batch window against the catalog |
| 18 | `run-binding` | Native node + endpoint: admits one run through `Census.bind_run`, refuses a second with both runs named, refuses a `Census/seal` request naming another cohort or another run's journal, reads the binding back after an endpoint restart, and checks the offline ladder names the unmeasured binding; no crash injected and no full census run |

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
Scenario 09 resolves its probe through Cargo's JSON artifact protocol and refuses any
artifact outside the selected target directory, so a Moon build under `CARGO_TARGET_DIR`
either runs the artifact it just built or fails closed; it records that artifact's hash
and re-checks it before and inside the private mount, and a deterministic routing check
proves a stale default-target probe is never selected or executed.

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
publishes a complete, no-overwrite, identity-bound schema-2 marker before acquisition and
holds for at most 60 seconds; expiry fails the injection rather than continuing
normally. Its owned blocking worker holds the store append fence across a single-snapshot
checkpoint, marker publication and the bounded hold. The checkpoint records exact physical
source table/key/value SHA-256 occurrences and full acknowledged receipts, not merged output
counts or a selected row sample. Marker encoding/readback is bounded to 32 MiB; configuration
remains bounded to 4 KiB. An oversized checkpoint fails rather than omitting evidence.
The host must still prove the original parent is owed, its original child is active and
awaited, its deployment pin and journaled registration match, and its actual durable reservation
exists. Configuration and marker are verified byte-exact after reboot; cold readback must retain
every checkpointed source occurrence and receipt. This reached seam proves reserved
pre-acquisition work, not an HTTP request, response or parse in flight, and does not certify all
scenario-03 phase boundaries or a full national census.

The host exercises reboot before the independent natural-midnight lane, so a guest-only clock
injection cannot roll back on reboot and confound that fault. It retains both original Sweep
and source recovery outcomes before propagating either error. Recovery polls original invocation
status and captures full bookended journals/inspections on completion or the final bounded check,
instead of duplicating them at every poll. The 32 MiB artifact limit and one-second interval are
unchanged. The recovery check budget is 3,200 polls, sized for the post-reboot jurisdiction run:
under TCG emulation the replayed jurisdiction sweep is fsync-bound on the guest data disk and
legitimately takes minutes, while the earlier 300-poll budget expired during a healthy run
(`var/vm-sol-20261006-02/extracted-reconciliation.json`: store recovered 22:43:58, both
`TeamsSource` children completed 22:44:02/22:44:05, the parent routed 120 rows at 22:44:10, the
budget expired ~22:49:15 and the parent completed ~22:49:47). The guest action deadline is
3,400 seconds and the host SSH transport allows 35,000 hundred-millisecond ticks, both below the
3,600-second process-observation cap; keep that ordering when changing any of the three. A
still-unfinished parent remains a failure with its exact obligations, not source acquisition or
national PASS.

The separate natural-midnight lane requires two successful, refreshed full-body public HTTPS
acquisitions through the production `census_crawl::net::Fetcher`, with distinct immutable
manifests and journal readback. A cache hit, 304, denied/challenged/error response or substituted
fixture cannot certify physical acquisition times. Guest clock bookends, actual `fetched_at`
values and stable run/cohort/season/source-unit identities must agree; no host clock changes
are permitted.

Scenario 17 executes feature-enabled actual CLI worker tests and requires three reached-boundary
certificates: `source_batch_staged_before_commit`, `source_chunk_committed_before_next` and
`derived_batch_staged_before_publish`. The store publishes schema-1 markers containing the real
operation/digest and receipt ordinal or staged generation. The tests wait for atomic marker
publication, deliver and reap SIGKILL, then cold-open the owned store. Exact recovered native
identifiers, full operation receipt sets, remaining work and unchanged physical replay are
required. The derived subcase also proves the previous generation remains visible after the
kill and a complete generation is published on restart. A guessed sleep, unreached worker,
missing certificate or skipped test fails the wrapper.


Guest artifacts survive only inside the run's `root.qcow2` (btrfs, not readable by `debugfs`).
To inspect them after a run, boot the preserved overlay read-only and read them over SSH:

```sh
qemu-system-x86_64 -machine pc -accel tcg,thread=multi -cpu max -smp 2 -m 4096 \
  -display none -monitor none -L <tools>/prefix/usr/share/qemu -bios <tools>/prefix/usr/share/qemu/bios-256k.bin \
  -serial file:<root>/evidence-boot-serial.log \
  -drive "file=<root>/root.qcow2,if=virtio,format=qcow2,snapshot=on,cache=directsync,aio=threads" \
  -drive "file=<root>/data.qcow2,if=virtio,format=qcow2,snapshot=on,cache=directsync,aio=threads" \
  -netdev user,id=bootstrap,hostfwd=tcp:127.0.0.1:2222-:22 -device virtio-net-pci,netdev=bootstrap
ssh -i <root>/ssh-key -o UserKnownHostsFile=<root>/known_hosts -p 2222 root@127.0.0.1 \
  'cat /srv/qualification/jurisdiction-recovery-recovery-reconciliation.json'
```

`snapshot=on` discards every guest write, so the preserved evidence is never mutated. The store
lives on the data disk (`/dev/vdb` mounted at `/srv/qualification`); boot both drives.

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
