# census-service — application entry points

This crate composes the census CLI, native Restate endpoint and supervised runtime.
[ARCHITECTURE.md](../../ARCHITECTURE.md) owns mission/crate boundaries;
[RESTATE_WORKFLOWS.md](../../RESTATE_WORKFLOWS.md) owns handler schemas and durable keys.
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
| Store maintenance | `fjall-stats`, `store-integrity`, `store-backup`, `store-restore` | Inspect or safely back up/restore owned storage |
| Public coach research | `import-coaches`, `merge-coaches`, `verify-coaches` | Research CSV handling and source-backed contact verification |
| Research artifacts | `qa-reports`, `export-data`, `school-names`, `census-doc` | Research report/CSV helpers; not a second product workflow |

Pipeline commands with serving/offline routing default to Restate ingress. An explicit `--store`
selects in-process access where supported and is rejected by service-only commands; it cannot be
combined with `--ingress`. Offline default storage is `var/census-service`.

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

Source policy is binding regardless of available CLI flags. The existing `--authorized-host` option
can alter robots handling; its presence is not permission to bypass access policy. Do not use it to
work around denial or challenges. Identity/review, cohort, contacts and PR semantics live in
[DOMAIN.md](../../DOMAIN.md), not command-specific variants.

Current workbook generation and seal checks are not proof of a recoverable same-snapshot atomic
bundle or full record-level reconciliation. The required output and oracle are in
[the delivery plan](../../docs/NATIONAL-CENSUS-PLAN.md). A successful command only establishes the
behavior actually observed and verified.

## Development and evidence

Source qualification belongs in [SOURCE_ADAPTER_GUIDE.md](../../SOURCE_ADAPTER_GUIDE.md),
gates in [TESTING.md](../../TESTING.md), benchmarks in [PERFORMANCE.md](../../PERFORMANCE.md),
and dated ingestion/recovery results in [VERIFICATION-EVIDENCE.md](../../docs/VERIFICATION-EVIDENCE.md).
The previous crate-local schema, workflow, adapter and measurement tables were consolidated into
those owners; do not reintroduce duplicate inventories here.
