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

## Owned native resources

`tools/durability/native-resources.sh` (Moon:
`env -u CI tools/moon-local run pipeline:native-resources`) verifies the pinned prerequisites every
fault lane consumes and prepares the owned scratch disks they write into. It reads no shared
service, mounts nothing outside a private namespace and never sets the host clock. It fails closed
(exit 1) when a pinned artifact, prerequisite tool or isolation capability is missing, and names
the scenarios that capability blocks.

| Resource | Pin and expected identity |
|---|---|
| Restate server | **1.7.10** at `$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server` (override `RESTATE_SERVER_BIN`); executable, ≤512 MiB, `--version` recorded, resolved parent must be the pinned install directory |
| QEMU prefix | **11.1.1** at `…/qemu/11.1.1/prefix/usr/bin/{qemu-system-x86_64,qemu-img}` plus `…/prefix/usr/share/qemu/bios-256k.bin` (≤512 MiB / ≤32 MiB); `--version` must name 11.1.1. The prefix carries its own runtime libraries (`…/prefix/usr/lib`: `libcapstone.so.5`, `libaio.so.1`), which the host loader cannot find, so `native-resources.sh` and `native-vm-qualification.sh` prepend that directory to `LD_LIBRARY_PATH` before invoking or measuring the binaries |
| Staged packages | `…/qemu/11.1.1/pkgs/SHA256SUMS.txt` must verify under `sha256sum --check --strict`; staged archives absent from that manifest are reported and recorded with their observed hashes |
| Guest image | `…/images/Arch-Linux-x86_64-cloudimg-20261001.604814.qcow2` (≤8 GiB) against `images/SHA256SUMS.txt` and the pinned digest `360f0fa4…e87` |
| Guest captures | `CENSUS_NATIVE_VM_CAPTURES`, default `var/native-vm-captures-sol-20261006.json` (≤32 MiB) with its four manifest-bound capture files |
| Capture verifier | `VERIFY_BINARY`, default `target/moon-build/x86_64-unknown-linux-gnu/release/examples/qualification_native_vm`; mandatory executable, built by the feature-enabled command below; absence refuses qualification |
| Scratch disks | `SCRATCH_STORE`, default `var/durability-scratch`, plus `tmp/`; owned, non-symlink, on local disk; a bounded `FAULT_MEDIA_MIB` (64) sparse probe proves the medium without allocating it |
| Isolation | `unshare --user --map-root-user --mount --propagation private` must work and a 64 MiB tmpfs must mount inside it — the ENOSPC media of scenarios 09 and 10; `/proc/sys/kernel/random/uuid` mints VM session tokens; `pgrep`, `ss` and `/proc/<pid>/{cmdline,environ}` capture owned-PID identity |
| Clock | The host clock is read-only: the verifier records the observed host UTC instant and never sets it; scenario 12 owns the guest-only midnight lane and scenario 03 the guest reboot lane |
| Guest disks | The persistent artifact parent `$HOME/.cache/cen-native-vm` (owned, non-symlink, mode 0700) that `native-vm-qualification.sh` allocates its `run-*/machine` roots under |

`--certificate <fresh-path>` additionally writes a schema-1 certificate of those observed
identities (paths, bytes, SHA-256, versions, tool paths, unlisted packages, scratch filesystem) and
refuses to overwrite an existing file, so a re-run needs a fresh owned path. Keep it with the run's
other evidence. Without it the verifier prints the same `RESOURCE:` lines and exits. Verified
resource identities are prerequisites, not scenario results: the certificate proves what the lanes
would consume, never that a lane ran.

## Invocation and configuration

```sh
env -u CI tools/moon-local run pipeline:build-portable
mkdir -p var/durability-scratch
SCRATCH_STORE="$PWD/var/durability-scratch" RESTATE_SERVER_BIN=<verified-server-path> \
  env -u CI tools/moon-local run pipeline:durability

SCRATCH_STORE="$PWD/var/durability-scratch" \
  env -u CI tools/moon-local run pipeline:durability -- scenario-01-endpoint-kill
```

Verify the pinned resources and prepare that scratch root first:

```sh
env -u CI tools/moon-local run pipeline:native-resources
```

Choose a fresh owned scratch root per evidence run. The runner exports `SCRATCH_STORE`, `TMPDIR`,
`BINARY`, `SERVE_BINARY`, `RESTATE_BINARY`, `RESTATE_SERVER_BIN`, `CORPUS_FIXTURE`, `ADMIN_PORT`,
`SERVICE_PORT`, `REPO_ROOT`, `BINARY_AVAILABLE` and `RESTATE_AVAILABLE`. Default scratch is under
`/tmp`; override it if that is tmpfs or shared. A scenario may additionally require namespace tools,
mount, curl, jq, timeout or the test harness's own prerequisites.

The release gate passes `ADMIN_PORT=19405` and `SERVICE_PORT=9405` unless the caller supplies
alternatives. It refuses malformed, duplicate, occupied, default-runner (`19095`/`9080`),
live-census (`18095`/`19095`) and shared-model (`15192`) ports before building or starting a
scenario. Each invocation receives a fresh `run-XXXXXXXX` child beneath the owned, non-symlink
`var/durability-scratch` parent; prior run artifacts remain untouched. A skipped scenario still
fails the runner and therefore the release lane. The guard tests verify these preconditions only,
not native durability.

Every seam-dependent host scenario (02, 05, 06 and 11) probes the endpoint named by `SERVE_BINARY`,
which defaults to the plain portable `census-serve`. A release run must point it at the
feature-enabled build, and scenario 04 reads that build from `target/moon-build` for V1 while
`SERVE_BINARY` stays the plain V2:

```sh
env -u CI tools/moon-local run pipeline:build -- --release -p census-service \
  --features native-fault-injection --bin census-serve --example qualification_native_vm
SERVE_BINARY="$PWD/target/moon-build/x86_64-unknown-linux-gnu/release/census-serve" \
  SCRATCH_STORE="$PWD/var/durability-scratch" RESTATE_SERVER_BIN=<verified-server-path> \
  env -u CI tools/moon-local run pipeline:durability
```

Without that endpoint the seam scenarios skip or fail on the missing seam instead of exercising it,
so a release pass exports `SERVE_BINARY` for the whole gate, not only for the durability lane. The
two guest lanes (03, 12) additionally read `CENSUS_NATIVE_VM_HOST` for the feature-built
`qualification_native_vm` example and `CENSUS_NATIVE_VM_CAPTURES` for the live public captures their
natural-midnight phase replays; without that host they resolve the example under the portable target
tree, which `pipeline:build-portable` leaves empty, and fail before booting.

`S04_V1_BINARY` and `S04_V2_BINARY` override scenario 04's pair directly; without the override
scenario 04 takes V1 from the feature build in `target/moon-build` and V2 from the plain portable
build, independent of `SERVE_BINARY`, so exporting the feature build for scenarios 02, 05, 06 and 11
can no longer collapse the pair into one binary. The release gate's durability lane
(`lane_durability` in `tools/gate.sh`) performs that wiring itself: it builds the feature endpoint
and its `qualification_native_vm` example when either is missing, builds the
`crates/census-store/examples/receipt-oracle.rs` reconciliation binary through
`S06_RECEIPT_ORACLE_BINARY` when it is missing, then exports `SERVE_BINARY`, `S04_V1_BINARY`,
`S04_V2_BINARY`, `S06_RECEIPT_ORACLE_BINARY`, `CENSUS_NATIVE_VM_HOST` and
`CENSUS_NATIVE_VM_CAPTURES` for the lane.

## Recording live public captures

The `qualification_native_vm` example ships a `record` mode that performs the two live HTTPS
acquisitions scenarios 03 and 12 require and writes a manifest-bound capture set. It uses the
production `census_crawl::net::Fetcher` with `refresh: true`, refuses cache-served or non-200
outcomes, validates `CollectionDisposition::Complete` on the team index and
`RosterVerdict::Complete` on the roster, and refuses to overwrite an existing manifest.

```sh
env -u CI tools/moon-local run pipeline:build -- --release -p census-service \
  --example qualification_native_vm
target/moon-build/x86_64-unknown-linux-gnu/release/examples/qualification_native_vm \
  record \
  --jurisdiction AL \
  --team-id 38332 \
  --out var/native-vm-captures-$(date -u +%Y%m%d).json \
  --cache var/capture-cache
```

The command prints the absolute manifest path on success. Point `CENSUS_NATIVE_VM_CAPTURES` at
that manifest for the durability lanes. Use a fresh `--out` path per recording; the recorder
refuses to overwrite. The `--cache` directory holds the HTTP cache and payload files; one cache
can serve multiple recording runs as long as each uses a distinct `--out` manifest. The optional
`--source` label is written into the fetcher's source tracking.

## Verifying retained inputs

Run from the repository root; manifest capture paths retain their recorded cwd-relative meaning:

```sh
target/moon-build/x86_64-unknown-linux-gnu/release/examples/qualification_native_vm \
  verify --manifest <capture-manifest>
```

This read-only command reuses preparation's capture/protocol/cohort validation. Optional
`--host-manifest <host-manifest>` additionally verifies the exact distinct qualification,
endpoint and Restate binary roles against their file digests. It opens no census store and
starts no network acquisition or VM. `live_claimed_by_metadata` is not independently witnessed
liveness: fresh acquisition, guest-midnight bookends and journal readback remain scenario duties.


## Current wrapper map

Static implementation inventory, not observed PASS results. Files are `scenario-NN-<name>.sh` in
this directory. A narrower invoked test does not discharge the broader same-number acceptance item.

| # | Name | Current implementation/limit |
|---|---|---|
| 01 | `endpoint-kill` | Runs service `restate_kill_restart` integration tests |
| 02 | `restate-kill-during-fanout` | Feature-enabled endpoint holds a reserved teams source (`jurisdiction:VT:2026-27:2/teams/milesplit`), SIGKILLs native Restate mid-fan-out, restarts from the same base-dir and requires every pre-kill invocation id to survive; the boundary directory must be a process-owned 0700 directory (the script chmods it) and the reservation marker appears within seconds of submission; skips without the feature-enabled endpoint, chromium, curl/python3/ss or free ports |
| 03 | `reboot-with-full-census` | Owned QEMU guest lane: sources `native-vm-qualification.sh` and drives the reboot phase with the staged feature-enabled endpoint and example; requires the VM tooling, images and deployment staging |
| 04 | `rolling-upgrade` | Feature-enabled V1 → plain V2 upgrade with an in-flight paused parent: single-writer handoff, schema gate, pinned replay, one settlement, then the superseded deployment is retired |
| 05 | `http-error-taxonomy` | Plain lane and real-Chromium lane against an owned local HTTP fault server (200, 429 + Retry-After, 500, challenge); requires the lane-origin seam endpoint and chromium |
| 06 | `no-duplicate-evidence` | Kills and restarts the feature-enabled endpoint at reached durable-effect boundaries and requires exactly-once acknowledged observations, receipts and Fjall effects after replay. Each reached kind runs `crates/census-store/examples/receipt-oracle.rs` (`S06_RECEIPT_ORACLE_BINARY`) against the stopped run's store: the oracle re-publishes the marker through its own `deny_unknown_fields` mirror, rejects placeholder identity, checks the sibling `boundary/` provenance, makes a cold `Store::backup` copy and compares marker acknowledgements with the copy's checkpoint, receipt and row digests, fails on duplicate receipt content, on observation rows that no new receipt appends, and on derived staging generations whose rows no publication made current and no reclaim has removed, requires clean integrity and no staged `.pending` batches, and keeps the original store byte-identical as a witness. Its recorded limits: the invocation id is unbound, journal/meta/derived-generation and capture-cache effects are outside the checkpoint, and the 100000-row checkpoint cap refuses very large stores at checkpoint time instead of covering them |
| 07 | `domain-dedup` | Native node + endpoint: two concurrent identical jurisdiction submissions under one explicit idempotency key admit one invocation with one settled outcome; the settle loop waits up to 1800x2s (measured 2026-10-11: a VT sweep on this host reached its terminal state after ~1080s) and records the terminal row in `settle-outcome.json`, where the documented incomplete-source refusal is recorded verbatim rather than converted into a pass — the sweep's completion remains the qualification/national acceptance |
| 08 | `global-budget` | Native CLI origin-budget regression: three independent workflows share one working-directory origin lock under 1000ms pacing, two rivals are refused by owner PID, and the certificate's exact redirect chain, cache facts and pacing are re-checked |
| 09 | `disk-full-fjall` | Private 64-MiB tmpfs with fail-closed cap/private-mount inspection; actual commit ENOSPC, exact acknowledged records/receipts, unchanged replay and new recovered writes; conditional namespace/tool skips |
| 10 | `disk-full-restate` | Owned processes/private bounded mount; demands an actual OS disk-full log and acknowledged-work recovery |
| 11 | `parent-exit` | Real parent HTTP trigger with an owned source reservation stopped mid-work; requires drain/reap accounting, no orphans and durable resumability, using the `ATHLETIC_FAULT_HTTP_EXIT` seam. Resume is accepted on the original invocation's journal growing across repeated inventories (or on completion with success) once no terminal `failure` is recorded; jurisdiction settlement inside the lane's bounded window is checked and echoed as evidence when reached, and its completion remains the qualification/national acceptance |
| 12 | `cross-midnight` | Owned QEMU guest lane: two live HTTPS acquisitions across a natural guest midnight with journal readback; requires the VM tooling and staged feature-enabled artifacts |
| 13 | `ai-review-failures` | Runs `census-review` HTTP transport tests plus the owned fault-proxy review phases; requires the approved local model servers at the two lane targets and skips when either proxy's upstream is unreachable |
| 14 | `seal-refuses` | Empty-store CLI refusal names the jurisdiction, source-object and run-binding items with their exact details; does not exercise every unmet acceptance item |
| 15 | `full-backup-restore` | Runs service `backup_restore` integration tests; not Restate recovery |
| 16 | `golden-census-determinism` | Runs exact `rebuilding_the_fixture_store_reproduces_semantics_not_publication_identity` regression and retains its log; not full frozen real-capture/advice replay |
| 17 | `recovery-tests` | Runs service `recovery` suite; verify the actual reached batch window against the catalog |
| 18 | `run-binding` | Native node + endpoint: admits one run through `Census.bind_run`, refuses a second with both runs named, refuses a `Census/seal` request naming another cohort or another run's journal, reads the binding back after an endpoint restart, and checks the offline ladder names the unmeasured binding; no crash injected and no full census run |

Scenario 10 also uses `restate-enospc-probe.sh`; its current helper prerequisites include Python.
That existing harness implementation is not permission to implement census pipeline logic in Python.
Do not convert a missing tool or inaccessible fault seam into simulated success.

Handler calls from these wrappers follow the ingress contract: handlers with no input (`state`,
`progress`, `inspection`) are invoked with `GET` and no body or `content-type`, because a JSON
content-type on them is rejected with `400 input validation error: Expected body and content-type
to be empty`; handlers that take input (`bind_run`, source submissions) use `POST` with a JSON body.

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
