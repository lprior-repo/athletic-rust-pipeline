# Operations runbook

`restate-server` owns invocation journals/ingress/admin; `census-serve` owns the Fjall store and
endpoint; `census-service` is the CLI. [Lifecycle](deployment-lifecycle.md) owns deployment and
handoff. This runbook owns project handler/retry procedures; [durable execution](restate/durable-execution.md)
is external vendor background. [CLI reference](../crates/census-service/README.md) owns command groups.

## Run selection and inspection

For a fresh census, bind a new named store to an unused durable namespace/revision under
[ADR-013](adr/ADR-013-fresh-national-source-census.md). Preserved historical stores, receipts and seals
do not certify it. A new directory with old Restate keys can replay old work; a changed revision must
not be used merely to reset an exhausted attempt budget.

The default store root is `var/census-service`; historical campaign evidence includes
`var/midwest-census`. Do not assume either is the fresh run. Choose and record the actual root,
namespace, scope, season and revision before acquisition. Never expose private admissions data.

```sh
cargo xtask census-status
cargo xtask coverage
census-service open-work
census-service national-report --help
```

Serving commands default to loopback ingress `http://127.0.0.1:18095/` and open no second store.
The ingress client rewrites the SDK's synchronous `/restate/call/...` and `/restate/invoke/...`
routes onto the served `/<service>[/<key>]/<handler>` path, and the SDK's asynchronous
`/restate/send/...` route onto the served `/<service>[/<key>]/<handler>/send` suffix, so `national`
and `jurisdiction` submit through the CLI. The SDK's invocation-handle routes have no ingress
equivalent on this server version, so a submission that returns a handle can be submitted with
`--detach` and inspected through `Census/open_work`, `JurisdictionCensus/<key>/state` or the admin
API; the same run can also be submitted on the workflow route itself:

```sh
curl -X POST http://127.0.0.1:18095/NationalCensus/national:<year>:<plan>:<revision>/run \
  -H 'content-type: application/json' -d '{"source_parallelism":4}'
```
Explicit `--store <dir>` selects an offline route where supported and requires the endpoint owner
stopped. Service-only commands reject it. The endpoint itself is loopback HTTP/2, conventionally
9080; the Restate admin API is conventionally 19095. Do not expose an unauthenticated endpoint.

Inspecting a run's server-side state needs the admin API's JSON accept header; without it the query
returns a binary body. With the admin API at `http://127.0.0.1:19095/`:

```sh
curl -s http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -H 'accept: application/json' \
  -d '{"query":"SELECT status, COUNT(*) as n FROM sys_invocation GROUP BY status"}'
```

`open-work` is the owed-work view: `jurisdiction sweeps owed` counts sweeps a run still owes, and
`source objects owed` reads `unmeasured` when the run cannot enumerate them. A client-side
observation timeout is not a job failure: `national` observes for `--timeout-seconds` (default
172800) and the run continues either way, so a submission that printed `Terminal error [500]: the
invocation stream was closed after the 'abort timeout' (1h) fired` may have completed server-side —
confirm through the admin API before concluding a jurisdiction failed. A submission whose run
identity already exists is deduplicated, not restarted: reattaching returns the existing invocation,
and starting a different run needs a revision bump, which remains the documented way to invalidate
completed work and must not be used merely to reset an exhausted attempt budget.

`teams`, `meets` and `collect` without `--states`/`--all-states` default to Wisconsin qualification.
`--all-states` selects the 49-jurisdiction `CENSUS_SCOPE`, not all 51 modeled locations. Provider
restriction defaults differ; consult that command's help. `collect --school-year` is the academic
starting year, not the graduation cohort: 2026 denotes 2026–2027. Workbook/bests use `--grad-year`.

## Acquisition and incremental work

Use the serving jurisdiction/national path for durable acquisition. Registry applicability, planned
source units and actually wired stage handlers are different things; inspect refused/unfinished
units rather than infer coverage from a registered name. A roster or meet page is shared evidence,
not per-athlete work. Respect source policy and shared physical admission, not a per-workflow budget.

Current public coach CSV import and some batch derivations remain separate command surfaces; the
active plan requires their coherent durable integration. Weekly staging units are operational legacy,
not a second canonical census or proof of a fresh source-to-output path. Do not run old offline
collection chains against the serving store or copy their output into a new run as its population.

An index rebuild and review application can change review-case populations. Retained verdicts and
unresolved candidates must reconcile; never choose a smaller intermediate case table to obtain a
seal. Reusing cached bytes does not advance the actual acquisition timestamp or establish freshness.

Request pacing has two independent dials, and both default to the polite setting:

- Per-source spacing. Each host (or, for a single-lane source family, the family as a whole) leaves
  its configured delay between turns. `--delay-ms` overrides the default host delay; a source's own
  robots `crawl-delay` raises it, an authorized host never drops below 500 ms, and a family budget in
  `default_family_delays` holds across that family's hosts. Raising concurrency never shortens this.
- `--source-parallelism <N>`. Above the default of 1, a source family admits N requests in flight and
  each of its hosts carries its own spacing slot, so a state whose work spans several subdomains of
  one family no longer serializes them behind a single family turn. At 1 the family keeps one slot and
  one turn at a time, which is the historic behavior. Hosts outside a registered family always keep
  their own slot. Use it to raise a family's ceiling, not its rate: it cannot make any single host
  faster than its spacing, and it never overrides robots or an access condition.

Aggregate throughput also follows the endpoint's `--max-concurrent` (handlers executing at once) and
the per-request `--concurrency` (in-state tasks). Every jurisdiction paces its own hosts, so raising
them parallelizes different hosts rather than shortening any one host's spacing. Measure the result:
count cache entries written per minute under the store's `http/` directory, since a cache hit adds no
file.

## Browser lane

Start the endpoint with a dedicated `--browser-profile <dir>` and an allowed browser executable.
Use an absolute path: the lane's settings validation rejects a relative profile directory or
executable, and binaries built before this tree report that rejection as an invalid tab count.
Use the headed profile required for Athletic.net; a headless flag's existence is not an authorization
to bypass that source policy. Then use ingress:

```sh
census-service browser-session start
census-service browser-session status
census-service browser-session stop
census-service browser-session fetch --url <url> --semantic-url <citation>
```

The endpoint owns the profile and default object key `profile-0`. A missing lane is an explicit
source refusal, not an empty page. `HumanRequired` stops affected admission until the operator
resolves access in that profile. Do not rotate egress, spoof identity, replay challenge cookies or
switch transport to clear it. Other permitted sources may continue.

Lane presence is part of the current plan fingerprint. Adding/removing it under a previously
recorded plan can be refused; preserve the existing run's obligations and make any capability/run
transition explicit. Do not silently resubmit every failed operation under new keys.
[Architecture §6](../ARCHITECTURE.md#6-admission-and-browser-state-10-26-28) owns admission requirements.

## Shutdown and diagnostics

SIGTERM first stops the endpoint listener and waits for SDK connection shutdown, then closes region
admission, drains owned work and flushes storage. `--drain-timeout` is the async-task grace period,
not a wall-clock bound on shutdown. The shipped unit uses `TimeoutStopSec=infinity` and
`SendSIGKILL=no`: already-started blocking effects must finish before the store can be finalized.
Retain the certificate:

```text
drained: accepted=<n> completed=<n> cancelled=<n> timed_out=<n> aborted=<n> panicked=<n>
```

A stop request is broadcast to cooperative region tasks before the drain begins. A non-zero
`timed_out` counts tasks still owned at the grace deadline; it overlaps terminal outcomes and is not
an additional completion bucket. Async survivors are aborted and reaped; started blocking effects
are awaited even beyond the deadline. Successful drain reports satisfy
`accepted = completed + cancelled + aborted + panicked`, with no remaining owned tasks.
Cancelled drain callers retain the region for a subsequent drain; admission never reopens.

The endpoint's own shutdown is the SDK's: `restate-sdk` waits up to ten seconds for open connections
to close before region admission closes. This ordering prevents shutdown refusal from becoming a
terminal result of an invocation's unstarted effect. Reconcile persisted unfinished work; a
deadline, abort or panic is not normal success to hide. A blocking effect that never returns prevents
clean shutdown; diagnose it rather than killing it and claiming a drain certificate. Confirm process
exit/store lock release before another owner starts. `RUST_LOG` controls structured diagnostics.

Use Restate admin queries for invocation state without opening Fjall:

```sh
curl -sS -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -H 'accept: application/json' -d '{"query":"SELECT id, status, target FROM sys_invocation"}'
curl -sS -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -H 'accept: application/json' -d '{"query":"SELECT service_key FROM state WHERE service_name = '\''Ingest'\'' ORDER BY service_key"}'
```

Paused invocations can retain object ownership and block later submissions. Identify the actual
failure, deployment and durable key before operator cancellation; do not mass-kill or erase journals.
`JurisdictionCensus` currently kills on retry exhaustion, while other definitions may pause.

## Export, verification and sealing

Until a recoverable shared input generation exists, stop acquisition before projection and keep it
stopped through independent readback/seal. Quiescence is an operational precaution, not atomic bundle
publication. Rebuild derived state before materializing its snapshots: `index` precedes `consolidate`,
then report/bests/workbook. Review decisions, if required, must be applied before the final generation.
Use actual CLI help for the chosen serving/offline route; do not mix stores or run revisions.
An explicit `index` invocation always re-derives mutable projections. Old input receipts do not
certify that those outputs still exist or reflect current rules; historical receipts are retained
but no longer skip this pass. This repair does not make the separately replaced tables one atomic
publication generation.

```sh
cargo xtask export --ingress --out out/census.xlsx --grad-year 2027
census-service --store <stopped-store> verify --workbook <exact-workbook>
census-service seal --ingress http://127.0.0.1:18095/ --workbook <exact-workbook> --write
```

For the offline verifier, stop the serving owner first, then restart the same compatible deployment
before the serving seal call. Name the exact artifact instead of trusting the newest `*.xlsx`.
Supply the actual `--season`, `--revision` and all applicable repeated `--source-object <key>` values;
defaults or an incomplete object list are not the run's denominator.

Current seal code checks phase/artifact prerequisites, workbook hash and selected Athletes/Coverage/
Run Metrics data. It does **not** independently reconcile all performance cells or prove a same-input
atomic workbook/sidecar bundle. Current open-work heuristics are narrower than the complete source
obligation contract. Offline missing jurisdiction/source-object measurements remain unknown and
must refuse, not default to zero. A separate verifier's pass proves only its exercised checks.

`--write` records `out/seal.json`. If an old seal shape cannot decode, preserve it and diagnose an
explicit version/migration decision; do not edit away required evidence or quietly ignore it. Old
seals do not certify changed evidence. The complete publication oracle and acceptance requirements
live in [the active plan](NATIONAL-CENSUS-PLAN.md), not in a command's exit status alone.

## Backup, deployment and release

Use [cold backup/restore](FJALL_BACKUP.md); preserve referenced raw captures and the separately owned
Restate durable directory. No live directory copy, cache-only backup or removed `import-legacy`
command can substitute. Deployment unit/config ownership is in [lifecycle](deployment-lifecycle.md).

[tools/gate.sh](../tools/gate.sh) owns gates; [VERIFICATION-EVIDENCE.md](VERIFICATION-EVIDENCE.md) owns dated
incidents and executed results. Report only the declared run/scope and observed verification, with
terminal access gaps, unresolved review and unfinished discovery visible.
