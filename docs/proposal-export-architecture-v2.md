# Export architecture — historical proposal record

This file is not an architecture owner. The accepted decision and its binding consequences are
[ADR-019](adr/ADR-019-export-derivation-single-home.md); current APIs are described by the crate
reference and [ARCHITECTURE.md](../ARCHITECTURE.md). What remains here is the measurement that
motivated the change, kept because the delivery loop cites it.

## Measured problem (before the change)

The export path decoded the Fjall store once per consumer and re-applied filtering in each:

| Consumer | Scans at the time |
|---|---|
| `build_census(store, Scope::Core)` / `(store, Scope::AllSources)` | full scan per call |
| `bests::build(store, …)` | athletes, schools, meets, events, teams |
| recruiting dataset load | athletes, schools, coaches, events, performances |
| performance rows | performances, plus lookups |
| coverage/projection | athletes, coaches, schools, meets, events |

The corpus then held ~3.07 M athletes and ~310 K performances; debug-build `serde_json` decode cost
put repeated decoding of that corpus in the tens of minutes for a full export, and every repeated
scan was also a second place where scope, cohort and evidence rules could drift.

## What replaced it

One `ExportDataset::load(store)` per run, one `Derivation::of(&dataset, scope, grad_year)` per
(scope, cohort), and consumers that take those by reference. Consumers do not repeat derivation
filtering, except for Coverage's documented administrative jurisdiction view; no consumer scans the
store for rows. See [ADR-019](adr/ADR-019-export-derivation-single-home.md) for the decision and
[xtask measurement procedures](../xtask/README.md#measurement-and-fixture-boundaries) for measurement,
with dated results in [VERIFICATION-EVIDENCE.md](VERIFICATION-EVIDENCE.md).
