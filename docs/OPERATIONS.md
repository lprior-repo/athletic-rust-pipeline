# Operations runbook

`restate-server` owns invocation journals/ingress/admin; `census-serve` owns the Fjall store and
endpoint; `census-service` is the CLI. [Lifecycle](deployment-lifecycle.md) owns deployment and
handoff, [workflow catalog](../RESTATE_WORKFLOWS.md) owns handler/retry contracts, and
[CLI reference](../crates/census-service/README.md) owns command groups.

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
Explicit `--store <dir>` selects an offline route where supported and requires the endpoint owner
stopped. Service-only commands reject it. The endpoint itself is loopback HTTP/2, conventionally
9080; the Restate admin API is conventionally 19095. Do not expose an unauthenticated endpoint.

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

## Browser lane

Start the endpoint with a dedicated `--browser-profile <dir>` and an allowed browser executable.
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
[CHROMIUM_DESIGN.md](../CHROMIUM_DESIGN.md) owns pool/admission and response-classification details.

## Shutdown and diagnostics

SIGTERM requests stop-intake, drain/finalize and storage flush. Keep systemd `TimeoutStopSec` above
`--drain-timeout` (the shipped defaults are 60 and 30 seconds). Retain the certificate:

```text
drained: accepted=<n> completed=<n> cancelled=<n> timed_out=<n> aborted=<n> panicked=<n>
```

Reconcile outcomes and persisted unfinished work. An abort/panic is not normal success to hide with
retries; started blocking effects may outlive a cancelled async waiter. Confirm process exit/store
lock release before a new owner starts. `RUST_LOG` controls structured runtime diagnostics.

Use Restate admin queries for invocation state without opening Fjall:

```sh
curl -sS -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -d '{"query":"SELECT id, status, target FROM sys_invocation"}'
curl -sS -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -d '{"query":"SELECT service_key FROM state WHERE service_name = '\''Ingest'\'' ORDER BY service_key"}'
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

[TESTING.md](../TESTING.md) owns gates; [VERIFICATION-EVIDENCE.md](VERIFICATION-EVIDENCE.md) owns dated
incidents and executed results. Report only the declared run/scope and observed verification, with
terminal access gaps, unresolved review and unfinished discovery visible.
