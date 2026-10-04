# tools — gate and fault-harness entry points

Moon is the only developer entrypoint: `env -u CI tools/moon-local run pipeline:gate`, or
`env -u CI tools/moon-local run pipeline:gate -- --release`. `tools/gate.sh` is an internal
implementation, not a parallel entrypoint. Rust measurement logic lives in `xtask`; Moon and the
gate wrapper orchestrate real tools rather than implementing census business rules.

| Need | Canonical reference |
|---|---|
| Gate lanes, development/release semantics and required evidence | [gate.sh](gate.sh), [xtask/README.md](../xtask/README.md) |
| Developer command syntax and measurement behavior | [xtask/README.md](../xtask/README.md) |
| Benchmark baseline/comparison procedures | [xtask measurement reference](../xtask/README.md#measurement-and-fixture-boundaries) |
| Native fault harness setup/invocation | [durability/README.md](durability/README.md) |
| Seventeen required faults and phase-boundary oracles | [NATIONAL-CENSUS-FAULTS.md](../docs/NATIONAL-CENSUS-FAULTS.md) |
| Actual dated command outcomes | [VERIFICATION-EVIDENCE.md](../docs/VERIFICATION-EVIDENCE.md) |

`quality-baseline.json` is measured debt, not a waiver. Update only for verified burndown or an
explicit owner-approved change; `--allow-increase` is a mechanism, not approval. The supply-chain
audit records formerly under `supply-chain/` were deleted on 2026-09-27 (commit `1db9157`,
1,991 lines). Cargo-vet is excluded by owner direction, and its obsolete gate invocation has been
removed. Historical vet failures remain historical evidence, not current release blockers.
License enforcement is also excluded; advisory, security and provenance requirements remain.
