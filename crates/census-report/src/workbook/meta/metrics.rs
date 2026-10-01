use crate::report::{ReportResult, Scope};
use crate::workbook::cells::{row, Cell};

mod census;
mod counters;
mod reconcile;
mod store;

use super::RunFacts;

pub(super) const METRIC_WIDTHS: [u16; 4] = [40, 18, 18, 12];

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
    cells.push(row!("Workbook scope", facts.recruiting.scope().as_str()));
    cells.push(row!(
        "Contact assessment school year",
        Cell::text(facts.school_year.short())
    ));
    cells.push(row!("Census scopes published", "core + all sources"));
    cells.push(row!(
        "Recruiting athletes",
        Cell::number(facts.recruiting.athletes().len())?
    ));
    cells.push(row!(
        "Best-mark rows reduced",
        Cell::number(facts.bests.len())?
    ));
    cells.push(row!("Cohort behind the counters", "class of 2027"));
    cells.push(row!());
    cells.extend(store::store_counters(facts.store)?);
    cells.push(row!());
    cells.extend(store::cache_block(facts.store)?);
    cells.push(row!());
    cells.extend(census::scope_counters(facts.core, facts.all_sources)?);
    cells.push(row!());
    cells.extend(census::method_notes(facts.core));
    cells.push(row!());
    let census = match facts.population.scope() {
        Scope::Core => facts.core,
        Scope::AllSources => facts.all_sources,
    };
    cells.extend(reconcile::reconciliation(rows, conflicts, census)?);
    Ok(cells)
}
