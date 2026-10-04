# census-service — application entry points

This crate composes the census CLI, native Restate endpoint and supervised runtime.
[ARCHITECTURE.md](../../ARCHITECTURE.md) owns mission/crate boundaries;
[durable execution](../../docs/restate/durable-execution.md) owns handler schemas and durable keys.
Do not interpret historical Midwest help text as the fresh census's run scope.

## Binaries

```sh
cargo run --release -p census-service --bin census-service -- --help
cargo run --release -p census-service --bin census-serve -- --help
```

`census-service serve` prints an endpoint command; it does not start one.
[Deployment lifecycle](../../docs/deployment-lifecycle.md) owns startup, registration and shutdown.
The endpoint is loopback-only HTTP/2; use Restate ingress for application submissions, not direct
HTTP/1.1 calls to the endpoint. A headed browser lane is available only when configured at startup.

## Command groups

Use `<command> --help` for exact flags. This index describes the current surface, not a claim that
all commands have identical durable/recovery semantics.

| Group | Commands | Purpose |
|---|---|---|
| Durable run | `national`, `jurisdiction`, `national-report`, `open-work` | Submit or observe existing Restate run/obligation state |
| Browser | `browser-session` | Start/status/stop/fetch through the endpoint-owned headed profile |
| Discovery/acquisition | `fetch`, `sites`, `teams`, `meets`, `collect`, `provider` | Source entry points; capability and route differ by command |
| Derivation | `consolidate`, `index`, `review`, `run` | Materialization, deterministic indexes, local advice and composed cycle |
| Publication | `report`, `bests`, `workbook`, `verify`, `seal` | Reports, compatible bests, workbook, current verifier and seal/refusal |
| Store maintenance | `fjall-stats`, `store-integrity`, `store-backup`, `store-restore`, `repair-retained-marks`, `repair-retained-events` | Inspect, safely back up/restore or append source-backed retained corrections |
| Public coach research | `import-coaches`, `merge-coaches`, `verify-coaches` | Research CSV handling and source-backed contact verification |
| Research artifacts | `qa-reports`, `export-data`, `school-names`, `census-doc`, `school-address`, `school-address-join` | Research report/CSV helpers, the school-directory corpus verb and the corpus-to-store join; not a second product workflow |

Pipeline commands with serving/offline routing default to Restate ingress. An explicit `--store`
selects in-process access where supported and is rejected by service-only commands; it cannot be
combined with `--ingress`. Offline default storage is `var/census-service`. `school-address` is the
one command here that touches no store at all: it reads operator-supplied directory artifacts into
the collapsed school corpus and exports it
([ADR-020](../../docs/adr/ADR-020-school-address-corpus-port.md)). `school-address-join` requires an
explicit `--store` and a stopped owner: it joins that verified corpus into owned postal claims and
the matched CCD website on canonical schools, or reports each school's outcome without changing
anything ([ADR-021](../../docs/adr/ADR-021-school-address-join-durable-stage.md)).

**One owning process per store:** stop `census-serve` before an offline command opens that database.
Prefer [xtask](../../xtask/README.md)'s serving status/coverage/export wrappers while it runs.
Store counts, scoped populations and accepted cohort counts have different meanings; physical
observations are not distinct athletes.

## Operating limits

[OPERATIONS.md](../../docs/OPERATIONS.md) owns runnable procedures and current limitations.
[Backup/restore](../../docs/FJALL_BACKUP.md) owns the cold-copy procedure; live MVCC read snapshots
do not authorize copying an open database. No current `import-legacy` command exists. Preserve old
stores and obey explicit historical schema contracts rather than claiming they all decode or must
be destroyed/rebuilt.

`repair-retained-marks` requires explicit `--store` and defaults to dry-run. With `--apply`, it
normalizes existing MileSplit result-row numeric `a/A` and `h/H` time qualifiers and exact `m/M`
metric units, preserves source owners/IDs/capture dates and appends correction evidence without
deleting original observations. It performs no acquisition, refuses contradictory known timing,
does not infer event identity from metric units and does not round sub-centimetre source marks.
Use the cold-backup/restore
procedure before applying it; [OPERATIONS.md](../../docs/OPERATIONS.md) owns the sequence.

`repair-retained-events` also requires explicit `--store`, a stopped owner and a cold backup.
Its default dry-run lists eligible retained event-kind corrections; `--apply` appends them in
bounded batches. Only recognized labels backed by their matching parsed source evidence and
without retained conflicts qualify. Event IDs, meet/sex/division/round context, source labels and
original observations survive. Unsupported or unbound labels remain unmapped; a second apply is
idempotent. This command neither discovers results nor merges distinct event contexts.

Source policy is binding regardless of available CLI flags. The existing `--authorized-host` option
can alter robots handling; its presence is not permission to bypass access policy. Do not use it to
work around denial or challenges. Identity/review, cohort, contacts and PR semantics live in
[ARCHITECTURE.md](../../ARCHITECTURE.md) §§8–9, not command-specific variants.

Workbook publication now captures a recoverable immutable input, verifies the complete
workbook/sidecar bundle and atomically switches its generation pointer. Standalone verification
does not open the live store; seal uses the same full-record oracle with a source fence.
See [OPERATIONS.md](../../docs/OPERATIONS.md) for publication-directory commands and bounds.
This does not certify national source coverage, unresolved identities or all native fault lanes;
those requirements remain in [the delivery plan](../../docs/NATIONAL-CENSUS-PLAN.md).

## CSV projections

With the store stopped, `census-service --store <root> export-data --data <directory>
--school-year 2026` loads one immutable export dataset and writes the six named CSV products.
Canonical schools, meets and coaches use the full all-sources population; the canonical athlete
and recruiting files select Class of 2027. Athletic.net seed rows retain published provider
identities, not inferred profile URLs.

`recruiting-co2027.csv` includes `athlete_id`, so unresolved same-name candidates remain individually
addressable rather than becoming indistinguishable CSV rows.

Recruiting contacts use the workbook's gender, sport and tenure rules. `identity_status` is the
durable decision status, separate from cohort confidence; unverified candidates are not accepted
distinct athletes. A dated coach source URL and observed date come from the same evidence record;
an identity-only URL fallback has no asserted observation date. A successful CSV export or workbook
verification does not establish national completeness.

## Development and evidence

Source qualification belongs in [xtask](../../xtask/README.md) and the registry's descriptor table,
gates in [tools/gate.sh](../../tools/gate.sh), benchmarks in `cargo xtask perf`,
and dated ingestion/recovery results in [VERIFICATION-EVIDENCE.md](../../docs/VERIFICATION-EVIDENCE.md).
The previous crate-local schema, workflow, adapter and measurement tables were consolidated into
those owners; do not reintroduce duplicate inventories here.
