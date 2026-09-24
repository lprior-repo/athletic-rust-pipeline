# ADR-006: Excel is the recruiter query layer; Fjall stays the evidence layer

## Status

Accepted. Implemented: census workbook writer (`crates/census-report/src/workbook/`,
`src/xlsx/**` (historical: root package deleted 2026-09-23)),
best-mark reduction (`crates/census-service/src/bests/`), census report (`.../report/`).

## Context

Two consumers pull on the data model in opposite directions:

* the **recruiter**, who needs to filter athletes by state, school, event, PR and coverage
  status, and to read one row per athlete without writing SQL;
* the **pipeline**, which must retain every observation, every source identity and every
  disagreement, durably, for days-long runs.

Building an interactive query service (or a relational warehouse) to serve the recruiter would
add a second source of truth, a migration surface and an availability dependency, while the
actual query workload is a handful of full exports per census run.

## Decision

* **The workbook is a projection, not the store.** One row per canonical Class-of-2027 athlete
  on `Athletes`, one row per athlete/event on `PRs`, partitioned sheets for
  `Performances_001..N`, plus `Coaches`, `Schools`, `Meets`, `Sources`, `Coverage`, `Conflicts`,
  `Review` and `Run Metrics`.
* **Fjall remains the only source of truth** (ADR-001). Every workbook row must map back to
  canonical stored evidence, and workbook counts must reconcile against the store — a run may
  not be sealed otherwise (§70).
* **The export is deterministic and verifiable.** The same sealed census produces the same
  workbook; a verification pass re-reads the workbook and checks it against the store rather
  than trusting the writer.
* **No interactive query surface is added for this purpose.** If one is ever required, it is
  built as a read-only projection over Fjall, never as a parallel store.

## Consequences

* Recruiters get the artifact they will actually use, with columns designed for filtering rather
  than normalized schema purity.
* Publishes both scopes in one artifact family: the full evidence scope and the core scope
  (Athletic.net and its AthleticLIVE derivative excluded) so an operator can see exactly what
  the census concludes without the platform's own strongest source (see `report/`'s core
  filter).
* Row limits force partitioning (`Performances_00N`) — a deliberate, documented shape rather
  than one giant sheet.
* Excel's own semantics become a correctness surface: duplicate header rows, column order and
  cell typing are tested (a real bug class, e.g. `has_headers(false)` on the bests CSV writer).
