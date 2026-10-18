use super::super::super::DocumentaryParticipationRow;
use super::super::PinnedParticipationRow;
use crate::export::ExportDataset;
use crate::grouping::Admission;
use crate::report::ReportResult;
use census_domain::model::PerformanceId;

pub(super) fn collision(
    dataset: &ExportDataset,
    rows: &std::collections::BTreeMap<PerformanceId, DocumentaryParticipationRow>,
    id: &PerformanceId,
    budget: &mut Admission,
) -> ReportResult<()> {
    for performance in &dataset.performances {
        budget.charge(0, 1)?;
        if &performance.id == id {
            return Err(super::super::invalid(
                "documentary performance collides with structured source",
            )
            .into());
        }
    }
    duplicate(rows, id, budget)
}

pub(super) fn extended(
    dataset: &ExportDataset,
    rows: &std::collections::BTreeMap<PerformanceId, DocumentaryParticipationRow>,
    mapped: &std::collections::BTreeMap<PerformanceId, PinnedParticipationRow>,
    id: &PerformanceId,
    budget: &mut Admission,
) -> ReportResult<()> {
    collision(dataset, rows, id, budget)?;
    for prior in mapped.keys() {
        budget.charge(0, 1)?;
        if prior == id {
            return Err(super::super::invalid(
                "documentary performance has duplicate or conflicting source observations",
            )
            .into());
        }
    }
    Ok(())
}

pub(super) fn mapped_duplicate(
    rows: &std::collections::BTreeMap<PerformanceId, DocumentaryParticipationRow>,
    mapped: &std::collections::BTreeMap<PerformanceId, PinnedParticipationRow>,
    id: &PerformanceId,
    budget: &mut Admission,
) -> ReportResult<()> {
    duplicate(rows, id, budget)?;
    for prior in mapped.keys() {
        budget.charge(0, 1)?;
        if prior == id {
            return Err(super::super::invalid(
                "documentary performance has duplicate or conflicting source observations",
            )
            .into());
        }
    }
    Ok(())
}

fn duplicate(
    rows: &std::collections::BTreeMap<PerformanceId, DocumentaryParticipationRow>,
    id: &PerformanceId,
    budget: &mut Admission,
) -> ReportResult<()> {
    for prior in rows.keys() {
        budget.charge(0, 1)?;
        if prior == id {
            return Err(super::super::invalid(
                "documentary performance has duplicate or conflicting source observations",
            )
            .into());
        }
    }
    Ok(())
}
pub(super) fn admit(
    dataset: &ExportDataset,
    rows: &mut std::collections::BTreeMap<PerformanceId, DocumentaryParticipationRow>,
    row: DocumentaryParticipationRow,
    budget: &mut Admission,
) -> ReportResult<()> {
    collision(dataset, rows, row.performance_id(), budget)?;
    let search = rows
        .len()
        .checked_add(1)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    let owner = std::mem::size_of::<(PerformanceId, DocumentaryParticipationRow)>()
        .checked_add(row.performance_id().as_str().len())
        .ok_or(census_store::StoreError::CounterOverflow)?;
    budget.charge(owner, search)?;
    rows.insert(row.performance_id().clone(), row);
    Ok(())
}
