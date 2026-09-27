# tools — gate and fault-harness entry points

`tools/gate.sh` is the shell entry point behind `cargo xtask gate`. Rust measurement logic lives in
`xtask`; the wrapper orchestrates real tools rather than implementing census business rules.

| Need | Canonical reference |
|---|---|
| Gate lanes, development/release semantics and required evidence | [TESTING.md](../TESTING.md) |
| Developer command syntax and measurement behavior | [xtask/README.md](../xtask/README.md) |
| Benchmark baseline/comparison procedures | [PERFORMANCE.md](../PERFORMANCE.md) |
| Native fault harness setup/invocation | [durability/README.md](durability/README.md) |
| Seventeen required faults and phase-boundary oracles | [NATIONAL-CENSUS-FAULTS.md](../docs/NATIONAL-CENSUS-FAULTS.md) |
| Actual dated command outcomes | [VERIFICATION-EVIDENCE.md](../docs/VERIFICATION-EVIDENCE.md) |

`quality-baseline.json` is measured debt, not a waiver. Update only for verified burndown or an
explicit owner-approved change; `--allow-increase` is a mechanism, not approval. Supply-chain audit
records remain under `supply-chain/`. License enforcement is excluded by owner direction; advisory,
security and provenance requirements remain in the testing contract.
