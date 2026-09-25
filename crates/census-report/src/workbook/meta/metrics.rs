//! The run-metrics sheet: what the store holds, what the run already fetched, both census scopes'
//! headline counters, and the block that reconciles every row-level sheet tally against the census
//! counter it must equal.
//!
//! The reconciliation is the point of the sheet. Each of the other meta sheets counts what its rows
//! are; the census document counts the same tables its own way. Both numbers are printed side by
//! side with a `reconciled`/`DIFFERS` status, so a workbook that disagrees with `report.json` names
//! the counter instead of leaving an operator to diff two artifacts. `DIFFERS` is reported, not
//! raised: the store is append-only and a collection process may write between the census and this
//! scan, which is exactly the drift a reader has to see.

use crate::report::{ReportResult, Scope};
use crate::workbook::cells::{row, Cell};

mod census;
mod counters;
mod reconcile;
mod store;

use super::{PerformanceSheetPopulation, RunFacts};

/// Widths for the metric blocks.
pub(super) const METRIC_WIDTHS: [u16; 4] = [40, 18, 18, 12];

/// The performance sheet's population declaration.
///
/// States the cohort, scope, athlete count, row count, and the rule that earlier-season and
/// out-of-state performances for cohort athletes are included by design.
pub(super) fn perf_population_block(
    pop: &PerformanceSheetPopulation,
) -> ReportResult<Vec<Vec<Cell>>> {
    let cohort = pop
        .cohort_year
        .map(|y| format!("class of {y}"))
        .unwrap_or_else(|| "all cohorts".to_string());
    let scope_str = match pop.scope {
        crate::report::Scope::Core => "core",
        crate::report::Scope::AllSources => "all sources",
    };
    let mut cells = vec![row!("Performance sheet population")];
    cells.push(row!("Cohort", Cell::text(cohort)));
    cells.push(row!("Scope", Cell::text(scope_str)));
    cells.push(row!("Cohort athletes", Cell::number(pop.cohort_athletes)?));
    cells.push(row!("Performance rows", Cell::number(pop.total_rows)?));
    cells.push(row!(
        "Design note",
        "Earlier-season and out-of-state performances for cohort athletes are included on purpose;          the row count reconciles with the seal because it counts the cohort, not the calendar year."
    ));
    Ok(cells)
}

/// The run's counters, in blocks: run identity, store counters, HTTP cache, both census scopes, and
/// the reconciliation of the row-level sheets against the census.
pub(super) fn metrics_sheet(
    facts: &RunFacts<'_>,
    rows: &super::StoreRows,
    conflicts: &[super::Family],
) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!("Run metric", "Value")];
    cells.push(row!(
        "Workbook generated on",
        Cell::text(facts.core.generated_on.clone())
    ));
    cells.push(row!("Store", Cell::text(facts.core.store_dir.clone())));
    cells.push(row!("Census scopes published", "core + all sources"));
    cells.push(row!(
        "Best-mark rows reduced",
        Cell::number(facts.bests.len())?
    ));
    cells.push(row!("Cohort behind the counters", "class of 2027"));
    cells.push(row!());
    cells.extend(perf_population_block(&facts.perf_population)?);
    cells.push(row!());
    cells.extend(store::store_counters(facts.store)?);
    cells.push(row!());
    cells.extend(store::cache_block(facts.store)?);
    cells.push(row!());
    cells.extend(census::scope_counters(facts.core, facts.all_sources)?);
    cells.push(row!());
    cells.extend(census::method_notes(facts.core));
    cells.push(row!());
    let census = match facts.scope {
        Scope::Core => facts.core,
        Scope::AllSources => facts.all_sources,
    };
    cells.extend(reconcile::reconciliation(rows, conflicts, census)?);
    Ok(cells)
}
