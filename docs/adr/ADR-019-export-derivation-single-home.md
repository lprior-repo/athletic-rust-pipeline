# ADR-019 — Export consumers derive from one dataset and one derivation

Status: accepted. Supersedes the "no filter-at-load / each consumer applies its own scope and cohort
filtering" principle of the export-architecture proposal. Unfinished conversion work belongs to the
[delivery plan](../NATIONAL-CENSUS-PLAN.md), not to this record.

Number amended on 2026-09-30 to remove the duplicate ADR-015 identity; the accepted export decision
is unchanged. ADR-015 remains the prototype acquisition port.

## Context

The export path (census JSON, workbook sheets, best-mark reduction, reconcile queues) decoded the
Fjall store once per consumer, and each consumer re-applied scope, cohort, jurisdiction and
core-evidence filtering to its own scan. The duplicated rules drifted: a Core-scope best row could
carry a non-core result URL, the workbook's Review sheet read persisted verdicts only on the legacy
store path while the dataset path passed an empty list, and two identity projections were built
from different inputs. Export cost and exported population both depended on how many consumers ran
and in which order.

## Decision

`ExportDataset::load(store)` performs one decode of the domain tables. `Derivation::of(&dataset,
scope, grad_year)` is the only place that applies run-jurisdiction, cohort, Core/AllSources and
core-evidence filtering for a run; it owns the filtered schools, athletes, meets, events, coaches,
the `&CanonicalPerformance` row set and the outside-scope populations the census notes report. A run
builds one derivation per (scope, cohort) and threads it into every consumer: `build_census`,
`workbook::build`, `bests::build_from_dataset`, the performance spill, the recruiting projection and
the meta sheets.

Binding consequences:

- No writer opens the store for rows. Export-path store access is `ExportDataset::load` plus store
  metadata (keyspace stats, HTTP cache directory, lineage).
- `Scope::primary_evidence` is the single scope-aware evidence-selection rule. `retain_core` and
  `retain_core_row` remain only for rows a sheet itself must store scope-filtered.
- Coverage is the deliberate exception: `coverage_report` keeps the dataset's unfiltered rows so it
  can publish exclusion populations, and re-applies the jurisdiction, cohort and core-evidence
  predicates inside `report/coverage` instead of consuming a `Derivation`. Its published counters
  must agree with the census, which reads the same dataset.
- `ExportDataset::identities()` is the identity-projection owner; consumers do not rebuild
  `AthleteIdentityIndex` on their own inputs.
- `retained_records`, the census JSON and the workbook take the same derivation inputs, so a store
  reader and a workbook reader cannot be shown different populations.
- `Census.generated_on` and the contact school year come from the run lineage
  (`dataset.lineage.generated_on`), not from the wall clock at write time.

## Difference ledger

Recorded behaviour changes against the superseded per-consumer reads, so a later delta in these
sheets is not mistaken for a regression:

- Schools, Meets, Conflicts, Review and Run Metrics rows carry the scope-filtered, cohort-free
  population the census counters describe, while the Athletes, PR, Coaches and performance sheets
  use the run's cohort derivation. The athlete-scoped Conflicts and Review families (cohort
  evidence, athlete identity, cohort unverified, identity unverified) select the class of 2027
  through a second explicit derivation over the same dataset, so that cohort predicate lives in
  `Derivation::of` and not in a sheet; the school-, meet- and contact-scoped families of those two
  sheets stay cohort-free. `--grad-year` trims the cohort sheets without moving the Run Metrics
  reconciliation off the census population it is compared against.
- Contacts and the Coaches sheet read per-observation coach rows
  (`ExportDataset::coach_observations`) so a mailbox keeps the tenure evidence of the observation
  that carried it; the census and the meta sheets count merged canonical coaches. The recruiting
  audit's `coach_rows` is the observation count and `contact_conflicts` the observation-level
  conflicts.
- A performance whose athlete has no canonical row is kept, with blank athlete, meet and event
  names, rather than dropped as an unjoinable row; a performance whose athlete row exists but is out
  of run scope or out of cohort is dropped. A blank name always means "no canonical row joined",
  never a raw id: the id has its own column.
- Coach and athlete jurisdiction resolves through the school-state map, so a row whose school id is
  absent from the Schools table lands in the unplaced bucket instead of being dropped.

The 2026-10-01 integration advances publication policy to revision 3. Accepted aliases collapse to
their canonical population across recruiting, PR support and coverage; postal fields retain exact
school/provider/capture ownership across supported affiliations; summary cells admit every whole
context that fits before declaring actual overflow. These change the immutable publication
projection, even when source rows are unchanged. Policy-1/2 frozen inputs and generations remain
historical evidence and are refused by the current verifier; they are not rewritten or silently
reused under policy 3. Export schema revision 1 remains compatible with the additive, empty-default
school postal representation.

The 2026-10-02 integration advances publication policy to revision 4. PR candidates must have a
mark compatible with their event; combined events accept scores, not times or distances. Result
conflicts require contradictory marks in the same known meet/event/date/round/heat context, rather
than matching meet names or collapsing separate rounds and heats. Policy-1/2/3 frozen inputs and
published generations remain historical evidence and are refused by the current verifier. They are
not rewritten or silently reused under policy 4; schema revision remains 1.

The subsequent 2026-10-02 cohort cutover advances publication policy to revision 5. Directly
published graduation-year claims participate in the same domain cohort confidence/conflict rules
as supported grade/year observations, without manufacturing grades. Coverage, review, identity
eligibility and recruiting publication consume that shared assessment. Policy-1/2/3/4 frozen
inputs and published generations remain historical evidence and are refused by the current
verifier; schema revision remains 1 for the additive empty-default canonical cohort field.

## Consequences

Scope, cohort and evidence semantics are the derivation's, not each sheet's; a sheet that wants a
different cohort asks for a second, explicit derivation. Cost becomes one decode per run plus one
filter pass per derivation, measured per [xtask procedures](../../xtask/README.md#measurement-and-fixture-boundaries); the extra filter
pass is O(rows) over borrowed rows and does not re-decode. Fixtures and tests build derivations from
a dataset instead of asserting on store states that bypass the derivation.
