# Class-of-2027 athletics census

A Rust, public-source-discovered high-school Track & Field / Cross Country census for the 48
contiguous states plus D.C. Restate coordinates durable work, Fjall retains evidence, Rust resolves
identity, and local Qwen advises on ambiguity. Excel is a verified projection, not a seed population.

**Status:** the workspace and substantial source evidence exist; the fresh national delivery is not
certified by historical stores, exports or seals. [The active plan](docs/NATIONAL-CENSUS-PLAN.md)
owns remaining work. [Dated verification evidence](docs/VERIFICATION-EVIDENCE.md) distinguishes
observed results from unverified requirements.

## Documentation map

| Need | Authoritative document |
|---|---|
| Work in this repository | [AGENTS.md](AGENTS.md) |
| Understand scope, boundaries and binding standards | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Understand why a decision was made | [ADR register](docs/adr/README.md) |
| Understand identity, cohort, marks and provenance | [ARCHITECTURE.md](ARCHITECTURE.md) §§8–9 |
| Implement the remaining delivery | [National plan](docs/NATIONAL-CENSUS-PLAN.md) |
| Integrate/qualify a source | [xtask](xtask/README.md), per-source `research/sources/<name>/SOURCE_REPORT.md` |
| Find public source observations and captures | [Source research](research/sources) |
| Understand headed browser acquisition | [Browser lane](ARCHITECTURE.md#6-admission-and-browser-state-10-26-28) |
| Understand persistence and snapshots | [Backup/restore](docs/FJALL_BACKUP.md) |
| Inspect durable handlers and replay boundaries | [Durable execution](docs/restate/durable-execution.md) |
| Run or inspect the census | [Operations](docs/OPERATIONS.md) |
| Deploy, stop or recover services | [Lifecycle](docs/deployment-lifecycle.md) |
| Back up or restore evidence | [Backup/restore](docs/FJALL_BACKUP.md) |
| Run gates, fixtures and fault scenarios | [Gates](tools/gate.sh), [17 fault obligations](docs/NATIONAL-CENSUS-FAULTS.md) |
| Measure performance | [Perf commands](xtask/README.md) |
| Use developer commands | [xtask](xtask/README.md) |
| Use service binaries and CLI | [census-service](crates/census-service/README.md) |

## Safe starting points

```sh
cargo xtask contract
cargo xtask census-status
cargo xtask coverage
```

Status/coverage/export developer commands default to the serving census through loopback ingress.
The explicit `--store` path opens Fjall in-process and requires the serving owner to be stopped;
a lock failure beside a live service is correct behavior. Follow the operations runbook before
starting acquisition, creating a fresh run or exporting.

Do not modify historical stores or private workbooks to exercise examples. Source access remains
bounded and policy-compliant; challenges require a human, and failures never prove athlete absence.
