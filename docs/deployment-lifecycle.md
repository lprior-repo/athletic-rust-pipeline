# Native deployment lifecycle

This document owns binary installation, registration and store handoff. Runtime commands and project
handler policies are in [OPERATIONS.md](OPERATIONS.md); [durable execution](restate/durable-execution.md)
is vendor background. Required V1→V2 fault evidence is [scenario 4](NATIONAL-CENSUS-FAULTS.md#required-scenarios).
An ordinary restart is not proof of an in-flight upgrade.

## Build entrypoint

Moon is the only repository build/test/gate entrypoint. From the repository root:

```sh
env -u CI tools/moon-local run pipeline:build-portable
env -u CI tools/moon-local run pipeline:gate -- --release
```

The build produces `census-service` and `census-serve` under
`target/moon-portable/x86_64-unknown-linux-gnu/release/`. Copy verified artifacts into a new immutable
deployment directory; never rebuild over a running executable. [The developer reference](../xtask/README.md)
owns Moon prerequisites, arguments and cache boundaries. A build is not release certification.

## Invariants

- Registered binaries are immutable artifacts. Bind a release ID to the actual binary/lockfile/
  configuration, not merely an unverified checkout SHA. Do not overwrite a running executable.
- Preserve native Restate's durable directory and deployment/invocation identity. Completed journal
  steps replay; unfinished external effects may repeat and require their idempotency contract.
- One process owns a Fjall root. Two ports or release directories do not permit concurrent owners
  of the same database. Separate empty stores do not provide continuity for one census.
- Do not unregister an old deployment with retained work unless an explicit supported migration or
  operator-approved terminal disposition accounts for every invocation. Forced deletion is not drain.

Keeping a binary immutable avoids accidental code drift but does not itself make replay deterministic.
Time, external responses, unordered iteration, wire shapes and durable call order need stable
journaled/run-bound semantics. A code change can cause RT0016 replay mismatch; not every rebuild
necessarily does, and a paused invocation is not proof of journal corruption.

## Shipped assets

| File | Role |
|---|---|
| `deploy/restate.toml` | Native node config; conventional loopback ingress 18095/admin 19095 |
| `deploy/systemd/restate-server.service` | Node and its separate durable state |
| `deploy/systemd/census-serve@.service` | Endpoint per release, binary under `/opt/athletic-rust-pipeline/releases/<release>/bin/census-serve` |
| `deploy/endpoint.env.example` | Per-release listen address environment |
| `deploy/systemd/census-service-collect.service` and `.timer` | Historical weekly staging collection; not the fresh census's canonical workflow |

The endpoint unit currently uses `/var/lib/census-service/<release>` and a per-release log directory.
That prevents lock sharing by default but does **not** migrate an existing run's evidence. Deliberately
bind the intended run/store before using a new instance; do not silently direct replay into a new
empty release root. The unit grants writable access to its declared data root, so any override must
also reconcile filesystem permissions and sandbox paths.

## Planned same-store handoff

1. Build and verify `census-serve` and the corresponding CLI into a new immutable release directory.
   Record hashes, build/schema/wire versions, intended store/run and rollback artifact. Never use the
   mutable build output as the registered long-lived executable.
2. Choose an unused loopback port and one consistent URI spelling. Inspect registrations and active
   invocations; `localhost` and `127.0.0.1` registrations need not be the same deployment.
3. Stop new run submissions. Let old deployment work drain to a demonstrated terminal boundary,
   or execute a separately qualified in-flight migration. Resolve paused work explicitly; do not
   interpret a missing/unknown active count as zero.
4. SIGTERM the old endpoint, retain its drain certificate and confirm process exit/store lock release.
   Take the required cold backup. Keep old binaries, registration metadata and evidence recoverable.
5. Start the immutable new endpoint against the **intended same store**, with compatible wire/schema
   and run semantics. Confirm endpoint discovery/health before registration. A fresh census instead
   needs its separately authorized new store and unused namespace.
6. Register the new URI with the existing node, then run an isolated canary with observed effects.
   Check that pending/replayed calls reach the intended build/store rather than merely accepting HTTP.
7. Retire the old registration only after its work is accounted for and the replacement is usable.
   Keep rollback artifacts; reuse a port only when no retained deployment needs that address.

A same-store handoff includes downtime between owners. Claiming zero-downtime overlapping writers
would violate the storage contract. Retained in-flight calls may remain pinned to the old deployment;
the sequence above is not an untested automatic reassignment procedure.

## Administration and stuck invocations

Use the loopback admin API to inspect deployments and query `sys_invocation`; examples are in the
operations runbook. Registration uses `POST /deployments` with `{"uri":"http://127.0.0.1:<port>/"}`.
Use the installed server/CLI version's supported inspection and retirement operations.

`JurisdictionCensus` now kills on retry exhaustion; other definitions can pause. A paused virtual
object can block subsequent calls behind its owned key. Preserve the failure, invocation IDs and
receipts before an operator deliberately cancels/kills it. Do not create replacement revisions merely
to evade blocked/exhausted work. Historical API retirement responses and timeout incidents are in
the verification ledger (removed 2026-10-09), not guarantees about every server version.

Store-backed handlers advertise long inactivity/abort timeouts; the browser uses SDK defaults.
Queueing without journal progress consumes that budget. Increasing a timeout is not a substitute
for bounded work, explicit checkpoints or supervised blocking effects.

## Helper limitations

`tools/deploy-lifecycle.sh` exposes `install`, `list` and `describe`, but it is **not a qualified
endpoint deployment tool**. Its current install path is repository `var/releases/<sha>` and it copies
`census-service`, not the required `census-serve`; its help/path claims and environment override do
not match implementation. Its listing can substitute zero for an absent invocation-count field.
Do not use that output as evidence that retirement is safe. These are implementation fixes, not
problems documentation can repair; use the explicit verified handoff above meanwhile.
