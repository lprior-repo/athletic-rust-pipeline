use crate::report::{ReportResult, Scope};
use crate::workbook::cells::{row, Cell};

mod census;
mod counters;
mod reconcile;
mod store;

use super::{PerformanceSheetPopulation, RunFacts};

pub(super) const METRIC_WIDTHS: [u16; 4] = [40, 18, 18, 12];

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
    cells.push(row!(
        "Contact assessment school year",
        Cell::text(facts.school_year.short())
    ));
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
